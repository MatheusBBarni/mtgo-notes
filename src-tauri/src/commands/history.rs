use serde::{Deserialize, Serialize};

use crate::disclosure::DisclosurePolicy;
use crate::domain::{EntityId, InternalPhase};
use crate::ipc::{CallerIdentity, CommandResult, panic_boundary};
use crate::notebook::NotebookRuntime;
use crate::notebook::repository::NotebookRepository;
use crate::services::history::{
    EncounterDetail, HistoryPage, HistoryQuery, HistoryService, ProfileDetail,
};
use crate::settings::AppState;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRequest {
    pub id: String,
}

pub fn search_history_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    phase: InternalPhase,
    request: HistoryQuery,
) -> CommandResult<HistoryPage> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match HistoryService::new(repository, &DisclosurePolicy).search(phase, request) {
        Ok(page) => CommandResult::success(page, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn get_profile_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    phase: InternalPhase,
    request: EntityRequest,
) -> CommandResult<ProfileDetail> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let result = EntityId::parse(request.id)
        .and_then(|id| HistoryService::new(repository, &DisclosurePolicy).get_profile(phase, &id));
    match result {
        Ok(profile) => {
            let revision = profile.profile.profile.revision.get();
            CommandResult::success(profile, revision)
        }
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn get_encounter_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    phase: InternalPhase,
    request: EntityRequest,
) -> CommandResult<EncounterDetail> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let result = EntityId::parse(request.id).and_then(|id| {
        HistoryService::new(repository, &DisclosurePolicy).get_encounter(phase, &id)
    });
    match result {
        Ok(encounter) => {
            let revision = encounter.summary.revision;
            CommandResult::success(encounter, revision)
        }
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

#[tauri::command]
pub fn search_history(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    state: tauri::State<'_, AppState>,
    request: HistoryQuery,
) -> CommandResult<HistoryPage> {
    panic_boundary("search-history-command", || {
        let caller = match CallerIdentity::from_window_label(window.label()) {
            Ok(caller) => caller,
            Err(error) => return CommandResult::failure(error),
        };
        let phase = match state.phase.lock() {
            Ok(phase) => *phase,
            Err(_) => {
                return CommandResult::failure(crate::ipc::AppError::internal(
                    "search-history-phase-lock",
                ));
            }
        };
        search_history_for(caller, &runtime.repository, phase, request)
    })
}

#[tauri::command]
pub fn get_profile(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    state: tauri::State<'_, AppState>,
    request: EntityRequest,
) -> CommandResult<ProfileDetail> {
    panic_boundary("get-profile-command", || {
        let caller = match CallerIdentity::from_window_label(window.label()) {
            Ok(caller) => caller,
            Err(error) => return CommandResult::failure(error),
        };
        let phase = match state.phase.lock() {
            Ok(phase) => *phase,
            Err(_) => {
                return CommandResult::failure(crate::ipc::AppError::internal(
                    "get-profile-phase-lock",
                ));
            }
        };
        get_profile_for(caller, &runtime.repository, phase, request)
    })
}

#[tauri::command]
pub fn get_encounter(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    state: tauri::State<'_, AppState>,
    request: EntityRequest,
) -> CommandResult<EncounterDetail> {
    panic_boundary("get-encounter-command", || {
        let caller = match CallerIdentity::from_window_label(window.label()) {
            Ok(caller) => caller,
            Err(error) => return CommandResult::failure(error),
        };
        let phase = match state.phase.lock() {
            Ok(phase) => *phase,
            Err(_) => {
                return CommandResult::failure(crate::ipc::AppError::internal(
                    "get-encounter-phase-lock",
                ));
            }
        };
        get_encounter_for(caller, &runtime.repository, phase, request)
    })
}
