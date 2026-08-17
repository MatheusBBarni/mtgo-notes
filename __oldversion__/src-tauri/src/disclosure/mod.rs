use serde::{Deserialize, Serialize};

use crate::domain::{InternalPhase, RepoError};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryKind {
    SearchHistory,
    GetProfile,
    GetEncounter,
    GetDeckDetails,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationView {
    pub id: String,
    pub text: String,
    pub editable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSnapshotView {
    pub label: String,
    pub format: String,
    pub published_at: i64,
    pub source_text: String,
    pub available: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotebookState {
    pub phase: InternalPhase,
    pub confirmed_handle: Option<String>,
    pub active_profile_deleted: bool,
    pub current_observations: Vec<ObservationView>,
    pub historical_observations: Vec<ObservationView>,
    pub public_snapshot: Option<PublicSnapshotView>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayView {
    pub phase: InternalPhase,
    pub confirmed_handle: Option<String>,
    pub current_observations: Vec<ObservationView>,
    pub historical_observations: Vec<ObservationView>,
    pub public_snapshot: Option<PublicSnapshotView>,
    pub history_editable: bool,
    pub needs_identity_resolution: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisclosureEmission {
    Replacement(OverlayView),
    Notification(String),
}

#[derive(Default)]
pub struct DisclosurePolicy;

impl DisclosurePolicy {
    pub fn authorize(&self, _query: QueryKind, phase: InternalPhase) -> Result<(), RepoError> {
        if phase.is_disclosure_restricted() {
            Err(RepoError::DisclosureRestricted)
        } else {
            Ok(())
        }
    }

    pub fn overlay(&self, state: &NotebookState) -> OverlayView {
        if state.active_profile_deleted {
            return OverlayView {
                phase: state.phase,
                confirmed_handle: None,
                current_observations: Vec::new(),
                historical_observations: Vec::new(),
                public_snapshot: None,
                history_editable: false,
                needs_identity_resolution: true,
            };
        }

        let Some(handle) = state.confirmed_handle.clone() else {
            return OverlayView {
                phase: state.phase,
                confirmed_handle: None,
                current_observations: Vec::new(),
                historical_observations: Vec::new(),
                public_snapshot: None,
                history_editable: false,
                needs_identity_resolution: false,
            };
        };

        let restricted = state.phase.is_disclosure_restricted();
        let finished = state.phase == InternalPhase::Finished;
        OverlayView {
            phase: state.phase,
            confirmed_handle: Some(handle),
            current_observations: state
                .current_observations
                .iter()
                .cloned()
                .map(|mut observation| {
                    observation.editable = finished;
                    observation
                })
                .collect(),
            historical_observations: if restricted {
                Vec::new()
            } else {
                state
                    .historical_observations
                    .iter()
                    .cloned()
                    .map(|mut observation| {
                        observation.editable = finished;
                        observation
                    })
                    .collect()
            },
            public_snapshot: if restricted {
                None
            } else {
                state.public_snapshot.as_ref().map(sanitize_public_snapshot)
            },
            history_editable: finished,
            needs_identity_resolution: false,
        }
    }

    pub fn transition_emissions(
        &self,
        state: &NotebookState,
        notification: impl Into<String>,
    ) -> Vec<DisclosureEmission> {
        vec![
            DisclosureEmission::Replacement(self.overlay(state)),
            DisclosureEmission::Notification(notification.into()),
        ]
    }
}

fn sanitize_public_snapshot(snapshot: &PublicSnapshotView) -> PublicSnapshotView {
    let contains_markup = snapshot.source_text.contains('<') || snapshot.source_text.contains('>');
    if contains_markup {
        let mut sanitized = snapshot.clone();
        sanitized.source_text = "Source unavailable".to_owned();
        sanitized.available = false;
        sanitized
    } else {
        snapshot.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(id: &str) -> ObservationView {
        ObservationView {
            id: id.into(),
            text: format!("note-{id}"),
            editable: false,
        }
    }

    fn state(phase: InternalPhase) -> NotebookState {
        NotebookState {
            phase,
            confirmed_handle: Some("Opponent_42".into()),
            active_profile_deleted: false,
            current_observations: vec![observation("current")],
            historical_observations: vec![observation("history")],
            public_snapshot: Some(PublicSnapshotView {
                label: "Deck label".into(),
                format: "Modern".into(),
                published_at: 1_753_689_600_000,
                source_text: "Official MTGO".into(),
                available: true,
            }),
        }
    }

    #[test]
    fn ut_021_pre_match_includes_permitted_context() {
        let view = DisclosurePolicy.overlay(&state(InternalPhase::PreMatch));
        assert_eq!(view.confirmed_handle.as_deref(), Some("Opponent_42"));
        assert_eq!(view.historical_observations.len(), 1);
        assert!(view.public_snapshot.is_some());
    }

    #[test]
    fn ut_022_in_game_contains_only_identity_and_current_observations() {
        let view = DisclosurePolicy.overlay(&state(InternalPhase::InGameRestricted));
        assert!(view.confirmed_handle.is_some());
        assert_eq!(view.current_observations.len(), 1);
        assert!(view.historical_observations.is_empty());
        assert!(view.public_snapshot.is_none());
    }

    #[test]
    fn ut_023_incomplete_possible_gameplay_is_restricted() {
        let view = DisclosurePolicy.overlay(&state(InternalPhase::Incomplete));
        assert!(view.historical_observations.is_empty());
        assert!(view.public_snapshot.is_none());
    }

    #[test]
    fn ut_024_finished_projection_allows_full_editing() {
        let view = DisclosurePolicy.overlay(&state(InternalPhase::Finished));
        assert!(view.history_editable);
        assert!(view.current_observations[0].editable);
        assert!(view.historical_observations[0].editable);
    }

    #[test]
    fn ut_025_search_is_denied_during_gameplay() {
        let error = DisclosurePolicy
            .authorize(QueryKind::SearchHistory, InternalPhase::InGameRestricted)
            .expect_err("restricted");
        assert_eq!(error, RepoError::DisclosureRestricted);
        assert_eq!(
            error.to_app_error().code,
            crate::ipc::ErrorCode::DisclosureRestricted
        );
    }

    #[test]
    fn ut_026_unconfirmed_opponent_exposes_no_history_or_external_data() {
        let mut notebook = state(InternalPhase::PreMatch);
        notebook.confirmed_handle = None;
        let view = DisclosurePolicy.overlay(&notebook);
        assert!(view.confirmed_handle.is_none());
        assert!(view.current_observations.is_empty());
        assert!(view.historical_observations.is_empty());
        assert!(view.public_snapshot.is_none());
    }

    #[test]
    fn ut_027_deleted_active_profile_clears_stale_context() {
        let mut notebook = state(InternalPhase::BetweenGames);
        notebook.active_profile_deleted = true;
        let view = DisclosurePolicy.overlay(&notebook);
        assert!(view.needs_identity_resolution);
        assert!(view.confirmed_handle.is_none());
        assert!(view.historical_observations.is_empty());
    }

    #[test]
    fn ut_028_restricted_replacement_precedes_notification() {
        let emissions = DisclosurePolicy
            .transition_emissions(&state(InternalPhase::InGameRestricted), "phase changed");
        assert!(matches!(
            emissions.first(),
            Some(DisclosureEmission::Replacement(OverlayView {
                historical_observations,
                public_snapshot: None,
                ..
            })) if historical_observations.is_empty()
        ));
        assert!(matches!(
            emissions.get(1),
            Some(DisclosureEmission::Notification(_))
        ));
    }

    #[test]
    fn ut_029_equivalent_states_serialize_byte_equivalently() {
        let first = serde_json::to_vec(&DisclosurePolicy.overlay(&state(InternalPhase::PreMatch)))
            .expect("serialize");
        let second = serde_json::to_vec(&DisclosurePolicy.overlay(&state(InternalPhase::PreMatch)))
            .expect("serialize");
        assert_eq!(first, second);
    }

    #[test]
    fn ut_030_malformed_public_markup_is_plain_unavailable_text() {
        let mut notebook = state(InternalPhase::PreMatch);
        notebook
            .public_snapshot
            .as_mut()
            .expect("snapshot")
            .source_text = "<script>alert(1)</script>".into();
        let snapshot = DisclosurePolicy
            .overlay(&notebook)
            .public_snapshot
            .expect("snapshot");
        assert!(!snapshot.available);
        assert_eq!(snapshot.source_text, "Source unavailable");
        assert!(!snapshot.source_text.contains('<'));
    }

    #[test]
    fn unresolved_and_unconfirmed_completion_phases_remain_restricted() {
        for phase in [InternalPhase::Candidate, InternalPhase::CompletionPending] {
            let view = DisclosurePolicy.overlay(&state(phase));
            assert!(view.historical_observations.is_empty());
            assert!(view.public_snapshot.is_none());
            assert_eq!(
                DisclosurePolicy.authorize(QueryKind::SearchHistory, phase),
                Err(RepoError::DisclosureRestricted)
            );
        }
    }
}
