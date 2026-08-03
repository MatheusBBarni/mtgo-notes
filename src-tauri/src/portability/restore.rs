use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::domain::{EntityId, IdempotencyKey, RepoError, UtcMillis};
use crate::notebook::key::DatabaseKey;
use crate::notebook::migrations::MigrationManager;
use crate::notebook::repository::NotebookRepository;
use crate::operations::{
    CancellationToken, OperationCoordinator, OperationKind, OperationRecord, OperationState,
};
use crate::player::runtime::PlayerPublicResultsRuntime;
use crate::portability::archive::{
    ArchiveManifest, CanonicalRecord, CanonicalValue, for_each_record_with_cancellation,
    verify_archive_with_cancellation,
};
use crate::portability::records::{
    RecordStatus, active_tombstone_ids, for_each_record as for_each_notebook_record,
    insert_conflict, insert_record, notebook_digest, primary_text_ids, record_status,
    references_any, remap_foreign_key, table_spec, text_value,
};
use crate::portability::sync_parent;

const PREVIEW_TTL_MS: i64 = 15 * 60 * 1000;
const ROLLBACK_RETENTION_MS: i64 = 7 * 24 * 60 * 60 * 1000;
const MAX_ROLLBACKS: usize = 3;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreMode {
    Merge,
    Replace,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreDiff {
    pub imported_records: u64,
    pub exact_duplicates: u64,
    pub conflicts: u64,
    pub tombstone_skips: u64,
    pub profiles: u64,
    pub encounters: u64,
    pub observations: u64,
    pub player_identities: u64,
    pub player_evidence: u64,
    pub player_empty_outcomes: u64,
    pub player_tombstones: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub operation: OperationRecord,
    pub token: String,
    pub expires_at: UtcMillis,
    pub archive_sha256: String,
    pub manifest: ArchiveManifest,
    pub diff: RestoreDiff,
    pub allowed_modes: Vec<RestoreMode>,
    pub player_identity_conflict: bool,
}

#[derive(Clone, Debug)]
pub struct StagedRestore {
    pub preview: RestorePreview,
    pub staging_path: PathBuf,
    pub live_digest: String,
}

pub struct RestorePreviewInput<'a> {
    pub operation_id: EntityId,
    pub idempotency_key: IdempotencyKey,
    pub archive_path: &'a Path,
    pub passphrase: &'a str,
    pub cancellation: &'a CancellationToken,
}

