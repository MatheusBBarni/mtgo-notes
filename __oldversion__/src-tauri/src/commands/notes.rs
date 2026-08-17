use serde::{Deserialize, Serialize};

use crate::domain::{EntityId, IdempotencyKey, RepoError, Revision};
use crate::ipc::{AppError, CallerIdentity, CommandResult, ErrorCode, panic_boundary};
use crate::notebook::NotebookRuntime;
use crate::notebook::key::DatabaseKey;
use crate::notebook::repository::NotebookRepository;
use crate::services::observations::{CardObservationInput, ObservationDetail, ObservationService};
use crate::services::profiles::{ProfileAggregate, ProfileService, ProfileSummary};
use tauri::Manager;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandleRequest {
    pub handle: String,
    #[serde(default)]
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileHandleRequest {
    pub profile_id: String,
    pub handle: String,
    #[serde(default)]
    pub idempotency_key: String,
    pub expected_revision: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestProfilesRequest {
    pub query: String,
    #[serde(default = "default_suggestion_limit")]
    pub limit: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveObservationRequest {
    pub encounter_id: String,
    pub text: String,
    #[serde(default)]
    pub cards: Vec<CardObservationInput>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub user_deck_label: Option<String>,
    #[serde(default)]
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateObservationRequest {
    pub observation_id: String,
    pub text: String,
    pub expected_revision: u64,
    #[serde(default)]
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCardObservationsRequest {
    pub observation_id: String,
    pub expected_revision: u64,
    pub cards: Vec<CardObservationInput>,
    #[serde(default)]
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTendencyTagsRequest {
    pub observation_id: String,
    pub expected_revision: u64,
    pub tags: Vec<String>,
    #[serde(default)]
    pub idempotency_key: String,
}

pub fn create_profile_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: HandleRequest,
) -> CommandResult<ProfileAggregate> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    profile_result(ProfileService::new(repository).create(&request.handle))
}

pub fn resolve_profile_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: HandleRequest,
) -> CommandResult<Option<ProfileSummary>> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match ProfileService::new(repository).resolve_exact(&request.handle) {
        Ok(profile) => CommandResult::success(profile, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn suggest_profiles_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: SuggestProfilesRequest,
) -> CommandResult<Vec<ProfileSummary>> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match ProfileService::new(repository).suggestions(&request.query, request.limit) {
        Ok(profiles) => CommandResult::success(profiles, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn add_alias_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: ProfileHandleRequest,
) -> CommandResult<ProfileAggregate> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    let profile_id = match EntityId::parse(request.profile_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    profile_result(ProfileService::new(repository).add_alias(&profile_id, &request.handle))
}

pub fn update_profile_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: ProfileHandleRequest,
) -> CommandResult<ProfileAggregate> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    let parsed = EntityId::parse(request.profile_id).and_then(|id| {
        request
            .expected_revision
            .ok_or(RepoError::InvalidRequest)
            .and_then(Revision::new)
            .map(|revision| (id, revision))
    });
    let (profile_id, expected_revision) = match parsed {
        Ok(value) => value,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    profile_result(ProfileService::new(repository).update_primary_handle(
        &profile_id,
        expected_revision,
        &request.handle,
    ))
}

pub fn save_observation_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: SaveObservationRequest,
) -> CommandResult<ObservationDetail> {
    if let Err(error) = caller.require(&[CallerIdentity::Main, CallerIdentity::Capture]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    let encounter_id = match EntityId::parse(request.encounter_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    if let Err(error) = ObservationService::validate_enrichment(
        &request.cards,
        &request.tags,
        request.user_deck_label.as_deref(),
    ) {
        return CommandResult::failure(error.to_app_error());
    }
    let service = ObservationService::new(repository);
    let mut detail = match service.create(&encounter_id, &request.text) {
        Ok(detail) => detail,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    if !request.cards.is_empty() {
        detail = match Revision::new(detail.revision).and_then(|revision| {
            service.set_cards(
                &EntityId::parse(detail.id.clone())?,
                revision,
                request.cards,
            )
        }) {
            Ok(detail) => detail,
            Err(error) => return CommandResult::failure(error.to_app_error()),
        };
    }
    if !request.tags.is_empty() {
        detail = match Revision::new(detail.revision).and_then(|revision| {
            service.set_tags(&EntityId::parse(detail.id.clone())?, revision, request.tags)
        }) {
            Ok(detail) => detail,
            Err(error) => return CommandResult::failure(error.to_app_error()),
        };
    }
    if request.user_deck_label.is_some() {
        detail = match Revision::new(detail.revision).and_then(|revision| {
            service.save_user_deck_label(
                &EntityId::parse(detail.id.clone())?,
                revision,
                request.user_deck_label.as_deref(),
            )
        }) {
            Ok(detail) => detail,
            Err(error) => return CommandResult::failure(error.to_app_error()),
        };
    }
    let revision = detail.revision;
    CommandResult::success(detail, revision)
}

pub fn save_observation_with_capture_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    key: &DatabaseKey,
    capture: &crate::commands::capture::CaptureRuntime,
    request: SaveObservationRequest,
) -> CommandResult<ObservationDetail> {
    let encounter_id = if caller == CallerIdentity::Capture {
        match EntityId::parse(&request.encounter_id) {
            Ok(encounter_id) => {
                if let Err(error) = capture.preserve(repository, key, &encounter_id, &request.text)
                {
                    return CommandResult::failure(error.to_app_error());
                }
                Some(encounter_id)
            }
            Err(error) => return CommandResult::failure(error.to_app_error()),
        }
    } else {
        None
    };
    let result = save_observation_for(caller, repository, request);
    if result.is_success()
        && let Some(encounter_id) = encounter_id
        && let Err(error) = capture.complete(repository, &encounter_id)
    {
        return CommandResult::failure(error.to_app_error());
    }
    result
}

pub fn update_observation_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: UpdateObservationRequest,
) -> CommandResult<ObservationDetail> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    observation_result(
        EntityId::parse(request.observation_id)
            .and_then(|id| Revision::new(request.expected_revision).map(|revision| (id, revision)))
            .and_then(|(id, revision)| {
                ObservationService::new(repository).update_text(&id, revision, &request.text)
            }),
    )
}

pub fn set_card_observations_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: SetCardObservationsRequest,
) -> CommandResult<ObservationDetail> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    observation_result(
        EntityId::parse(request.observation_id)
            .and_then(|id| Revision::new(request.expected_revision).map(|revision| (id, revision)))
            .and_then(|(id, revision)| {
                ObservationService::new(repository).set_cards(&id, revision, request.cards)
            }),
    )
}

pub fn set_tendency_tags_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: SetTendencyTagsRequest,
) -> CommandResult<ObservationDetail> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    observation_result(
        EntityId::parse(request.observation_id)
            .and_then(|id| Revision::new(request.expected_revision).map(|revision| (id, revision)))
            .and_then(|(id, revision)| {
                ObservationService::new(repository).set_tags(&id, revision, request.tags)
            }),
    )
}

