use std::sync::Mutex;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use crate::commands::notes::require_idempotency_key;
use crate::detection::{ConfidenceClass, ContextEvidence, ContextField, EvidenceProvenance};
use crate::disclosure::{
    DisclosurePolicy, NotebookState, ObservationView, OverlayView, PublicSnapshotView,
};
use crate::domain::{EncounterStatus, EntityId, InternalPhase, RepoError, Revision, UtcMillis};
use crate::encounters::{
    ActiveEncounter, ContextEvidence as ReducerEvidence, EncounterAction, EncounterReducer,
    EncounterRuntime as ReducerRuntime, EvidenceKind, EvidenceSource,
};
use crate::ipc::{
    CallerIdentity, CommandResult, EventName, ReplacementEvent, next_event_revision, panic_boundary,
};
use crate::notebook::{NotebookRuntime, repository::NotebookRepository};
use crate::services::history::{EncounterDetail, HistoryService};
use crate::services::profiles::{ProfileService, normalize_handle};

const TRANSITION_UNDO_WINDOW_MS: i64 = 30_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpponentCandidate {
    pub display_handle: String,
    pub normalized_handle: String,
    pub provider_session: String,
    pub generation: u64,
    pub sequence: u64,
    pub provenance: EvidenceProvenance,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterStateView {
    pub candidate: Option<OpponentCandidate>,
    pub encounter: Option<EncounterCommandView>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterCommandView {
    pub id: String,
    pub profile_id: String,
    pub primary_handle: String,
    pub phase: InternalPhase,
    pub generation: u64,
    pub revision: u64,
    pub undo_group_id: Option<String>,
    pub undo_deadline: Option<i64>,
}

#[derive(Clone, Debug)]
struct UndoState {
    group_id: EntityId,
    deadline: i64,
}

#[derive(Default)]
struct EncounterCommandState {
    candidate: Option<OpponentCandidate>,
    undo: Option<UndoState>,
    reducer_runtime: Option<ReducerRuntime>,
    ocr_phase_seen: Option<(u64, String, u64)>,
}

#[derive(Default)]
pub struct EncounterCommandRuntime {
    state: Mutex<EncounterCommandState>,
}

impl EncounterCommandRuntime {
    pub fn accept_detector_evidence(
        &self,
        evidence: &ContextEvidence,
    ) -> Result<Option<OpponentCandidate>, RepoError> {
        if evidence.field != ContextField::Opponent
            || evidence.confidence_class == ConfidenceClass::Ineligible
        {
            return Ok(None);
        }
        let candidate = OpponentCandidate {
            display_handle: evidence.display_value.clone(),
            normalized_handle: evidence.normalized_value.clone(),
            provider_session: evidence.provider_session.clone(),
            generation: evidence.generation,
            sequence: evidence.sequence,
            provenance: evidence.provenance,
        };
        self.state
            .lock()
            .map_err(|_| RepoError::InvalidTransition)?
            .candidate = Some(candidate.clone());
        Ok(Some(candidate))
    }

    pub fn apply_detector_evidence(
        &self,
        repository: &NotebookRepository,
        evidence: &ContextEvidence,
        now: UtcMillis,
    ) -> Result<Option<EncounterCommandView>, RepoError> {
        if evidence.field == ContextField::Opponent {
            self.accept_detector_evidence(evidence)?;
            return Ok(None);
        }
        if evidence.field != ContextField::Phase
            || evidence.confidence_class == ConfidenceClass::Ineligible
        {
            return Ok(None);
        }
        let active = match repository.active_encounter()? {
            Some(active) => active,
            None => return Ok(None),
        };
        let encounter_id = EntityId::parse(&active.id)?;
        let profile_id = EntityId::parse(&active.profile_id)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepoError::InvalidTransition)?;
        let replace_runtime = state.reducer_runtime.as_ref().is_none_or(|runtime| {
            runtime.provider_session != evidence.provider_session
                || runtime.generation != evidence.generation
                || runtime.active.as_ref().map(|active| active.id.as_str())
                    != Some(encounter_id.as_str())
        });
        if replace_runtime {
            state.reducer_runtime = Some(ReducerRuntime {
                provider_session: evidence.provider_session.clone(),
                generation: evidence.generation,
                last_sequence: 0,
                phase: active.phase,
                active: Some(ActiveEncounter {
                    id: encounter_id.clone(),
                    profile_id,
                    status: EncounterStatus::Active,
                    unconfirmed_deck_present: false,
                }),
            });
            state.ocr_phase_seen = None;
        }
        let stable_for_ms = if evidence.provenance == EvidenceProvenance::Ocr {
            match state.ocr_phase_seen.as_ref() {
                Some((generation, value, first_seen))
                    if *generation == evidence.generation
                        && value == &evidence.normalized_value =>
                {
                    evidence.monotonic_ms.saturating_sub(*first_seen)
                }
                _ => {
                    state.ocr_phase_seen = Some((
                        evidence.generation,
                        evidence.normalized_value.clone(),
                        evidence.monotonic_ms,
                    ));
                    0
                }
            }
        } else {
            state.ocr_phase_seen = None;
            u64::MAX
        };
        let target_phase = phase_from_normalized(&evidence.normalized_value);
        let kind = if target_phase == InternalPhase::InGameRestricted {
            EvidenceKind::StrongGameplay
        } else {
            EvidenceKind::TrustedPhase {
                phase: target_phase,
                stable_for_ms,
            }
        };
        let reducer_evidence = ReducerEvidence {
            provider_session: evidence.provider_session.clone(),
            generation: evidence.generation,
            sequence: evidence.sequence,
            monotonic_ms: evidence.monotonic_ms,
            source: match evidence.provenance {
                EvidenceProvenance::Uia => EvidenceSource::TrustedUia,
                EvidenceProvenance::Ocr => EvidenceSource::Ocr,
                EvidenceProvenance::Manual => EvidenceSource::Manual,
            },
            evidence: kind,
        };
        let current_runtime = state
            .reducer_runtime
            .as_ref()
            .ok_or(RepoError::InvalidTransition)?;
        let reduction = EncounterReducer.reduce(current_runtime, reducer_evidence)?;
        let mut changed = false;
        let mut revision = active.revision;
        for action in &reduction.actions {
            if let EncounterAction::ChangePhase {
                encounter_id, to, ..
            } = action
            {
                revision = repository
                    .correct_encounter_phase(
                        encounter_id,
                        Revision::new(revision)?,
                        *to,
                        now,
                        match evidence.provenance {
                            EvidenceProvenance::Uia => "automatic_uia",
                            EvidenceProvenance::Ocr => "automatic_ocr",
                            EvidenceProvenance::Manual => "manual_correction",
                        },
                    )?
                    .get();
                changed = true;
            }
        }
        state.reducer_runtime = Some(reduction.runtime);
        drop(state);
        if changed {
            let mut view = active_view(repository, None, None)?;
            view.revision = revision;
            Ok(Some(view))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmOpponentRequest {
    pub provider_session: String,
    pub candidate_generation: u64,
    pub candidate_sequence: u64,
    pub corrected_handle: Option<String>,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterOpponentRequest {
    pub handle: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectPhaseRequest {
    pub encounter_id: String,
    pub phase: InternalPhase,
    pub expected_revision: u64,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterMutationRequest {
    pub encounter_id: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoTransitionRequest {
    pub undo_group_id: String,
    pub idempotency_key: String,
}

pub fn confirm_opponent_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    runtime: &EncounterCommandRuntime,
    request: ConfirmOpponentRequest,
    now: UtcMillis,
) -> CommandResult<EncounterCommandView> {
    if let Err(error) = caller.require(&[CallerIdentity::Main, CallerIdentity::Overlay]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    let candidate = match runtime
        .state
        .lock()
        .map_err(|_| RepoError::InvalidTransition)
        .and_then(|state| state.candidate.clone().ok_or(RepoError::CandidateStale))
    {
        Ok(candidate)
            if candidate.provider_session == request.provider_session
                && candidate.generation == request.candidate_generation
                && candidate.sequence == request.candidate_sequence =>
        {
            candidate
        }
        Ok(_) | Err(RepoError::CandidateStale) => {
            return CommandResult::failure(RepoError::CandidateStale.to_app_error());
        }
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let handle = request
        .corrected_handle
        .as_deref()
        .unwrap_or(&candidate.display_handle);
    start_or_reuse(
        repository,
        runtime,
        handle,
        candidate.generation,
        now,
        match candidate.provenance {
            EvidenceProvenance::Uia => "uia",
            EvidenceProvenance::Ocr => "ocr",
            EvidenceProvenance::Manual => "manual",
        },
        &candidate.provider_session,
    )
}

pub fn enter_opponent_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    runtime: &EncounterCommandRuntime,
    request: EnterOpponentRequest,
    now: UtcMillis,
) -> CommandResult<EncounterCommandView> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    match repository.active_encounter() {
        Ok(Some(_)) => {
            return CommandResult::failure(RepoError::ExplicitCorrectionRequired.to_app_error());
        }
        Err(error) => return CommandResult::failure(error.to_app_error()),
        Ok(None) => {}
    }
    let generation = runtime
        .state
        .lock()
        .ok()
        .and_then(|state| {
            state
                .candidate
                .as_ref()
                .map(|candidate| candidate.generation + 1)
        })
        .unwrap_or(1);
    start_or_reuse(
        repository,
        runtime,
        &request.handle,
        generation,
        now,
        "manual",
        "manual",
    )
}

pub fn correct_phase_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: CorrectPhaseRequest,
    now: UtcMillis,
) -> CommandResult<EncounterCommandView> {
    if let Err(error) = caller.require(&[CallerIdentity::Main, CallerIdentity::Overlay]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    if matches!(
        request.phase,
        InternalPhase::Idle
            | InternalPhase::Candidate
            | InternalPhase::Finished
            | InternalPhase::Incomplete
    ) {
        return CommandResult::failure(RepoError::InvalidTransition.to_app_error());
    }
    let parsed = EntityId::parse(request.encounter_id)
        .and_then(|id| Revision::new(request.expected_revision).map(|revision| (id, revision)));
    let (encounter_id, expected_revision) = match parsed {
        Ok(parsed) => parsed,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    match repository.correct_encounter_phase(
        &encounter_id,
        expected_revision,
        request.phase,
        now,
        "manual_correction",
    ) {
        Ok(revision) => match active_view(repository, None, None) {
            Ok(mut view) => {
                view.revision = revision.get();
                CommandResult::success(view, revision.get())
            }
            Err(error) => CommandResult::failure(error.to_app_error()),
        },
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn finish_encounter_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: EncounterMutationRequest,
    now: UtcMillis,
) -> CommandResult<EncounterCommandView> {
    if let Err(error) = caller.require(&[CallerIdentity::Main, CallerIdentity::Overlay]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    let encounter_id = match EntityId::parse(request.encounter_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let before = match active_view(repository, None, None) {
        Ok(view) if view.id == encounter_id.as_str() => view,
        Ok(_) => return CommandResult::failure(RepoError::InvalidTransition.to_app_error()),
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    match repository.finish_encounter(&encounter_id, now) {
        Ok(()) => {
            let view = EncounterCommandView {
                phase: InternalPhase::Finished,
                revision: before.revision.saturating_add(1),
                ..before
            };
            CommandResult::success(view.clone(), view.revision)
        }
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn reopen_encounter_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    request: EncounterMutationRequest,
) -> CommandResult<EncounterDetail> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    let encounter_id = match EntityId::parse(request.encounter_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    match HistoryService::new(repository, &DisclosurePolicy)
        .get_encounter(InternalPhase::Finished, &encounter_id)
    {
        Ok(detail) => {
            let revision = detail.summary.revision;
            CommandResult::success(detail, revision)
        }
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn undo_transition_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    runtime: &EncounterCommandRuntime,
    request: UndoTransitionRequest,
    now: UtcMillis,
) -> CommandResult<EncounterCommandView> {
    if let Err(error) = caller.require(&[CallerIdentity::Main, CallerIdentity::Overlay]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    let group_id = match EntityId::parse(request.undo_group_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let valid = runtime
        .state
        .lock()
        .ok()
        .and_then(|state| state.undo.clone())
        .filter(|undo| undo.group_id == group_id && now.get() <= undo.deadline);
    if valid.is_none() {
        return CommandResult::failure(RepoError::UndoExpired.to_app_error());
    }
    match repository.undo_encounter_replacement(&group_id) {
        Ok(_) => {
            if let Ok(mut state) = runtime.state.lock() {
                state.undo = None;
            }
            match active_view(repository, None, None) {
                Ok(view) => CommandResult::success(view.clone(), view.revision),
                Err(error) => CommandResult::failure(error.to_app_error()),
            }
        }
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

#[tauri::command]
pub fn confirm_opponent(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, EncounterCommandRuntime>,
    request: ConfirmOpponentRequest,
) -> CommandResult<EncounterCommandView> {
    panic_boundary("confirm-opponent-command", || {
        let result = with_caller(&window, |caller| {
            confirm_opponent_for(
                caller,
                &notebook.repository,
                runtime.inner(),
                request,
                UtcMillis::now(),
            )
        });
        emit_overlay_for_result(&window, &notebook.repository, &result);
        result
    })
}

#[tauri::command]
pub fn enter_opponent(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, EncounterCommandRuntime>,
    request: EnterOpponentRequest,
) -> CommandResult<EncounterCommandView> {
    panic_boundary("enter-opponent-command", || {
        let result = with_caller(&window, |caller| {
            enter_opponent_for(
                caller,
                &notebook.repository,
                runtime.inner(),
                request,
                UtcMillis::now(),
            )
        });
        emit_overlay_for_result(&window, &notebook.repository, &result);
        result
    })
}

#[tauri::command]
pub fn correct_phase(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    request: CorrectPhaseRequest,
) -> CommandResult<EncounterCommandView> {
    panic_boundary("correct-phase-command", || {
        let result = with_caller(&window, |caller| {
            correct_phase_for(caller, &notebook.repository, request, UtcMillis::now())
        });
        emit_overlay_for_result(&window, &notebook.repository, &result);
        result
    })
}

#[tauri::command]
pub fn finish_encounter(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    request: EncounterMutationRequest,
) -> CommandResult<EncounterCommandView> {
    panic_boundary("finish-encounter-command", || {
        let result = with_caller(&window, |caller| {
            finish_encounter_for(caller, &notebook.repository, request, UtcMillis::now())
        });
        emit_overlay_for_result(&window, &notebook.repository, &result);
        result
    })
}

#[tauri::command]
pub fn reopen_encounter(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    request: EncounterMutationRequest,
) -> CommandResult<EncounterDetail> {
    panic_boundary("reopen-encounter-command", || {
        with_caller(&window, |caller| {
            reopen_encounter_for(caller, &notebook.repository, request)
        })
    })
}

#[tauri::command]
pub fn undo_transition(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, EncounterCommandRuntime>,
    request: UndoTransitionRequest,
) -> CommandResult<EncounterCommandView> {
    panic_boundary("undo-transition-command", || {
        let result = with_caller(&window, |caller| {
            undo_transition_for(
                caller,
                &notebook.repository,
                runtime.inner(),
                request,
                UtcMillis::now(),
            )
        });
        emit_overlay_for_result(&window, &notebook.repository, &result);
        result
    })
}

fn start_or_reuse(
    repository: &NotebookRepository,
    runtime: &EncounterCommandRuntime,
    handle: &str,
    generation: u64,
    now: UtcMillis,
    source: &str,
    provider_session: &str,
) -> CommandResult<EncounterCommandView> {
    let normalized = match normalize_handle(handle) {
        Ok(normalized) => normalized,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let encounter_id = EntityId::new();
    let undo_group = EntityId::new();
    let confirmed = match repository.confirm_opponent_encounter(
        &encounter_id,
        &normalized.display,
        &normalized.key,
        now,
        generation,
        &undo_group,
        source,
    ) {
        Ok(confirmed) => confirmed,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let profile = match ProfileService::new(repository).get(&confirmed.profile_id) {
        Ok(profile) => profile,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    if !confirmed.started_new {
        let active = match repository.active_encounter() {
            Ok(Some(active)) => active,
            Ok(None) => {
                return CommandResult::failure(RepoError::InvalidTransition.to_app_error());
            }
            Err(error) => return CommandResult::failure(error.to_app_error()),
        };
        {
            if let Ok(mut state) = runtime.state.lock() {
                state.candidate = None;
                state.reducer_runtime =
                    reducer_runtime_for(&active, provider_session, active.phase).ok();
            }
        }
        return match active_view(repository, None, None) {
            Ok(view) => CommandResult::success(view.clone(), view.revision),
            Err(error) => CommandResult::failure(error.to_app_error()),
        };
    }
    let (undo_group, undo_deadline) = if confirmed.replaced_encounter_id.is_some() {
        (
            Some(undo_group),
            Some(now.get().saturating_add(TRANSITION_UNDO_WINDOW_MS)),
        )
    } else {
        (None, None)
    };
    if let Ok(mut state) = runtime.state.lock() {
        state.candidate = None;
        state.undo = undo_group
            .clone()
            .zip(undo_deadline)
            .map(|(group_id, deadline)| UndoState { group_id, deadline });
        state.reducer_runtime = Some(ReducerRuntime {
            provider_session: provider_session.to_owned(),
            generation,
            last_sequence: 0,
            phase: InternalPhase::PreMatch,
            active: Some(ActiveEncounter {
                id: encounter_id.clone(),
                profile_id: profile.profile.id.clone(),
                status: EncounterStatus::Active,
                unconfirmed_deck_present: false,
            }),
        });
        state.ocr_phase_seen = None;
    }
    let view = EncounterCommandView {
        id: encounter_id.to_string(),
        profile_id: profile.profile.id.to_string(),
        primary_handle: profile.profile.primary_handle,
        phase: InternalPhase::PreMatch,
        generation,
        revision: 1,
        undo_group_id: undo_group.map(|group| group.to_string()),
        undo_deadline,
    };
    CommandResult::success(view, 1)
}

fn reducer_runtime_for(
    active: &crate::notebook::repository::ActiveEncounterRecord,
    provider_session: &str,
    phase: InternalPhase,
) -> Result<ReducerRuntime, RepoError> {
    Ok(ReducerRuntime {
        provider_session: provider_session.to_owned(),
        generation: active.generation,
        last_sequence: 0,
        phase,
        active: Some(ActiveEncounter {
            id: EntityId::parse(&active.id)?,
            profile_id: EntityId::parse(&active.profile_id)?,
            status: EncounterStatus::Active,
            unconfirmed_deck_present: false,
        }),
    })
}

fn phase_from_normalized(value: &str) -> InternalPhase {
    match value {
        "pre_match" => InternalPhase::PreMatch,
        "between_games" => InternalPhase::BetweenGames,
        "completion_pending" | "finished" => InternalPhase::CompletionPending,
        _ => InternalPhase::InGameRestricted,
    }
}

fn active_view(
    repository: &NotebookRepository,
    undo_group_id: Option<String>,
    undo_deadline: Option<i64>,
) -> Result<EncounterCommandView, RepoError> {
    let active = repository
        .active_encounter()?
        .ok_or(RepoError::NoActiveEncounter)?;
    let profile = ProfileService::new(repository).get(&EntityId::parse(&active.profile_id)?)?;
    Ok(EncounterCommandView {
        id: active.id,
        profile_id: active.profile_id,
        primary_handle: profile.profile.primary_handle,
        phase: active.phase,
        generation: active.generation,
        revision: active.revision,
        undo_group_id,
        undo_deadline,
    })
}

fn emit_overlay_for_result(
    source: &tauri::WebviewWindow,
    repository: &NotebookRepository,
    result: &CommandResult<EncounterCommandView>,
) {
    let CommandResult::Success { data, .. } = result else {
        return;
    };
    let encounter_event = ReplacementEvent::v1(
        EventName::EncounterState,
        next_event_revision(),
        EncounterStateView {
            candidate: None,
            encounter: Some(data.clone()),
        },
    );
    let _ = source.emit("encounter://state-v1", encounter_event.clone());
    if let Some(main) = source.app_handle().get_webview_window("main")
        && main.label() != source.label()
    {
        let _ = main.emit("encounter://state-v1", encounter_event);
    }
    let Ok(view) = overlay_view(repository, data) else {
        return;
    };
    if let Some(overlay) = source.app_handle().get_webview_window("overlay") {
        let _ = overlay.emit(
            "overlay://view-v1",
            ReplacementEvent::v1(EventName::OverlayView, next_event_revision(), view),
        );
        if !overlay_enabled(source.app_handle()) {
            let _ = overlay.hide();
            return;
        }
        if overlay.is_visible().ok() == Some(false) {
            let _ = overlay.set_ignore_cursor_events(true);
            let _ = overlay.show();
        }
    }
}

pub fn emit_opponent_candidate(app: &tauri::AppHandle, candidate: OpponentCandidate) {
    let event = ReplacementEvent::v1(
        EventName::EncounterState,
        next_event_revision(),
        EncounterStateView {
            candidate: Some(candidate),
            encounter: None,
        },
    );
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.emit("encounter://state-v1", event.clone());
    }
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.emit("encounter://state-v1", event);
    }
}

pub fn emit_current_overlay(
    app: &tauri::AppHandle,
    repository: &NotebookRepository,
) -> Result<(), RepoError> {
    let encounter = active_view(repository, None, None)?;
    let projection = overlay_view(repository, &encounter)?;
    if let Some(overlay) = app.get_webview_window("overlay") {
        overlay
            .emit(
                "overlay://view-v1",
                ReplacementEvent::v1(EventName::OverlayView, next_event_revision(), projection),
            )
            .map_err(|_| RepoError::OverlayUnavailable)?;
        if !overlay_enabled(app) {
            overlay.hide().map_err(|_| RepoError::OverlayUnavailable)?;
            return Ok(());
        }
        if overlay.is_visible().ok() == Some(false) {
            overlay
                .set_ignore_cursor_events(true)
                .map_err(|_| RepoError::OverlayUnavailable)?;
            overlay.show().map_err(|_| RepoError::OverlayUnavailable)?;
        }
    }
    Ok(())
}

fn overlay_enabled(app: &tauri::AppHandle) -> bool {
    app.state::<crate::settings::AppState>()
        .settings
        .lock()
        .map(|store| store.settings.overlay_enabled)
        .unwrap_or(false)
}

pub fn restrict_active_for_provider_interruption(
    repository: &NotebookRepository,
    trigger: &str,
    now: UtcMillis,
) -> Result<bool, RepoError> {
    let Some(active) = repository.active_encounter()? else {
        return Ok(false);
    };
    if active.phase == InternalPhase::InGameRestricted {
        return Ok(true);
    }
    repository.correct_encounter_phase(
        &EntityId::parse(&active.id)?,
        Revision::new(active.revision)?,
        InternalPhase::InGameRestricted,
        now,
        trigger,
    )?;
    Ok(true)
}

pub fn emit_fail_closed_overlay(app: &tauri::AppHandle) {
    let projection = DisclosurePolicy.overlay(&NotebookState {
        phase: InternalPhase::InGameRestricted,
        confirmed_handle: None,
        active_profile_deleted: false,
        current_observations: Vec::new(),
        historical_observations: Vec::new(),
        public_snapshot: None,
    });
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.emit(
            "overlay://view-v1",
            ReplacementEvent::v1(EventName::OverlayView, next_event_revision(), projection),
        );
    }
}

fn overlay_view(
    repository: &NotebookRepository,
    encounter: &EncounterCommandView,
) -> Result<OverlayView, RepoError> {
    let current = repository.with_connection(|connection| {
        let mut statement = connection
            .connection
            .prepare(
                "SELECT id, text FROM observations
                 WHERE encounter_id = ?1 AND deleted_at IS NULL
                 ORDER BY created_at DESC, id DESC LIMIT 10",
            )
            .map_err(|_| RepoError::NotebookInvalid)?;
        statement
            .query_map([&encounter.id], |row| {
                Ok(ObservationView {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    editable: false,
                })
            })
            .map_err(|_| RepoError::NotebookInvalid)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| RepoError::NotebookInvalid)
    })?;
    let historical = repository.with_connection(|connection| {
        let mut statement = connection
            .connection
            .prepare(
                "SELECT observation.id, observation.text
                 FROM observations observation
                 JOIN encounters encounter ON encounter.id = observation.encounter_id
                 WHERE encounter.profile_id = ?1
                   AND encounter.id <> ?2
                   AND encounter.status IN ('finished', 'incomplete')
                   AND encounter.deleted_at IS NULL
                   AND observation.deleted_at IS NULL
                 ORDER BY observation.created_at DESC, observation.id DESC LIMIT 20",
            )
            .map_err(|_| RepoError::NotebookInvalid)?;
        statement
            .query_map([&encounter.profile_id, &encounter.id], |row| {
                Ok(ObservationView {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    editable: false,
                })
            })
            .map_err(|_| RepoError::NotebookInvalid)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| RepoError::NotebookInvalid)
    })?;
    let public_snapshot = repository.with_connection(|connection| {
        connection
            .connection
            .query_row(
                "SELECT coalesce(deck.provider_label, 'Official MTGO deck'),
                        snapshot.format, snapshot.publication_date, snapshot.provider
                 FROM public_snapshots snapshot
                 JOIN deck_revisions revision ON revision.id = snapshot.deck_revision_id
                 JOIN deck_records deck ON deck.id = revision.deck_id
                 WHERE snapshot.confirmed = 1
                   AND snapshot.encounter_id = ?1
                   AND deck.profile_id = ?2
                 ORDER BY snapshot.publication_date DESC, snapshot.id DESC LIMIT 1",
                [&encounter.id, &encounter.profile_id],
                |row| {
                    Ok(PublicSnapshotView {
                        label: row.get(0)?,
                        format: row.get(1)?,
                        published_at: row.get(2)?,
                        source_text: row.get(3)?,
                        available: true,
                    })
                },
            )
            .optional()
            .map_err(|_| RepoError::NotebookInvalid)
    })?;
    Ok(DisclosurePolicy.overlay(&NotebookState {
        phase: encounter.phase,
        confirmed_handle: Some(encounter.primary_handle.clone()),
        active_profile_deleted: false,
        current_observations: current,
        historical_observations: historical,
        public_snapshot,
    }))
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
    use crate::notebook::key::DatabaseKey;
    use crate::notebook::migrations::MigrationManager;

    fn repository() -> (TempDir, NotebookRepository) {
        let directory = TempDir::new().expect("temp");
        let key = DatabaseKey::generate().expect("key");
        MigrationManager::default()
            .migrate(directory.path().join("notebook.db"), &key)
            .expect("migrate");
        let repository =
            NotebookRepository::open(directory.path().join("notebook.db"), &key).expect("open");
        (directory, repository)
    }

    fn key() -> String {
        EntityId::new().to_string()
    }

    #[test]
    fn automatic_candidate_requires_current_generation_and_confirmation() {
        let (_directory, repository) = repository();
        let runtime = EncounterCommandRuntime::default();
        runtime
            .accept_detector_evidence(&ContextEvidence {
                provider_session: "session".into(),
                generation: 2,
                sequence: 4,
                monotonic_ms: 10,
                field: ContextField::Opponent,
                normalized_value: "opponent".into(),
                display_value: "Opponent".into(),
                confidence: 1.0,
                confidence_class: ConfidenceClass::Trusted,
                provenance: crate::detection::EvidenceProvenance::Uia,
            })
            .expect("candidate");
        let stale = confirm_opponent_for(
            CallerIdentity::Overlay,
            &repository,
            &runtime,
            ConfirmOpponentRequest {
                provider_session: "session".into(),
                candidate_generation: 1,
                candidate_sequence: 4,
                corrected_handle: None,
                idempotency_key: key(),
            },
            UtcMillis::now(),
        );
        assert!(!stale.is_success());
        assert!(repository.active_encounter().expect("active").is_none());
        let confirmed = confirm_opponent_for(
            CallerIdentity::Overlay,
            &repository,
            &runtime,
            ConfirmOpponentRequest {
                provider_session: "session".into(),
                candidate_generation: 2,
                candidate_sequence: 4,
                corrected_handle: None,
                idempotency_key: key(),
            },
            UtcMillis::now(),
        );
        assert!(confirmed.is_success());
    }

    #[test]
    fn manual_entry_requires_explicit_correction_when_active() {
        let (_directory, repository) = repository();
        let runtime = EncounterCommandRuntime::default();
        let first = enter_opponent_for(
            CallerIdentity::Main,
            &repository,
            &runtime,
            EnterOpponentRequest {
                handle: "First".into(),
                idempotency_key: key(),
            },
            UtcMillis::now(),
        );
        assert!(first.is_success());
        let second = enter_opponent_for(
            CallerIdentity::Main,
            &repository,
            &runtime,
            EnterOpponentRequest {
                handle: "Second".into(),
                idempotency_key: key(),
            },
            UtcMillis::now(),
        );
        assert!(!second.is_success());
        assert!(repository.active_encounter().expect("active").is_some());
    }

    #[test]
    fn detector_phase_uses_authoritative_reducer_and_persists_restricted_state() {
        let (_directory, repository) = repository();
        let runtime = EncounterCommandRuntime::default();
        assert!(
            enter_opponent_for(
                CallerIdentity::Main,
                &repository,
                &runtime,
                EnterOpponentRequest {
                    handle: "PhaseTarget".into(),
                    idempotency_key: key(),
                },
                UtcMillis::now(),
            )
            .is_success()
        );
        let changed = runtime
            .apply_detector_evidence(
                &repository,
                &ContextEvidence {
                    provider_session: "selected-window".into(),
                    generation: 1,
                    sequence: 1,
                    monotonic_ms: 10,
                    field: ContextField::Phase,
                    normalized_value: "in_game_restricted".into(),
                    display_value: "in_game_restricted".into(),
                    confidence: 1.0,
                    confidence_class: ConfidenceClass::Trusted,
                    provenance: EvidenceProvenance::Uia,
                },
                UtcMillis::now(),
            )
            .expect("apply")
            .expect("changed");
        assert_eq!(changed.phase, InternalPhase::InGameRestricted);
        assert_eq!(
            repository
                .active_encounter()
                .expect("active")
                .expect("encounter")
                .phase,
            InternalPhase::InGameRestricted
        );
    }

    #[test]
    fn ocr_cannot_leave_restricted_until_the_same_signal_is_stable() {
        let (_directory, repository) = repository();
        let runtime = EncounterCommandRuntime::default();
        assert!(
            enter_opponent_for(
                CallerIdentity::Main,
                &repository,
                &runtime,
                EnterOpponentRequest {
                    handle: "StableTarget".into(),
                    idempotency_key: key(),
                },
                UtcMillis::now(),
            )
            .is_success()
        );
        let restricted = ContextEvidence {
            provider_session: "selected-window".into(),
            generation: 1,
            sequence: 1,
            monotonic_ms: 10,
            field: ContextField::Phase,
            normalized_value: "in_game_restricted".into(),
            display_value: "in_game_restricted".into(),
            confidence: 1.0,
            confidence_class: ConfidenceClass::Trusted,
            provenance: EvidenceProvenance::Uia,
        };
        runtime
            .apply_detector_evidence(&repository, &restricted, UtcMillis::now())
            .expect("restrict");
        let between_games = ContextEvidence {
            provider_session: "selected-window".into(),
            generation: 1,
            sequence: 2,
            monotonic_ms: 100,
            field: ContextField::Phase,
            normalized_value: "between_games".into(),
            display_value: "between_games".into(),
            confidence: 0.9,
            confidence_class: ConfidenceClass::Trusted,
            provenance: EvidenceProvenance::Ocr,
        };
        assert!(
            runtime
                .apply_detector_evidence(&repository, &between_games, UtcMillis::now(),)
                .expect("unstable")
                .is_none()
        );
        let mut stable = between_games;
        stable.sequence = 3;
        stable.monotonic_ms = 1_600;
        assert_eq!(
            runtime
                .apply_detector_evidence(&repository, &stable, UtcMillis::now())
                .expect("stable")
                .expect("changed")
                .phase,
            InternalPhase::BetweenGames
        );
    }

    #[test]
    fn profile_creation_and_rollover_rollback_together_on_transition_failure() {
        let (_directory, repository) = repository();
        let original = ProfileService::new(&repository)
            .create("OriginalOpponent")
            .expect("profile");
        let conflicting_encounter_id = EntityId::new();
        repository
            .start_encounter(
                &conflicting_encounter_id,
                &original.profile.id,
                UtcMillis::now(),
                1,
            )
            .expect("encounter");
        let candidate = normalize_handle("AtomicNewOpponent").expect("handle");

        assert!(
            repository
                .confirm_opponent_encounter(
                    &conflicting_encounter_id,
                    &candidate.display,
                    &candidate.key,
                    UtcMillis::now(),
                    2,
                    &EntityId::new(),
                    "uia",
                )
                .is_err()
        );
        assert!(
            ProfileService::new(&repository)
                .resolve_exact("AtomicNewOpponent")
                .expect("resolve")
                .is_none()
        );
        assert_eq!(
            repository
                .active_encounter()
                .expect("active")
                .expect("encounter")
                .id,
            conflicting_encounter_id.as_str()
        );
    }

    #[test]
    fn provider_interruption_persists_restricted_state_before_disclosure_refresh() {
        let (_directory, repository) = repository();
        let runtime = EncounterCommandRuntime::default();
        assert!(
            enter_opponent_for(
                CallerIdentity::Main,
                &repository,
                &runtime,
                EnterOpponentRequest {
                    handle: "InterruptedOpponent".into(),
                    idempotency_key: key(),
                },
                UtcMillis::now(),
            )
            .is_success()
        );

        assert!(
            restrict_active_for_provider_interruption(
                &repository,
                "provider_paused",
                UtcMillis::now(),
            )
            .expect("restrict")
        );
        assert_eq!(
            repository
                .active_encounter()
                .expect("active")
                .expect("encounter")
                .phase,
            InternalPhase::InGameRestricted
        );
    }
}
