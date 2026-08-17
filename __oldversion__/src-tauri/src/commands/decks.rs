use serde::{Deserialize, Serialize};

use crate::commands::notes::require_idempotency_key;
use crate::domain::{EntityId, RepoError, UtcMillis};
use crate::ipc::{AppError, CallerIdentity, CommandResult, ErrorCode, panic_boundary};
use crate::notebook::NotebookRuntime;
use crate::notebook::repository::NotebookRepository;
use crate::providers::decks::{
    DeckCandidate, LookupOutcome, OfficialDeckProvider, PROVIDER_ID, ProviderConsent,
};
use crate::services::decks::{DeckDetails, DeckService, SaveCompleteDeck};
use crate::settings::AppState;

use super::classifier::DeckEnrichmentRuntime;

const DISCLOSED_FIELDS: [&str; 2] = ["confirmed_handle", "format"];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDeckProviderConsentRequest {
    pub granted: bool,
    #[serde(default)]
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckProviderStatus {
    pub provider_id: String,
    pub consent_granted: bool,
    pub access_mode: crate::providers::decks::AccessMode,
    pub disclosed_fields: Vec<String>,
    pub automatic_access_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupOfficialDeckRequest {
    pub encounter_id: String,
    pub encounter_generation: u64,
    pub request_token: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPublicSnapshotRequest {
    pub encounter_id: String,
    pub candidate: DeckCandidate,
    pub active_generation: u64,
    pub active_format: String,
    #[serde(default)]
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveCompleteDeckRequest {
    pub deck: SaveCompleteDeck,
    #[serde(default)]
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckDetailsRequest {
    pub deck_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOfficialPageRequest {
    pub url: String,
}

pub fn set_deck_provider_consent_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: SetDeckProviderConsentRequest,
) -> CommandResult<DeckProviderStatus> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    let fields = DISCLOSED_FIELDS.map(str::to_owned).to_vec();
    let result = serde_json::to_string(&fields)
        .map_err(|_| RepoError::InvalidRequest)
        .and_then(|json| {
            repository.set_provider_consent(PROVIDER_ID, request.granted, &json, UtcMillis::now())
        });
    match result {
        Ok(()) => CommandResult::success(
            DeckProviderStatus {
                provider_id: PROVIDER_ID.into(),
                consent_granted: request.granted,
                access_mode: crate::providers::decks::AccessMode::InteractiveRequired,
                disclosed_fields: fields,
                automatic_access_enabled: false,
            },
            1,
        ),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn lookup_official_deck_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    provider: &OfficialDeckProvider,
    request: LookupOfficialDeckRequest,
) -> CommandResult<LookupOutcome> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let consent = repository
        .provider_consent(PROVIDER_ID)
        .and_then(|stored| match stored {
            Some((true, json)) => {
                let disclosed_fields =
                    serde_json::from_str(&json).map_err(|_| RepoError::InvalidRequest)?;
                Ok(ProviderConsent {
                    granted: true,
                    disclosed_fields,
                })
            }
            _ => Err(RepoError::ConsentRequired),
        });
    let outcome = consent.and_then(|consent| {
        let encounter_id = EntityId::parse(request.encounter_id)?;
        repository
            .with_connection(|connection| {
                connection
                    .connection
                    .query_row(
                        "SELECT profile.primary_handle, encounter.format, encounter.generation
                         FROM encounters encounter
                         JOIN opponent_profiles profile ON profile.id = encounter.profile_id
                         WHERE encounter.id = ?1
                           AND encounter.deleted_at IS NULL
                           AND profile.deleted_at IS NULL",
                        [encounter_id.as_str()],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, i64>(2)?,
                            ))
                        },
                    )
                    .map_err(crate::notebook::repository::map_database_error)
            })
            .and_then(|(confirmed_handle, format, generation)| {
                if u64::try_from(generation).ok() != Some(request.encounter_generation) {
                    return Err(RepoError::StaleProviderResult);
                }
                provider.lookup(
                    &consent,
                    &confirmed_handle,
                    &format,
                    request.encounter_generation,
                    &request.request_token,
                )
            })
    });
    match outcome {
        Ok(outcome) => CommandResult::success(outcome, request.encounter_generation),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn confirm_public_snapshot_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    runtime: &DeckEnrichmentRuntime,
    request: ConfirmPublicSnapshotRequest,
) -> CommandResult<DeckDetails> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    runtime
        .reclassification_priority
        .set_interactive_operation(true);
    let result = EntityId::parse(request.encounter_id).and_then(|encounter_id| {
        DeckService::new(repository, &runtime.assets).confirm_public_snapshot(
            &runtime.provider,
            &encounter_id,
            &request.candidate,
            request.active_generation,
            &request.active_format,
        )
    });
    runtime
        .reclassification_priority
        .set_interactive_operation(false);
    deck_result(result)
}

pub fn save_complete_deck_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    runtime: &DeckEnrichmentRuntime,
    request: SaveCompleteDeckRequest,
) -> CommandResult<DeckDetails> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    runtime
        .reclassification_priority
        .set_interactive_operation(true);
    let result = DeckService::new(repository, &runtime.assets).save_complete_deck(request.deck);
    runtime
        .reclassification_priority
        .set_interactive_operation(false);
    deck_result(result)
}