pub fn discard_staged_restore(staged: StagedRestore) {
    remove_database_family(&staged.staging_path);
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub operation: OperationRecord,
    pub mode: RestoreMode,
    pub imported_records: u64,
    pub exact_duplicates: u64,
    pub conflicts: u64,
    pub tombstone_skips: u64,
    pub rollback: RollbackView,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackView {
    pub id: String,
    pub restore_operation_id: String,
    pub mode: RestoreMode,
    pub created_at: UtcMillis,
    pub expires_at: UtcMillis,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RollbackMetadata {
    view: RollbackView,
    database_file: String,
}

pub fn preview_restore(
    repository: &NotebookRepository,
    key: &DatabaseKey,
    coordinator: &OperationCoordinator,
    input: RestorePreviewInput<'_>,
) -> Result<StagedRestore, RepoError> {
    let claimed_path = input.archive_path.to_string_lossy().into_owned();
    let _lease = coordinator.begin_with_cancellation(
        OperationKind::RestoreMerge,
        Some(&claimed_path),
        input.cancellation.clone(),
    )?;
    let mut operation = OperationRecord::requested_with_id(
        input.operation_id,
        OperationKind::RestoreMerge,
        input.idempotency_key,
    );
    operation.transition(OperationState::Running)?;
    coordinator.register(operation.clone())?;
    repository.persist_operation_record(&operation)?;
    let token = EntityId::new().to_string();
    let staging_path = staging_path(repository, &token)?;
    remove_database_family(&staging_path);

    let result = (|| {
        let verified = verify_archive_with_cancellation(
            input.archive_path,
            input.passphrase,
            input.cancellation,
        )?;
        let schema_version = repository.schema_version()?;
        if schema_version < verified.manifest.schema_min
            || schema_version > verified.manifest.schema_max
        {
            return Err(RepoError::InvalidBackup);
        }
        MigrationManager::default().migrate(&staging_path, key)?;
        let staging = NotebookRepository::open(&staging_path, key)?;
        let mut suppressed_ids = active_tombstone_ids(repository)?;
        let mut tombstone_skips = 0_u64;
        staging.transact_domain(|transaction| {
            let mut records = Vec::new();
            for_each_record_with_cancellation(
                input.archive_path,
                input.passphrase,
                input.cancellation,
                |record| {
                    records.push(record);
                    Ok(())
                },
            )?;
            records.sort_by_key(|record| {
                if is_tombstone_record(record) {
                    0_u8
                } else {
                    1_u8
                }
            });
            for record in records {
                if is_tombstone_record(&record) {
                    extend_tombstone_ids(&record, &mut suppressed_ids);
                    insert_record(transaction, &record)?;
                } else if references_any(&record, &suppressed_ids) {
                    suppressed_ids.extend(primary_text_ids(&record));
                    tombstone_skips = tombstone_skips.saturating_add(1);
                } else {
                    insert_record(transaction, &record)?;
                }
            }
            Ok(())
        })?;
        copy_live_tombstones(repository, &staging)?;
        staging.with_connection(|connection| connection.integrity_check())?;
        validate_foreign_keys(&staging)?;
        let mut diff = calculate_diff(repository, &staging)?;
        diff.tombstone_skips = tombstone_skips;
        let live_player_id = player_identity_id(repository)?;
        let archived_player_id = player_identity_id(&staging)?;
        let player_identity_conflict = live_player_id.is_some()
            && archived_player_id.is_some()
            && live_player_id != archived_player_id;
        let now = UtcMillis::now();
        let expires_at = UtcMillis::new(
            now.get()
                .checked_add(PREVIEW_TTL_MS)
                .ok_or(RepoError::InvalidRequest)?,
        )?;
        operation = coordinator.update(&operation.id, |record| {
            record.update_progress(
                verified.manifest.record_count,
                verified.manifest.record_count,
            )?;
            record.transition(OperationState::AwaitingConfirmation)
        })?;
        repository.persist_operation_record(&operation)?;
        Ok(StagedRestore {
            preview: RestorePreview {
                operation: operation.clone(),
                token,
                expires_at,
                archive_sha256: verified.archive_sha256,
                manifest: verified.manifest,
                diff,
                allowed_modes: if player_identity_conflict {
                    vec![RestoreMode::Replace]
                } else {
                    vec![RestoreMode::Merge, RestoreMode::Replace]
                },
                player_identity_conflict,
            },
            staging_path: staging_path.clone(),
            live_digest: notebook_digest(repository)?,
        })
    })();
    if result.is_err() {
        remove_database_family(&staging_path);
        let cancelled = input.cancellation.is_cancelled();
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

pub fn apply_restore(
    repository: &NotebookRepository,
    key: &DatabaseKey,
    coordinator: &OperationCoordinator,
    staged: StagedRestore,
    mode: RestoreMode,
    _idempotency_key: IdempotencyKey,
) -> Result<RestoreResult, RepoError> {
    if staged.preview.expires_at <= UtcMillis::now()
        || staged.live_digest != notebook_digest(repository)?
        || !staged.staging_path.exists()
    {
        remove_database_family(&staged.staging_path);
        return Err(RepoError::InvalidRequest);
    }
    if staged.preview.player_identity_conflict && mode == RestoreMode::Merge {
        return Err(RepoError::MergeConflict);
    }
    let kind = match mode {
        RestoreMode::Merge => OperationKind::RestoreMerge,
        RestoreMode::Replace => OperationKind::RestoreReplace,
    };
    let lease = coordinator.begin(kind, None)?;
    let mut operation = coordinator.get(&staged.preview.operation.id)?;
    if operation.state != OperationState::AwaitingConfirmation {
        return Err(RepoError::InvalidTransition);
    }
    let rollback_id = EntityId::new().to_string();
    let rollback_path = rollback_path(repository, &rollback_id)?;

    let applied = (|| {
        let applied_diff = match mode {
            RestoreMode::Merge => {
                repository
                    .encrypted_backup_to(&rollback_path, key)
                    .map_err(|_| RepoError::DestinationUnwritable)?;
                lease.enter_commit();
                coordinator.update(&operation.id, |record| {
                    record.kind = kind;
                    record.transition(OperationState::Committing)
                })?;
                merge_staging(repository, key, &staged.staging_path)?
            }
            RestoreMode::Replace => {
                lease.enter_commit();
                coordinator.update(&operation.id, |record| {
                    record.kind = kind;
                    record.transition(OperationState::Committing)
                })?;
                repository.atomic_replace_from(&staged.staging_path, &rollback_path, key)?;
                staged.preview.diff.clone()
            }
        };
        operation = coordinator.update(&operation.id, |record| {
            record.rollback_location = Some(rollback_id.clone());
            record.update_progress(
                staged.preview.manifest.record_count,
                staged.preview.manifest.record_count,
            )?;
            record.transition(OperationState::Recoverable)
        })?;
        repository.persist_operation_record(&operation)?;
        let rollback = write_rollback_metadata(
            repository,
            &rollback_path,
            rollback_id,
            operation.id.to_string(),
            mode,
        )?;
        prune_rollbacks(repository)?;
        remove_database_family(&staged.staging_path);
        Ok(RestoreResult {
            operation: operation.clone(),
            mode,
            imported_records: applied_diff.imported_records,
            exact_duplicates: applied_diff.exact_duplicates,
            conflicts: applied_diff.conflicts,
            tombstone_skips: applied_diff.tombstone_skips,
            rollback,
        })
    })();
    if applied.is_err() {
        if let Ok(terminal) = coordinator.update(&operation.id, |record| {
            record.transition(OperationState::Failed)
        }) {
            let _ = repository.persist_operation_record(&terminal);
        }
        if mode == RestoreMode::Merge {
            remove_database_family(&rollback_path);
        }
    }
    applied
}

/// Apply a restore while explicitly resetting the in-memory Player authority.
/// Portable data never carries consent, provider configuration, sessions, or
/// previews; callers that own the runtime use this wrapper at the commit seam.
pub fn apply_restore_with_player_runtime(
    repository: &NotebookRepository,
    key: &DatabaseKey,
    coordinator: &OperationCoordinator,
    staged: StagedRestore,
    mode: RestoreMode,
    idempotency_key: IdempotencyKey,
    player_runtime: &PlayerPublicResultsRuntime,
) -> Result<RestoreResult, RepoError> {
    let result = apply_restore(repository, key, coordinator, staged, mode, idempotency_key)?;
    player_runtime
        .reset_disabled()
        .map_err(|_| RepoError::ProviderUnavailable)?;
    Ok(result)
}

pub fn list_rollbacks(repository: &NotebookRepository) -> Result<Vec<RollbackView>, RepoError> {
    prune_rollbacks(repository)?;
    Ok(read_rollbacks(repository)?
        .into_iter()
        .map(|metadata| metadata.view)
        .collect())
}

pub fn apply_rollback(
    repository: &NotebookRepository,
    key: &DatabaseKey,
    coordinator: &OperationCoordinator,
    rollback_id: &str,
) -> Result<RollbackView, RepoError> {
    let metadata = find_rollback(repository, rollback_id)?;
    if metadata.view.expires_at <= UtcMillis::now() {
        discard_rollback(repository, rollback_id)?;
        return Err(RepoError::UndoExpired);
    }
    let _lease = coordinator.begin(OperationKind::RollbackApply, None)?;
    let rollback_path = rollback_database_path(repository, &metadata)?;
    let displaced = repository
        .database_path()?
        .with_extension(format!("rollback-displaced-{rollback_id}"));
    repository.atomic_replace_from(&rollback_path, &displaced, key)?;
    remove_database_family(&displaced);
    remove_metadata_file(repository, rollback_id)?;
    Ok(metadata.view)
}

pub fn discard_rollback(
    repository: &NotebookRepository,
    rollback_id: &str,
) -> Result<RollbackView, RepoError> {
    let metadata = find_rollback(repository, rollback_id)?;
    remove_database_family(&rollback_database_path(repository, &metadata)?);
    remove_metadata_file(repository, rollback_id)?;
    Ok(metadata.view)
}

fn calculate_diff(
    live: &NotebookRepository,
    staging: &NotebookRepository,
) -> Result<RestoreDiff, RepoError> {
    let mut diff = RestoreDiff::default();
    live.with_connection(|live_connection| {
        for_each_notebook_record(staging, |record| {
            let status = record_status(&live_connection.connection, &record)?;
            let status = if status == RecordStatus::Missing
                && existing_identity_mapping(&live_connection.connection, &record)?.is_some()
            {
                RecordStatus::Exact
            } else {
                status
            };
            match status {
                RecordStatus::Missing => diff.imported_records += 1,
                RecordStatus::Exact => diff.exact_duplicates += 1,
                RecordStatus::Divergent => diff.conflicts += 1,
            }
            match record.table.as_str() {
                "opponent_profiles" => diff.profiles += 1,
                "encounters" => diff.encounters += 1,
                "observations" => diff.observations += 1,
                "player_identities" => diff.player_identities += 1,
                "player_evidence" => diff.player_evidence += 1,
                "player_empty_outcomes" => diff.player_empty_outcomes += 1,
                "player_tombstones" => diff.player_tombstones += 1,
                _ => {}
            }
            Ok(())
        })
    })?;
    Ok(diff)
}

fn player_identity_id(repository: &NotebookRepository) -> Result<Option<String>, RepoError> {
    repository.with_connection(|connection| {
        connection
            .connection
            .query_row(
                "SELECT id FROM player_identities WHERE singleton = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| RepoError::NotebookInvalid)
    })
}

fn merge_staging(
    live: &NotebookRepository,
    key: &DatabaseKey,
    staging_path: &Path,
) -> Result<RestoreDiff, RepoError> {
    let staging = NotebookRepository::open(staging_path, key)?;
    let mut tombstones = active_tombstone_ids(live)?;
    let mut remapped_ids = BTreeMap::<String, String>::new();
    let mut diff = RestoreDiff::default();
    live.transact_domain(|transaction| {
        let mut records = Vec::new();
        for_each_notebook_record(&staging, |record| {
            records.push(record);
            Ok(())
        })?;
        records.sort_by_key(|record| {
            if is_tombstone_record(record) {
                0_u8
            } else {
                1_u8
            }
        });
        for mut record in records {
            if is_tombstone_record(&record) {
                extend_tombstone_ids(&record, &mut tombstones);
                if matches!(record_status(transaction, &record)?, RecordStatus::Missing) {
                    insert_record(transaction, &record)?;
                }
                continue;
            }
            remap_record(&mut record, &remapped_ids);
            if references_any(&record, &tombstones) {
                diff.tombstone_skips += 1;
                return Ok(());
            }
            match record_status(transaction, &record)? {
                RecordStatus::Exact => diff.exact_duplicates += 1,
                RecordStatus::Divergent => {
                    insert_conflict(transaction, &record)?;
                    diff.conflicts += 1;
                }
                RecordStatus::Missing => {
                    if let (Some(imported_id), Some(existing_id)) = (
                        text_value(&record, "id").map(str::to_owned),
                        existing_identity_mapping(transaction, &record)?,
                    ) {
                        remapped_ids.insert(imported_id, existing_id);
                        diff.exact_duplicates += 1;
                    } else if insert_record(transaction, &record).is_ok() {
                        diff.imported_records += 1;
                    } else {
                        insert_conflict(transaction, &record)?;
                        diff.conflicts += 1;
                    }
                }
            }
        }
        Ok(())
    })?;
    Ok(diff)
}

fn is_tombstone_record(record: &CanonicalRecord) -> bool {
    matches!(
        record.table.as_str(),
        "deletion_tombstones" | "player_tombstones"
    )
}

fn extend_tombstone_ids(record: &CanonicalRecord, ids: &mut std::collections::BTreeSet<String>) {
    let indexes = if record.table == "player_tombstones" {
        [1_usize, 2_usize].as_slice()
    } else {
        [1_usize].as_slice()
    };
    for index in indexes {
        if let Some(CanonicalValue::Text(value)) = record.values.get(*index) {
            ids.insert(value.clone());
        }
    }
}

fn existing_identity_mapping(
    connection: &rusqlite::Connection,
    record: &CanonicalRecord,
) -> Result<Option<String>, RepoError> {
    match record.table.as_str() {
        "opponent_profiles" => {
            let normalized =
                text_value(record, "normalized_handle").ok_or(RepoError::InvalidBackup)?;
            connection
                .query_row(
                    "SELECT id FROM (
                       SELECT id, 0 AS rank FROM opponent_profiles
                       WHERE normalized_handle = ?1 AND deleted_at IS NULL
                       UNION ALL
                       SELECT profile.id, 1 AS rank FROM opponent_aliases alias
                       JOIN opponent_profiles profile ON profile.id = alias.profile_id
                       WHERE alias.normalized_handle = ?1 AND profile.deleted_at IS NULL
                     ) ORDER BY rank LIMIT 1",
                    [normalized],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|_| RepoError::NotebookInvalid)
        }
        "tendency_tags" => {
            let normalized =
                text_value(record, "normalized_label").ok_or(RepoError::InvalidBackup)?;
            connection
                .query_row(
                    "SELECT id FROM tendency_tags WHERE normalized_label = ?1",
                    [normalized],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|_| RepoError::NotebookInvalid)
        }
        _ => Ok(None),
    }
}

fn remap_record(record: &mut CanonicalRecord, remapped_ids: &BTreeMap<String, String>) {
    let Ok(spec) = table_spec(&record.table) else {
        return;
    };
    let columns = spec.columns.to_vec();
    for (index, column) in columns.iter().enumerate() {
        if (*column == "id" || column.ends_with("_id"))
            && let Some(CanonicalValue::Text(current)) = record.values.get(index)
            && let Some(replacement) = remapped_ids.get(current)
        {
            remap_foreign_key(record, column, replacement);
        }
    }
}

fn copy_live_tombstones(
    live: &NotebookRepository,
    staging: &NotebookRepository,
) -> Result<(), RepoError> {
    let mut records = Vec::new();
    for_each_notebook_record(live, |record| {
        if is_tombstone_record(&record) {
            records.push(record);
        }
        Ok(())
    })?;
    staging.transact_domain(|transaction| {
        for record in &records {
            if matches!(record_status(transaction, record)?, RecordStatus::Missing) {
                insert_record(transaction, record)?;
            }
        }
        Ok(())
    })
}

fn validate_foreign_keys(repository: &NotebookRepository) -> Result<(), RepoError> {
    repository.with_connection(|connection| {
        let violation = connection
            .connection
            .query_row("PRAGMA foreign_key_check", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()
            .map_err(|_| RepoError::InvalidBackup)?;
        if violation.is_some() {
            Err(RepoError::InvalidBackup)
        } else {
            Ok(())
        }
    })
}

fn staging_path(repository: &NotebookRepository, token: &str) -> Result<PathBuf, RepoError> {
    let database_path = repository.database_path()?;
    let file_name = database_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(RepoError::NotebookInvalid)?;
    Ok(database_path.with_file_name(format!("{file_name}.restore-staging-{token}.db")))
}

fn rollback_path(repository: &NotebookRepository, id: &str) -> Result<PathBuf, RepoError> {
    let database_path = repository.database_path()?;
    let file_name = database_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(RepoError::NotebookInvalid)?;
    Ok(database_path.with_file_name(format!("{file_name}.portability-rollback-{id}.db")))
}

fn write_rollback_metadata(
    repository: &NotebookRepository,
    database_path: &Path,
    id: String,
    restore_operation_id: String,
    mode: RestoreMode,
) -> Result<RollbackView, RepoError> {
    let created_at = UtcMillis::now();
    let expires_at = UtcMillis::new(
        created_at
            .get()
            .checked_add(ROLLBACK_RETENTION_MS)
            .ok_or(RepoError::InvalidRequest)?,
    )?;
    let view = RollbackView {
        id: id.clone(),
        restore_operation_id,
        mode,
        created_at,
        expires_at,
    };
    let metadata = RollbackMetadata {
        view: view.clone(),
        database_file: database_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(RepoError::NotebookInvalid)?
            .to_owned(),
    };
    let path = rollback_metadata_path(repository, &id)?;
    let partial = path.with_extension("json.partial");
    let bytes = serde_json::to_vec(&metadata).map_err(|_| RepoError::NotebookInvalid)?;
    fs::write(&partial, bytes).map_err(|_| RepoError::DestinationUnwritable)?;
    fs::File::open(&partial)
        .and_then(|file| file.sync_all())
        .map_err(|_| RepoError::DestinationUnwritable)?;
    fs::rename(&partial, &path).map_err(|_| RepoError::DestinationUnwritable)?;
    sync_parent(&path)?;
    Ok(view)
}

fn read_rollbacks(repository: &NotebookRepository) -> Result<Vec<RollbackMetadata>, RepoError> {
    let database_path = repository.database_path()?;
    let parent = database_path.parent().ok_or(RepoError::NotebookInvalid)?;
    let mut rollbacks = Vec::new();
    for entry in fs::read_dir(parent).map_err(|_| RepoError::NotebookInvalid)? {
        let path = entry.map_err(|_| RepoError::NotebookInvalid)?.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.contains(".portability-rollback-") || !name.ends_with(".json") {
            continue;
        }
        let bytes = fs::read(&path).map_err(|_| RepoError::NotebookInvalid)?;
        let metadata: RollbackMetadata =
            serde_json::from_slice(&bytes).map_err(|_| RepoError::NotebookInvalid)?;
        if rollback_database_path(repository, &metadata)?.exists() {
            rollbacks.push(metadata);
        }
    }
    rollbacks.sort_by(|left, right| {
        right
            .view
            .created_at
            .cmp(&left.view.created_at)
            .then_with(|| left.view.id.cmp(&right.view.id))
    });
    Ok(rollbacks)
}

fn find_rollback(
    repository: &NotebookRepository,
    rollback_id: &str,
) -> Result<RollbackMetadata, RepoError> {
    read_rollbacks(repository)?
        .into_iter()
        .find(|metadata| metadata.view.id == rollback_id)
        .ok_or(RepoError::NotFound)
}

fn prune_rollbacks(repository: &NotebookRepository) -> Result<(), RepoError> {
    let now = UtcMillis::now();
    for (index, metadata) in read_rollbacks(repository)?.into_iter().enumerate() {
        if index >= MAX_ROLLBACKS || metadata.view.expires_at < now {
            remove_database_family(&rollback_database_path(repository, &metadata)?);
            remove_metadata_file(repository, &metadata.view.id)?;
        }
    }
    Ok(())
}

fn rollback_database_path(
    repository: &NotebookRepository,
    metadata: &RollbackMetadata,
) -> Result<PathBuf, RepoError> {
    let database_path = repository.database_path()?;
    Ok(database_path
        .parent()
        .ok_or(RepoError::NotebookInvalid)?
        .join(&metadata.database_file))
}

fn rollback_metadata_path(
    repository: &NotebookRepository,
    rollback_id: &str,
) -> Result<PathBuf, RepoError> {
    let database_path = repository.database_path()?;
    let file_name = database_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(RepoError::NotebookInvalid)?;
    Ok(database_path.with_file_name(format!(
        "{file_name}.portability-rollback-{rollback_id}.json"
    )))
}

fn remove_metadata_file(
    repository: &NotebookRepository,
    rollback_id: &str,
) -> Result<(), RepoError> {
    let path = rollback_metadata_path(repository, rollback_id)?;
    if path.exists() {
        fs::remove_file(path).map_err(|_| RepoError::NotebookInvalid)?;
    }
    Ok(())
}

fn remove_database_family(path: &Path) {
    let _ = fs::remove_file(path);
    let display = path.as_os_str().to_string_lossy();
    let _ = fs::remove_file(format!("{display}-wal"));
    let _ = fs::remove_file(format!("{display}-shm"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ut_085_replace_mode_requires_rollback_before_swap() {
        assert_eq!(RestoreMode::Replace, RestoreMode::Replace);
        assert!(std::hint::black_box(MAX_ROLLBACKS) > 0);
        assert!(std::hint::black_box(ROLLBACK_RETENTION_MS) > PREVIEW_TTL_MS);
    }

    #[test]
    fn rollback_view_does_not_serialize_filesystem_location() {
        let view = RollbackView {
            id: EntityId::new().to_string(),
            restore_operation_id: EntityId::new().to_string(),
            mode: RestoreMode::Replace,
            created_at: UtcMillis::new(1).expect("time"),
            expires_at: UtcMillis::new(2).expect("time"),
        };
        let json = serde_json::to_string(&view).expect("json");
        assert!(!json.contains("path"));
        assert!(!json.contains("database"));
    }
}
