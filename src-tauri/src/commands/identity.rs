use serde::{Deserialize, Serialize};

use crate::domain::{EntityId, IdempotencyKey};
use crate::ipc::{AppError, CallerIdentity, CommandResult, ErrorCode, panic_boundary};
use crate::notebook::NotebookRuntime;
use crate::notebook::repository::NotebookRepository;
use crate::services::identity::{IdentityService, MergePreview, MergeResult, UnmergePreview};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewMergeRequest {
    pub left_profile_id: String,
    pub right_profile_id: String,
    pub primary_profile_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMergeRequest {
    pub preview: MergePreview,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewUnmergeRequest {
    pub merge_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyUnmergeRequest {
    pub preview: UnmergePreview,
    pub idempotency_key: String,
}

pub fn preview_merge_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: PreviewMergeRequest,
) -> CommandResult<MergePreview> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let result = EntityId::parse(request.left_profile_id)
        .and_then(|left| EntityId::parse(request.right_profile_id).map(|right| (left, right)))
        .and_then(|(left, right)| {
            EntityId::parse(request.primary_profile_id).map(|primary| (left, right, primary))
        })
        .and_then(|(left, right, primary)| {
            IdentityService::new(repository).preview_merge(&left, &right, &primary)
        });
    match result {
        Ok(preview) => CommandResult::success(preview, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn apply_merge_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: ApplyMergeRequest,
) -> CommandResult<MergeResult> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let key = match require_idempotency_key(&request.idempotency_key) {
        Ok(key) => key,
        Err(error) => return CommandResult::failure(error),
    };
    match IdentityService::new(repository).apply_merge(&request.preview, &key) {
        Ok(result) => {
            let revision = result.canonical_revision;
            CommandResult::success(result, revision)
        }
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn preview_unmerge_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: PreviewUnmergeRequest,
) -> CommandResult<UnmergePreview> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let result = EntityId::parse(request.merge_id)
        .and_then(|id| IdentityService::new(repository).preview_unmerge(&id));
    match result {
        Ok(preview) => CommandResult::success(preview, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn apply_unmerge_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: ApplyUnmergeRequest,
) -> CommandResult<MergeResult> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let key = match require_idempotency_key(&request.idempotency_key) {
        Ok(key) => key,
        Err(error) => return CommandResult::failure(error),
    };
    match IdentityService::new(repository).apply_unmerge(&request.preview, &key) {
        Ok(result) => {
            let revision = result.canonical_revision;
            CommandResult::success(result, revision)
        }
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

#[tauri::command]
pub fn preview_merge(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: PreviewMergeRequest,
) -> CommandResult<MergePreview> {
    panic_boundary("preview-merge-command", || {
        with_caller(&window, |caller| {
            preview_merge_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn apply_merge(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: ApplyMergeRequest,
) -> CommandResult<MergeResult> {
    panic_boundary("apply-merge-command", || {
        with_caller(&window, |caller| {
            apply_merge_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn preview_unmerge(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: PreviewUnmergeRequest,
) -> CommandResult<UnmergePreview> {
    panic_boundary("preview-unmerge-command", || {
        with_caller(&window, |caller| {
            preview_unmerge_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn apply_unmerge(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: ApplyUnmergeRequest,
) -> CommandResult<MergeResult> {
    panic_boundary("apply-unmerge-command", || {
        with_caller(&window, |caller| {
            apply_unmerge_for(caller, &runtime.repository, request)
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
