use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::diagnostics::{
    DiagnosticsBundleResult, DiagnosticsPreview, DiagnosticsService, LogLevel, SafeLogEvent,
};
use crate::domain::IdempotencyKey;
use crate::ipc::{AppError, CallerIdentity, CommandResult, ErrorCode, panic_boundary};
use crate::settings::AppState;

const DIAGNOSTICS_SELECTION_TTL_MS: i64 = 10 * 60 * 1_000;

struct DiagnosticsDestination {
    path: PathBuf,
    expires_at: i64,
}

pub struct DiagnosticsRuntime {
    pub service: DiagnosticsService,
    log_directory: Mutex<PathBuf>,
    destinations: Mutex<HashMap<String, DiagnosticsDestination>>,
}

impl Default for DiagnosticsRuntime {
    fn default() -> Self {
        Self {
            service: DiagnosticsService::default(),
            log_directory: Mutex::new(std::env::temp_dir().join("mtgo-notes-diagnostics")),
            destinations: Mutex::new(HashMap::new()),
        }
    }
}

impl DiagnosticsRuntime {
    pub fn set_log_directory(&self, path: PathBuf) -> Result<(), AppError> {
        *self
            .log_directory
            .lock()
            .map_err(|_| AppError::internal("diagnostics-directory-lock"))? = path;
        Ok(())
    }

    fn log_directory(&self) -> Result<PathBuf, AppError> {
        self.log_directory
            .lock()
            .map_err(|_| AppError::internal("diagnostics-directory-lock"))
            .map(|path| path.clone())
    }

    fn register_destination(&self, path: PathBuf) -> Result<DiagnosticsPathSelection, AppError> {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(invalid_selection)?
            .to_owned();
        let selection_token = crate::domain::EntityId::new().to_string();
        self.destinations
            .lock()
            .map_err(|_| AppError::internal("diagnostics-destination-lock"))?
            .insert(
                selection_token.clone(),
                DiagnosticsDestination {
                    path,
                    expires_at: now_ms().saturating_add(DIAGNOSTICS_SELECTION_TTL_MS),
                },
            );
        Ok(DiagnosticsPathSelection {
            selection_token,
            file_name,
        })
    }

    fn resolve_destination(&self, selection_token: &str) -> Result<PathBuf, AppError> {
        let mut destinations = self
            .destinations
            .lock()
            .map_err(|_| AppError::internal("diagnostics-destination-lock"))?;
        destinations.retain(|_, destination| destination.expires_at > now_ms());
        destinations
            .get(selection_token)
            .map(|destination| destination.path.clone())
            .ok_or_else(invalid_selection)
    }

