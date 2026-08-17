use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use zeroize::Zeroize;

use crate::domain::{EntityId, IdempotencyKey, RepoError, UtcMillis};
use crate::ipc::{
    AppError, CallerIdentity, CommandResult, ErrorCode, EventName, ReplacementEvent, panic_boundary,
};
use crate::notebook::NotebookRuntime;
use crate::operations::{CancellationToken, OperationRecord};
use crate::portability::backup::{BackupRequest, BackupResult, create_backup};
use crate::portability::export::{ExportRequest, ExportResult, ExportScope, create_export};
use crate::portability::restore::{
    RestoreMode, RestorePreview, RestorePreviewInput, RestoreResult, RollbackView,
    apply_restore as apply_staged_restore, discard_rollback, list_rollbacks,
    preview_restore as stage_restore,
};
use crate::portability::{PathSelection, PortabilityRuntime, SelectionPurpose};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectPathRequest {
    pub purpose: SelectionPurpose,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartBackupRequest {
    pub operation_id: String,
    pub selection_token: String,
    pub passphrase: String,
    pub passphrase_acknowledged: bool,
    pub confirm_empty: bool,
    pub overwrite: bool,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRestoreRequest {
    pub operation_id: String,
    pub idempotency_key: String,
    pub selection_token: String,
    pub passphrase: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRestoreRequest {
    pub preview_token: String,
    pub mode: RestoreMode,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartExportRequest {
    pub operation_id: String,
    pub selection_token: String,
    pub scope: ExportScope,
    pub plaintext_acknowledged: bool,
    pub confirm_empty: bool,
    pub unsaved_edits_resolved: bool,
    pub overwrite: bool,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRequest {
    pub operation_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackRequest {
    pub rollback_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedRollbackRequest {
    pub rollback_id: String,
    pub confirmation_token: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackConfirmation {
    pub rollback: RollbackView,
    pub confirmation_token: String,
    pub expires_at: UtcMillis,
}

pub fn register_path_for(
    caller: CallerIdentity,
    runtime: &PortabilityRuntime,
    purpose: SelectionPurpose,
    path: PathBuf,
) -> CommandResult<PathSelection> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match runtime.register_selection(purpose, path) {
        Ok(selection) => CommandResult::success(selection, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn start_backup_for(
    caller: CallerIdentity,
    notebook: &NotebookRuntime,
    runtime: &PortabilityRuntime,
    request: StartBackupRequest,
) -> CommandResult<BackupResult> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let destination = match runtime.resolve_selection(
        &request.selection_token,
        SelectionPurpose::BackupDestination,
    ) {
        Ok(path) => path,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let idempotency_key = match require_idempotency_key(&request.idempotency_key) {
        Ok(key) => key,
        Err(error) => return CommandResult::failure(error),
    };
    let operation_id = match require_operation_id(&request.operation_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error),
    };
    let backup_request = BackupRequest {
        operation_id: operation_id.clone(),
        destination,
        passphrase_acknowledged: request.passphrase_acknowledged,
        confirm_empty: request.confirm_empty,
        overwrite: request.overwrite,
        idempotency_key,
    };
    let cancellation = CancellationToken::new();
    if let Err(error) = runtime.register_cancellation(operation_id.as_str(), cancellation.clone()) {
        return CommandResult::failure(error.to_app_error());
    }
    let mut passphrase = request.passphrase;
    let result = create_backup(
        &notebook.repository,
        &notebook.key,
        &runtime.coordinator,
        &backup_request,
        &passphrase,
        &cancellation,
    );
    passphrase.zeroize();
    runtime.remove_cancellation(operation_id.as_str());
    match result {
        Ok(result) => CommandResult::success(result, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn preview_restore_for(
    caller: CallerIdentity,
    notebook: &NotebookRuntime,
    runtime: &PortabilityRuntime,
    request: PreviewRestoreRequest,
) -> CommandResult<RestorePreview> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let archive_path = match runtime
        .resolve_selection(&request.selection_token, SelectionPurpose::RestoreSource)
    {
        Ok(path) => path,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let operation_id = match require_operation_id(&request.operation_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error),
    };
    let idempotency_key = match require_idempotency_key(&request.idempotency_key) {
        Ok(key) => key,
        Err(error) => return CommandResult::failure(error),
    };
    let cancellation = CancellationToken::new();
    if let Err(error) = runtime.register_cancellation(operation_id.as_str(), cancellation.clone()) {
        return CommandResult::failure(error.to_app_error());
    }
    let mut passphrase = request.passphrase;
    let result = stage_restore(
        &notebook.repository,
        &notebook.key,
        &runtime.coordinator,
        RestorePreviewInput {
            operation_id: operation_id.clone(),
            idempotency_key,
            archive_path: &archive_path,
            passphrase: &passphrase,
            cancellation: &cancellation,
        },
    );
    passphrase.zeroize();
    match result {
        Ok(staged) => {
            let preview = staged.preview.clone();
            if let Err(error) = runtime.store_preview(staged) {
                runtime.remove_cancellation(operation_id.as_str());
                return CommandResult::failure(error.to_app_error());
            }
            CommandResult::success(preview, 1)
        }
        Err(error) => {
            runtime.remove_cancellation(operation_id.as_str());
            CommandResult::failure(error.to_app_error())
        }
    }
}

pub fn apply_restore_for(
    caller: CallerIdentity,
    notebook: &NotebookRuntime,
    runtime: &PortabilityRuntime,
    request: ApplyRestoreRequest,
) -> CommandResult<RestoreResult> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let staged = match runtime.take_preview(&request.preview_token) {
        Ok(staged) => staged,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    runtime.remove_cancellation(staged.preview.operation.id.as_str());
    let idempotency_key = match require_idempotency_key(&request.idempotency_key) {
        Ok(key) => key,
        Err(error) => return CommandResult::failure(error),
    };
    match apply_staged_restore(
        &notebook.repository,
        &notebook.key,
        &runtime.coordinator,
        staged,
        request.mode,
        idempotency_key,
    ) {
        Ok(result) => CommandResult::success(result, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn start_export_for(
    caller: CallerIdentity,
    notebook: &NotebookRuntime,
    runtime: &PortabilityRuntime,
    request: StartExportRequest,
) -> CommandResult<ExportResult> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let destination = match runtime.resolve_selection(
        &request.selection_token,
        SelectionPurpose::ExportDestination,
    ) {
        Ok(path) => path,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let idempotency_key = match require_idempotency_key(&request.idempotency_key) {
        Ok(key) => key,
        Err(error) => return CommandResult::failure(error),
    };
    let operation_id = match require_operation_id(&request.operation_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error),
    };
    let export_request = ExportRequest {
        operation_id: operation_id.clone(),
        destination,
        scope: request.scope,
        plaintext_acknowledged: request.plaintext_acknowledged,
        confirm_empty: request.confirm_empty,
        unsaved_edits_resolved: request.unsaved_edits_resolved,
        overwrite: request.overwrite,
        idempotency_key,
    };
    let cancellation = CancellationToken::new();
    if let Err(error) = runtime.register_cancellation(operation_id.as_str(), cancellation.clone()) {
        return CommandResult::failure(error.to_app_error());
    }
    let result = create_export(
        &notebook.repository,
        &notebook.key,
        &runtime.coordinator,
        &export_request,
        &cancellation,
    );
    runtime.remove_cancellation(operation_id.as_str());
    match result {
        Ok(result) => CommandResult::success(result, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn cancel_operation_for(
    caller: CallerIdentity,
    notebook: &NotebookRuntime,
    runtime: &PortabilityRuntime,
    request: OperationRequest,
) -> CommandResult<OperationRecord> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let operation_id = match EntityId::parse(request.operation_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    if let Err(error) = runtime.cancel(operation_id.as_str()) {
        return CommandResult::failure(error.to_app_error());
    }
    if matches!(
        runtime.coordinator.get(&operation_id),
        Ok(OperationRecord {
            state: crate::operations::OperationState::AwaitingConfirmation,
            ..
        })
    ) {
        let _ = runtime.discard_preview_for_operation(operation_id.as_str());
        runtime.remove_cancellation(operation_id.as_str());
        match runtime.coordinator.update(&operation_id, |record| {
            record.transition(crate::operations::OperationState::Cancelled)
        }) {
            Ok(operation) => {
                let _ = notebook.repository.persist_operation_record(&operation);
                return CommandResult::success(operation, 1);
            }
            Err(error) => return CommandResult::failure(error.to_app_error()),
        }
    }
    match runtime.coordinator.get(&operation_id) {
        Ok(operation) => CommandResult::success(operation, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn get_operation_for(
    caller: CallerIdentity,
    runtime: &PortabilityRuntime,
    request: OperationRequest,
) -> CommandResult<OperationRecord> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let operation_id = match EntityId::parse(request.operation_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    match runtime.coordinator.get(&operation_id) {
        Ok(operation) => CommandResult::success(operation, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn list_rollbacks_for(
    caller: CallerIdentity,
    notebook: &NotebookRuntime,
) -> CommandResult<Vec<RollbackView>> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match list_rollbacks(&notebook.repository) {
        Ok(rollbacks) => CommandResult::success(rollbacks, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn confirm_rollback_for(
    caller: CallerIdentity,
    notebook: &NotebookRuntime,
    runtime: &PortabilityRuntime,
    request: RollbackRequest,
) -> CommandResult<RollbackConfirmation> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let rollback = match list_rollbacks(&notebook.repository).and_then(|rollbacks| {
        rollbacks
            .into_iter()
            .find(|rollback| rollback.id == request.rollback_id)
            .ok_or(RepoError::NotFound)
    }) {
        Ok(rollback) => rollback,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    match runtime.confirm_rollback(&rollback.id, true) {
        Ok((confirmation_token, expires_at)) => CommandResult::success(
            RollbackConfirmation {
                rollback,
                confirmation_token,
                expires_at,
            },
            1,
        ),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn apply_rollback_for(
    caller: CallerIdentity,
    notebook: &NotebookRuntime,
    runtime: &PortabilityRuntime,
    request: ConfirmedRollbackRequest,
) -> CommandResult<RollbackView> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    if let Err(error) =
        runtime.consume_rollback_confirmation(&request.confirmation_token, &request.rollback_id)
    {
        return CommandResult::failure(error.to_app_error());
    }
    match crate::portability::restore::apply_rollback(
        &notebook.repository,
        &notebook.key,
        &runtime.coordinator,
        &request.rollback_id,
    ) {
        Ok(rollback) => CommandResult::success(rollback, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn discard_rollback_for(
    caller: CallerIdentity,
    notebook: &NotebookRuntime,
    runtime: &PortabilityRuntime,
    request: ConfirmedRollbackRequest,
) -> CommandResult<RollbackView> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    if let Err(error) =
        runtime.consume_rollback_confirmation(&request.confirmation_token, &request.rollback_id)
    {
        return CommandResult::failure(error.to_app_error());
    }
    match discard_rollback(&notebook.repository, &request.rollback_id) {
        Ok(rollback) => CommandResult::success(rollback, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

#[tauri::command]
pub fn select_portability_path(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, PortabilityRuntime>,
    request: SelectPathRequest,
) -> CommandResult<Option<PathSelection>> {
    panic_boundary("select-portability-path-command", || {
        with_caller(&window, |caller| {
            if let Err(error) = caller.require(&[CallerIdentity::Main]) {
                return CommandResult::failure(error);
            }
            #[cfg(windows)]
            let path = {
                let dialog = match request.purpose {
                    SelectionPurpose::BackupDestination => rfd::FileDialog::new()
                        .add_filter("MTGO Notes backup", &["mtgonotes"])
                        .set_file_name("mtgo-notes-backup.mtgonotes"),
                    SelectionPurpose::RestoreSource => {
                        rfd::FileDialog::new().add_filter("MTGO Notes backup", &["mtgonotes"])
                    }
                    SelectionPurpose::ExportDestination => rfd::FileDialog::new()
                        .add_filter("Plain text", &["txt"])
                        .set_file_name("mtgo-opponent-notes.txt"),
                };
                match request.purpose {
                    SelectionPurpose::RestoreSource => dialog.pick_file(),
                    SelectionPurpose::BackupDestination | SelectionPurpose::ExportDestination => {
                        dialog.save_file()
                    }
                }
            };
            #[cfg(not(windows))]
            let path: Option<PathBuf> = None;

            match path {
                Some(path) => match runtime.register_selection(request.purpose, path) {
                    Ok(selection) => CommandResult::success(Some(selection), 1),
                    Err(error) => CommandResult::failure(error.to_app_error()),
                },
                None => CommandResult::success(None, 1),
            }
        })
    })
}

#[tauri::command]
pub async fn start_backup(
    window: tauri::WebviewWindow,
    request: StartBackupRequest,
) -> CommandResult<BackupResult> {
    let caller = match CallerIdentity::from_window_label(window.label()) {
        Ok(caller) => caller,
        Err(error) => return CommandResult::failure(error),
    };
    let app = window.app_handle().clone();
    let result = match tauri::async_runtime::spawn_blocking(move || {
        panic_boundary("start-backup-command", || {
            let notebook = app.state::<NotebookRuntime>();
            let runtime = app.state::<PortabilityRuntime>();
            start_backup_for(caller, &notebook, &runtime, request)
        })
    })
    .await
    {
        Ok(result) => result,
        Err(_) => CommandResult::failure(AppError::internal("start-backup-task")),
    };
    if let CommandResult::Success { data, .. } = &result {
        emit_operation_progress(&window, &data.operation);
    }
    result
}

#[tauri::command]
pub async fn preview_restore(
    window: tauri::WebviewWindow,
    request: PreviewRestoreRequest,
) -> CommandResult<RestorePreview> {
    let caller = match CallerIdentity::from_window_label(window.label()) {
        Ok(caller) => caller,
        Err(error) => return CommandResult::failure(error),
    };
    let app = window.app_handle().clone();
    let result = match tauri::async_runtime::spawn_blocking(move || {
        panic_boundary("preview-restore-command", || {
            let notebook = app.state::<NotebookRuntime>();
            let runtime = app.state::<PortabilityRuntime>();
            preview_restore_for(caller, &notebook, &runtime, request)
        })
    })
    .await
    {
        Ok(result) => result,
        Err(_) => CommandResult::failure(AppError::internal("preview-restore-task")),
    };
    if let CommandResult::Success { data, .. } = &result {
        emit_operation_progress(&window, &data.operation);
    }
    result
}

#[tauri::command]
pub async fn apply_restore(
    window: tauri::WebviewWindow,
    request: ApplyRestoreRequest,
) -> CommandResult<RestoreResult> {
    let caller = match CallerIdentity::from_window_label(window.label()) {
        Ok(caller) => caller,
        Err(error) => return CommandResult::failure(error),
    };
    let app = window.app_handle().clone();
    let result = match tauri::async_runtime::spawn_blocking(move || {
        panic_boundary("apply-restore-command", || {
            let notebook = app.state::<NotebookRuntime>();
            let runtime = app.state::<PortabilityRuntime>();
            let result = apply_restore_for(caller, &notebook, &runtime, request);
            if matches!(&result, CommandResult::Success { .. }) {
                let enrichment = app.state::<crate::commands::classifier::DeckEnrichmentRuntime>();
                if let Ok(assets) = enrichment.assets.current() {
                    let _ = enrichment
                        .reclassification
                        .start(&notebook.repository, &assets);
                }
            }
            result
        })
    })
    .await
    {
        Ok(result) => result,
        Err(_) => CommandResult::failure(AppError::internal("apply-restore-task")),
    };
    if let CommandResult::Success { data, .. } = &result {
        emit_operation_progress(&window, &data.operation);
    }
    result
}

#[tauri::command]
pub async fn start_export(
    window: tauri::WebviewWindow,
    request: StartExportRequest,
) -> CommandResult<ExportResult> {
    let caller = match CallerIdentity::from_window_label(window.label()) {
        Ok(caller) => caller,
        Err(error) => return CommandResult::failure(error),
    };
    let app = window.app_handle().clone();
    let result = match tauri::async_runtime::spawn_blocking(move || {
        panic_boundary("start-export-command", || {
            let notebook = app.state::<NotebookRuntime>();
            let runtime = app.state::<PortabilityRuntime>();
            start_export_for(caller, &notebook, &runtime, request)
        })
    })
    .await
    {
        Ok(result) => result,
        Err(_) => CommandResult::failure(AppError::internal("start-export-task")),
    };
    if let CommandResult::Success { data, .. } = &result {
        emit_operation_progress(&window, &data.operation);
    }
    result
}

#[tauri::command]
pub fn cancel_operation(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, PortabilityRuntime>,
    request: OperationRequest,
) -> CommandResult<OperationRecord> {
    panic_boundary("cancel-operation-command", || {
        with_caller(&window, |caller| {
            cancel_operation_for(caller, &notebook, &runtime, request)
        })
    })
}

#[tauri::command]
pub fn get_operation(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, PortabilityRuntime>,
    request: OperationRequest,
) -> CommandResult<OperationRecord> {
    panic_boundary("get-operation-command", || {
        with_caller(&window, |caller| {
            get_operation_for(caller, &runtime, request)
        })
    })
}

#[tauri::command]
pub fn list_portability_rollbacks(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
) -> CommandResult<Vec<RollbackView>> {
    panic_boundary("list-portability-rollbacks-command", || {
        with_caller(&window, |caller| list_rollbacks_for(caller, &notebook))
    })
}

#[tauri::command]
pub fn confirm_portability_rollback(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, PortabilityRuntime>,
    request: RollbackRequest,
) -> CommandResult<RollbackConfirmation> {
    panic_boundary("confirm-portability-rollback-command", || {
        with_caller(&window, |caller| {
            confirm_rollback_for(caller, &notebook, &runtime, request)
        })
    })
}

#[tauri::command]
pub fn apply_portability_rollback(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, PortabilityRuntime>,
    request: ConfirmedRollbackRequest,
) -> CommandResult<RollbackView> {
    panic_boundary("apply-portability-rollback-command", || {
        with_caller(&window, |caller| {
            apply_rollback_for(caller, &notebook, &runtime, request)
        })
    })
}

#[tauri::command]
pub fn discard_portability_rollback(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, PortabilityRuntime>,
    request: ConfirmedRollbackRequest,
) -> CommandResult<RollbackView> {
    panic_boundary("discard-portability-rollback-command", || {
        with_caller(&window, |caller| {
            discard_rollback_for(caller, &notebook, &runtime, request)
        })
    })
}

fn require_idempotency_key(value: &str) -> Result<IdempotencyKey, AppError> {
    IdempotencyKey::parse(value).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidRequest,
            "A valid idempotency key is required.",
            false,
        )
        .with_field("idempotencyKey")
    })
}

fn require_operation_id(value: &str) -> Result<EntityId, AppError> {
    EntityId::parse(value).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidRequest,
            "A valid operation ID is required.",
            false,
        )
        .with_field("operationId")
    })
}

fn emit_operation_progress(window: &tauri::WebviewWindow, operation: &OperationRecord) {
    let event = ReplacementEvent::v1(
        EventName::OperationProgress,
        operation.revision.get(),
        operation.clone(),
    );
    let _ = window.emit("operation://progress-v1", event);
}

fn with_caller<T>(
    window: &tauri::WebviewWindow,
    operation: impl FnOnce(CallerIdentity) -> CommandResult<T>,
) -> CommandResult<T> {
    match CallerIdentity::from_window_label(window.label()) {
        Ok(caller) => operation(caller),
        Err(error) => CommandResult::failure(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_confirmation_does_not_contain_a_path_or_secret() {
        let view = RollbackView {
            id: EntityId::new().to_string(),
            restore_operation_id: EntityId::new().to_string(),
            mode: RestoreMode::Replace,
            created_at: UtcMillis::new(1).expect("time"),
            expires_at: UtcMillis::new(2).expect("time"),
        };
        let confirmation = RollbackConfirmation {
            rollback: view,
            confirmation_token: EntityId::new().to_string(),
            expires_at: UtcMillis::new(2).expect("time"),
        };
        let json = serde_json::to_string(&confirmation).expect("json");
        assert!(!json.contains("path"));
        assert!(!json.contains("passphrase"));
    }
}
