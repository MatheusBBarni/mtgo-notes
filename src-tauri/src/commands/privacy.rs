use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use crate::domain::{IdempotencyKey, UtcMillis};
use crate::ipc::{
    AppError, CallerIdentity, CommandResult, ErrorCode, EventName, ReplacementEvent, panic_boundary,
};
use crate::notebook::NotebookRuntime;
use crate::notebook::repository::NotebookRepository;
use crate::operations::{OperationCoordinator, OperationKind};
use crate::portability::PortabilityRuntime;
use crate::services::deletion::{
    DeletionEntityType, DeletionPreview, DeletionResult, DeletionService,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDeletionRequest {
    pub entity_type: DeletionEntityType,
    pub entity_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestDeletionRequest {
    pub preview: DeletionPreview,
    pub confirmation: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoDeletionRequest {
    pub entity_type: DeletionEntityType,
    pub entity_id: String,
    pub undo_token: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionUndoResult {
    pub entity_type: DeletionEntityType,
    pub entity_id: String,
    pub restored: bool,
}

pub fn preview_deletion_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: PreviewDeletionRequest,
) -> CommandResult<DeletionPreview> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match DeletionService::new(repository).preview(request.entity_type, &request.entity_id) {
        Ok(preview) => CommandResult::success(preview, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn request_deletion_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: RequestDeletionRequest,
) -> CommandResult<DeletionResult> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let key = match require_idempotency_key(&request.idempotency_key) {
        Ok(key) => key,
        Err(error) => return CommandResult::failure(error),
    };
    match DeletionService::new(repository).request(&request.preview, &request.confirmation, &key) {
        Ok(result) => CommandResult::success(result, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn request_deletion_coordinated_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    coordinator: &OperationCoordinator,
    request: RequestDeletionRequest,
) -> CommandResult<DeletionResult> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let _lease = match coordinator.begin(OperationKind::Purge, None) {
        Ok(lease) => lease,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    request_deletion_for(caller, repository, request)
}

pub fn undo_deletion_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: UndoDeletionRequest,
    now: UtcMillis,
) -> CommandResult<DeletionUndoResult> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match DeletionService::new(repository).undo(
        request.entity_type,
        &request.entity_id,
        &request.undo_token,
        now,
    ) {
        Ok(()) => CommandResult::success(
            DeletionUndoResult {
                entity_type: request.entity_type,
                entity_id: request.entity_id,
                restored: true,
            },
            1,
        ),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

#[tauri::command]
pub fn preview_deletion(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: PreviewDeletionRequest,
) -> CommandResult<DeletionPreview> {
    panic_boundary("preview-deletion-command", || {
        with_caller(&window, |caller| {
            preview_deletion_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn request_deletion(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    portability: tauri::State<'_, PortabilityRuntime>,
    request: RequestDeletionRequest,
) -> CommandResult<DeletionResult> {
    let result = panic_boundary("request-deletion-command", || {
        with_caller(&window, |caller| {
            request_deletion_coordinated_for(
                caller,
                &runtime.repository,
                &portability.coordinator,
                request,
            )
        })
    });
    if let CommandResult::Success { data, .. } = &result
        && data.tombstone_state == "pending"
    {
        schedule_purge(&window, data.undo_deadline);
    }
    result
}

#[tauri::command]
pub fn undo_deletion(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: UndoDeletionRequest,
) -> CommandResult<DeletionUndoResult> {
    panic_boundary("undo-deletion-command", || {
        with_caller(&window, |caller| {
            undo_deletion_for(caller, &runtime.repository, request, UtcMillis::now())
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

fn with_caller<T>(
    window: &tauri::WebviewWindow,
    operation: impl FnOnce(CallerIdentity) -> CommandResult<T>,
) -> CommandResult<T> {
    match CallerIdentity::from_window_label(window.label()) {
        Ok(caller) => operation(caller),
        Err(error) => CommandResult::failure(error),
    }
}

fn schedule_purge(window: &tauri::WebviewWindow, deadline: i64) {
    let app = window.app_handle().clone();
    let window = window.clone();
    let delay_ms = deadline.saturating_sub(UtcMillis::now().get());
    let _task = tauri::async_runtime::spawn_blocking(move || {
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(
                u64::try_from(delay_ms).unwrap_or_default(),
            ));
        }
        let notebook = app.state::<NotebookRuntime>();
        let portability = app.state::<PortabilityRuntime>();
        if let Ok(operation) = DeletionService::new(&notebook.repository)
            .purge_due_operation(&portability.coordinator, UtcMillis::now())
        {
            let event = ReplacementEvent::v1(
                EventName::OperationProgress,
                operation.revision.get(),
                operation,
            );
            let _ = window.emit("operation://progress-v1", event);
        }
    });
}