    fn consume_destination(&self, selection_token: &str) {
        if let Ok(mut destinations) = self.destinations.lock() {
            destinations.remove(selection_token);
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsPathSelection {
    pub selection_token: String,
    pub file_name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDiagnosticsRequest {
    pub idempotency_key: String,
    pub preview_token: String,
    pub selection_token: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelDiagnosticsRequest {
    pub preview_token: String,
}

pub fn preview_diagnostics_for(
    caller: CallerIdentity,
    state: &AppState,
    runtime: &DiagnosticsRuntime,
) -> CommandResult<DiagnosticsPreview> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let settings = match state.settings.lock() {
        Ok(store) => store.settings.clone(),
        Err(_) => {
            return CommandResult::failure(AppError::internal("diagnostics-settings-lock"));
        }
    };
    if !settings.diagnostics_enabled {
        return CommandResult::failure(AppError::new(
            ErrorCode::ConsentRequired,
            "Enable private diagnostics before previewing a support bundle.",
            false,
        ));
    }
    let now = now_ms();
    let environment = SafeLogEvent {
        timestamp: now,
        level: LogLevel::Info,
        component: "application".into(),
        event_code: "diagnostics.preview".into(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        schema_version: settings.schema_version,
        classifier_version: None,
        duration_bucket: None,
        error_code: None,
    };
    let result = runtime
        .log_directory()
        .and_then(|directory| runtime.service.preview(&directory, environment, now));
    match result {
        Ok(preview) => CommandResult::success(preview, 1),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn create_diagnostics_for(
    caller: CallerIdentity,
    state: &AppState,
    runtime: &DiagnosticsRuntime,
    request: CreateDiagnosticsRequest,
) -> CommandResult<DiagnosticsBundleResult> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if IdempotencyKey::parse(&request.idempotency_key).is_err() {
        return CommandResult::failure(
            AppError::new(
                ErrorCode::InvalidRequest,
                "A valid idempotency key is required.",
                false,
            )
            .with_field("idempotencyKey"),
        );
    }
    match state.settings.lock() {
        Ok(store) if store.settings.diagnostics_enabled => {}
        Ok(_) => {
            return CommandResult::failure(AppError::new(
                ErrorCode::ConsentRequired,
                "Enable private diagnostics before creating a support bundle.",
                false,
            ));
        }
        Err(_) => {
            return CommandResult::failure(AppError::internal("diagnostics-settings-lock"));
        }
    }
    let destination = match runtime.resolve_destination(&request.selection_token) {
        Ok(destination) => destination,
        Err(error) => return CommandResult::failure(error),
    };
    let result = runtime
        .service
        .create_bundle(&request.preview_token, &destination, now_ms());
    match result {
        Ok(bundle) => {
            runtime.consume_destination(&request.selection_token);
            CommandResult::success(bundle, 1)
        }
        Err(error) => CommandResult::failure(error),
    }
}

pub fn register_diagnostics_path_for(
    caller: CallerIdentity,
    runtime: &DiagnosticsRuntime,
    path: PathBuf,
) -> CommandResult<DiagnosticsPathSelection> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match runtime.register_destination(path) {
        Ok(selection) => CommandResult::success(selection, 1),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn cancel_diagnostics_for(
    caller: CallerIdentity,
    runtime: &DiagnosticsRuntime,
    request: CancelDiagnosticsRequest,
) -> CommandResult<bool> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match runtime.service.cancel(&request.preview_token) {
        Ok(()) => CommandResult::success(true, 1),
        Err(error) => CommandResult::failure(error),
    }
}

#[tauri::command]
pub fn select_diagnostics_path(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, DiagnosticsRuntime>,
) -> CommandResult<Option<DiagnosticsPathSelection>> {
    panic_boundary("select-diagnostics-path-command", || {
        with_caller(&window, |caller| {
            if let Err(error) = caller.require(&[CallerIdentity::Main]) {
                return CommandResult::failure(error);
            }
            #[cfg(windows)]
            let path = rfd::FileDialog::new()
                .add_filter("MTGO Notes diagnostics", &["mtgodiag"])
                .set_file_name("mtgo-notes-support.mtgodiag")
                .save_file();
            #[cfg(not(windows))]
            let path: Option<PathBuf> = None;

            match path {
                Some(path) => match runtime.register_destination(path) {
                    Ok(selection) => CommandResult::success(Some(selection), 1),
                    Err(error) => CommandResult::failure(error),
                },
                None => CommandResult::success(None, 1),
            }
        })
    })
}

#[tauri::command]
pub fn preview_diagnostics(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, DiagnosticsRuntime>,
) -> CommandResult<DiagnosticsPreview> {
    panic_boundary("preview-diagnostics-command", || {
        with_caller(&window, |caller| {
            preview_diagnostics_for(caller, state.inner(), runtime.inner())
        })
    })
}

#[tauri::command]
pub fn create_diagnostics(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, DiagnosticsRuntime>,
    request: CreateDiagnosticsRequest,
) -> CommandResult<DiagnosticsBundleResult> {
    panic_boundary("create-diagnostics-command", || {
        with_caller(&window, |caller| {
            create_diagnostics_for(caller, state.inner(), runtime.inner(), request)
        })
    })
}

#[tauri::command]
pub fn cancel_diagnostics(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, DiagnosticsRuntime>,
    request: CancelDiagnosticsRequest,
) -> CommandResult<bool> {
    panic_boundary("cancel-diagnostics-command", || {
        with_caller(&window, |caller| {
            cancel_diagnostics_for(caller, runtime.inner(), request)
        })
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

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn invalid_selection() -> AppError {
    AppError::new(
        ErrorCode::InvalidRequest,
        "Choose a diagnostics destination again.",
        true,
    )
    .with_field("selectionToken")
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::settings::Settings;

    fn enabled_state() -> AppState {
        let state = AppState::default();
        state.settings.lock().expect("settings").settings = Settings {
            diagnostics_enabled: true,
            ..Settings::default()
        };
        state
    }

    #[test]
    fn it_227_preview_returns_classes_and_counts_without_private_values() {
        let directory = tempdir().expect("temp");
        let runtime = DiagnosticsRuntime::default();
        runtime
            .set_log_directory(directory.path().to_owned())
            .expect("directory");
        let result = preview_diagnostics_for(CallerIdentity::Main, &enabled_state(), &runtime);
        let CommandResult::Success { data, .. } = result else {
            panic!("preview failed");
        };
        let json = serde_json::to_string(&data).expect("json");
        assert!(!json.contains("handle"));
        assert!(!json.contains("note"));
        assert!(!data.artifacts.is_empty());
    }

    #[test]
    fn it_228_preview_bound_bundle_is_local_and_canary_clean() {
        let directory = tempdir().expect("temp");
        let runtime = DiagnosticsRuntime::default();
        runtime
            .set_log_directory(directory.path().join("logs"))
            .expect("directory");
        let CommandResult::Success { data: preview, .. } =
            preview_diagnostics_for(CallerIdentity::Main, &enabled_state(), &runtime)
        else {
            panic!("preview failed");
        };
        let destination = directory.path().join("support.mtgodiag");
        let CommandResult::Success {
            data: selection, ..
        } = register_diagnostics_path_for(CallerIdentity::Main, &runtime, destination.clone())
        else {
            panic!("selection failed");
        };
        let result = create_diagnostics_for(
            CallerIdentity::Main,
            &enabled_state(),
            &runtime,
            CreateDiagnosticsRequest {
                idempotency_key: IdempotencyKey::new().as_str().to_owned(),
                preview_token: preview.preview_token,
                selection_token: selection.selection_token,
            },
        );
        let CommandResult::Success { data, .. } = result else {
            panic!("bundle failed");
        };
        assert_eq!(data.network_requests, 0);
        assert!(destination.exists());
        assert!(
            !std::fs::read_to_string(destination)
                .expect("bundle")
                .contains("CANARY_PRIVATE_")
        );
    }

    #[test]
    fn it_260_redaction_failure_writes_no_bundle() {
        let directory = tempdir().expect("temp");
        let logs = directory.path().join("logs");
        std::fs::create_dir_all(&logs).expect("logs");
        std::fs::write(
            logs.join("events.jsonl"),
            "{\"eventCode\":\"CANARY_PRIVATE_NOTE\",\"timestamp\":1}\n",
        )
        .expect("write");
        let runtime = DiagnosticsRuntime::default();
        runtime.set_log_directory(logs).expect("directory");
        let result = preview_diagnostics_for(CallerIdentity::Main, &enabled_state(), &runtime);
        assert!(matches!(
            result,
            CommandResult::Failure {
                error: AppError {
                    code: ErrorCode::RedactionFailed,
                    ..
                },
                ..
            }
        ));
        assert!(!directory.path().join("support.mtgodiag").exists());
    }

    #[test]
    fn renderer_cannot_supply_an_arbitrary_diagnostics_path() {
        let runtime = DiagnosticsRuntime::default();
        let result = create_diagnostics_for(
            CallerIdentity::Main,
            &enabled_state(),
            &runtime,
            CreateDiagnosticsRequest {
                idempotency_key: IdempotencyKey::new().as_str().to_owned(),
                preview_token: "preview".into(),
                selection_token: "/renderer/chosen/path".into(),
            },
        );
        assert!(matches!(
            result,
            CommandResult::Failure {
                error: AppError {
                    code: ErrorCode::InvalidRequest,
                    ..
                },
                ..
            }
        ));
    }
}
