use tauri::{
    Emitter, Manager, WebviewWindowBuilder, WindowEvent,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::commands::capture::CaptureRuntime;
use crate::commands::capture::{OpenCaptureRequest, open_capture_for};
use crate::commands::providers::{PauseDetectionRequest, pause_detection_for};
use crate::detection::DetectionRuntime;
use crate::domain::{EntityId, UtcMillis};
use crate::ipc::CallerIdentity;
use crate::ipc::{AppError, ErrorCode, EventName, ReplacementEvent, next_event_revision};
use crate::notebook::{NotebookRuntime, repository::NotebookRepository};
use crate::settings::AppState;

const TRAY_ID: &str = "mtgo-notes";
const PRIMARY_WINDOW_LABEL: &str = "main";

pub fn is_allowed_navigation(url: &tauri::Url) -> bool {
    match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        "http" if url.host_str() == Some("tauri.localhost") => true,
        "http" if cfg!(debug_assertions) => {
            matches!(url.host_str(), Some("127.0.0.1" | "localhost")) && url.port() == Some(1420)
        }
        _ => false,
    }
}

pub fn create_configured_windows(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window_configs = app.config().app.windows.clone();

    for config in window_configs {
        let label = config.label.clone();
        let window = match WebviewWindowBuilder::from_config(app.handle(), &config)
            .map(|builder| builder.on_navigation(is_allowed_navigation))
            .and_then(WebviewWindowBuilder::build)
        {
            Ok(window) => window,
            Err(error) if !window_failure_is_fatal(&label) => {
                eprintln!("optional window {label} is unavailable: {error}");
                continue;
            }
            Err(error) => return Err(error.into()),
        };

        if window.label() == "overlay" {
            if let Err(error) = window.set_ignore_cursor_events(true) {
                eprintln!("overlay click-through initialization failed: {error}");
                let _ = window.destroy();
                continue;
            }
            clamp_window_to_current_monitor(&window);
        }
    }

    Ok(())
}

pub fn apply_tray_choice(app: &tauri::AppHandle, enabled: bool) -> Result<(), AppError> {
    if !enabled {
        app.remove_tray_by_id(TRAY_ID);
        return Ok(());
    }
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }

    let open = MenuItemBuilder::with_id("open", "Open MTGO Opponent Notes")
        .build(app)
        .map_err(|_| tray_error())?;
    let overlay = MenuItemBuilder::with_id("overlay", "Show/Hide Overlay")
        .build(app)
        .map_err(|_| tray_error())?;
    let pause = MenuItemBuilder::with_id("pause", "Toggle Detection Pause")
        .build(app)
        .map_err(|_| tray_error())?;
    let quit = MenuItemBuilder::with_id("quit", "Quit")
        .build(app)
        .map_err(|_| tray_error())?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &overlay, &pause, &quit])
        .build()
        .map_err(|_| tray_error())?;
    let icon = app.default_window_icon().cloned().ok_or_else(tray_error)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("MTGO Opponent Notes")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "overlay" => toggle_overlay(app),
            "pause" => {
                if let Some(runtime) = app.try_state::<DetectionRuntime>() {
                    let paused = runtime
                        .engine
                        .lock()
                        .map(|engine| engine.status().paused)
                        .unwrap_or(false);
                    let result = pause_detection_for(
                        CallerIdentity::Main,
                        runtime.inner(),
                        PauseDetectionRequest {
                            paused: !paused,
                            idempotency_key: EntityId::new().to_string(),
                        },
                    );
                    if !paused
                        && result.is_success()
                        && let Some(notebook) = app.try_state::<NotebookRuntime>()
                    {
                        match crate::commands::encounters::restrict_active_for_provider_interruption(
                            &notebook.repository,
                            "provider_paused",
                            UtcMillis::now(),
                        ) {
                            Ok(true) => {
                                if crate::commands::encounters::emit_current_overlay(
                                    app,
                                    &notebook.repository,
                                )
                                .is_err()
                                {
                                    crate::commands::encounters::emit_fail_closed_overlay(app);
                                }
                            }
                            Ok(false) => {}
                            Err(_) => {
                                crate::commands::encounters::emit_fail_closed_overlay(app)
                            }
                        }
                    }
                }
            }
            "quit" => request_safe_shutdown(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|_| tray_error())?;
    Ok(())
}

