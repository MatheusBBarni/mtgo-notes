use rusqlite::{OptionalExtension, Row, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{RepoError, Revision, UtcMillis};
use crate::notebook::repository::{NotebookRepository, map_database_error};

use super::models::*;

const MAX_PAGE_SIZE: usize = 100;

#[derive(Clone, Debug)]
pub struct SaveIdentityInput {
    pub id: PlayerId,
    pub display_nickname: String,
    pub expected_revision: Option<Revision>,
    pub now: UtcMillis,
}

#[derive(Clone, Debug)]
pub struct VerifiedImportBatch {
    pub operation_key: PlayerOperationKey,
    pub command_kind: String,
    pub request_digest: String,
    pub evidence: PlayerEvidence,
    pub selected_fields: Value,
    pub cards: Vec<PlayerCard>,
    pub now: UtcMillis,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub evidence_id: PlayerEvidenceId,
    pub inserted: bool,
    pub receipt: PlayerOperationReceipt,
}

#[derive(Clone, Debug)]
pub struct AppendSelectionInput {
    pub operation_key: Option<PlayerOperationKey>,
    pub command_kind: String,
    pub request_digest: Option<String>,
    pub evidence_id: PlayerEvidenceId,
    pub expected_revision: Revision,
    pub selected_fields: Value,
    pub now: UtcMillis,
}

#[derive(Clone, Debug)]
pub struct EmptyOutcomeInput {
    pub operation_key: PlayerOperationKey,
    pub player_identity_id: PlayerId,
    pub provider_id: String,
    pub lookup_nickname: String,
    pub exact_match_rule: String,
    pub scope: Value,
    pub provider_configuration_version: String,
    pub now: UtcMillis,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidencePage {
    pub items: Vec<PlayerEvidence>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptReplay {
    pub request_digest: String,
    pub result_code: String,
    pub result_locator: Option<String>,
}

pub struct PlayerStore<'a> {
    repository: &'a NotebookRepository,
}

impl<'a> PlayerStore<'a> {
    pub fn new(repository: &'a NotebookRepository) -> Self {
        Self { repository }
    }

    pub fn identity(&self) -> Result<Option<PlayerIdentity>, RepoError> {
        self.repository.with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT id, display_nickname, normalized_nickname, created_at, updated_at, revision
                     FROM player_identities WHERE singleton = 1",
                    [],
                    map_identity,
                )
                .optional()
                .map_err(map_database_error)
        })
    }

    pub fn save_identity(&self, input: SaveIdentityInput) -> Result<PlayerIdentity, RepoError> {
        let normalized = normalize_player_nickname(&input.display_nickname)?;
        self.repository.transact_domain(|transaction| {
            let current = transaction
                .query_row(
                    "SELECT id, display_nickname, normalized_nickname, created_at, updated_at, revision
                     FROM player_identities WHERE singleton = 1",
                    [],
                    map_identity,
                )
                .optional()
                .map_err(map_database_error)?;
            match current {
                None => {
                    if input.expected_revision.is_some() {
                        return Err(RepoError::RevisionConflict);
                    }
                    transaction
                        .execute(
                            "INSERT INTO player_identities(
                                singleton, id, display_nickname, normalized_nickname,
                                created_at, updated_at, revision
                             ) VALUES (1, ?1, ?2, ?3, ?4, ?4, 1)",
                            params![
                                input.id.as_str(),
                                normalized.display,
                                normalized.normalized,
                                input.now.get()
                            ],
                        )
                        .map_err(map_database_error)?;
                }
                Some(existing) => {
                    if existing.id != input.id {
                        return Err(RepoError::IdentityConflict);
                    }
                    let expected = input.expected_revision.ok_or(RepoError::RevisionConflict)?;
                    if existing.revision != expected {
                        return Err(RepoError::RevisionConflict);
                    }
                    transaction
                        .execute(
                            "UPDATE player_identities
                             SET display_nickname = ?1, normalized_nickname = ?2,
                                 updated_at = ?3, revision = revision + 1
                             WHERE singleton = 1 AND revision = ?4",
                            params![
                                normalized.display,
                                normalized.normalized,
                                input.now.get(),
                                i64::try_from(expected.get()).map_err(|_| RepoError::InvalidRequest)?
                            ],
                        )
                        .map_err(map_database_error)?;
                }
            }
            transaction
                .query_row(
                    "SELECT id, display_nickname, normalized_nickname, created_at, updated_at, revision
                     FROM player_identities WHERE singleton = 1",
                    [],
                    map_identity,
                )
                .map_err(map_database_error)
        })
    }

    pub fn create_identity(
        &self,
        id: PlayerId,
        nickname: &str,
        now: UtcMillis,
    ) -> Result<PlayerIdentity, RepoError> {
        self.save_identity(SaveIdentityInput {
            id,
            display_nickname: nickname.to_owned(),
            expected_revision: None,
            now,
        })
    }

    pub fn update_identity(
        &self,
        id: PlayerId,
        nickname: &str,
        expected_revision: Revision,
        now: UtcMillis,
    ) -> Result<PlayerIdentity, RepoError> {
        self.save_identity(SaveIdentityInput {
            id,
            display_nickname: nickname.to_owned(),
            expected_revision: Some(expected_revision),
            now,
        })
    }

    pub fn receipt(
        &self,
        operation_key: &PlayerOperationKey,
        command_kind: &str,
    ) -> Result<Option<ReceiptReplay>, RepoError> {
        self.repository.with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT request_digest, result_code, result_locator
                     FROM player_operation_receipts
                     WHERE operation_key = ?1 AND command_kind = ?2",
                    params![operation_key.as_str(), command_kind],
                    |row| {
                        Ok(ReceiptReplay {
                            request_digest: row.get(0)?,
                            result_code: row.get(1)?,
                            result_locator: row.get(2)?,
                        })
                    },
                )
                .optional()
                .map_err(map_database_error)
        })
    }

    /// Import evidence, cards, the first selection revision, and the durable
    /// receipt in one transaction.  A duplicate source key/digest resolves the
    /// existing row; a different digest creates a linked immutable version.
    pub fn import_batch(&self, batch: VerifiedImportBatch) -> Result<ImportOutcome, RepoError> {
        validate_digest(&batch.request_digest)?;
        if batch.cards != batch.evidence.cards {
            return Err(RepoError::InvalidRequest);
        }
        validate_selected_fields(&batch.selected_fields, &batch.evidence.payload)?;
        validate_cards(
            &batch.cards,
            batch
                .evidence
                .payload
                .get("contents")
                .and_then(Value::as_str)
                == Some("complete_deck"),
        )?;
        let retained_payload =
            retain_selected_payload(&batch.evidence.payload, &batch.selected_fields)?;
        let mut evidence = batch.evidence.clone();
        evidence.payload = retained_payload;
        evidence.selected_fields = batch.selected_fields.clone();
        self.repository.transact_domain(|transaction| {
            let identity_exists = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM player_identities WHERE id = ?1)",
                    [batch.evidence.player_identity_id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(map_database_error)?;
            if identity_exists == 0 {
                return Err(RepoError::NotFound);
            }
            if let Some(replay) =
                receipt_in_transaction(transaction, &batch.operation_key, &batch.command_kind)?
            {
                if replay.request_digest != batch.request_digest {
                    return Err(RepoError::InvalidRequest);
                }
                let id = replay
                    .result_locator
                    .ok_or(RepoError::NotebookInvalid)
                    .and_then(PlayerEvidenceId::parse)?;
                let receipt = load_receipt(transaction, &batch.operation_key, &batch.command_kind)?;
                return Ok(ImportOutcome {
                    evidence_id: id,
                    inserted: replay.result_code == "imported",
                    receipt,
                });
            }

            let existing = transaction
                .query_row(
                    "SELECT id FROM player_evidence
                     WHERE player_identity_id = ?1 AND source_key = ?2 AND source_digest = ?3",
                    params![
                        batch.evidence.player_identity_id.as_str(),
                        batch.evidence.source_key,
                        batch.evidence.source_digest
                    ],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(map_database_error)?;
            let (evidence_id, inserted) = if let Some(id) = existing {
                (PlayerEvidenceId::parse(id)?, false)
            } else {
                let supersedes = if let Some(supersedes) = &evidence.supersedes_evidence_id {
                    let prior = transaction
                        .query_row(
                            "SELECT player_identity_id, source_key, source_digest FROM player_evidence WHERE id = ?1",
                            [supersedes.as_str()],
                            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
                        )
                        .optional()
                        .map_err(map_database_error)?
                        .ok_or(RepoError::NotFound)?;
                    if prior.0 != evidence.player_identity_id.as_str()
                        || prior.1 != evidence.source_key
                        || prior.2 == evidence.source_digest
                    {
                        return Err(RepoError::InvalidRequest);
                    }
                    Some(supersedes.clone())
                } else {
                    transaction
                        .query_row(
                            "SELECT id FROM player_evidence
                             WHERE player_identity_id = ?1 AND source_key = ?2
                             ORDER BY imported_at DESC, id DESC LIMIT 1",
                            params![evidence.player_identity_id.as_str(), evidence.source_key],
                            |row| row.get::<_, String>(0),
                        )
                        .optional()
                        .map_err(map_database_error)?
                        .map(PlayerEvidenceId::parse)
                        .transpose()?
                };
                let payload_json = json_string(&evidence.payload)?;
                let selected_json = json_string(&batch.selected_fields)?;
                let scope_json = json_string(&evidence.scope)?;
                transaction
                    .execute(
                        "INSERT INTO player_evidence(
                            id, player_identity_id, evidence_schema_version, kind,
                            provenance_mode, provider_id, attribution_url, canonical_source_url,
                            lookup_nickname, source_nickname, exact_match_rule, scope_json,
                            observed_at, imported_at, source_key, source_digest, preview_digest,
                            payload_json, selected_fields_json, supersedes_evidence_id
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                                   ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
                        params![
                            evidence.id.as_str(),
                            evidence.player_identity_id.as_str(),
                            i64::from(evidence.evidence_schema_version),
                            kind_name(&evidence.kind),
                            provenance_name(&evidence.provenance_mode),
                            evidence.provider_id,
                            evidence.attribution_url,
                            evidence.canonical_source_url,
                            evidence.lookup_nickname,
                            evidence.source_nickname,
                            evidence.exact_match_rule,
                            scope_json,
                            evidence.observed_at.get(),
                            evidence.imported_at.get(),
                            evidence.source_key,
                            evidence.source_digest,
                            evidence.preview_digest,
                            payload_json,
                            selected_json,
                            supersedes.as_ref().map(PlayerEvidenceId::as_str)
                        ],
                    )
                    .map_err(map_database_error)?;
                for card in &batch.cards {
                    transaction
                        .execute(
                            "INSERT INTO player_evidence_cards(
                                evidence_id, oracle_id, display_name, zone, quantity, basic_land
                             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            params![
                                evidence.id.as_str(),
                                card.oracle_id,
                                card.display_name,
                                card.zone,
                                i64::from(card.quantity),
                                i64::from(card.basic_land)
                            ],
                        )
                        .map_err(map_database_error)?;
                }
                transaction
                    .execute(
                        "INSERT INTO player_selection_revisions(
                            id, evidence_id, revision_number, selected_fields_json, created_at
                         ) VALUES (?1, ?2, 1, ?3, ?4)",
                        params![
                            PlayerSelectionId::new().as_str(),
                            evidence.id.as_str(),
                            json_string(&batch.selected_fields)?,
                            batch.now.get()
                        ],
                    )
                    .map_err(map_database_error)?;
                (evidence.id.clone(), true)
            };
            let receipt = insert_receipt(
                transaction,
                &batch.operation_key,
                &batch.command_kind,
                &batch.evidence.player_identity_id,
                &batch.request_digest,
                if inserted {
                    "imported"
                } else {
                    "already_imported"
                },
                Some(evidence_id.as_str()),
                batch.now,
            )?;
            Ok(ImportOutcome {
                evidence_id,
                inserted,
                receipt,
            })
        })
    }

    pub fn append_selection(
        &self,
        input: AppendSelectionInput,
    ) -> Result<PlayerSelectionRevision, RepoError> {
        if let Some(digest) = &input.request_digest {
            validate_digest(digest)?;
        }
        self.repository.transact_domain(|transaction| {
            let (player_identity_id, payload_json) = transaction
                .query_row(
                    "SELECT player_identity_id, payload_json FROM player_evidence WHERE id = ?1",
                    [input.evidence_id.as_str()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(map_database_error)?
                .ok_or(RepoError::NotFound)?;
            let payload: Value = serde_json::from_str(&payload_json).map_err(|_| RepoError::NotebookInvalid)?;
            validate_selected_fields(&input.selected_fields, &payload)?;
            if let Some(operation_key) = &input.operation_key
                && let Some(replay) =
                    receipt_in_transaction(transaction, operation_key, &input.command_kind)?
            {
                    if input.request_digest.as_deref() != Some(replay.request_digest.as_str()) {
                        return Err(RepoError::InvalidRequest);
                    }
                    let revision = transaction
                        .query_row(
                            "SELECT revision_number FROM player_selection_revisions WHERE evidence_id = ?1 ORDER BY revision_number DESC LIMIT 1",
                            [input.evidence_id.as_str()],
                            |row| row.get::<_, i64>(0),
                        )
                        .map_err(map_database_error)?;
                    let selected_json = transaction
                        .query_row(
                            "SELECT selected_fields_json FROM player_selection_revisions WHERE evidence_id = ?1 ORDER BY revision_number DESC LIMIT 1",
                            [input.evidence_id.as_str()],
                            |row| row.get::<_, String>(0),
                        )
                        .map_err(map_database_error)?;
                    return Ok(PlayerSelectionRevision {
                        id: PlayerSelectionId::new(),
                        evidence_id: input.evidence_id.clone(),
                        revision_number: Revision::new(u64::try_from(revision).map_err(|_| RepoError::NotebookInvalid)?)?,
                        selected_fields: serde_json::from_str(&selected_json).map_err(|_| RepoError::NotebookInvalid)?,
                        created_at: input.now,
                    });
            }
            let current = transaction
                .query_row(
                    "SELECT coalesce(max(revision_number), 0)
                     FROM player_selection_revisions WHERE evidence_id = ?1",
                    [input.evidence_id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(map_database_error)?;
            if current == 0 {
                return Err(RepoError::NotFound);
            }
            let expected = i64::try_from(input.expected_revision.get())
                .map_err(|_| RepoError::InvalidRequest)?;
            if current != expected {
                return Err(RepoError::RevisionConflict);
            }
            let next =
                Revision::new(u64::try_from(current + 1).map_err(|_| RepoError::InvalidRequest)?)?;
            let id = PlayerSelectionId::new();
            transaction
                .execute(
                    "INSERT INTO player_selection_revisions(
                        id, evidence_id, revision_number, selected_fields_json, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        id.as_str(),
                        input.evidence_id.as_str(),
                        i64::try_from(next.get()).map_err(|_| RepoError::InvalidRequest)?,
                        json_string(&input.selected_fields)?,
                        input.now.get()
                    ],
                )
                .map_err(map_database_error)?;
            let revision = PlayerSelectionRevision {
                id,
                evidence_id: input.evidence_id,
                revision_number: next,
                selected_fields: input.selected_fields,
                created_at: input.now,
            };
            if let (Some(operation_key), Some(request_digest)) =
                (&input.operation_key, &input.request_digest)
            {
                insert_receipt(
                    transaction,
                    operation_key,
                    &input.command_kind,
                    &PlayerId::parse(player_identity_id)?,
                    request_digest,
                    "selection_updated",
                    Some(revision.id.as_str()),
                    input.now,
                )?;
            }
            Ok(revision)
        })
    }

    pub fn insert_empty_outcome(
        &self,
        input: EmptyOutcomeInput,
    ) -> Result<PlayerEmptyOutcome, RepoError> {
        self.repository.transact_domain(|transaction| {
            if let Some(existing) = transaction
                .query_row(
                    "SELECT id, player_identity_id, provider_id, lookup_nickname,
                            exact_match_rule, scope_json, provider_configuration_version,
                            completed_at, operation_key
                     FROM player_empty_outcomes
                     WHERE player_identity_id = ?1 AND operation_key = ?2",
                    params![
                        input.player_identity_id.as_str(),
                        input.operation_key.as_str()
                    ],
                    map_empty,
                )
                .optional()
                .map_err(map_database_error)?
            {
                return Ok(existing);
            }
            transaction
                .execute(
                    "INSERT INTO player_empty_outcomes(
                        id, player_identity_id, provider_id, lookup_nickname, exact_match_rule,
                        scope_json, provider_configuration_version, completed_at, operation_key
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        PlayerEmptyOutcomeId::new().as_str(),
                        input.player_identity_id.as_str(),
                        input.provider_id,
                        input.lookup_nickname,
                        input.exact_match_rule,
                        json_string(&input.scope)?,
                        input.provider_configuration_version,
                        input.now.get(),
                        input.operation_key.as_str()
                    ],
                )
                .map_err(map_database_error)?;
            transaction
                .query_row(
                    "SELECT id, player_identity_id, provider_id, lookup_nickname,
                            exact_match_rule, scope_json, provider_configuration_version,
                            completed_at, operation_key
                     FROM player_empty_outcomes
                     WHERE player_identity_id = ?1 AND operation_key = ?2",
                    params![
                        input.player_identity_id.as_str(),
                        input.operation_key.as_str()
                    ],
                    map_empty,
                )
                .map_err(map_database_error)
        })
    }

    pub fn insert_classification(
        &self,
        run: PlayerClassificationRun,
    ) -> Result<PlayerClassificationRun, RepoError> {
        self.repository.transact_domain(|transaction| {
            let exists: i64 = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM player_evidence WHERE id = ?1)",
                    [run.evidence_id.as_str()],
                    |row| row.get(0),
                )
                .map_err(map_database_error)?;
            if exists == 0 {
                return Err(RepoError::NotFound);
            }
            transaction
                .execute(
                    "INSERT INTO player_classification_runs(
                        id, evidence_id, classifier_version, classifier_digest, result_id,
                        result_name, method, confidence, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT(evidence_id, classifier_version, classifier_digest) DO NOTHING",
                    params![
                        run.id.as_str(),
                        run.evidence_id.as_str(),
                        run.classifier_version,
                        run.classifier_digest,
                        run.result_id,
                        run.result_name,
                        method_name(&run.method),
                        run.confidence,
                        run.created_at.get()
                    ],
                )
                .map_err(map_database_error)?;
            Ok(run)
        })
    }

    pub fn selection_history(
        &self,
        evidence_id: &PlayerEvidenceId,
    ) -> Result<Vec<PlayerSelectionRevision>, RepoError> {
        self.repository.with_connection(|connection| {
            let mut statement = connection
                .connection
                .prepare(
                    "SELECT id, evidence_id, revision_number, selected_fields_json, created_at
                     FROM player_selection_revisions WHERE evidence_id = ?1
                     ORDER BY revision_number ASC",
                )
                .map_err(map_database_error)?;
            statement
                .query_map([evidence_id.as_str()], |row| {
                    Ok(PlayerSelectionRevision {
                        id: PlayerSelectionId::parse(row.get::<_, String>(0)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        evidence_id: PlayerEvidenceId::parse(row.get::<_, String>(1)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        revision_number: Revision::new(
                            u64::try_from(row.get::<_, i64>(2)?)
                                .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        )
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        selected_fields: serde_json::from_str(&row.get::<_, String>(3)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        created_at: UtcMillis::new(row.get(4)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    })
                })
                .map_err(map_database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_database_error)
        })
    }

    pub fn evidence_page(
        &self,
        player_identity_id: &PlayerId,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<EvidencePage, RepoError> {
        let limit = limit.min(MAX_PAGE_SIZE);
        let after = cursor.map(parse_cursor).transpose()?;
        self.repository.with_connection(|connection| {
            let mut statement = connection
                .connection
                .prepare(
                    "SELECT id, player_identity_id, evidence_schema_version, kind,
                            provenance_mode, provider_id, attribution_url, canonical_source_url,
                            lookup_nickname, source_nickname, exact_match_rule, scope_json,
                            observed_at, imported_at, source_key, source_digest, preview_digest,
                            payload_json, selected_fields_json, supersedes_evidence_id
                     FROM player_evidence
                     WHERE player_identity_id = ?1
                       AND (?2 IS NULL OR imported_at < ?2 OR (imported_at = ?2 AND id < ?3))
                     ORDER BY imported_at DESC, id DESC LIMIT ?4",
                )
                .map_err(map_database_error)?;
            let rows = statement
                .query_map(
                    params![
                        player_identity_id.as_str(),
                        after.as_ref().map(|(time, _)| *time),
                        after.as_ref().map(|(_, id)| id),
                        i64::try_from(limit + 1).map_err(|_| RepoError::InvalidRequest)?
                    ],
                    map_evidence,
                )
                .map_err(map_database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_database_error)?;
            let mut items = rows;
            for item in &mut items {
                item.cards = load_cards(&connection.connection, &item.id)?;
            }
            let next_cursor = if items.len() > limit {
                let last = items.remove(limit - 1);
                Some(format!("{}|{}", last.imported_at.get(), last.id.as_str()))
            } else {
                None
            };
            Ok(EvidencePage { items, next_cursor })
        })
    }

    pub fn evidence(
        &self,
        evidence_id: &PlayerEvidenceId,
    ) -> Result<Option<PlayerEvidence>, RepoError> {
        self.repository.with_connection(|connection| {
            let mut evidence = connection
                .connection
                .query_row(
                    "SELECT id, player_identity_id, evidence_schema_version, kind,
                            provenance_mode, provider_id, attribution_url, canonical_source_url,
                            lookup_nickname, source_nickname, exact_match_rule, scope_json,
                            observed_at, imported_at, source_key, source_digest, preview_digest,
                            payload_json, selected_fields_json, supersedes_evidence_id
                     FROM player_evidence WHERE id = ?1",
                    [evidence_id.as_str()],
                    map_evidence,
                )
                .optional()
                .map_err(map_database_error)?;
            if let Some(item) = evidence.as_mut() {
                item.cards = load_cards(&connection.connection, evidence_id)?;
            }
            Ok(evidence)
        })
    }
}

fn map_identity(row: &Row<'_>) -> rusqlite::Result<PlayerIdentity> {
    Ok(PlayerIdentity {
        id: PlayerId::parse(row.get::<_, String>(0)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        display_nickname: row.get(1)?,
        normalized_nickname: row.get(2)?,
        created_at: UtcMillis::new(row.get(3)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        updated_at: UtcMillis::new(row.get(4)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        revision: Revision::new(
            u64::try_from(row.get::<_, i64>(5)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        )
        .map_err(|_| rusqlite::Error::InvalidQuery)?,
    })
}

fn map_empty(row: &Row<'_>) -> rusqlite::Result<PlayerEmptyOutcome> {
    Ok(PlayerEmptyOutcome {
        id: PlayerEmptyOutcomeId::parse(row.get::<_, String>(0)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        player_identity_id: PlayerId::parse(row.get::<_, String>(1)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        provider_id: row.get(2)?,
        lookup_nickname: row.get(3)?,
        exact_match_rule: row.get(4)?,
        scope: serde_json::from_str(&row.get::<_, String>(5)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        provider_configuration_version: row.get(6)?,
        completed_at: UtcMillis::new(row.get(7)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        operation_key: PlayerOperationKey::parse(row.get::<_, String>(8)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
    })
}

fn map_evidence(row: &Row<'_>) -> rusqlite::Result<PlayerEvidence> {
    let id = PlayerEvidenceId::parse(row.get::<_, String>(0)?)
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let evidence = PlayerEvidence {
        id: id.clone(),
        player_identity_id: PlayerId::parse(row.get::<_, String>(1)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        evidence_schema_version: u32::try_from(row.get::<_, i64>(2)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        kind: parse_kind(&row.get::<_, String>(3)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        provenance_mode: parse_provenance(&row.get::<_, String>(4)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        provider_id: row.get(5)?,
        attribution_url: row.get(6)?,
        canonical_source_url: row.get(7)?,
        lookup_nickname: row.get(8)?,
        source_nickname: row.get(9)?,
        exact_match_rule: row.get(10)?,
        scope: serde_json::from_str(&row.get::<_, String>(11)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        observed_at: UtcMillis::new(row.get(12)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        imported_at: UtcMillis::new(row.get(13)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        source_key: row.get(14)?,
        source_digest: row.get(15)?,
        preview_digest: row.get(16)?,
        payload: serde_json::from_str(&row.get::<_, String>(17)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        selected_fields: serde_json::from_str(&row.get::<_, String>(18)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        supersedes_evidence_id: row
            .get::<_, Option<String>>(19)?
            .map(PlayerEvidenceId::parse)
            .transpose()
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        cards: Vec::new(),
    };
    Ok(evidence)
}

fn load_cards(
    connection: &rusqlite::Connection,
    evidence_id: &PlayerEvidenceId,
) -> Result<Vec<PlayerCard>, RepoError> {
    let mut statement = connection
        .prepare(
            "SELECT oracle_id, display_name, zone, quantity, basic_land
             FROM player_evidence_cards WHERE evidence_id = ?1
             ORDER BY zone, oracle_id",
        )
        .map_err(map_database_error)?;
    statement
        .query_map([evidence_id.as_str()], |row| {
            Ok(PlayerCard {
                oracle_id: row.get(0)?,
                display_name: row.get(1)?,
                zone: row.get(2)?,
                quantity: u16::try_from(row.get::<_, i64>(3)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                basic_land: row.get::<_, i64>(4)? == 1,
            })
        })
        .map_err(map_database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_database_error)
}

fn map_receipt(row: &Row<'_>) -> rusqlite::Result<PlayerOperationReceipt> {
    Ok(PlayerOperationReceipt {
        operation_key: PlayerOperationKey::parse(row.get::<_, String>(0)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        command_kind: row.get(1)?,
        player_identity_id: PlayerId::parse(row.get::<_, String>(2)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        request_digest: row.get(3)?,
        result_code: row.get(4)?,
        result_locator: row.get(5)?,
        created_at: UtcMillis::new(row.get(6)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
    })
}

fn load_receipt(
    transaction: &Transaction<'_>,
    operation_key: &PlayerOperationKey,
    command_kind: &str,
) -> Result<PlayerOperationReceipt, RepoError> {
    transaction
        .query_row(
            "SELECT operation_key, command_kind, player_identity_id, request_digest,
                    result_code, result_locator, created_at
             FROM player_operation_receipts WHERE operation_key = ?1 AND command_kind = ?2",
            params![operation_key.as_str(), command_kind],
            map_receipt,
        )
        .map_err(map_database_error)
}

fn receipt_in_transaction(
    transaction: &Transaction<'_>,
    operation_key: &PlayerOperationKey,
    command_kind: &str,
) -> Result<Option<ReceiptReplay>, RepoError> {
    transaction
        .query_row(
            "SELECT request_digest, result_code, result_locator
             FROM player_operation_receipts WHERE operation_key = ?1 AND command_kind = ?2",
            params![operation_key.as_str(), command_kind],
            |row| {
                Ok(ReceiptReplay {
                    request_digest: row.get(0)?,
                    result_code: row.get(1)?,
                    result_locator: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(map_database_error)
}

#[allow(clippy::too_many_arguments)]
fn insert_receipt(
    transaction: &Transaction<'_>,
    operation_key: &PlayerOperationKey,
    command_kind: &str,
    player_identity_id: &PlayerId,
    request_digest: &str,
    result_code: &str,
    result_locator: Option<&str>,
    created_at: UtcMillis,
) -> Result<PlayerOperationReceipt, RepoError> {
    transaction
        .execute(
            "INSERT INTO player_operation_receipts(
                operation_key, command_kind, player_identity_id, request_digest,
                result_code, result_locator, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                operation_key.as_str(),
                command_kind,
                player_identity_id.as_str(),
                request_digest,
                result_code,
                result_locator,
                created_at.get()
            ],
        )
        .map_err(map_database_error)?;
    load_receipt(transaction, operation_key, command_kind)
}

fn parse_cursor(value: &str) -> Result<(i64, String), RepoError> {
    let (time, id) = value.split_once('|').ok_or(RepoError::InvalidRequest)?;
    let time = time.parse::<i64>().map_err(|_| RepoError::InvalidRequest)?;
    PlayerEvidenceId::parse(id.to_owned())?;
    Ok((time, id.to_owned()))
}

fn validate_digest(value: &str) -> Result<(), RepoError> {
    if value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(RepoError::InvalidRequest)
    }
}

fn json_string(value: &Value) -> Result<String, RepoError> {
    serde_json::to_string(value).map_err(|_| RepoError::InvalidRequest)
}

fn kind_name(value: &EvidenceKind) -> &'static str {
    match value {
        EvidenceKind::MocsLeaderboardEntry => "mocs_leaderboard_entry",
        EvidenceKind::OfficialPublishedDecklist => "official_published_decklist",
    }
}

fn provenance_name(value: &EvidenceProvenance) -> &'static str {
    match value {
        EvidenceProvenance::ProviderObserved => "provider_observed",
        EvidenceProvenance::UserAttestedOfficialSource => "user_attested_official_source",
    }
}

fn method_name(value: &ClassificationMethod) -> &'static str {
    match value {
        ClassificationMethod::Signature => "signature",
        ClassificationMethod::Knn => "knn",
        ClassificationMethod::Unsupported => "unsupported",
    }
}

fn parse_kind(value: &str) -> Result<EvidenceKind, RepoError> {
    match value {
        "mocs_leaderboard_entry" => Ok(EvidenceKind::MocsLeaderboardEntry),
        "official_published_decklist" => Ok(EvidenceKind::OfficialPublishedDecklist),
        _ => Err(RepoError::NotebookInvalid),
    }
}

fn parse_provenance(value: &str) -> Result<EvidenceProvenance, RepoError> {
    match value {
        "provider_observed" => Ok(EvidenceProvenance::ProviderObserved),
        "user_attested_official_source" => Ok(EvidenceProvenance::UserAttestedOfficialSource),
        _ => Err(RepoError::NotebookInvalid),
    }
}
