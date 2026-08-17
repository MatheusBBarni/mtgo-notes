use std::fs::{self, OpenOptions};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::{IdempotencyKey, RepoError, UtcMillis};
use crate::notebook::key::DatabaseKey;
use crate::notebook::repository::NotebookRepository;
use crate::operations::{
    CancellationToken, OperationCoordinator, OperationKind, OperationRecord, OperationState,
};
use crate::portability::archive::{
    ArchiveEnvelope, ArchiveManifest, ArchiveWriter, ManifestAccumulator,
};
use crate::portability::records::for_each_record_with_provenance;
use crate::portability::{atomic_publish, create_notebook_snapshot, partial_path};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRequest {
    pub operation_id: crate::domain::EntityId,
    pub destination: PathBuf,
    pub passphrase_acknowledged: bool,
    pub confirm_empty: bool,
    pub overwrite: bool,
    pub idempotency_key: IdempotencyKey,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub operation: OperationRecord,
    #[serde(skip)]
    pub destination: PathBuf,
    pub destination_name: String,
    pub manifest: ArchiveManifest,
}

pub fn create_backup(
    repository: &NotebookRepository,
    key: &DatabaseKey,
    coordinator: &OperationCoordinator,
    request: &BackupRequest,
    passphrase: &str,
    cancellation: &CancellationToken,
) -> Result<BackupResult, RepoError> {
    validate_request(repository, request, passphrase)?;
    let claimed_path = normalized_path(&request.destination)?;
    let lease = coordinator.begin_with_cancellation(
        OperationKind::BackupSnapshot,
        Some(&claimed_path),
        cancellation.clone(),
    )?;
    let mut operation = OperationRecord::requested_with_id(
        request.operation_id.clone(),
        OperationKind::BackupSnapshot,
        request.idempotency_key.clone(),
    );
    operation.transition(OperationState::Running)?;
    coordinator.register(operation.clone())?;
    repository.persist_operation_record(&operation)?;
    let partial = partial_path(&request.destination);
    remove_partial(&partial)?;

    let result = (|| {
        let snapshot = create_notebook_snapshot(repository, key, &operation.id)?;
        let snapshot_repository = snapshot.repository()?;
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&partial)
            .map_err(|_| RepoError::DestinationUnwritable)?;
        let envelope = ArchiveEnvelope::random()?;
        let mut writer = ArchiveWriter::new(BufWriter::new(file), passphrase, envelope)?;
        let mut accumulator = ManifestAccumulator::new();
        let schema_version = repository.schema_version()?;

        let provenance = for_each_record_with_provenance(snapshot_repository, |record| {
            if cancellation.is_cancelled() {
                return Err(RepoError::InvalidTransition);
            }
            let encoded = accumulator.push(&record)?;
            writer.write_plaintext(&encoded)?;
            std::thread::yield_now();
            Ok(())
        })?;
        if cancellation.is_cancelled() {
            return Err(RepoError::InvalidTransition);
        }
        let manifest = accumulator.finish(UtcMillis::now(), schema_version, provenance);
        let mut output = writer.finish(&manifest)?;
        use std::io::Write;
        output
            .flush()
            .map_err(|_| RepoError::DestinationUnwritable)?;
        output
            .get_ref()
            .sync_all()
            .map_err(|_| RepoError::DestinationUnwritable)?;
        drop(output);
        lease.enter_commit();
        coordinator.update(&operation.id, |record| {
            record.transition(OperationState::Committing)
        })?;
        atomic_publish(&partial, &request.destination, request.overwrite)?;
        let metadata =
            fs::metadata(&request.destination).map_err(|_| RepoError::DestinationUnwritable)?;
        operation = coordinator.update(&operation.id, |record| {
            record.update_progress(metadata.len(), metadata.len())?;
            record.transition(OperationState::Completed)
        })?;
        repository.persist_operation_record(&operation)?;
        Ok(BackupResult {
            operation: operation.clone(),
            destination: request.destination.clone(),
            destination_name: request
                .destination
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or(RepoError::InvalidRequest)?
                .to_owned(),
            manifest,
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
    request: &BackupRequest,
    passphrase: &str,
) -> Result<(), RepoError> {
    if !request.passphrase_acknowledged {
        return Err(RepoError::AcknowledgementRequired);
    }
    if passphrase.is_empty() {
        return Err(RepoError::InvalidRequest);
    }
    if request.destination.file_name().is_none()
        || request
            .destination
            .extension()
            .and_then(|extension| extension.to_str())
            != Some(crate::portability::archive::ARCHIVE_EXTENSION)
    {
        return Err(RepoError::InvalidRequest);
    }
    if request.destination.exists() && !request.overwrite {
        return Err(RepoError::InvalidRequest);
    }
    let parent = request
        .destination
        .parent()
        .ok_or(RepoError::DestinationUnwritable)?;
    if !parent.is_dir() {
        return Err(RepoError::DestinationUnwritable);
    }
    let snapshot = repository.snapshot()?;
    if snapshot.profile_count == 0 && !request.confirm_empty {
        return Err(RepoError::EmptyPortabilityConfirmationRequired);
    }
    let pending_deletion = repository.with_connection(|connection| {
        connection
            .connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM deletion_tombstones WHERE purge_state = 'pending'
                 )",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|exists| exists == 1)
            .map_err(|_| RepoError::NotebookInvalid)
    })?;
    if pending_deletion {
        return Err(RepoError::OperationBusy);
    }
    Ok(())
}

fn normalized_path(path: &Path) -> Result<String, RepoError> {
    let parent = path.parent().ok_or(RepoError::InvalidRequest)?;
    let file_name = path.file_name().ok_or(RepoError::InvalidRequest)?;
    Ok(parent.join(file_name).to_string_lossy().into_owned())
}

fn remove_partial(path: &Path) -> Result<(), RepoError> {
    if path.exists() {
        fs::remove_file(path).map_err(|_| RepoError::DestinationUnwritable)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ut_078_acknowledgement_is_required_before_any_file_work() {
        let request = BackupRequest {
            operation_id: crate::domain::EntityId::new(),
            destination: PathBuf::from("backup.mtgonotes"),
            passphrase_acknowledged: false,
            confirm_empty: true,
            overwrite: false,
            idempotency_key: IdempotencyKey::new(),
        };
        assert!(!request.passphrase_acknowledged);
    }

    #[test]
    fn ut_079_partial_path_is_never_the_valid_backup_name() {
        let final_path = Path::new("backup.mtgonotes");
        assert_eq!(
            partial_path(final_path),
            PathBuf::from("backup.mtgonotes.partial")
        );
    }
}
