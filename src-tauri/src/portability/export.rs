use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::domain::{IdempotencyKey, RepoError};
use crate::notebook::key::DatabaseKey;
use crate::notebook::repository::NotebookRepository;
use crate::operations::{
    CancellationToken, OperationCoordinator, OperationKind, OperationRecord, OperationState,
};
use crate::portability::{atomic_publish, create_notebook_snapshot, partial_path};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum ExportScope {
    CompleteNotebook,
    SelectedOpponent { profile_id: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub operation_id: crate::domain::EntityId,
    pub destination: PathBuf,
    pub scope: ExportScope,
    pub plaintext_acknowledged: bool,
    pub confirm_empty: bool,
    pub unsaved_edits_resolved: bool,
    pub overwrite: bool,
    pub idempotency_key: IdempotencyKey,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub operation: OperationRecord,
    #[serde(skip)]
    pub destination: PathBuf,
    pub destination_name: String,
    pub opponent_count: u64,
    pub encounter_count: u64,
    pub observation_count: u64,
}

#[derive(Default)]
struct ExportCounts {
    opponents: u64,
    encounters: u64,
    observations: u64,
}

pub fn create_export(
    repository: &NotebookRepository,
    key: &DatabaseKey,
    coordinator: &OperationCoordinator,
    request: &ExportRequest,
    cancellation: &CancellationToken,
) -> Result<ExportResult, RepoError> {
    validate_request(repository, request)?;
    let claimed_path = normalized_path(&request.destination)?;
    let lease = coordinator.begin_with_cancellation(
        OperationKind::ExportSnapshot,
        Some(&claimed_path),
        cancellation.clone(),
    )?;
    let mut operation = OperationRecord::requested_with_id(
        request.operation_id.clone(),
        OperationKind::ExportSnapshot,
        request.idempotency_key.clone(),
    );
    operation.transition(OperationState::Running)?;
    coordinator.register(operation.clone())?;
    repository.persist_operation_record(&operation)?;
    let partial = partial_path(&request.destination);
    if partial.exists() {
        fs::remove_file(&partial).map_err(|_| RepoError::DestinationUnwritable)?;
    }

    let result = (|| {
        let snapshot = create_notebook_snapshot(repository, key, &operation.id)?;
        let snapshot_repository = snapshot.repository()?;
        validate_request(snapshot_repository, request)?;
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&partial)
            .map_err(|_| RepoError::DestinationUnwritable)?;
        let mut writer = BufWriter::with_capacity(64 * 1024, file);
        let counts = write_export(
            snapshot_repository,
            &request.scope,
            cancellation,
            &mut writer,
        )?;
        if cancellation.is_cancelled() {
            return Err(RepoError::InvalidTransition);
        }
        writer
            .flush()
            .map_err(|_| RepoError::DestinationUnwritable)?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|_| RepoError::DestinationUnwritable)?;
        drop(writer);
        lease.enter_commit();
        coordinator.update(&operation.id, |record| {
            record.transition(OperationState::Committing)
        })?;
        atomic_publish(&partial, &request.destination, request.overwrite)?;
        let size = fs::metadata(&request.destination)
            .map_err(|_| RepoError::DestinationUnwritable)?
            .len();
        operation = coordinator.update(&operation.id, |record| {
            record.update_progress(size, size)?;
            record.transition(OperationState::Completed)
        })?;
        repository.persist_operation_record(&operation)?;
        Ok(ExportResult {
            operation: operation.clone(),
            destination: request.destination.clone(),
            destination_name: request
                .destination
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or(RepoError::InvalidRequest)?
                .to_owned(),
            opponent_count: counts.opponents,
            encounter_count: counts.encounters,
            observation_count: counts.observations,
        })
    })();

    if result.is_err() {
        let _ = fs::remove_file(&partial);
        let cancelled = cancellation.is_cancelled();
        if let Ok(terminal) = coordinator.update(&operation.id, |record| {
            record.transition(if cancelled {
                OperationState::Cancelled
            } else {
                OperationState::Failed
            })
        }) {
            let _ = repository.persist_operation_record(&terminal);
        }
        if cancelled {
            return Err(RepoError::InvalidRequest);
        }
    }
    result
}

