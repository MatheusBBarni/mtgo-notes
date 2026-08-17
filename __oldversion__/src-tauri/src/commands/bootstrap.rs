use serde::{Deserialize, Serialize};

use crate::domain::InternalPhase;
use crate::ipc::{CallerIdentity, CommandResult, panic_boundary};
use crate::settings::{AppState, Settings};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSummary {
    pub name: String,
    pub version: String,
    pub local_only: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapState {
    pub app: AppSummary,
    pub settings: Settings,
    pub encounter: Option<EncounterBootstrapView>,
    pub caller: CallerIdentity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterBootstrapView {
    pub id: String,
    pub phase: InternalPhase,
    pub revision: u64,
}

pub fn bootstrap_for(caller: CallerIdentity, state: &AppState) -> CommandResult<BootstrapState> {
    match state.notebook_error.lock() {
        Ok(guard) => {
            if let Some(error) = guard.clone() {
                return CommandResult::failure(error);
            }
        }
        Err(_) => {
            return CommandResult::failure(crate::ipc::AppError::internal(
                "bootstrap-notebook-state-lock",
            ));
        }
    }

    let store = match state.settings.lock() {
        Ok(store) => store,
        Err(_) => {
            return CommandResult::failure(crate::ipc::AppError::internal("bootstrap-state-lock"));
        }
    };

    CommandResult::success(
        BootstrapState {
            app: AppSummary {
                name: "MTGO Opponent Notes".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                local_only: true,
            },
            settings: store.settings.clone(),
            encounter: None,
            caller,
        },
        store.revision,
    )
}

#[tauri::command]
pub fn bootstrap(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> CommandResult<BootstrapState> {
    panic_boundary("bootstrap-command", || {
        let caller = match CallerIdentity::from_window_label(window.label()) {
            Ok(caller) => caller,
            Err(error) => return CommandResult::failure(error),
        };
        bootstrap_for(caller, state.inner())
    })
}
