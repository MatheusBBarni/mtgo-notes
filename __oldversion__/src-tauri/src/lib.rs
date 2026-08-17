pub mod classifier;
pub mod commands;
pub mod detection;
pub mod diagnostics;
pub mod disclosure;
pub mod domain;
pub mod encounters;
pub mod ipc;
pub mod notebook;
pub mod operations;
pub mod portability;
pub mod providers;
pub mod resilience;
pub mod services;
pub mod settings;
pub mod shell;

use settings::AppState;
use tauri::Manager;

pub fn builder() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            shell::show_main_window(app);
        }))
        .manage(AppState::default())
        .manage(detection::DetectionRuntime::default())
        .manage(commands::capture::CaptureRuntime::default())
        .manage(commands::encounters::EncounterCommandRuntime::default())
        .manage(commands::diagnostics::DiagnosticsRuntime::default())
        .manage(portability::PortabilityRuntime::default())
        .manage(commands::updates::UpdateRuntime::default())
        .manage(shell::autostart::AutostartController::default())
        .setup(|app| {
            let enrichment = commands::classifier::DeckEnrichmentRuntime::builtin()
                .map_err(|_| "bundled classifier assets are invalid")?;
            app.manage(enrichment);
            initialize_operational_state(app)?;
            initialize_notebook(app)?;
            shell::create_configured_windows(app)?;
            shell::register_quick_capture_shortcut(app)?;
            let tray_enabled = app
                .state::<AppState>()
                .settings
                .lock()
                .map_err(|_| "local settings state unavailable")?
                .settings
                .tray_enabled;
            shell::apply_tray_choice(app.handle(), tray_enabled)
                .map_err(|_| "tray choice could not be restored")?;
            Ok(())
        })
        .on_window_event(shell::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap::bootstrap,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::providers::list_providers,
            commands::providers::list_mtgo_windows,
            commands::providers::set_provider_consent,
            commands::providers::select_mtgo_window,
            commands::providers::pause_detection,
            commands::encounters::confirm_opponent,
            commands::encounters::enter_opponent,
            commands::encounters::correct_phase,
            commands::encounters::finish_encounter,
            commands::encounters::reopen_encounter,
            commands::encounters::undo_transition,
            commands::capture::open_capture,
            commands::capture::discard_draft,
            commands::shell::set_overlay_interaction,
            commands::diagnostics::select_diagnostics_path,
            commands::diagnostics::preview_diagnostics,
            commands::diagnostics::create_diagnostics,
            commands::diagnostics::cancel_diagnostics,
            commands::updates::check_update,
            commands::updates::install_update,
            commands::updates::check_classifier_update,
            commands::updates::install_classifier_update,
            commands::notes::create_profile,
            commands::notes::resolve_profile,
            commands::notes::suggest_profiles,
            commands::notes::add_alias,
            commands::notes::update_profile,
            commands::notes::save_observation,
            commands::notes::update_observation,
            commands::notes::set_card_observations,
            commands::notes::set_tendency_tags,
            commands::history::search_history,
            commands::history::get_profile,
            commands::history::get_encounter,
            commands::identity::preview_merge,
            commands::identity::apply_merge,
            commands::identity::preview_unmerge,
            commands::identity::apply_unmerge,
            commands::privacy::preview_deletion,
            commands::privacy::request_deletion,
            commands::privacy::undo_deletion,
            commands::decks::set_deck_provider_consent,
            commands::decks::lookup_official_deck,
            commands::decks::confirm_public_snapshot,
            commands::decks::save_complete_deck,
            commands::decks::get_deck_details,
            commands::decks::open_official_deck_page,
            commands::classifier::get_classification,
            commands::classifier::start_reclassification,
            commands::portability::select_portability_path,
            commands::portability::start_backup,
            commands::portability::preview_restore,
            commands::portability::apply_restore,
            commands::portability::start_export,
            commands::portability::cancel_operation,
            commands::portability::get_operation,
            commands::portability::list_portability_rollbacks,
            commands::portability::confirm_portability_rollback,
            commands::portability::apply_portability_rollback,
            commands::portability::discard_portability_rollback,
        ])
}

fn initialize_operational_state(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let directory = app.path().app_local_data_dir()?;
    let state = app.state::<AppState>();
    state
        .configure_settings_path(directory.join("settings.json"))
        .map_err(|_| "local settings initialization failed")?;
    let launch_with_windows = state
        .settings
        .lock()
        .map_err(|_| "local settings state unavailable")?
        .settings
        .launch_with_windows;
    app.state::<shell::autostart::AutostartController>()
        .apply_explicit_choice(launch_with_windows)
        .map_err(|_| "Windows startup choice could not be restored")?;
    let diagnostics = app.state::<commands::diagnostics::DiagnosticsRuntime>();
    diagnostics
        .set_log_directory(directory.join("diagnostics"))
        .map_err(|_| "local diagnostics initialization failed")?;
    diagnostics
        .service
        .cleanup(&directory.join("diagnostics"), std::time::SystemTime::now())
        .map_err(|_| "diagnostic retention cleanup failed")?;
    diagnostics
        .service
        .append(
            &directory.join("diagnostics"),
            &diagnostics::SafeLogEvent {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
                    .try_into()
                    .unwrap_or(i64::MAX),
                level: diagnostics::LogLevel::Info,
                component: "application".into(),
                event_code: "application.started".into(),
                app_version: env!("CARGO_PKG_VERSION").into(),
                schema_version: settings::SETTINGS_SCHEMA_VERSION,
                classifier_version: None,
                duration_bucket: None,
                error_code: None,
            },
        )
        .map_err(|_| "diagnostic startup event failed")?;
    Ok(())
}

#[cfg(windows)]
fn initialize_notebook(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::Manager;

    let directory = app.path().app_local_data_dir()?;
    std::fs::create_dir_all(&directory)?;
    match notebook::NotebookBootstrap::new(
        directory.join("notebook.db"),
        directory.join("notebook.key"),
        notebook::key::CurrentUserDpapi,
    )
    .initialize()
    {
        Ok(runtime) => {
            portability::cleanup_transient_files(&runtime.repository)
                .map_err(|_| "portability recovery cleanup failed")?;
            commands::providers::hydrate_provider_consent(
                app.state::<detection::DetectionRuntime>().inner(),
                &runtime.repository,
            )
            .map_err(|_| "provider consent restoration failed")?;
            services::deletion::DeletionService::new(&runtime.repository)
                .purge_due_coordinated(
                    &app.state::<portability::PortabilityRuntime>().coordinator,
                    domain::UtcMillis::now(),
                )
                .map_err(|_| "pending notebook purge recovery failed")?;
            app.manage(runtime);
        }
        Err(error) => {
            let state = app.state::<AppState>();
            let mut notebook_error = state
                .notebook_error
                .lock()
                .map_err(|_| "notebook state lock poisoned")?;
            *notebook_error = Some(error.to_app_error());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn initialize_notebook(_app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    builder()
        .run(tauri::generate_context!())
        .expect("failed to run MTGO Opponent Notes");
}