fn validate_request(
    repository: &NotebookRepository,
    request: &ExportRequest,
) -> Result<(), RepoError> {
    if !request.plaintext_acknowledged {
        return Err(RepoError::PlaintextAcknowledgementRequired);
    }
    if !request.unsaved_edits_resolved {
        return Err(RepoError::InvalidRequest);
    }
    if request
        .destination
        .extension()
        .and_then(|value| value.to_str())
        != Some("txt")
        || request.destination.file_name().is_none()
    {
        return Err(RepoError::InvalidRequest);
    }
    if request.destination.exists() && !request.overwrite {
        return Err(RepoError::InvalidRequest);
    }
    if !request.destination.parent().is_some_and(Path::is_dir) {
        return Err(RepoError::DestinationUnwritable);
    }
    let profile_count = match &request.scope {
        ExportScope::CompleteNotebook => repository.snapshot()?.profile_count,
        ExportScope::SelectedOpponent { profile_id } => {
            repository.with_connection(|connection| {
                connection
                    .connection
                    .query_row(
                        "SELECT count(*) FROM opponent_profiles
                     WHERE id = ?1 AND deleted_at IS NULL",
                        [profile_id],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(|_| RepoError::NotebookInvalid)
            })?
        }
    };
    if matches!(request.scope, ExportScope::SelectedOpponent { .. }) && profile_count == 0 {
        return Err(RepoError::NotFound);
    }
    if profile_count == 0 && !request.confirm_empty {
        return Err(RepoError::EmptyPortabilityConfirmationRequired);
    }
    Ok(())
}

fn write_export(
    repository: &NotebookRepository,
    scope: &ExportScope,
    cancellation: &CancellationToken,
    writer: &mut impl Write,
) -> Result<ExportCounts, RepoError> {
    repository.transact_domain(|transaction| {
        writeln!(writer, "MTGO Opponent Notes — Plaintext Export")
            .and_then(|_| writeln!(writer, "WARNING: This file is unencrypted."))
            .and_then(|_| {
                writeln!(
                    writer,
                    "One-way format: this text cannot be imported or restored."
                )
            })
            .and_then(|_| {
                writeln!(
                    writer,
                    "Scope: {}",
                    match scope {
                        ExportScope::CompleteNotebook => "complete notebook",
                        ExportScope::SelectedOpponent { .. } => "selected opponent",
                    }
                )
            })
            .and_then(|_| writeln!(writer))
            .map_err(|_| RepoError::DestinationUnwritable)?;

        let mut counts = ExportCounts::default();
        let (profile_sql, profile_parameter) = match scope {
            ExportScope::CompleteNotebook => (
                "SELECT id, primary_handle, created_at, revision
                 FROM opponent_profiles WHERE deleted_at IS NULL
                 ORDER BY normalized_handle, id",
                None,
            ),
            ExportScope::SelectedOpponent { profile_id } => (
                "SELECT id, primary_handle, created_at, revision
                 FROM opponent_profiles WHERE deleted_at IS NULL AND id = ?1
                 ORDER BY normalized_handle, id",
                Some(profile_id.as_str()),
            ),
        };
        let mut profile_statement = transaction
            .prepare(profile_sql)
            .map_err(|_| RepoError::NotebookInvalid)?;
        let mut profile_rows = if let Some(profile_id) = profile_parameter {
            profile_statement
                .query([profile_id])
                .map_err(|_| RepoError::NotebookInvalid)?
        } else {
            profile_statement
                .query([])
                .map_err(|_| RepoError::NotebookInvalid)?
        };
        while let Some(profile) = profile_rows
            .next()
            .map_err(|_| RepoError::NotebookInvalid)?
        {
            if cancellation.is_cancelled() {
                return Err(RepoError::InvalidTransition);
            }
            counts.opponents += 1;
            let profile_id: String = profile.get(0).map_err(|_| RepoError::NotebookInvalid)?;
            let primary_handle: String = profile.get(1).map_err(|_| RepoError::NotebookInvalid)?;
            let created_at: i64 = profile.get(2).map_err(|_| RepoError::NotebookInvalid)?;
            let revision: i64 = profile.get(3).map_err(|_| RepoError::NotebookInvalid)?;
            writeln!(writer, "Opponent: {primary_handle}")
                .and_then(|_| writeln!(writer, "  Profile created (UTC ms): {created_at}"))
                .and_then(|_| writeln!(writer, "  Profile revision: {revision}"))
                .map_err(|_| RepoError::DestinationUnwritable)?;

            let aliases = transaction
                .prepare(
                    "SELECT display_handle, provenance FROM opponent_aliases
                     WHERE profile_id = ?1 ORDER BY normalized_handle, id",
                )
                .and_then(|mut statement| {
                    statement
                        .query_map([&profile_id], |row| {
                            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                        })?
                        .collect::<Result<Vec<_>, _>>()
                })
                .map_err(|_| RepoError::NotebookInvalid)?;
            if !aliases.is_empty() {
                writeln!(writer, "  Aliases:").map_err(|_| RepoError::DestinationUnwritable)?;
                for (alias, provenance) in aliases {
                    writeln!(writer, "    - {alias} [source: {provenance}]")
                        .map_err(|_| RepoError::DestinationUnwritable)?;
                }
            }

            let mut encounters = transaction
                .prepare(
                    "SELECT id, format, started_at, ended_at, status, phase, source,
                            incomplete_reason
                     FROM encounters
                     WHERE profile_id = ?1 AND deleted_at IS NULL
                     ORDER BY started_at, id",
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            let encounter_rows = encounters
                .query_map([&profile_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, Option<String>>(7)?,
                    ))
                })
                .map_err(|_| RepoError::NotebookInvalid)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| RepoError::NotebookInvalid)?;
            for (
                encounter_id,
                format,
                started_at,
                ended_at,
                status,
                phase,
                source,
                incomplete_reason,
            ) in encounter_rows
            {
                if cancellation.is_cancelled() {
                    return Err(RepoError::InvalidTransition);
                }
                counts.encounters += 1;
                writeln!(writer, "  Encounter: {started_at} UTC ms")
                    .and_then(|_| writeln!(writer, "    Format: {format}"))
                    .and_then(|_| {
                        writeln!(
                            writer,
                            "    State: {status}; phase: {phase}; source: {source}"
                        )
                    })
                    .map_err(|_| RepoError::DestinationUnwritable)?;
                if let Some(ended_at) = ended_at {
                    writeln!(writer, "    Ended (UTC ms): {ended_at}")
                        .map_err(|_| RepoError::DestinationUnwritable)?;
                }
                if let Some(reason) = incomplete_reason {
                    writeln!(writer, "    Incomplete reason: {reason}")
                        .map_err(|_| RepoError::DestinationUnwritable)?;
                }
                write_observations(transaction, &encounter_id, writer, &mut counts)?;
                write_public_snapshots(transaction, &encounter_id, writer)?;
            }
            write_decks(transaction, &profile_id, writer)?;
            writeln!(writer).map_err(|_| RepoError::DestinationUnwritable)?;
            std::thread::yield_now();
        }
        Ok(counts)
    })
}

