use crate::domain::IdempotencyKey;
use crate::ipc::{AppError, CallerIdentity, CommandResult, ErrorCode, panic_boundary};
use crate::settings::{AppState, Settings, UpdateSettingsRequest};
use tauri::Manager;

pub fn get_settings_for(caller: CallerIdentity, state: &AppState) -> CommandResult<Settings> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }

    match state.settings.lock() {
        Ok(store) => CommandResult::success(store.settings.clone(), store.revision),
        Err(_) => CommandResult::failure(AppError::internal("get-settings-state-lock")),
    }
}

pub fn update_settings_for(
    caller: CallerIdentity,
    state: &AppState,
    request: UpdateSettingsRequest,
) -> CommandResult<Settings> {
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

    let mut store = match state.settings.lock() {
        Ok(store) => store,
        Err(_) => {
            return CommandResult::failure(AppError::internal("update-settings-state-lock"));
        }
    };

    if let Some((settings, revision)) = store
        .idempotent_results
        .get(&request.idempotency_key)
        .cloned()
    {
        return CommandResult::success(settings, revision);
    }

    if request.expected_revision != store.revision {
        return CommandResult::failure(
            AppError::new(
                ErrorCode::RevisionConflict,
                "Settings changed after this view was loaded.",
                true,
            )
            .with_field("expectedRevision"),
        );
    }

    let revision = store.revision.saturating_add(1);
    if let Err(error) = store.persist_candidate(&request.settings, revision) {
        return CommandResult::failure(error);
    }
    store.revision = revision;
    store.settings = request.settings;
    let settings = store.settings.clone();
    store
        .idempotent_results
        .insert(request.idempotency_key, (settings.clone(), revision));

    CommandResult::success(settings, revision)
}

#[tauri::command]
pub fn get_settings(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Settings> {
    panic_boundary("get-settings-command", || {
        let caller = match CallerIdentity::from_window_label(window.label()) {
            Ok(caller) => caller,
            Err(error) => return CommandResult::failure(error),
        };
        get_settings_for(caller, state.inner())
    })
}

#[tauri::command]
pub fn update_settings(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
    autostart: tauri::State<'_, crate::shell::autostart::AutostartController>,
    request: UpdateSettingsRequest,
) -> CommandResult<Settings> {
    panic_boundary("update-settings-command", || {
        let caller = match CallerIdentity::from_window_label(window.label()) {
            Ok(caller) => caller,
            Err(error) => return CommandResult::failure(error),
        };
        match state.settings.lock() {
            Ok(store) => {
                if let Some((settings, revision)) = store
                    .idempotent_results
                    .get(&request.idempotency_key)
                    .cloned()
                {
                    return CommandResult::success(settings, revision);
                }
            }
            Err(_) => {
                return CommandResult::failure(AppError::internal("update-settings-state-lock"));
            }
        }
        let requested_settings = request.settings.clone();
        let previous_settings = match state.settings.lock() {
            Ok(store) => store.settings.clone(),
            Err(_) => {
                return CommandResult::failure(AppError::internal("update-settings-state-lock"));
            }
        };
        if requested_settings.launch_with_windows != previous_settings.launch_with_windows
            && let Err(error) =
                autostart.apply_explicit_choice(requested_settings.launch_with_windows)
        {
            return CommandResult::failure(error);
        }
        if requested_settings.tray_enabled != previous_settings.tray_enabled
            && let Err(error) = crate::shell::apply_tray_choice(
                window.app_handle(),
                requested_settings.tray_enabled,
            )
        {
            let _ = autostart.apply_explicit_choice(previous_settings.launch_with_windows);
            return CommandResult::failure(error);
        }
        let result = update_settings_for(caller, state.inner(), request);
        if matches!(result, CommandResult::Success { .. }) {
            if !requested_settings.overlay_enabled
                && let Some(overlay) = window.app_handle().get_webview_window("overlay")
            {
                let _ = overlay.hide();
            }
        } else {
            if requested_settings.launch_with_windows != previous_settings.launch_with_windows {
                let _ = autostart.apply_explicit_choice(previous_settings.launch_with_windows);
            }
            if requested_settings.tray_enabled != previous_settings.tray_enabled {
                let _ = crate::shell::apply_tray_choice(
                    window.app_handle(),
                    previous_settings.tray_enabled,
                );
            }
        }
        result
    })
}