pub fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(PRIMARY_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_overlay(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let overlay_enabled = app
            .state::<AppState>()
            .settings
            .lock()
            .map(|store| store.settings.overlay_enabled)
            .unwrap_or(false);
        if !overlay_enabled {
            let _ = window.hide();
            return;
        }
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.set_ignore_cursor_events(false);
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

pub fn set_overlay_expanded(window: &tauri::WebviewWindow, expanded: bool) -> Result<(), AppError> {
    if window.label() != "overlay" {
        return Err(AppError::new(
            ErrorCode::UnauthorizedCaller,
            "Only the overlay can change its interaction state.",
            false,
        ));
    }
    window.set_ignore_cursor_events(!expanded).map_err(|_| {
        AppError::new(
            ErrorCode::OverlayUnavailable,
            "The overlay interaction state could not be changed.",
            true,
        )
    })?;
    if expanded {
        window.set_focus().map_err(|_| {
            AppError::new(
                ErrorCode::OverlayUnavailable,
                "The overlay could not receive requested focus.",
                true,
            )
        })?;
    }
    Ok(())
}

pub fn register_quick_capture_shortcut(
    app: &mut tauri::App,
) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyN);
    let registered = shortcut;
    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, incoming, event| {
                if incoming != &registered || event.state() != ShortcutState::Pressed {
                    return;
                }
                let Some(notebook) = app.try_state::<NotebookRuntime>() else {
                    show_main_window(app);
                    return;
                };
                let Some(capture_runtime) = app.try_state::<CaptureRuntime>() else {
                    show_main_window(app);
                    return;
                };
                let result = open_capture_for(
                    CallerIdentity::Overlay,
                    &notebook.repository,
                    &notebook.key,
                    capture_runtime.inner(),
                    OpenCaptureRequest {
                        idempotency_key: EntityId::new().to_string(),
                    },
                );
                match result {
                    crate::ipc::CommandResult::Success { data, .. } => {
                        if let Some(window) = app.get_webview_window("capture") {
                            let _ = window.emit(
                                "capture://draft-v1",
                                ReplacementEvent::v1(
                                    EventName::CaptureDraft,
                                    next_event_revision(),
                                    data,
                                ),
                            );
                            let _ = window.show();
                            let _ = window.set_ignore_cursor_events(false);
                            let _ = window.set_focus();
                        }
                    }
                    crate::ipc::CommandResult::Failure { error, .. }
                        if error.code == ErrorCode::AlreadyOpen =>
                    {
                        if let Some(window) = app.get_webview_window("capture") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    crate::ipc::CommandResult::Failure { .. } => {}
                }
            })
            .build(),
    )?;
    app.global_shortcut().register(shortcut)?;
    Ok(())
}

fn clamp_window_to_current_monitor(window: &tauri::WebviewWindow) {
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };
    let Ok(position) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let (x, y) = clamp_position(
        (position.x, position.y),
        (size.width, size.height),
        (
            monitor_position.x,
            monitor_position.y,
            monitor_size.width,
            monitor_size.height,
        ),
    );
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

fn clamp_position(
    position: (i32, i32),
    window_size: (u32, u32),
    work_area: (i32, i32, u32, u32),
) -> (i32, i32) {
    let max_x = work_area
        .0
        .saturating_add(i32::try_from(work_area.2.saturating_sub(window_size.0)).unwrap_or(0));
    let max_y = work_area
        .1
        .saturating_add(i32::try_from(work_area.3.saturating_sub(window_size.1)).unwrap_or(0));
    (
        position.0.clamp(work_area.0, max_x.max(work_area.0)),
        position.1.clamp(work_area.1, max_y.max(work_area.1)),
    )
}

pub fn request_safe_shutdown(app: &tauri::AppHandle) {
    if let Some(notebook) = app.try_state::<NotebookRuntime>() {
        let _ = prepare_repository_for_shutdown(&notebook.repository);
    }
    app.exit(0);
}

fn prepare_repository_for_shutdown(
    repository: &NotebookRepository,
) -> Result<(), crate::domain::RepoError> {
    repository
        .mark_active_encounter_incomplete("shutdown_without_confident_end", UtcMillis::now())?;
    repository.mark_clean_shutdown()
}