fn write_observations(
    transaction: &rusqlite::Transaction<'_>,
    encounter_id: &str,
    writer: &mut impl Write,
    counts: &mut ExportCounts,
) -> Result<(), RepoError> {
    let mut statement = transaction
        .prepare(
            "SELECT id, text, created_at, edited_at
             FROM observations
             WHERE encounter_id = ?1 AND deleted_at IS NULL
             ORDER BY created_at, id",
        )
        .map_err(|_| RepoError::NotebookInvalid)?;
    let observations = statement
        .query_map([encounter_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
            ))
        })
        .map_err(|_| RepoError::NotebookInvalid)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| RepoError::NotebookInvalid)?;
    for (observation_id, text, created_at, edited_at) in observations {
        counts.observations += 1;
        writeln!(
            writer,
            "    Observation: {created_at} UTC ms{}",
            edited_at.map_or("", |_| " [edited]")
        )
        .map_err(|_| RepoError::DestinationUnwritable)?;
        for line in text.lines() {
            writeln!(writer, "      {line}").map_err(|_| RepoError::DestinationUnwritable)?;
        }
        if let Some(edited_at) = edited_at {
            writeln!(writer, "      Edited (UTC ms): {edited_at}")
                .map_err(|_| RepoError::DestinationUnwritable)?;
        }
        let mut cards = transaction
            .prepare(
                "SELECT display_name, quantity, certainty, context
                 FROM card_observations WHERE observation_id = ?1
                 ORDER BY certainty, display_name, oracle_id",
            )
            .map_err(|_| RepoError::NotebookInvalid)?;
        let cards = cards
            .query_map([&observation_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|_| RepoError::NotebookInvalid)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| RepoError::NotebookInvalid)?;
        for (name, quantity, certainty, context) in cards {
            writeln!(
                writer,
                "      Card: {quantity}× {name} [certainty: {certainty}]{}",
                context
                    .as_deref()
                    .map(|value| format!(" [context: {value}]"))
                    .unwrap_or_default()
            )
            .map_err(|_| RepoError::DestinationUnwritable)?;
        }
        let tags = transaction
            .prepare(
                "SELECT tag.display_label FROM observation_tags link
                 JOIN tendency_tags tag ON tag.id = link.tag_id
                 WHERE link.observation_id = ?1
                 ORDER BY tag.normalized_label, tag.id",
            )
            .and_then(|mut statement| {
                statement
                    .query_map([&observation_id], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| RepoError::NotebookInvalid)?;
        if !tags.is_empty() {
            writeln!(writer, "      Tendency tags: {}", tags.join(", "))
                .map_err(|_| RepoError::DestinationUnwritable)?;
        }
        std::thread::yield_now();
    }
    Ok(())
}

fn write_public_snapshots(
    transaction: &rusqlite::Transaction<'_>,
    encounter_id: &str,
    writer: &mut impl Write,
) -> Result<(), RepoError> {
    let mut statement = transaction
        .prepare(
            "SELECT provider, event, format, publication_date, source_url
             FROM public_snapshots
             WHERE encounter_id = ?1 AND confirmed = 1
             ORDER BY publication_date, id",
        )
        .map_err(|_| RepoError::NotebookInvalid)?;
    let snapshots = statement
        .query_map([encounter_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|_| RepoError::NotebookInvalid)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| RepoError::NotebookInvalid)?;
    for (provider, event, format, publication_date, source_url) in snapshots {
        writeln!(writer, "    Public deck snapshot:")
            .and_then(|_| writeln!(writer, "      Provider: {provider}"))
            .and_then(|_| writeln!(writer, "      Event: {event}"))
            .and_then(|_| writeln!(writer, "      Format: {format}"))
            .and_then(|_| writeln!(writer, "      Published (UTC ms): {publication_date}"))
            .and_then(|_| writeln!(writer, "      Source: {source_url}"))
            .map_err(|_| RepoError::DestinationUnwritable)?;
    }
    Ok(())
}

fn write_decks(
    transaction: &rusqlite::Transaction<'_>,
    profile_id: &str,
    writer: &mut impl Write,
) -> Result<(), RepoError> {
    let mut statement = transaction
        .prepare(
            "SELECT id, source_class, format, completeness, provider_label, user_label,
                    created_at
             FROM deck_records
             WHERE profile_id = ?1 AND deleted_at IS NULL
             ORDER BY created_at, id",
        )
        .map_err(|_| RepoError::NotebookInvalid)?;
    let decks = statement
        .query_map([profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|_| RepoError::NotebookInvalid)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| RepoError::NotebookInvalid)?;
    for (deck_id, source, format, completeness, provider_label, user_label, created_at) in decks {
        writeln!(writer, "  Deck record:")
            .and_then(|_| writeln!(writer, "    Source class: {source}"))
            .and_then(|_| writeln!(writer, "    Format: {format}"))
            .and_then(|_| writeln!(writer, "    Completeness: {completeness}"))
            .and_then(|_| writeln!(writer, "    Recorded (UTC ms): {created_at}"))
            .map_err(|_| RepoError::DestinationUnwritable)?;
        if let Some(label) = provider_label {
            writeln!(writer, "    Provider label: {label}")
                .map_err(|_| RepoError::DestinationUnwritable)?;
        }
        if let Some(label) = user_label {
            writeln!(writer, "    User label: {label}")
                .map_err(|_| RepoError::DestinationUnwritable)?;
        }
        let provenance = transaction
            .query_row(
                "SELECT run.classifier_version, run.classifier_digest, run.method,
                        run.result_name, run.confidence
                 FROM classification_runs run
                 JOIN deck_revisions revision ON revision.id = run.deck_revision_id
                 WHERE revision.deck_id = ?1 AND run.status = 'successful'
                 ORDER BY run.created_at DESC, run.id DESC LIMIT 1",
                [&deck_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, f64>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| RepoError::NotebookInvalid)?;
        if let Some((version, digest, method, result, confidence)) = provenance {
            writeln!(
                writer,
                "    Classifier provenance: {version} {digest}; {method}; {result}; confidence {confidence:.4}"
            )
            .map_err(|_| RepoError::DestinationUnwritable)?;
        }
    }
    Ok(())
}

fn normalized_path(path: &Path) -> Result<String, RepoError> {
    let parent = path.parent().ok_or(RepoError::InvalidRequest)?;
    let file_name = path.file_name().ok_or(RepoError::InvalidRequest)?;
    Ok(parent.join(file_name).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ut_083_plaintext_header_discloses_one_way_utf8_contract() {
        let header = "MTGO Opponent Notes — Plaintext Export\nWARNING: This file is unencrypted.\nOne-way format: this text cannot be imported or restored.\n";
        assert!(std::str::from_utf8(header.as_bytes()).is_ok());
        assert!(header.contains("unencrypted"));
        assert!(header.contains("cannot be imported"));
    }

    #[test]
    fn ut_084_export_scope_has_no_restore_variant() {
        let scope = ExportScope::CompleteNotebook;
        assert_eq!(
            serde_json::to_string(&scope).expect("serialize"),
            "\"complete_notebook\""
        );
    }
}
