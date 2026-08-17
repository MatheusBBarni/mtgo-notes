use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use tauri::{Emitter, Manager};

use crate::commands::notes::require_idempotency_key;
use crate::detection::{AuthorizedWindow, DetectionRuntime, ProviderStatus};
use crate::domain::{RepoError, UtcMillis};
use crate::ipc::{
    CallerIdentity, CommandResult, EventName, ReplacementEvent, next_event_revision, panic_boundary,
};
use crate::notebook::{NotebookRuntime, repository::NotebookRepository};

const PROVIDER_ID: &str = "windows_visible_mtgo";
const MAX_IDEMPOTENCY_KEYS: usize = 64;
const DISCLOSED_FIELDS: [&str; 3] = [
    "visible opponent handle",
    "visible match phase",
    "visible format, game, and result labels",
];

pub fn hydrate_provider_consent(
    runtime: &DetectionRuntime,
    repository: &NotebookRepository,
) -> Result<(), RepoError> {
    let Some((granted, disclosed_fields_json)) = repository.provider_consent(PROVIDER_ID)? else {
        return Ok(());
    };
    let disclosed_fields: Vec<String> =
        serde_json::from_str(&disclosed_fields_json).map_err(|_| RepoError::NotebookInvalid)?;
    if disclosed_fields
        .iter()
        .any(|field| !DISCLOSED_FIELDS.contains(&field.as_str()))
    {
        return Err(RepoError::NotebookInvalid);
    }
    runtime
        .engine
        .lock()
        .map_err(|_| RepoError::ProviderUnavailable)?
        .set_consent(granted, disclosed_fields);
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetProviderConsentRequest {
    pub provider_id: String,
    pub disclosure_version: u16,
    pub disclosed_fields: Vec<String>,
    pub granted: bool,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectMtgoWindowRequest {
    pub native_handle: u64,
    pub class_name: String,
    pub visible_title: String,
    pub visible: bool,
    pub minimized: bool,
    pub usable_bounds: bool,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseDetectionRequest {
    pub paused: bool,
    pub idempotency_key: String,
}

pub fn list_providers_for(
    caller: CallerIdentity,
    runtime: &DetectionRuntime,
) -> CommandResult<Vec<ProviderStatus>> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match current(runtime) {
        Ok((status, revision)) => CommandResult::success(vec![status], revision),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn list_mtgo_windows_for(caller: CallerIdentity) -> CommandResult<Vec<AuthorizedWindow>> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    #[cfg(windows)]
    let windows = match crate::detection::windows::list_visible_mtgo_windows() {
        Ok(windows) => windows,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    #[cfg(not(windows))]
    let windows = Vec::new();
    CommandResult::success(windows, 1)
}

pub fn set_provider_consent_for(
    caller: CallerIdentity,
    runtime: &DetectionRuntime,
    repository: Option<&NotebookRepository>,
    request: SetProviderConsentRequest,
) -> CommandResult<ProviderStatus> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    if request.provider_id != PROVIDER_ID
        || request.disclosure_version != 1
        || request.disclosed_fields.is_empty()
        || request
            .disclosed_fields
            .iter()
            .any(|field| !DISCLOSED_FIELDS.contains(&field.as_str()))
        || request
            .disclosed_fields
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != request.disclosed_fields.len()
    {
        return CommandResult::failure(RepoError::InvalidRequest.to_app_error());
    }
    if let Some(result) = replay(runtime, &request.idempotency_key) {
        return result;
    }
    if let Some(repository) = repository
        && let Err(error) = repository.set_provider_consent(
            PROVIDER_ID,
            request.granted,
            &serde_json::to_string(&request.disclosed_fields).unwrap_or_else(|_| "[]".to_owned()),
            UtcMillis::now(),
        )
    {
        return CommandResult::failure(error.to_app_error());
    }
    let result = mutate(runtime, &request.idempotency_key, |engine| {
        engine.set_consent(request.granted, request.disclosed_fields);
        Ok(())
    });
    status_result(result)
}

pub fn select_mtgo_window_for(
    caller: CallerIdentity,
    runtime: &DetectionRuntime,
    request: SelectMtgoWindowRequest,
) -> CommandResult<ProviderStatus> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    if let Some(result) = replay(runtime, &request.idempotency_key) {
        return result;
    }
    let window = AuthorizedWindow {
        native_handle: request.native_handle,
        class_name: request.class_name,
        visible_title: request.visible_title,
        selected_at: UtcMillis::now().get(),
        visible: request.visible,
        minimized: request.minimized,
        usable_bounds: request.usable_bounds,
    };
    if window.class_name.trim().is_empty()
        || window.visible_title.trim().is_empty()
        || window
            .class_name
            .chars()
            .chain(window.visible_title.chars())
            .any(char::is_control)
    {
        return CommandResult::failure(RepoError::WindowNotFound.to_app_error());
    }
    #[cfg(windows)]
    if let Err(error) = crate::detection::windows::validate_selected_window(&window) {
        return CommandResult::failure(error.to_app_error());
    }
    let result = mutate(runtime, &request.idempotency_key, |engine| {
        engine.select_window(window).map(|_| ())
    });
    status_result(result)
}

pub fn pause_detection_for(
    caller: CallerIdentity,
    runtime: &DetectionRuntime,
    request: PauseDetectionRequest,
) -> CommandResult<ProviderStatus> {
    if let Err(error) = caller.require(&[CallerIdentity::Main, CallerIdentity::Overlay]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    if let Some(result) = replay(runtime, &request.idempotency_key) {
        return result;
    }
    let result = mutate(runtime, &request.idempotency_key, |engine| {
        engine.pause(request.paused)
    });
    status_result(result)
}

#[tauri::command]
pub fn list_providers(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, DetectionRuntime>,
) -> CommandResult<Vec<ProviderStatus>> {
    panic_boundary("list-providers-command", || {
        with_caller(&window, |caller| {
            list_providers_for(caller, runtime.inner())
        })
    })
}

#[tauri::command]
pub fn list_mtgo_windows(window: tauri::WebviewWindow) -> CommandResult<Vec<AuthorizedWindow>> {
    panic_boundary("list-mtgo-windows-command", || {
        with_caller(&window, list_mtgo_windows_for)
    })
}

#[tauri::command]
pub fn set_provider_consent(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, DetectionRuntime>,
    notebook: tauri::State<'_, NotebookRuntime>,
    request: SetProviderConsentRequest,
) -> CommandResult<ProviderStatus> {
    panic_boundary("set-provider-consent-command", || {
        let revoked = !request.granted;
        let result = with_caller(&window, |caller| {
            set_provider_consent_for(caller, runtime.inner(), Some(&notebook.repository), request)
        });
        if result.is_success() && revoked {
            restrict_disclosure(&window, &notebook.repository, "provider_revoked");
        }
        emit_provider_status(&window, &result);
        result
    })
}

#[tauri::command]
pub fn select_mtgo_window(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, DetectionRuntime>,
    notebook: tauri::State<'_, NotebookRuntime>,
    request: SelectMtgoWindowRequest,
) -> CommandResult<ProviderStatus> {
    panic_boundary("select-mtgo-window-command", || {
        #[cfg(windows)]
        let native_handle = request.native_handle;
        let result = with_caller(&window, |caller| {
            select_mtgo_window_for(caller, runtime.inner(), request)
        });
        if result.is_success() {
            restrict_disclosure(&window, &notebook.repository, "provider_generation_changed");
            #[cfg(windows)]
            if let CommandResult::Success { data, .. } = &result
                && crate::detection::windows::spawn_detection_worker(
                    window.app_handle().clone(),
                    native_handle,
                    data.generation,
                )
                .is_err()
            {
                restrict_disclosure(&window, &notebook.repository, "provider_unavailable");
            }
        }
        emit_provider_status(&window, &result);
        result
    })
}

#[tauri::command]
pub fn pause_detection(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, DetectionRuntime>,
    notebook: tauri::State<'_, NotebookRuntime>,
    request: PauseDetectionRequest,
) -> CommandResult<ProviderStatus> {
    panic_boundary("pause-detection-command", || {
        let paused = request.paused;
        let result = with_caller(&window, |caller| {
            pause_detection_for(caller, runtime.inner(), request)
        });
        if result.is_success() && paused {
            restrict_disclosure(&window, &notebook.repository, "provider_paused");
        }
        emit_provider_status(&window, &result);
        result
    })
}

fn restrict_disclosure(
    source: &tauri::WebviewWindow,
    repository: &NotebookRepository,
    trigger: &str,
) {
    match crate::commands::encounters::restrict_active_for_provider_interruption(
        repository,
        trigger,
        UtcMillis::now(),
    ) {
        Ok(true) => {
            if crate::commands::encounters::emit_current_overlay(source.app_handle(), repository)
                .is_err()
            {
                crate::commands::encounters::emit_fail_closed_overlay(source.app_handle());
            }
        }
        Ok(false) => {}
        Err(_) => crate::commands::encounters::emit_fail_closed_overlay(source.app_handle()),
    }
}

fn mutate(
    runtime: &DetectionRuntime,
    idempotency_key: &str,
    operation: impl FnOnce(&mut crate::detection::DetectionEngine) -> Result<(), RepoError>,
) -> Result<(ProviderStatus, u64), RepoError> {
    let mut engine = runtime
        .engine
        .lock()
        .map_err(|_| RepoError::ProviderUnavailable)?;
    operation(&mut engine)?;
    let mut revision = runtime
        .revision
        .lock()
        .map_err(|_| RepoError::ProviderUnavailable)?;
    *revision = revision.saturating_add(1);
    let revision_value = *revision;
    let status = engine.status();
    drop(engine);
    record_idempotency(runtime, idempotency_key)?;
    Ok((status, revision_value))
}

fn replay(
    runtime: &DetectionRuntime,
    idempotency_key: &str,
) -> Option<CommandResult<ProviderStatus>> {
    let was_applied = runtime
        .applied_idempotency_keys
        .lock()
        .ok()?
        .iter()
        .any(|key| key == idempotency_key);
    if !was_applied {
        return None;
    }
    current(runtime)
        .ok()
        .map(|(status, revision)| CommandResult::success(status, revision))
}

fn record_idempotency(runtime: &DetectionRuntime, idempotency_key: &str) -> Result<(), RepoError> {
    let mut applied = runtime
        .applied_idempotency_keys
        .lock()
        .map_err(|_| RepoError::ProviderUnavailable)?;
    if applied.len() == MAX_IDEMPOTENCY_KEYS {
        applied.pop_front();
    }
    applied.push_back(idempotency_key.to_owned());
    Ok(())
}

fn current(runtime: &DetectionRuntime) -> Result<(ProviderStatus, u64), RepoError> {
    let status = runtime
        .engine
        .lock()
        .map_err(|_| RepoError::ProviderUnavailable)?
        .status();
    let revision = *runtime
        .revision
        .lock()
        .map_err(|_| RepoError::ProviderUnavailable)?;
    Ok((status, revision))
}

fn status_result(
    result: Result<(ProviderStatus, u64), RepoError>,
) -> CommandResult<ProviderStatus> {
    match result {
        Ok((status, revision)) => CommandResult::success(status, revision),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
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

fn emit_provider_status(source: &tauri::WebviewWindow, result: &CommandResult<ProviderStatus>) {
    let CommandResult::Success { data, .. } = result else {
        return;
    };
    let event = ReplacementEvent::v1(
        EventName::ProviderStatus,
        next_event_revision(),
        data.clone(),
    );
    let _ = source.emit("provider://status-v1", event.clone());
    if let Some(overlay) = source.app_handle().get_webview_window("overlay") {
        let _ = overlay.emit("provider://status-v1", event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::EntityId;
    use crate::notebook::key::DatabaseKey;
    use crate::notebook::migrations::MigrationManager;

    fn key() -> String {
        EntityId::new().to_string()
    }

    fn consent(runtime: &DetectionRuntime) -> CommandResult<ProviderStatus> {
        set_provider_consent_for(
            CallerIdentity::Main,
            runtime,
            None,
            SetProviderConsentRequest {
                provider_id: PROVIDER_ID.into(),
                disclosure_version: 1,
                disclosed_fields: vec!["visible opponent handle".into()],
                granted: true,
                idempotency_key: key(),
            },
        )
    }

    #[test]
    fn consent_and_window_selection_are_explicit_and_idempotent() {
        let runtime = DetectionRuntime::default();
        assert!(consent(&runtime).is_success());
        let request = SelectMtgoWindowRequest {
            native_handle: 42,
            class_name: "SyntheticMtgoWindow".into(),
            visible_title: "Magic Online".into(),
            visible: true,
            minimized: false,
            usable_bounds: true,
            idempotency_key: key(),
        };
        let first = select_mtgo_window_for(CallerIdentity::Main, &runtime, request.clone());
        let replay = select_mtgo_window_for(CallerIdentity::Main, &runtime, request);
        assert!(first.is_success());
        assert_eq!(first.revision(), replay.revision());
    }

    #[test]
    fn invalid_or_unauthorized_selection_fails_without_capture() {
        let runtime = DetectionRuntime::default();
        let result = select_mtgo_window_for(
            CallerIdentity::Main,
            &runtime,
            SelectMtgoWindowRequest {
                native_handle: 0,
                class_name: String::new(),
                visible_title: String::new(),
                visible: false,
                minimized: true,
                usable_bounds: false,
                idempotency_key: key(),
            },
        );
        assert!(!result.is_success());
        assert!(
            runtime
                .engine
                .lock()
                .expect("engine")
                .status()
                .selected_window
                .is_none()
        );
    }

    #[test]
    fn pause_does_not_remove_selected_context() {
        let runtime = DetectionRuntime::default();
        assert!(consent(&runtime).is_success());
        assert!(
            select_mtgo_window_for(
                CallerIdentity::Main,
                &runtime,
                SelectMtgoWindowRequest {
                    native_handle: 42,
                    class_name: "SyntheticMtgoWindow".into(),
                    visible_title: "Magic Online".into(),
                    visible: true,
                    minimized: false,
                    usable_bounds: true,
                    idempotency_key: key(),
                },
            )
            .is_success()
        );
        let result = pause_detection_for(
            CallerIdentity::Main,
            &runtime,
            PauseDetectionRequest {
                paused: true,
                idempotency_key: key(),
            },
        );
        assert!(result.is_success());
        assert!(
            runtime
                .engine
                .lock()
                .expect("engine")
                .status()
                .selected_window
                .is_some()
        );
    }

    #[test]
    fn persisted_consent_is_hydrated_without_restoring_window_authority() {
        let directory = tempfile::tempdir().expect("temp");
        let database_path = directory.path().join("notebook.db");
        let key = DatabaseKey::generate().expect("key");
        MigrationManager::default()
            .migrate(&database_path, &key)
            .expect("migrate");
        let repository = NotebookRepository::open(&database_path, &key).expect("repository");
        repository
            .set_provider_consent(
                PROVIDER_ID,
                true,
                r#"["visible opponent handle"]"#,
                UtcMillis::now(),
            )
            .expect("persist consent");

        let runtime = DetectionRuntime::default();
        hydrate_provider_consent(&runtime, &repository).expect("hydrate");
        let status = runtime.engine.lock().expect("engine").status();
        assert!(status.consent_granted);
        assert!(status.paused);
        assert!(status.selected_window.is_none());
        assert_eq!(
            status.disclosed_fields,
            vec!["visible opponent handle".to_owned()]
        );
    }
}
