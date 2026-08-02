pub mod archive;
pub mod backup;
pub mod export;
pub mod records;
pub mod restore;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::domain::{EntityId, RepoError, UtcMillis};
use crate::notebook::key::DatabaseKey;
use crate::notebook::repository::NotebookRepository;
use crate::operations::{CancellationToken, OperationCoordinator};
use crate::portability::restore::StagedRestore;

const SELECTION_TTL_MS: i64 = 10 * 60 * 1000;

pub struct NotebookSnapshot {
    repository: Option<NotebookRepository>,
    path: PathBuf,
}

impl NotebookSnapshot {
    pub fn repository(&self) -> Result<&NotebookRepository, RepoError> {
        self.repository.as_ref().ok_or(RepoError::NotebookInvalid)
    }
}

impl Drop for NotebookSnapshot {
    fn drop(&mut self) {
        drop(self.repository.take());
        remove_database_family(&self.path);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionPurpose {
    BackupDestination,
    RestoreSource,
    ExportDestination,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathSelection {
    pub token: String,
    pub purpose: SelectionPurpose,
    pub display_name: String,
    pub expires_at: UtcMillis,
    #[serde(skip)]
    path: PathBuf,
}

#[derive(Default)]
pub struct PortabilityRuntime {
    pub coordinator: OperationCoordinator,
    selections: Mutex<BTreeMap<String, PathSelection>>,
    previews: Mutex<BTreeMap<String, StagedRestore>>,
    rollback_confirmations: Mutex<BTreeMap<String, (String, UtcMillis)>>,
    cancellations: Mutex<BTreeMap<String, CancellationToken>>,
}

impl PortabilityRuntime {
    pub fn register_selection(
        &self,
        purpose: SelectionPurpose,
        path: PathBuf,
    ) -> Result<PathSelection, RepoError> {
        validate_selected_path(purpose, &path)?;
        let token = EntityId::new().to_string();
        let expires_at = UtcMillis::new(
            UtcMillis::now()
                .get()
                .checked_add(SELECTION_TTL_MS)
                .ok_or(RepoError::InvalidRequest)?,
        )?;
        let selection = PathSelection {
            token: token.clone(),
            purpose,
            display_name: path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or(RepoError::InvalidRequest)?
                .to_owned(),
            expires_at,
            path,
        };
        self.selections
            .lock()
            .map_err(|_| RepoError::OperationBusy)?
            .insert(token, selection.clone());
        Ok(selection)
    }

    pub fn resolve_selection(
        &self,
        token: &str,
        purpose: SelectionPurpose,
    ) -> Result<PathBuf, RepoError> {
        let mut selections = self
            .selections
            .lock()
            .map_err(|_| RepoError::OperationBusy)?;
        let selection = selections.remove(token).ok_or(RepoError::InvalidRequest)?;
        if selection.purpose != purpose || selection.expires_at < UtcMillis::now() {
            return Err(RepoError::InvalidRequest);
        }
        Ok(selection.path)
    }

    pub fn store_preview(&self, staged: StagedRestore) -> Result<(), RepoError> {
        self.previews
            .lock()
            .map_err(|_| RepoError::OperationBusy)?
            .insert(staged.preview.token.clone(), staged);
        Ok(())
    }

    pub fn take_preview(&self, token: &str) -> Result<StagedRestore, RepoError> {
        self.previews
            .lock()
            .map_err(|_| RepoError::OperationBusy)?
            .remove(token)
            .ok_or(RepoError::InvalidRequest)
    }

    pub fn discard_preview_for_operation(&self, operation_id: &str) -> Result<bool, RepoError> {
        let staged = {
            let mut previews = self.previews.lock().map_err(|_| RepoError::OperationBusy)?;
            let token = previews.iter().find_map(|(token, staged)| {
                (staged.preview.operation.id.as_str() == operation_id).then(|| token.clone())
            });
            token.and_then(|token| previews.remove(&token))
        };
        if let Some(staged) = staged {
            crate::portability::restore::discard_staged_restore(staged);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn confirm_rollback(
        &self,
        rollback_id: &str,
        exists: bool,
    ) -> Result<(String, UtcMillis), RepoError> {
        if !exists {
            return Err(RepoError::NotFound);
        }
        let token = EntityId::new().to_string();
        let expires_at = UtcMillis::new(
            UtcMillis::now()
                .get()
                .checked_add(5 * 60 * 1000)
                .ok_or(RepoError::InvalidRequest)?,
        )?;
        self.rollback_confirmations
            .lock()
            .map_err(|_| RepoError::OperationBusy)?
            .insert(token.clone(), (rollback_id.to_owned(), expires_at));
        Ok((token, expires_at))
    }

    pub fn consume_rollback_confirmation(
        &self,
        token: &str,
        rollback_id: &str,
    ) -> Result<(), RepoError> {
        let (confirmed_id, expires_at) = self
            .rollback_confirmations
            .lock()
            .map_err(|_| RepoError::OperationBusy)?
            .remove(token)
            .ok_or(RepoError::InvalidRequest)?;
        if confirmed_id != rollback_id || expires_at < UtcMillis::now() {
            return Err(RepoError::InvalidRequest);
        }
        Ok(())
    }

    pub fn register_cancellation(
        &self,
        operation_id: &str,
        cancellation: CancellationToken,
    ) -> Result<(), RepoError> {
        self.cancellations
            .lock()
            .map_err(|_| RepoError::OperationBusy)?
            .insert(operation_id.to_owned(), cancellation);
        Ok(())
    }

    pub fn cancel(&self, operation_id: &str) -> Result<(), RepoError> {
        self.cancellations
            .lock()
            .map_err(|_| RepoError::OperationBusy)?
            .get(operation_id)
            .ok_or(RepoError::NotFound)?
            .cancel()
    }

    pub fn remove_cancellation(&self, operation_id: &str) {
        if let Ok(mut cancellations) = self.cancellations.lock() {
            cancellations.remove(operation_id);
        }
    }
}

pub fn create_notebook_snapshot(
    repository: &NotebookRepository,
    key: &DatabaseKey,
    operation_id: &EntityId,
) -> Result<NotebookSnapshot, RepoError> {
    let database_path = repository.database_path()?;
    let file_name = database_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(RepoError::NotebookInvalid)?;
    let path = database_path.with_file_name(format!(
        "{file_name}.portability-snapshot-{operation_id}.db"
    ));
    remove_database_family(&path);
    let independent_source = NotebookRepository::open(&database_path, key)?;
    independent_source
        .encrypted_backup_to(&path, key)
        .map_err(|_| RepoError::NotebookInvalid)?;
    drop(independent_source);
    let snapshot_repository = NotebookRepository::open(&path, key)?;
    Ok(NotebookSnapshot {
        repository: Some(snapshot_repository),
        path,
    })
}

pub fn cleanup_transient_files(repository: &NotebookRepository) -> Result<(), RepoError> {
    let database_path = repository.database_path()?;
    let parent = database_path.parent().ok_or(RepoError::NotebookInvalid)?;
    let file_name = database_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(RepoError::NotebookInvalid)?;
    let transient_prefixes = [
        format!("{file_name}.portability-snapshot-"),
        format!("{file_name}.restore-staging-"),
    ];
    for entry in fs::read_dir(parent).map_err(|_| RepoError::NotebookInvalid)? {
        let path = entry.map_err(|_| RepoError::NotebookInvalid)?.path();
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|candidate| {
                transient_prefixes
                    .iter()
                    .any(|prefix| candidate.starts_with(prefix))
            })
        {
            remove_database_family(&path);
        }
    }
    Ok(())
}

pub fn partial_path(final_path: &Path) -> PathBuf {
    let mut value = final_path.as_os_str().to_os_string();
    value.push(".partial");
    PathBuf::from(value)
}

pub fn atomic_publish(partial: &Path, final_path: &Path, overwrite: bool) -> Result<(), RepoError> {
    if !partial.is_file() {
        return Err(RepoError::DestinationUnwritable);
    }
    let previous = final_path.with_extension(format!(
        "{}.previous-{}",
        final_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("file"),
        EntityId::new()
    ));
    if final_path.exists() {
        if !overwrite {
            return Err(RepoError::InvalidRequest);
        }
        fs::rename(final_path, &previous).map_err(|_| RepoError::DestinationUnwritable)?;
    }
    if fs::rename(partial, final_path).is_err() {
        if previous.exists() {
            let _ = fs::rename(&previous, final_path);
        }
        return Err(RepoError::DestinationUnwritable);
    }
    if previous.exists() {
        fs::remove_file(previous).map_err(|_| RepoError::DestinationUnwritable)?;
    }
    sync_parent(final_path)
}

pub fn sync_parent(path: &Path) -> Result<(), RepoError> {
    let parent = path.parent().ok_or(RepoError::DestinationUnwritable)?;
    #[cfg(unix)]
    {
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| RepoError::DestinationUnwritable)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;

        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| RepoError::DestinationUnwritable)
    }
}

fn remove_database_family(path: &Path) {
    let _ = fs::remove_file(path);
    let display = path.as_os_str().to_string_lossy();
    let _ = fs::remove_file(format!("{display}-wal"));
    let _ = fs::remove_file(format!("{display}-shm"));
}

fn validate_selected_path(purpose: SelectionPurpose, path: &Path) -> Result<(), RepoError> {
    let expected_extension = match purpose {
        SelectionPurpose::BackupDestination | SelectionPurpose::RestoreSource => {
            archive::ARCHIVE_EXTENSION
        }
        SelectionPurpose::ExportDestination => "txt",
    };
    if path.extension().and_then(|value| value.to_str()) != Some(expected_extension)
        || path.file_name().is_none()
    {
        return Err(RepoError::InvalidRequest);
    }
    match purpose {
        SelectionPurpose::RestoreSource if !path.is_file() => Err(RepoError::NotFound),
        SelectionPurpose::BackupDestination | SelectionPurpose::ExportDestination
            if !path.parent().is_some_and(Path::is_dir) =>
        {
            Err(RepoError::DestinationUnwritable)
        }
        _ => Ok(()),
    }
}
