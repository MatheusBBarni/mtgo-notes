use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::domain::{EntityId, IdempotencyKey, RepoError, UtcMillis};
use crate::notebook::repository::NotebookRepository;
use crate::operations::{OperationCoordinator, OperationKind, OperationRecord, OperationState};
use crate::services::{contract_token, database_error};

const DEFAULT_UNDO_WINDOW_MS: i64 = 10_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletionEntityType {
    Observation,
    Encounter,
    Profile,
    Notebook,
}

impl DeletionEntityType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Observation => "observation",
            Self::Encounter => "encounter",
            Self::Profile => "profile",
            Self::Notebook => "notebook",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionCounts {
    pub profiles: u64,
    pub aliases: u64,
    pub encounters: u64,
    pub observations: u64,
    pub decks: u64,
    pub public_snapshots: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionPreview {
    pub entity_type: DeletionEntityType,
    pub entity_id: String,
    pub display_name: String,
    pub counts: DeletionCounts,
    pub dependencies: Vec<String>,
    pub confirmation: String,
    pub scope_token: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionResult {
    pub entity_type: DeletionEntityType,
    pub entity_id: String,
    pub requested_at: i64,
    pub undo_deadline: i64,
    pub undo_token: String,
    pub tombstone_state: String,
}

pub struct DeletionService<'a> {
    repository: &'a NotebookRepository,
}

impl<'a> DeletionService<'a> {
    pub fn new(repository: &'a NotebookRepository) -> Self {
        Self { repository }
    }

    pub fn preview(
        &self,
        entity_type: DeletionEntityType,
        entity_id: &str,
    ) -> Result<DeletionPreview, RepoError> {
        self.repository.with_connection(|connection| {
            build_preview(&connection.connection, entity_type, entity_id)
        })
    }

    pub fn request(
        &self,
        preview: &DeletionPreview,
        confirmation: &str,
        idempotency_key: &IdempotencyKey,
    ) -> Result<DeletionResult, RepoError> {
        if confirmation != preview.confirmation {
            return Err(RepoError::ScopeMismatch);
        }
        if contract_token(
            b"mtgo-notes-deletion-scope-v1",
            &preview_without_token(preview),
        )? != preview.scope_token
        {
            return Err(RepoError::ScopeMismatch);
        }
        let now = UtcMillis::now().get();
        let deadline = now
            .checked_add(DEFAULT_UNDO_WINDOW_MS)
            .ok_or(RepoError::InvalidRequest)?;
        if preview.entity_type == DeletionEntityType::Notebook
            && preview.counts == DeletionCounts::default()
        {
            return Ok(DeletionResult {
                entity_type: preview.entity_type,
                entity_id: preview.entity_id.clone(),
                requested_at: now,
                undo_deadline: now,
                undo_token: String::new(),
                tombstone_state: "no_op".to_owned(),
            });
        }
        self.repository.transact_domain(|transaction| {
            if let Some(existing) = transaction
                .query_row(
                    "SELECT result_json FROM operation_records
                     WHERE idempotency_key = ?1 AND state = 'completed'",
                    [idempotency_key.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(database_error)?
            {
                return serde_json::from_str(&existing).map_err(|_| RepoError::NotebookInvalid);
            }
            let live = build_preview(transaction, preview.entity_type, &preview.entity_id)?;
            if live.scope_token != preview.scope_token {
                return Err(RepoError::ScopeMismatch);
            }
            if !live.dependencies.is_empty() {
                return Err(RepoError::OperationBusy);
            }
            let undo_token = EntityId::new().to_string();
            let undo_digest = contract_token(b"mtgo-notes-deletion-undo-v1", &undo_token)?;
            transaction
                .execute(
                    "INSERT INTO deletion_tombstones(
                        entity_type, entity_id, requested_at, effective_at,
                        undo_token_digest, purge_state
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 'pending')
                     ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                        requested_at = excluded.requested_at,
                        effective_at = excluded.effective_at,
                        undo_token_digest = excluded.undo_token_digest
                     WHERE deletion_tombstones.purge_state <> 'purged'",
                    params![
                        preview.entity_type.as_str(),
                        preview.entity_id,
                        now,
                        deadline,
                        undo_digest
                    ],
                )
                .map_err(database_error)?;
            tombstone_scope(
                transaction,
                preview.entity_type,
                &preview.entity_id,
                now,
                deadline,
            )?;
            let result = DeletionResult {
                entity_type: preview.entity_type,
                entity_id: preview.entity_id.clone(),
                requested_at: now,
                undo_deadline: deadline,
                undo_token,
                tombstone_state: "pending".to_owned(),
            };
            transaction
                .execute(
                    "INSERT INTO operation_records(
                        id, kind, idempotency_key, state, requested_at,
                        completed_at, result_json, revision
                     ) VALUES (?1, 'request_deletion', ?2, 'completed', ?3, ?3, ?4, 1)",
                    params![
                        EntityId::new().as_str(),
                        idempotency_key.as_str(),
                        now,
                        serde_json::to_string(&result).map_err(|_| RepoError::NotebookInvalid)?
                    ],
                )
                .map_err(database_error)?;
            Ok(result)
        })
    }

    pub fn undo(
        &self,
        entity_type: DeletionEntityType,
        entity_id: &str,
        undo_token: &str,
        now: UtcMillis,
    ) -> Result<(), RepoError> {
        let digest = contract_token(b"mtgo-notes-deletion-undo-v1", &undo_token)?;
        self.repository.transact_domain(|transaction| {
            let tombstone = transaction
                .query_row(
                    "SELECT requested_at, effective_at, undo_token_digest, purge_state
                     FROM deletion_tombstones
                     WHERE entity_type = ?1 AND entity_id = ?2",
                    params![entity_type.as_str(), entity_id],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(database_error)?
                .ok_or(RepoError::NotFound)?;
            if tombstone.3 == "purged" || now.get() > tombstone.1 || tombstone.2 != digest {
                return Err(RepoError::UndoExpired);
            }
            restore_scope(transaction, entity_type, entity_id, tombstone.0)?;
            transaction
                .execute(
                    "DELETE FROM deletion_tombstones
                     WHERE entity_type = ?1 AND entity_id = ?2",
                    params![entity_type.as_str(), entity_id],
                )
                .map_err(database_error)?;
            Ok(())
        })
    }

    pub fn purge_due(&self, now: UtcMillis) -> Result<usize, RepoError> {
        self.repository.transact_domain(|transaction| {
            let mut statement = transaction
                .prepare(
                    "SELECT entity_type, entity_id
                     FROM deletion_tombstones
                     WHERE purge_state = 'pending' AND effective_at <= ?1
                     ORDER BY effective_at, entity_type, entity_id",
                )
                .map_err(database_error)?;
            let rows = statement
                .query_map([now.get()], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            drop(statement);
            for (entity_type, entity_id) in &rows {
                purge_scope(transaction, parse_entity_type(entity_type)?, entity_id)?;
                transaction
                    .execute(
                        "UPDATE deletion_tombstones SET purge_state = 'purged'
                         WHERE entity_type = ?1 AND entity_id = ?2",
                        params![entity_type, entity_id],
                    )
                    .map_err(database_error)?;
            }
            Ok(rows.len())
        })
    }

    pub fn purge_due_operation(
        &self,
        coordinator: &OperationCoordinator,
        now: UtcMillis,
    ) -> Result<OperationRecord, RepoError> {
        let _lease = coordinator.begin(OperationKind::Purge, None)?;
        let mut operation = OperationRecord::requested(OperationKind::Purge, IdempotencyKey::new());
        coordinator.register(operation.clone())?;
        self.repository.persist_operation_record(&operation)?;
        operation.transition(OperationState::Running)?;
        let total = self.purge_work_units_due(now)?;
        operation.update_progress(0, total)?;
        coordinator.update(&operation.id, |record| {
            *record = operation.clone();
            Ok(())
        })?;
        self.repository.persist_operation_record(&operation)?;

        let purge_result = self.purge_due(now).and_then(|_| {
            self.repository.with_connection(|connection| {
                connection
                    .connection
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                    .map_err(database_error)
            })
        });
        match purge_result {
            Ok(_) => {
                operation.update_progress(total, total)?;
                operation.transition(OperationState::Completed)?;
                coordinator.update(&operation.id, |record| {
                    *record = operation.clone();
                    Ok(())
                })?;
                self.repository.persist_operation_record(&operation)?;
                Ok(operation)
            }
            Err(error) => {
                operation.transition(OperationState::Failed)?;
                let _ = coordinator.update(&operation.id, |record| {
                    *record = operation.clone();
                    Ok(())
                });
                let _ = self.repository.persist_operation_record(&operation);
                Err(error)
            }
        }
    }

    pub fn purge_due_coordinated(
        &self,
        coordinator: &OperationCoordinator,
        now: UtcMillis,
    ) -> Result<usize, RepoError> {
        let operation = self.purge_due_operation(coordinator, now)?;
        usize::try_from(operation.completed).map_err(|_| RepoError::InvalidRequest)
    }

    pub fn is_tombstoned(
        &self,
        entity_type: DeletionEntityType,
        entity_id: &str,
    ) -> Result<bool, RepoError> {
        self.repository.with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM deletion_tombstones
                        WHERE entity_type = ?1 AND entity_id = ?2
                    )",
                    params![entity_type.as_str(), entity_id],
                    |row| row.get::<_, i64>(0),
                )
                .map(|exists| exists == 1)
                .map_err(database_error)
        })
    }

    fn purge_work_units_due(&self, now: UtcMillis) -> Result<u64, RepoError> {
        self.repository.with_connection(|connection| {
            let mut statement = connection
                .connection
                .prepare(
                    "SELECT entity_type, entity_id FROM deletion_tombstones
                     WHERE purge_state = 'pending' AND effective_at <= ?1",
                )
                .map_err(database_error)?;
            let scopes = statement
                .query_map([now.get()], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            drop(statement);
            let mut total = 0_u64;
            for (entity_type, entity_id) in scopes {
                let units = match parse_entity_type(&entity_type)? {
                    DeletionEntityType::Observation => scalar(
                        &connection.connection,
                        "SELECT count(*) FROM observations WHERE id = ?1",
                        &entity_id,
                    )?,
                    DeletionEntityType::Encounter => scalar(
                        &connection.connection,
                        "SELECT count(*) FROM encounters WHERE id = ?1",
                        &entity_id,
                    )?
                    .checked_add(scalar(
                        &connection.connection,
                        "SELECT count(*) FROM observations WHERE encounter_id = ?1",
                        &entity_id,
                    )?)
                    .ok_or(RepoError::InvalidRequest)?,
                    DeletionEntityType::Profile => {
                        profile_work_units(&connection.connection, &entity_id)?
                    }
                    DeletionEntityType::Notebook => notebook_work_units(&connection.connection)?,
                };
                total = total.checked_add(units).ok_or(RepoError::InvalidRequest)?;
            }
            Ok(total)
        })
    }
}

