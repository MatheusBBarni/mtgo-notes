use std::collections::BTreeSet;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rusqlite::types::{Value, ValueRef};
use rusqlite::{Connection, Transaction, params_from_iter};
use sha2::{Digest, Sha256};

use crate::domain::RepoError;
use crate::notebook::repository::NotebookRepository;
use crate::portability::archive::{CanonicalRecord, CanonicalValue, ClassifierProvenance};

pub struct TableSpec {
    pub name: &'static str,
    pub columns: &'static [&'static str],
    pub primary_key_indexes: &'static [usize],
    pub select_sql: &'static str,
}

pub const TABLE_SPECS: &[TableSpec] = &[
    TableSpec {
        name: "opponent_profiles",
        columns: &[
            "id",
            "primary_handle",
            "normalized_handle",
            "created_at",
            "revision",
            "deleted_at",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT id, primary_handle, normalized_handle, created_at, revision, deleted_at
                     FROM opponent_profiles WHERE deleted_at IS NULL ORDER BY id",
    },
    TableSpec {
        name: "opponent_aliases",
        columns: &[
            "id",
            "profile_id",
            "display_handle",
            "normalized_handle",
            "provenance",
            "created_at",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT alias.id, alias.profile_id, alias.display_handle,
                            alias.normalized_handle, alias.provenance, alias.created_at
                     FROM opponent_aliases alias
                     JOIN opponent_profiles profile ON profile.id = alias.profile_id
                     WHERE profile.deleted_at IS NULL ORDER BY alias.id",
    },
    TableSpec {
        name: "encounters",
        columns: &[
            "id",
            "profile_id",
            "format",
            "started_at",
            "ended_at",
            "status",
            "phase",
            "source",
            "generation",
            "revision",
            "incomplete_reason",
            "deleted_at",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT encounter.id, encounter.profile_id, encounter.format,
                            encounter.started_at, encounter.ended_at, encounter.status,
                            encounter.phase, encounter.source, encounter.generation,
                            encounter.revision, encounter.incomplete_reason, encounter.deleted_at
                     FROM encounters encounter
                     JOIN opponent_profiles profile ON profile.id = encounter.profile_id
                     WHERE encounter.deleted_at IS NULL AND profile.deleted_at IS NULL
                     ORDER BY encounter.id",
    },
    TableSpec {
        name: "encounter_transitions",
        columns: &[
            "id",
            "encounter_id",
            "sequence",
            "from_phase",
            "to_phase",
            "trigger",
            "confidence_class",
            "created_at",
            "undo_group_id",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT transition.id, transition.encounter_id, transition.sequence,
                            transition.from_phase, transition.to_phase, transition.trigger,
                            transition.confidence_class, transition.created_at,
                            transition.undo_group_id
                     FROM encounter_transitions transition
                     JOIN encounters encounter ON encounter.id = transition.encounter_id
                     WHERE encounter.deleted_at IS NULL ORDER BY transition.id",
    },
    TableSpec {
        name: "observations",
        columns: &[
            "id",
            "encounter_id",
            "text",
            "created_at",
            "edited_at",
            "revision",
            "searchable",
            "deletion_deadline",
            "deleted_at",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT observation.id, observation.encounter_id, observation.text,
                            observation.created_at, observation.edited_at, observation.revision,
                            observation.searchable, observation.deletion_deadline,
                            observation.deleted_at
                     FROM observations observation
                     JOIN encounters encounter ON encounter.id = observation.encounter_id
                     WHERE observation.deleted_at IS NULL AND encounter.deleted_at IS NULL
                     ORDER BY observation.id",
    },
    TableSpec {
        name: "card_observations",
        columns: &[
            "observation_id",
            "oracle_id",
            "display_name",
            "quantity",
            "certainty",
            "context",
        ],
        primary_key_indexes: &[0, 1, 4],
        select_sql: "SELECT card.observation_id, card.oracle_id, card.display_name,
                            card.quantity, card.certainty, card.context
                     FROM card_observations card
                     JOIN observations observation ON observation.id = card.observation_id
                     WHERE observation.deleted_at IS NULL
                     ORDER BY card.observation_id, card.oracle_id, card.certainty",
    },
    TableSpec {
        name: "tendency_tags",
        columns: &["id", "normalized_label", "display_label"],
        primary_key_indexes: &[0],
        select_sql: "SELECT tag.id, tag.normalized_label, tag.display_label
                     FROM tendency_tags tag
                     WHERE EXISTS (
                       SELECT 1 FROM observation_tags link
                       JOIN observations observation ON observation.id = link.observation_id
                       WHERE link.tag_id = tag.id AND observation.deleted_at IS NULL
                     ) ORDER BY tag.id",
    },
    TableSpec {
        name: "observation_tags",
        columns: &["observation_id", "tag_id"],
        primary_key_indexes: &[0, 1],
        select_sql: "SELECT link.observation_id, link.tag_id
                     FROM observation_tags link
                     JOIN observations observation ON observation.id = link.observation_id
                     WHERE observation.deleted_at IS NULL
                     ORDER BY link.observation_id, link.tag_id",
    },
    TableSpec {
        name: "deck_records",
        columns: &[
            "id",
            "profile_id",
            "source_class",
            "format",
            "completeness",
            "provider_label",
            "user_label",
            "current_revision",
            "revision",
            "created_at",
            "deleted_at",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT deck.id, deck.profile_id, deck.source_class, deck.format,
                            deck.completeness, deck.provider_label, deck.user_label,
                            deck.current_revision, deck.revision, deck.created_at, deck.deleted_at
                     FROM deck_records deck
                     JOIN opponent_profiles profile ON profile.id = deck.profile_id
                     WHERE deck.deleted_at IS NULL AND profile.deleted_at IS NULL
                     ORDER BY deck.id",
    },
    TableSpec {
        name: "deck_revisions",
        columns: &[
            "id",
            "deck_id",
            "revision_number",
            "canonical_digest",
            "complete",
            "created_at",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT revision.id, revision.deck_id, revision.revision_number,
                            revision.canonical_digest, revision.complete, revision.created_at
                     FROM deck_revisions revision
                     JOIN deck_records deck ON deck.id = revision.deck_id
                     WHERE deck.deleted_at IS NULL ORDER BY revision.id",
    },
    TableSpec {
        name: "deck_cards",
        columns: &[
            "deck_revision_id",
            "oracle_id",
            "display_name",
            "zone",
            "quantity",
            "basic_land",
        ],
        primary_key_indexes: &[0, 1, 3],
        select_sql: "SELECT card.deck_revision_id, card.oracle_id, card.display_name,
                            card.zone, card.quantity, card.basic_land
                     FROM deck_cards card
                     JOIN deck_revisions revision ON revision.id = card.deck_revision_id
                     JOIN deck_records deck ON deck.id = revision.deck_id
                     WHERE deck.deleted_at IS NULL
                     ORDER BY card.deck_revision_id, card.oracle_id, card.zone",
    },
    TableSpec {
        name: "public_snapshots",
        columns: &[
            "id",
            "encounter_id",
            "deck_revision_id",
            "provider",
            "event",
            "format",
            "publication_date",
            "source_url",
            "confirmed",
            "source_token",
            "created_at",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT snapshot.id, snapshot.encounter_id, snapshot.deck_revision_id,
                            snapshot.provider, snapshot.event, snapshot.format,
                            snapshot.publication_date, snapshot.source_url, snapshot.confirmed,
                            snapshot.source_token, snapshot.created_at
                     FROM public_snapshots snapshot
                     JOIN encounters encounter ON encounter.id = snapshot.encounter_id
                     JOIN deck_revisions revision ON revision.id = snapshot.deck_revision_id
                     JOIN deck_records deck ON deck.id = revision.deck_id
                     WHERE encounter.deleted_at IS NULL AND deck.deleted_at IS NULL
                     ORDER BY snapshot.id",
    },
    TableSpec {
        name: "classification_runs",
        columns: &[
            "id",
            "deck_revision_id",
            "classifier_version",
            "classifier_digest",
            "result_id",
            "result_name",
            "method",
            "confidence",
            "explanation_json",
            "status",
            "created_at",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT run.id, run.deck_revision_id, run.classifier_version,
                            run.classifier_digest, run.result_id, run.result_name, run.method,
                            run.confidence, run.explanation_json, run.status, run.created_at
                     FROM classification_runs run
                     JOIN deck_revisions revision ON revision.id = run.deck_revision_id
                     JOIN deck_records deck ON deck.id = revision.deck_id
                     WHERE deck.deleted_at IS NULL ORDER BY run.id",
    },
    TableSpec {
        name: "profile_merges",
        columns: &[
            "id",
            "primary_profile_id",
            "state",
            "created_at",
            "reversed_at",
            "reassignment_plan_json",
            "revision",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT merge.id, merge.primary_profile_id, merge.state, merge.created_at,
                            merge.reversed_at, merge.reassignment_plan_json, merge.revision
                     FROM profile_merges merge
                     JOIN opponent_profiles profile ON profile.id = merge.primary_profile_id
                     WHERE profile.deleted_at IS NULL ORDER BY merge.id",
    },
    TableSpec {
        name: "deletion_tombstones",
        columns: &[
            "entity_type",
            "entity_id",
            "requested_at",
            "effective_at",
            "undo_token_digest",
            "purge_state",
        ],
        primary_key_indexes: &[0, 1],
        select_sql: "SELECT entity_type, entity_id, requested_at, effective_at,
                            undo_token_digest, purge_state
                     FROM deletion_tombstones ORDER BY entity_type, entity_id",
    },
    // Player portability is intentionally a separate, FK-safe graph. Consent,
    // operation receipts, runtime state, and machine-bound configuration are
    // never archive records.
    TableSpec {
        name: "player_identities",
        columns: &[
            "singleton",
            "id",
            "display_nickname",
            "normalized_nickname",
            "created_at",
            "updated_at",
            "revision",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT singleton, id, display_nickname, normalized_nickname,
                            created_at, updated_at, revision
                     FROM player_identities ORDER BY singleton",
    },
    TableSpec {
        name: "player_evidence",
        columns: &[
            "id",
            "player_identity_id",
            "evidence_schema_version",
            "kind",
            "provenance_mode",
            "provider_id",
            "attribution_url",
            "canonical_source_url",
            "lookup_nickname",
            "source_nickname",
            "exact_match_rule",
            "scope_json",
            "observed_at",
            "imported_at",
            "source_key",
            "source_digest",
            "preview_digest",
            "payload_json",
            "selected_fields_json",
            "supersedes_evidence_id",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT id, player_identity_id, evidence_schema_version, kind,
                            provenance_mode, provider_id, attribution_url, canonical_source_url,
                            lookup_nickname, source_nickname, exact_match_rule, scope_json,
                            observed_at, imported_at, source_key, source_digest, preview_digest,
                            payload_json, selected_fields_json, supersedes_evidence_id
                     FROM player_evidence ORDER BY id",
    },
    TableSpec {
        name: "player_evidence_cards",
        columns: &[
            "evidence_id",
            "oracle_id",
            "display_name",
            "zone",
            "quantity",
            "basic_land",
        ],
        primary_key_indexes: &[0, 1, 3],
        select_sql: "SELECT evidence_id, oracle_id, display_name, zone, quantity, basic_land
                     FROM player_evidence_cards ORDER BY evidence_id, oracle_id, zone",
    },
    TableSpec {
        name: "player_selection_revisions",
        columns: &[
            "id",
            "evidence_id",
            "revision_number",
            "selected_fields_json",
            "created_at",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT id, evidence_id, revision_number, selected_fields_json, created_at
                     FROM player_selection_revisions ORDER BY evidence_id, revision_number, id",
    },
    TableSpec {
        name: "player_classification_runs",
        columns: &[
            "id",
            "evidence_id",
            "classifier_version",
            "classifier_digest",
            "result_id",
            "result_name",
            "method",
            "confidence",
            "created_at",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT id, evidence_id, classifier_version, classifier_digest,
                            result_id, result_name, method, confidence, created_at
                     FROM player_classification_runs ORDER BY evidence_id, created_at, id",
    },
    TableSpec {
        name: "player_empty_outcomes",
        columns: &[
            "id",
            "player_identity_id",
            "provider_id",
            "lookup_nickname",
            "exact_match_rule",
            "scope_json",
            "provider_configuration_version",
            "completed_at",
            "operation_key",
        ],
        primary_key_indexes: &[0],
        select_sql: "SELECT id, player_identity_id, provider_id, lookup_nickname,
                            exact_match_rule, scope_json, provider_configuration_version,
                            completed_at, operation_key
                     FROM player_empty_outcomes ORDER BY completed_at, id",
    },
    TableSpec {
        name: "player_tombstones",
        columns: &[
            "entity_kind",
            "entity_id",
            "player_identity_id",
            "deleted_at",
        ],
        primary_key_indexes: &[0, 1],
        select_sql: "SELECT entity_kind, entity_id, player_identity_id, deleted_at
                     FROM player_tombstones ORDER BY entity_kind, entity_id",
    },
];

pub fn for_each_record(
    repository: &NotebookRepository,
    operation: impl FnMut(CanonicalRecord) -> Result<(), RepoError>,
) -> Result<(), RepoError> {
    for_each_record_with_provenance(repository, operation).map(|_| ())
}

pub fn for_each_record_with_provenance(
    repository: &NotebookRepository,
    mut operation: impl FnMut(CanonicalRecord) -> Result<(), RepoError>,
) -> Result<Vec<ClassifierProvenance>, RepoError> {
    repository.transact_domain(|transaction| {
        let mut provenance_statement = transaction
            .prepare(
                "SELECT DISTINCT classifier_version, classifier_digest
                 FROM (
                   SELECT classifier_version, classifier_digest
                   FROM classification_runs WHERE status = 'successful'
                   UNION ALL
                   SELECT classifier_version, classifier_digest
                   FROM player_classification_runs
                 )
                 ORDER BY classifier_version, classifier_digest",
            )
            .map_err(|_| RepoError::NotebookInvalid)?;
        let provenance = provenance_statement
            .query_map([], |row| {
                Ok(ClassifierProvenance {
                    version: row.get(0)?,
                    digest: row.get(1)?,
                })
            })
            .map_err(|_| RepoError::NotebookInvalid)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| RepoError::NotebookInvalid)?;
        drop(provenance_statement);
        for spec in TABLE_SPECS {
            let mut statement = transaction
                .prepare(spec.select_sql)
                .map_err(|_| RepoError::NotebookInvalid)?;
            let mut rows = statement
                .query([])
                .map_err(|_| RepoError::NotebookInvalid)?;
            while let Some(row) = rows.next().map_err(|_| RepoError::NotebookInvalid)? {
                operation(row_to_record(row, spec)?)?;
            }
        }
        Ok(provenance)
    })
}

pub fn insert_record(
    transaction: &Transaction<'_>,
    record: &CanonicalRecord,
) -> Result<(), RepoError> {
    let spec = table_spec(&record.table)?;
    if record.values.len() != spec.columns.len() || record.key != record_key(spec, &record.values)?
    {
        return Err(RepoError::InvalidBackup);
    }
    let placeholders = (1..=spec.columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "INSERT INTO {}({}) VALUES ({})",
        spec.name,
        spec.columns.join(","),
        placeholders
    );
    let values = record
        .values
        .iter()
        .map(canonical_to_sql)
        .collect::<Result<Vec<_>, _>>()?;
    transaction
        .execute(&sql, params_from_iter(values))
        .map_err(|_| RepoError::InvalidBackup)?;
    Ok(())
}

pub fn insert_conflict(
    transaction: &Transaction<'_>,
    record: &CanonicalRecord,
) -> Result<(), RepoError> {
    let record_json = serde_json::to_string(record).map_err(|_| RepoError::InvalidBackup)?;
    let digest = hex_digest(Sha256::digest(record_json.as_bytes()));
    transaction
        .execute(
            "INSERT OR IGNORE INTO restore_conflicts(
               id, source_table, source_key, imported_record_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                digest,
                &record.table,
                &record.key,
                record_json,
                crate::domain::UtcMillis::now().get(),
            ),
        )
        .map_err(|_| RepoError::NotebookInvalid)?;
    Ok(())
}

pub fn record_status(
    connection: &Connection,
    record: &CanonicalRecord,
) -> Result<RecordStatus, RepoError> {
    let spec = table_spec(&record.table)?;
    let where_clause = spec
        .primary_key_indexes
        .iter()
        .enumerate()
        .map(|(parameter, index)| format!("{} = ?{}", spec.columns[*index], parameter + 1))
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!(
        "SELECT {} FROM {} WHERE {}",
        spec.columns.join(","),
        spec.name,
        where_clause
    );
    let key_values = spec
        .primary_key_indexes
        .iter()
        .map(|index| canonical_to_sql(&record.values[*index]))
        .collect::<Result<Vec<_>, _>>()?;
    let mut statement = connection
        .prepare(&sql)
        .map_err(|_| RepoError::NotebookInvalid)?;
    let mut rows = statement
        .query(params_from_iter(key_values))
        .map_err(|_| RepoError::NotebookInvalid)?;
    let Some(row) = rows.next().map_err(|_| RepoError::NotebookInvalid)? else {
        return Ok(RecordStatus::Missing);
    };
    let existing = row_to_record(row, spec)?;
    if existing == *record {
        Ok(RecordStatus::Exact)
    } else {
        Ok(RecordStatus::Divergent)
    }
}

pub fn is_tombstoned(
    connection: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> Result<bool, RepoError> {
    connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM deletion_tombstones
               WHERE entity_type = ?1 AND entity_id = ?2
                 AND purge_state IN ('pending','purged')
             )",
            (entity_type, entity_id),
            |row| row.get::<_, i64>(0),
        )
        .map(|exists| exists == 1)
        .map_err(|_| RepoError::NotebookInvalid)
}

pub fn table_spec(table: &str) -> Result<&'static TableSpec, RepoError> {
    TABLE_SPECS
        .iter()
        .find(|spec| spec.name == table)
        .ok_or(RepoError::InvalidBackup)
}

pub fn remap_foreign_key(record: &mut CanonicalRecord, column: &str, value: &str) {
    if let Ok(spec) = table_spec(&record.table)
        && let Some(index) = spec
            .columns
            .iter()
            .position(|candidate| *candidate == column)
    {
        record.values[index] = CanonicalValue::Text(value.to_owned());
        if spec.primary_key_indexes.contains(&index)
            && let Ok(key) = record_key(spec, &record.values)
        {
            record.key = key;
        }
    }
}

pub fn text_value<'a>(record: &'a CanonicalRecord, column: &str) -> Option<&'a str> {
    let spec = table_spec(&record.table).ok()?;
    let index = spec
        .columns
        .iter()
        .position(|candidate| *candidate == column)?;
    match record.values.get(index)? {
        CanonicalValue::Text(value) => Some(value),
        _ => None,
    }
}

pub fn references_any(record: &CanonicalRecord, entity_ids: &BTreeSet<String>) -> bool {
    let Ok(spec) = table_spec(&record.table) else {
        return true;
    };
    spec.columns
        .iter()
        .enumerate()
        .filter(|(_, column)| **column == "id" || column.ends_with("_id"))
        .any(|(index, _)| {
            matches!(
                record.values.get(index),
                Some(CanonicalValue::Text(value)) if entity_ids.contains(value)
            )
        })
}

pub fn primary_text_ids(record: &CanonicalRecord) -> Vec<String> {
    let Ok(spec) = table_spec(&record.table) else {
        return Vec::new();
    };
    spec.primary_key_indexes
        .iter()
        .filter_map(|index| match record.values.get(*index) {
            Some(CanonicalValue::Text(value)) => Some(value.clone()),
            _ => None,
        })
        .collect()
}

pub fn active_tombstone_ids(
    repository: &NotebookRepository,
) -> Result<BTreeSet<String>, RepoError> {
    repository.with_connection(|encrypted| {
        let mut statement = encrypted
            .connection
            .prepare(
                "SELECT entity_id FROM deletion_tombstones
                 WHERE purge_state IN ('pending','purged')
                 UNION
                 SELECT entity_id FROM player_tombstones
                 ORDER BY entity_id",
            )
            .map_err(|_| RepoError::NotebookInvalid)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| RepoError::NotebookInvalid)?;
        rows.collect::<Result<BTreeSet<_>, _>>()
            .map_err(|_| RepoError::NotebookInvalid)
    })
}

pub fn notebook_digest(repository: &NotebookRepository) -> Result<String, RepoError> {
    let mut hasher = Sha256::new();
    for_each_record(repository, |record| {
        let encoded = serde_json::to_vec(&record).map_err(|_| RepoError::NotebookInvalid)?;
        hasher.update(encoded);
        hasher.update(b"\n");
        Ok(())
    })?;
    Ok(hex_digest(hasher.finalize()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordStatus {
    Missing,
    Exact,
    Divergent,
}

fn row_to_record(row: &rusqlite::Row<'_>, spec: &TableSpec) -> Result<CanonicalRecord, RepoError> {
    let values = (0..spec.columns.len())
        .map(|index| {
            row.get_ref(index)
                .map_err(|_| RepoError::NotebookInvalid)
                .and_then(sql_to_canonical)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CanonicalRecord {
        table: spec.name.to_owned(),
        key: record_key(spec, &values)?,
        values,
    })
}

fn record_key(spec: &TableSpec, values: &[CanonicalValue]) -> Result<String, RepoError> {
    let key_values = spec
        .primary_key_indexes
        .iter()
        .map(|index| values.get(*index).ok_or(RepoError::InvalidBackup))
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_string(&key_values).map_err(|_| RepoError::InvalidBackup)
}

fn sql_to_canonical(value: ValueRef<'_>) -> Result<CanonicalValue, RepoError> {
    Ok(match value {
        ValueRef::Null => CanonicalValue::Null,
        ValueRef::Integer(value) => CanonicalValue::Integer(value),
        ValueRef::Real(value) => {
            if !value.is_finite() {
                return Err(RepoError::InvalidBackup);
            }
            CanonicalValue::Real(value)
        }
        ValueRef::Text(value) => CanonicalValue::Text(
            std::str::from_utf8(value)
                .map_err(|_| RepoError::InvalidBackup)?
                .to_owned(),
        ),
        ValueRef::Blob(value) => CanonicalValue::Blob(STANDARD.encode(value)),
    })
}

fn canonical_to_sql(value: &CanonicalValue) -> Result<Value, RepoError> {
    Ok(match value {
        CanonicalValue::Null => Value::Null,
        CanonicalValue::Integer(value) => Value::Integer(*value),
        CanonicalValue::Real(value) if value.is_finite() => Value::Real(*value),
        CanonicalValue::Real(_) => return Err(RepoError::InvalidBackup),
        CanonicalValue::Text(value) => Value::Text(value.clone()),
        CanonicalValue::Blob(value) => Value::Blob(
            STANDARD
                .decode(value)
                .map_err(|_| RepoError::InvalidBackup)?,
        ),
    })
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