pub fn get_deck_details_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    runtime: &DeckEnrichmentRuntime,
    request: DeckDetailsRequest,
) -> CommandResult<DeckDetails> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    deck_result(EntityId::parse(request.deck_id).and_then(|deck_id| {
        DeckService::new(repository, &runtime.assets).get_deck_details(&deck_id)
    }))
}

pub fn open_official_deck_page_for(
    caller: CallerIdentity,
    request: OpenOfficialPageRequest,
) -> CommandResult<()> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match crate::shell::open_official_mtgo_url(&request.url) {
        Ok(()) => CommandResult::success((), 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

fn deck_result(result: Result<DeckDetails, RepoError>) -> CommandResult<DeckDetails> {
    match result {
        Ok(details) => {
            let revision = details.revision_number;
            CommandResult::success(details, revision)
        }
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

#[tauri::command]
pub fn set_deck_provider_consent(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    request: SetDeckProviderConsentRequest,
) -> CommandResult<DeckProviderStatus> {
    panic_boundary("set-deck-provider-consent-command", || {
        with_caller(&window, |caller| {
            set_deck_provider_consent_for(caller, &notebook.repository, request)
        })
    })
}

#[tauri::command]
pub fn lookup_official_deck(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, DeckEnrichmentRuntime>,
    request: LookupOfficialDeckRequest,
) -> CommandResult<LookupOutcome> {
    panic_boundary("lookup-official-deck-command", || {
        with_caller(&window, |caller| {
            if let Err(error) = require_provider_access(state.inner()) {
                return CommandResult::failure(error);
            }
            lookup_official_deck_for(caller, &notebook.repository, &runtime.provider, request)
        })
    })
}

#[tauri::command]
pub fn confirm_public_snapshot(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, DeckEnrichmentRuntime>,
    request: ConfirmPublicSnapshotRequest,
) -> CommandResult<DeckDetails> {
    panic_boundary("confirm-public-snapshot-command", || {
        with_caller(&window, |caller| {
            confirm_public_snapshot_for(caller, &notebook.repository, &runtime, request)
        })
    })
}

#[tauri::command]
pub fn save_complete_deck(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, DeckEnrichmentRuntime>,
    request: SaveCompleteDeckRequest,
) -> CommandResult<DeckDetails> {
    panic_boundary("save-complete-deck-command", || {
        with_caller(&window, |caller| {
            save_complete_deck_for(caller, &notebook.repository, &runtime, request)
        })
    })
}

#[tauri::command]
pub fn get_deck_details(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, DeckEnrichmentRuntime>,
    request: DeckDetailsRequest,
) -> CommandResult<DeckDetails> {
    panic_boundary("get-deck-details-command", || {
        with_caller(&window, |caller| {
            get_deck_details_for(caller, &notebook.repository, &runtime, request)
        })
    })
}

#[tauri::command]
pub fn open_official_deck_page(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
    request: OpenOfficialPageRequest,
) -> CommandResult<()> {
    panic_boundary("open-official-deck-page-command", || {
        with_caller(&window, |caller| {
            if let Err(error) = require_provider_access(state.inner()) {
                return CommandResult::failure(error);
            }
            open_official_deck_page_for(caller, request)
        })
    })
}

fn require_provider_access(state: &AppState) -> Result<(), AppError> {
    match state.settings.lock() {
        Ok(store) if store.settings.provider_access_enabled => Ok(()),
        Ok(_) => Err(AppError::new(
            ErrorCode::ConsentRequired,
            "Enable disclosed provider access before opening an external provider.",
            false,
        )),
        Err(_) => Err(AppError::internal("provider-access-settings-lock")),
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

#[cfg(test)]
mod operational_tests {
    use super::*;

    #[test]
    fn provider_access_setting_fails_closed_before_any_external_path() {
        let state = AppState::default();
        assert_eq!(
            require_provider_access(&state)
                .expect_err("private default")
                .code,
            ErrorCode::ConsentRequired
        );
        state
            .settings
            .lock()
            .expect("settings")
            .settings
            .provider_access_enabled = true;
        require_provider_access(&state).expect("explicit access");
    }
}