fn preview_without_token(preview: &DeletionPreview) -> DeletionPreview {
    let mut value = preview.clone();
    value.scope_token.clear();
    value
}

fn build_preview(
    connection: &rusqlite::Connection,
    entity_type: DeletionEntityType,
    entity_id: &str,
) -> Result<DeletionPreview, RepoError> {
    let (display_name, counts, dependencies) = match entity_type {
        DeletionEntityType::Observation => {
            let display = connection
                .query_row(
                    "SELECT substr(text, 1, 80) FROM observations
                     WHERE id = ?1 AND deleted_at IS NULL",
                    [entity_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(database_error)?
                .ok_or(RepoError::NotFound)?;
            (
                display,
                DeletionCounts {
                    observations: 1,
                    ..DeletionCounts::default()
                },
                Vec::new(),
            )
        }
        DeletionEntityType::Encounter => {
            let exists = connection
                .query_row(
                    "SELECT format FROM encounters WHERE id = ?1 AND deleted_at IS NULL",
                    [entity_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(database_error)?
                .ok_or(RepoError::NotFound)?;
            let observations = scalar(
                connection,
                "SELECT count(*) FROM observations
                 WHERE encounter_id = ?1 AND deleted_at IS NULL",
                entity_id,
            )?;
            let snapshots = scalar(
                connection,
                "SELECT count(*) FROM public_snapshots WHERE encounter_id = ?1",
                entity_id,
            )?;
            (
                format!("{exists} encounter"),
                DeletionCounts {
                    encounters: 1,
                    observations,
                    public_snapshots: snapshots,
                    ..DeletionCounts::default()
                },
                Vec::new(),
            )
        }
        DeletionEntityType::Profile => {
            let display = connection
                .query_row(
                    "SELECT primary_handle FROM opponent_profiles
                     WHERE id = ?1 AND deleted_at IS NULL",
                    [entity_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(database_error)?
                .ok_or(RepoError::NotFound)?;
            let aliases = scalar(
                connection,
                "SELECT count(*) FROM opponent_aliases WHERE profile_id = ?1",
                entity_id,
            )?;
            let encounters = scalar(
                connection,
                "SELECT count(*) FROM encounters
                 WHERE profile_id = ?1 AND deleted_at IS NULL",
                entity_id,
            )?;
            let observations = connection
                .query_row(
                    "SELECT count(*) FROM observations observation
                     JOIN encounters encounter ON encounter.id = observation.encounter_id
                     WHERE encounter.profile_id = ?1 AND observation.deleted_at IS NULL",
                    [entity_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            let decks = scalar(
                connection,
                "SELECT count(*) FROM deck_records
                 WHERE profile_id = ?1 AND deleted_at IS NULL",
                entity_id,
            )?;
            let snapshots = connection
                .query_row(
                    "SELECT count(*) FROM public_snapshots snapshot
                     JOIN encounters encounter ON encounter.id = snapshot.encounter_id
                     WHERE encounter.profile_id = ?1",
                    [entity_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            let pending_merge = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM profile_merges
                        WHERE state = 'applied'
                          AND (
                            primary_profile_id = ?1
                            OR reassignment_plan_json LIKE '%' || ?1 || '%'
                          )
                    )",
                    [entity_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            (
                display,
                DeletionCounts {
                    profiles: 1,
                    aliases,
                    encounters,
                    observations: u64::try_from(observations)
                        .map_err(|_| RepoError::NotebookInvalid)?,
                    decks,
                    public_snapshots: u64::try_from(snapshots)
                        .map_err(|_| RepoError::NotebookInvalid)?,
                },
                if pending_merge == 1 {
                    vec!["active_profile_merge".to_owned()]
                } else {
                    Vec::new()
                },
            )
        }
        DeletionEntityType::Notebook => {
            if entity_id != "notebook" {
                return Err(RepoError::NotFound);
            }
            let counts = DeletionCounts {
                profiles: scalar_all(
                    connection,
                    "SELECT count(*) FROM opponent_profiles WHERE deleted_at IS NULL",
                )?,
                aliases: scalar_all(connection, "SELECT count(*) FROM opponent_aliases")?,
                encounters: scalar_all(
                    connection,
                    "SELECT count(*) FROM encounters WHERE deleted_at IS NULL",
                )?,
                observations: scalar_all(
                    connection,
                    "SELECT count(*) FROM observations WHERE deleted_at IS NULL",
                )?,
                decks: scalar_all(
                    connection,
                    "SELECT count(*) FROM deck_records WHERE deleted_at IS NULL",
                )?,
                public_snapshots: scalar_all(connection, "SELECT count(*) FROM public_snapshots")?,
            };
            if counts == DeletionCounts::default() {
                return finalize_preview(
                    entity_type,
                    entity_id,
                    "Empty notebook".to_owned(),
                    counts,
                    Vec::new(),
                );
            }
            ("Entire local notebook".to_owned(), counts, Vec::new())
        }
    };
    finalize_preview(entity_type, entity_id, display_name, counts, dependencies)
}

fn finalize_preview(
    entity_type: DeletionEntityType,
    entity_id: &str,
    display_name: String,
    counts: DeletionCounts,
    dependencies: Vec<String>,
) -> Result<DeletionPreview, RepoError> {
    let confirmation = format!("DELETE {} {}", entity_type.as_str(), display_name);
    let mut preview = DeletionPreview {
        entity_type,
        entity_id: entity_id.to_owned(),
        display_name,
        counts,
        dependencies,
        confirmation,
        scope_token: String::new(),
    };
    preview.scope_token = contract_token(
        b"mtgo-notes-deletion-scope-v1",
        &preview_without_token(&preview),
    )?;
    Ok(preview)
}

fn tombstone_scope(
    transaction: &rusqlite::Transaction<'_>,
    entity_type: DeletionEntityType,
    entity_id: &str,
    requested_at: i64,
    undo_deadline: i64,
) -> Result<(), RepoError> {
    match entity_type {
        DeletionEntityType::Observation => {
            changed(transaction.execute(
                "UPDATE observations
                     SET deleted_at = ?1, deletion_deadline = ?2,
                         searchable = 0, revision = revision + 1
                     WHERE id = ?3 AND deleted_at IS NULL",
                params![requested_at, undo_deadline, entity_id],
            ))?;
        }
        DeletionEntityType::Encounter => {
            changed(transaction.execute(
                "UPDATE encounters
                     SET deleted_at = ?1, status = 'deleted', revision = revision + 1
                     WHERE id = ?2 AND deleted_at IS NULL",
                params![requested_at, entity_id],
            ))?;
            transaction
                .execute(
                    "UPDATE observations
                     SET deleted_at = ?1, searchable = 0, revision = revision + 1
                     WHERE encounter_id = ?2 AND deleted_at IS NULL",
                    params![requested_at, entity_id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE deck_records SET deleted_at = ?1, revision = revision + 1
                     WHERE id IN (
                        SELECT revision.deck_id FROM public_snapshots snapshot
                        JOIN deck_revisions revision ON revision.id = snapshot.deck_revision_id
                        WHERE snapshot.encounter_id = ?2
                     ) AND deleted_at IS NULL",
                    params![requested_at, entity_id],
                )
                .map_err(database_error)?;
        }
        DeletionEntityType::Profile => {
            changed(transaction.execute(
                "UPDATE opponent_profiles
                     SET deleted_at = ?1, revision = revision + 1
                     WHERE id = ?2 AND deleted_at IS NULL",
                params![requested_at, entity_id],
            ))?;
            transaction
                .execute(
                    "UPDATE encounters
                     SET deleted_at = ?1, status = 'deleted', revision = revision + 1
                     WHERE profile_id = ?2 AND deleted_at IS NULL",
                    params![requested_at, entity_id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE observations SET deleted_at = ?1, searchable = 0, revision = revision + 1
                     WHERE encounter_id IN (
                        SELECT id FROM encounters WHERE profile_id = ?2
                     ) AND deleted_at IS NULL",
                    params![requested_at, entity_id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE deck_records SET deleted_at = ?1, revision = revision + 1
                     WHERE profile_id = ?2 AND deleted_at IS NULL",
                    params![requested_at, entity_id],
                )
                .map_err(database_error)?;
        }
        DeletionEntityType::Notebook => {
            transaction
                .execute(
                    "UPDATE opponent_profiles SET deleted_at = ?1, revision = revision + 1
                     WHERE deleted_at IS NULL",
                    [requested_at],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE encounters SET deleted_at = ?1, status = 'deleted',
                        revision = revision + 1 WHERE deleted_at IS NULL",
                    [requested_at],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE observations SET deleted_at = ?1, searchable = 0,
                        revision = revision + 1 WHERE deleted_at IS NULL",
                    [requested_at],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE deck_records SET deleted_at = ?1, revision = revision + 1
                     WHERE deleted_at IS NULL",
                    [requested_at],
                )
                .map_err(database_error)?;
        }
    }
    Ok(())
}

fn restore_scope(
    transaction: &rusqlite::Transaction<'_>,
    entity_type: DeletionEntityType,
    entity_id: &str,
    requested_at: i64,
) -> Result<(), RepoError> {
    match entity_type {
        DeletionEntityType::Observation => {
            transaction
                .execute(
                    "UPDATE observations
                     SET deleted_at = NULL, deletion_deadline = NULL,
                         searchable = 1, revision = revision + 1
                     WHERE id = ?1 AND deleted_at = ?2",
                    params![entity_id, requested_at],
                )
                .map_err(database_error)?;
        }
        DeletionEntityType::Encounter => {
            transaction
                .execute(
                    "UPDATE encounters SET deleted_at = NULL, status = 'incomplete',
                        phase = 'incomplete', revision = revision + 1
                     WHERE id = ?1 AND deleted_at = ?2",
                    params![entity_id, requested_at],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE observations SET deleted_at = NULL, searchable = 1,
                        revision = revision + 1
                     WHERE encounter_id = ?1 AND deleted_at = ?2",
                    params![entity_id, requested_at],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE deck_records SET deleted_at = NULL, revision = revision + 1
                     WHERE deleted_at = ?1 AND id IN (
                        SELECT revision.deck_id FROM public_snapshots snapshot
                        JOIN deck_revisions revision ON revision.id = snapshot.deck_revision_id
                        WHERE snapshot.encounter_id = ?2
                     )",
                    params![requested_at, entity_id],
                )
                .map_err(database_error)?;
        }
        DeletionEntityType::Profile => {
            transaction
                .execute(
                    "UPDATE opponent_profiles SET deleted_at = NULL, revision = revision + 1
                     WHERE id = ?1 AND deleted_at = ?2",
                    params![entity_id, requested_at],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE encounters SET deleted_at = NULL, status = 'incomplete',
                        phase = 'incomplete', revision = revision + 1
                     WHERE profile_id = ?1 AND deleted_at = ?2",
                    params![entity_id, requested_at],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE observations SET deleted_at = NULL, searchable = 1,
                        revision = revision + 1
                     WHERE deleted_at = ?1 AND encounter_id IN (
                        SELECT id FROM encounters WHERE profile_id = ?2
                     )",
                    params![requested_at, entity_id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE deck_records SET deleted_at = NULL, revision = revision + 1
                     WHERE profile_id = ?1 AND deleted_at = ?2",
                    params![entity_id, requested_at],
                )
                .map_err(database_error)?;
        }
        DeletionEntityType::Notebook => {
            transaction
                .execute(
                    "UPDATE opponent_profiles SET deleted_at = NULL, revision = revision + 1
                     WHERE deleted_at = ?1",
                    [requested_at],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE encounters SET deleted_at = NULL, status = 'incomplete',
                        phase = 'incomplete', revision = revision + 1
                     WHERE deleted_at = ?1",
                    [requested_at],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE observations SET deleted_at = NULL, searchable = 1,
                        revision = revision + 1 WHERE deleted_at = ?1",
                    [requested_at],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE deck_records SET deleted_at = NULL, revision = revision + 1
                     WHERE deleted_at = ?1",
                    [requested_at],
                )
                .map_err(database_error)?;
        }
    }
    Ok(())
}

fn purge_scope(
    transaction: &rusqlite::Transaction<'_>,
    entity_type: DeletionEntityType,
    entity_id: &str,
) -> Result<(), RepoError> {
    match entity_type {
        DeletionEntityType::Observation => {
            transaction
                .execute("DELETE FROM observations WHERE id = ?1", [entity_id])
                .map_err(database_error)?;
        }
        DeletionEntityType::Encounter => {
            transaction
                .execute("DELETE FROM encounters WHERE id = ?1", [entity_id])
                .map_err(database_error)?;
        }
        DeletionEntityType::Profile => {
            transaction
                .execute(
                    "DELETE FROM profile_merges WHERE primary_profile_id = ?1",
                    [entity_id],
                )
                .map_err(database_error)?;
            transaction
                .execute("DELETE FROM opponent_profiles WHERE id = ?1", [entity_id])
                .map_err(database_error)?;
        }
        DeletionEntityType::Notebook => {
            transaction
                .execute("DELETE FROM profile_merges", [])
                .map_err(database_error)?;
            transaction
                .execute("DELETE FROM opponent_profiles", [])
                .map_err(database_error)?;
        }
    }
    Ok(())
}

fn changed(result: Result<usize, rusqlite::Error>) -> Result<(), RepoError> {
    if result.map_err(database_error)? == 0 {
        Err(RepoError::NotFound)
    } else {
        Ok(())
    }
}

fn scalar(connection: &rusqlite::Connection, sql: &str, value: &str) -> Result<u64, RepoError> {
    let value = connection
        .query_row(sql, [value], |row| row.get::<_, i64>(0))
        .map_err(database_error)?;
    u64::try_from(value).map_err(|_| RepoError::NotebookInvalid)
}

fn scalar_all(connection: &rusqlite::Connection, sql: &str) -> Result<u64, RepoError> {
    let value = connection
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .map_err(database_error)?;
    u64::try_from(value).map_err(|_| RepoError::NotebookInvalid)
}

fn profile_work_units(
    connection: &rusqlite::Connection,
    profile_id: &str,
) -> Result<u64, RepoError> {
    [
        "SELECT count(*) FROM opponent_profiles WHERE id = ?1",
        "SELECT count(*) FROM opponent_aliases WHERE profile_id = ?1",
        "SELECT count(*) FROM encounters WHERE profile_id = ?1",
        "SELECT count(*) FROM observations
         WHERE encounter_id IN (SELECT id FROM encounters WHERE profile_id = ?1)",
        "SELECT count(*) FROM deck_records WHERE profile_id = ?1",
        "SELECT count(*) FROM public_snapshots
         WHERE encounter_id IN (SELECT id FROM encounters WHERE profile_id = ?1)",
    ]
    .into_iter()
    .try_fold(0_u64, |total, sql| {
        total
            .checked_add(scalar(connection, sql, profile_id)?)
            .ok_or(RepoError::InvalidRequest)
    })
}

fn notebook_work_units(connection: &rusqlite::Connection) -> Result<u64, RepoError> {
    [
        "SELECT count(*) FROM opponent_profiles",
        "SELECT count(*) FROM opponent_aliases",
        "SELECT count(*) FROM encounters",
        "SELECT count(*) FROM observations",
        "SELECT count(*) FROM deck_records",
        "SELECT count(*) FROM public_snapshots",
    ]
    .into_iter()
    .try_fold(0_u64, |total, sql| {
        total
            .checked_add(scalar_all(connection, sql)?)
            .ok_or(RepoError::InvalidRequest)
    })
}

fn parse_entity_type(value: &str) -> Result<DeletionEntityType, RepoError> {
    match value {
        "observation" => Ok(DeletionEntityType::Observation),
        "encounter" => Ok(DeletionEntityType::Encounter),
        "profile" => Ok(DeletionEntityType::Profile),
        "notebook" => Ok(DeletionEntityType::Notebook),
        _ => Err(RepoError::NotebookInvalid),
    }
}