#[tauri::command]
pub fn create_profile(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: HandleRequest,
) -> CommandResult<ProfileAggregate> {
    panic_boundary("create-profile-command", || {
        with_caller(&window, |caller| {
            create_profile_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn resolve_profile(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: HandleRequest,
) -> CommandResult<Option<ProfileSummary>> {
    panic_boundary("resolve-profile-command", || {
        with_caller(&window, |caller| {
            resolve_profile_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn suggest_profiles(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: SuggestProfilesRequest,
) -> CommandResult<Vec<ProfileSummary>> {
    panic_boundary("suggest-profiles-command", || {
        with_caller(&window, |caller| {
            suggest_profiles_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn add_alias(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: ProfileHandleRequest,
) -> CommandResult<ProfileAggregate> {
    panic_boundary("add-alias-command", || {
        with_caller(&window, |caller| {
            add_alias_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn update_profile(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: ProfileHandleRequest,
) -> CommandResult<ProfileAggregate> {
    panic_boundary("update-profile-command", || {
        with_caller(&window, |caller| {
            update_profile_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn save_observation(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    capture: tauri::State<'_, crate::commands::capture::CaptureRuntime>,
    request: SaveObservationRequest,
) -> CommandResult<ObservationDetail> {
    panic_boundary("save-observation-command", || {
        with_caller(&window, |caller| {
            let result = save_observation_with_capture_for(
                caller,
                &runtime.repository,
                &runtime.key,
                capture.inner(),
                request,
            );
            if result.is_success() {
                let _ = crate::commands::encounters::emit_current_overlay(
                    window.app_handle(),
                    &runtime.repository,
                );
            }
            result
        })
    })
}

#[tauri::command]
pub fn update_observation(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: UpdateObservationRequest,
) -> CommandResult<ObservationDetail> {
    panic_boundary("update-observation-command", || {
        with_caller(&window, |caller| {
            update_observation_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn set_card_observations(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: SetCardObservationsRequest,
) -> CommandResult<ObservationDetail> {
    panic_boundary("set-card-observations-command", || {
        with_caller(&window, |caller| {
            set_card_observations_for(caller, &runtime.repository, request)
        })
    })
}

#[tauri::command]
pub fn set_tendency_tags(
    window: tauri::WebviewWindow,
    runtime: tauri::State<'_, NotebookRuntime>,
    request: SetTendencyTagsRequest,
) -> CommandResult<ObservationDetail> {
    panic_boundary("set-tendency-tags-command", || {
        with_caller(&window, |caller| {
            set_tendency_tags_for(caller, &runtime.repository, request)
        })
    })
}

fn default_suggestion_limit() -> usize {
    20
}

pub(crate) fn require_idempotency_key(value: &str) -> Result<IdempotencyKey, AppError> {
    IdempotencyKey::parse(value).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidRequest,
            "A valid idempotency key is required.",
            false,
        )
        .with_field("idempotencyKey")
    })
}

fn profile_result(result: Result<ProfileAggregate, RepoError>) -> CommandResult<ProfileAggregate> {
    match result {
        Ok(profile) => {
            let revision = profile.profile.revision.get();
            CommandResult::success(profile, revision)
        }
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

fn observation_result(
    result: Result<ObservationDetail, RepoError>,
) -> CommandResult<ObservationDetail> {
    match result {
        Ok(observation) => {
            let revision = observation.revision;
            CommandResult::success(observation, revision)
        }
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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::commands::capture::{CaptureRuntime, OpenCaptureRequest, open_capture_for};
    use crate::domain::UtcMillis;
    use crate::notebook::migrations::MigrationManager;
    use crate::services::profiles::ProfileService;

    fn request(encounter_id: &EntityId, text: &str) -> SaveObservationRequest {
        SaveObservationRequest {
            encounter_id: encounter_id.to_string(),
            text: text.into(),
            cards: Vec::new(),
            tags: Vec::new(),
            user_deck_label: None,
            idempotency_key: EntityId::new().to_string(),
        }
    }

    #[test]
    fn it_245_failed_capture_save_preserves_encrypted_draft_and_success_removes_it() {
        let directory = TempDir::new().expect("temp");
        let key = DatabaseKey::generate().expect("key");
        MigrationManager::default()
            .migrate(directory.path().join("notebook.db"), &key)
            .expect("migrate");
        let repository =
            NotebookRepository::open(directory.path().join("notebook.db"), &key).expect("open");
        let profile = ProfileService::new(&repository)
            .create("DraftOpponent")
            .expect("profile");
        let encounter_id = EntityId::new();
        repository
            .start_encounter(&encounter_id, &profile.profile.id, UtcMillis::now(), 1)
            .expect("encounter");
        let capture = CaptureRuntime::default();
        assert!(
            open_capture_for(
                CallerIdentity::Main,
                &repository,
                &key,
                &capture,
                OpenCaptureRequest {
                    idempotency_key: EntityId::new().to_string(),
                },
            )
            .is_success()
        );

        assert!(
            !save_observation_with_capture_for(
                CallerIdentity::Capture,
                &repository,
                &key,
                &capture,
                request(&encounter_id, " "),
            )
            .is_success()
        );
        assert!(
            repository
                .capture_draft(&encounter_id)
                .expect("draft")
                .is_some()
        );

        assert!(
            save_observation_with_capture_for(
                CallerIdentity::Capture,
                &repository,
                &key,
                &capture,
                request(&encounter_id, "Saved after retry"),
            )
            .is_success()
        );
        assert!(
            repository
                .capture_draft(&encounter_id)
                .expect("draft")
                .is_none()
        );
    }
}