fn window_failure_is_fatal(label: &str) -> bool {
    label != "overlay"
}

fn tray_error() -> AppError {
    AppError::new(
        ErrorCode::SaveFailed,
        "The tray choice could not be changed.",
        true,
    )
}

pub fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if should_hide_main_on_close(
        window.label(),
        window
            .app_handle()
            .state::<AppState>()
            .settings
            .lock()
            .map(|store| store.settings.tray_enabled)
            .unwrap_or(false),
    ) && let WindowEvent::CloseRequested { api, .. } = event
    {
        api.prevent_close();
        let _ = window.hide();
    }
}

fn should_hide_main_on_close(label: &str, tray_enabled: bool) -> bool {
    label == PRIMARY_WINDOW_LABEL && tray_enabled
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::notebook::key::DatabaseKey;
    use crate::notebook::migrations::MigrationManager;
    use crate::services::profiles::ProfileService;

    #[test]
    fn only_local_application_navigation_is_allowed() {
        for allowed in [
            "tauri://localhost/index.html",
            "http://tauri.localhost/index.html",
            "http://127.0.0.1:1420/overlay.html",
            "http://localhost:1420/capture.html",
        ] {
            assert!(is_allowed_navigation(&allowed.parse().expect("valid URL")));
        }

        for denied in [
            "https://example.com",
            "https://localhost:1420",
            "http://localhost:9999",
            "file:///C:/private.txt",
            "javascript:alert(1)",
            "data:text/html,unsafe",
        ] {
            assert!(!is_allowed_navigation(&denied.parse().expect("valid URL")));
        }
    }

    #[test]
    fn ut_095_main_close_hides_only_when_tray_residency_is_enabled() {
        assert!(should_hide_main_on_close("main", true));
        assert!(!should_hide_main_on_close("main", false));
        assert!(!should_hide_main_on_close("overlay", true));
    }

    #[test]
    fn ut_096_shutdown_marks_unresolved_encounter_incomplete_before_clean_exit() {
        let directory = TempDir::new().expect("temp");
        let key = DatabaseKey::generate().expect("key");
        MigrationManager::default()
            .migrate(directory.path().join("notebook.db"), &key)
            .expect("migrate");
        let repository =
            NotebookRepository::open(directory.path().join("notebook.db"), &key).expect("open");
        let profile = ProfileService::new(&repository)
            .create("ShutdownOpponent")
            .expect("profile");
        repository
            .start_encounter(&EntityId::new(), &profile.profile.id, UtcMillis::now(), 1)
            .expect("encounter");

        prepare_repository_for_shutdown(&repository).expect("shutdown");

        assert!(repository.active_encounter().expect("active").is_none());
        let (status, clean): (String, i64) = repository
            .with_connection(|connection| {
                let status = connection
                    .connection
                    .query_row(
                        "SELECT status FROM encounters ORDER BY started_at DESC LIMIT 1",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|_| crate::domain::RepoError::NotebookInvalid)?;
                let clean = connection
                    .connection
                    .query_row(
                        "SELECT clean_shutdown FROM runtime_state WHERE singleton = 1",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|_| crate::domain::RepoError::NotebookInvalid)?;
                Ok((status, clean))
            })
            .expect("state");
        assert_eq!(status, "incomplete");
        assert_eq!(clean, 1);
    }

    #[test]
    fn ut_097_relaunch_target_is_the_existing_main_window() {
        assert_eq!(PRIMARY_WINDOW_LABEL, "main");
    }

    #[test]
    fn ut_102_overlay_failure_is_optional_but_main_and_capture_remain_required() {
        assert!(!window_failure_is_fatal("overlay"));
        assert!(window_failure_is_fatal("main"));
        assert!(window_failure_is_fatal("capture"));
    }

    #[test]
    fn ut_103_overlay_position_clamps_to_current_monitor_bounds() {
        assert_eq!(
            clamp_position((4_000, -500), (360, 220), (1_920, 0, 1_920, 1_080)),
            (3_480, 0)
        );
        assert_eq!(
            clamp_position((-50, 2_000), (360, 220), (0, 0, 1_280, 720)),
            (0, 500)
        );
    }
}
