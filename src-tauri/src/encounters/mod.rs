use serde::{Deserialize, Serialize};

use crate::domain::{EncounterStatus, EntityId, InternalPhase, RepoError};

const OCR_STABLE_DURATION_MS: u64 = 1_500;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    TrustedUia,
    Ocr,
    Manual,
    System,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EvidenceKind {
    ConfirmedOpponent {
        profile_id: EntityId,
        encounter_id: EntityId,
    },
    TrustedPhase {
        phase: InternalPhase,
        stable_for_ms: u64,
    },
    UnknownPossibleGameplay,
    StrongGameplay,
    End,
    CompletionIgnored,
    Reopen {
        encounter_id: EntityId,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEvidence {
    pub provider_session: String,
    pub generation: u64,
    pub sequence: u64,
    pub monotonic_ms: u64,
    pub source: EvidenceSource,
    pub evidence: EvidenceKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEncounter {
    pub id: EntityId,
    pub profile_id: EntityId,
    pub status: EncounterStatus,
    pub unconfirmed_deck_present: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterRuntime {
    pub provider_session: String,
    pub generation: u64,
    pub last_sequence: u64,
    pub phase: InternalPhase,
    pub active: Option<ActiveEncounter>,
}

impl EncounterRuntime {
    pub fn idle(provider_session: impl Into<String>) -> Self {
        Self {
            provider_session: provider_session.into(),
            generation: 0,
            last_sequence: 0,
            phase: InternalPhase::Idle,
            active: None,
        }
    }

    pub fn recover(
        provider_session: impl Into<String>,
        generation: u64,
        active: ActiveEncounter,
    ) -> Self {
        Self {
            provider_session: provider_session.into(),
            generation,
            last_sequence: 0,
            phase: InternalPhase::InGameRestricted,
            active: Some(active),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum EncounterAction {
    ResolveProfile {
        profile_id: EntityId,
    },
    StartEncounter {
        encounter_id: EntityId,
        profile_id: EntityId,
        undo_group: Option<EntityId>,
    },
    FinishEncounter {
        encounter_id: EntityId,
        undo_group: Option<EntityId>,
    },
    ChangePhase {
        encounter_id: EntityId,
        from: InternalPhase,
        to: InternalPhase,
    },
    MarkIncomplete {
        encounter_id: EntityId,
        excluded_unconfirmed_deck: bool,
    },
    OpenHistoricalEditor {
        encounter_id: EntityId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reduction {
    pub runtime: EncounterRuntime,
    pub actions: Vec<EncounterAction>,
}

#[derive(Default)]
pub struct EncounterReducer;

impl EncounterReducer {
    pub fn reduce(
        &self,
        current: &EncounterRuntime,
        evidence: ContextEvidence,
    ) -> Result<Reduction, RepoError> {
        if evidence.provider_session != current.provider_session
            || evidence.generation < current.generation
            || (evidence.generation == current.generation
                && evidence.sequence <= current.last_sequence)
        {
            return Ok(Reduction {
                runtime: current.clone(),
                actions: Vec::new(),
            });
        }

        let starts_generation = matches!(evidence.evidence, EvidenceKind::ConfirmedOpponent { .. });
        if evidence.generation > current.generation && !starts_generation {
            return Ok(Reduction {
                runtime: current.clone(),
                actions: Vec::new(),
            });
        }

        let mut runtime = current.clone();
        runtime.last_sequence = evidence.sequence;
        let mut actions = Vec::new();

        match evidence.evidence {
            EvidenceKind::ConfirmedOpponent {
                profile_id,
                encounter_id,
            } => {
                if runtime.active.as_ref().map(|active| &active.profile_id) == Some(&profile_id) {
                    runtime.generation = evidence.generation;
                    return Ok(Reduction { runtime, actions });
                }

                let undo_group = runtime.active.as_ref().map(|_| EntityId::new());
                if let Some(previous) = runtime.active.take() {
                    actions.push(EncounterAction::FinishEncounter {
                        encounter_id: previous.id,
                        undo_group: undo_group.clone(),
                    });
                }
                actions.push(EncounterAction::ResolveProfile {
                    profile_id: profile_id.clone(),
                });
                actions.push(EncounterAction::StartEncounter {
                    encounter_id: encounter_id.clone(),
                    profile_id: profile_id.clone(),
                    undo_group,
                });
                runtime.generation = evidence.generation;
                runtime.phase = InternalPhase::PreMatch;
                runtime.active = Some(ActiveEncounter {
                    id: encounter_id,
                    profile_id,
                    status: EncounterStatus::Active,
                    unconfirmed_deck_present: false,
                });
            }
            EvidenceKind::UnknownPossibleGameplay | EvidenceKind::StrongGameplay => {
                self.change_phase(&mut runtime, InternalPhase::InGameRestricted, &mut actions)?;
            }
            EvidenceKind::TrustedPhase {
                phase,
                stable_for_ms,
            } => {
                if runtime.phase == InternalPhase::InGameRestricted
                    && evidence.source == EvidenceSource::Ocr
                    && stable_for_ms < OCR_STABLE_DURATION_MS
                {
                    return Ok(Reduction { runtime, actions });
                }
                self.change_phase(&mut runtime, phase, &mut actions)?;
            }
            EvidenceKind::End => {
                let Some(active) = runtime.active.take() else {
                    if runtime.phase == InternalPhase::Finished {
                        return Ok(Reduction { runtime, actions });
                    }
                    return Err(RepoError::InvalidTransition);
                };
                actions.push(EncounterAction::FinishEncounter {
                    encounter_id: active.id,
                    undo_group: None,
                });
                runtime.phase = InternalPhase::Finished;
            }
            EvidenceKind::CompletionIgnored => {
                let Some(active) = runtime.active.take() else {
                    return Err(RepoError::InvalidTransition);
                };
                actions.push(EncounterAction::MarkIncomplete {
                    encounter_id: active.id,
                    excluded_unconfirmed_deck: active.unconfirmed_deck_present,
                });
                runtime.phase = InternalPhase::Incomplete;
            }
            EvidenceKind::Reopen { encounter_id } => {
                actions.push(EncounterAction::OpenHistoricalEditor { encounter_id });
            }
        }

        Ok(Reduction { runtime, actions })
    }

    fn change_phase(
        &self,
        runtime: &mut EncounterRuntime,
        phase: InternalPhase,
        actions: &mut Vec<EncounterAction>,
    ) -> Result<(), RepoError> {
        let active = runtime
            .active
            .as_ref()
            .ok_or(RepoError::InvalidTransition)?;
        if runtime.phase == phase {
            return Ok(());
        }
        actions.push(EncounterAction::ChangePhase {
            encounter_id: active.id.clone(),
            from: runtime.phase,
            to: phase,
        });
        runtime.phase = phase;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(runtime: &EncounterRuntime, sequence: u64, kind: EvidenceKind) -> ContextEvidence {
        ContextEvidence {
            provider_session: runtime.provider_session.clone(),
            generation: runtime.generation,
            sequence,
            monotonic_ms: sequence * 10,
            source: EvidenceSource::TrustedUia,
            evidence: kind,
        }
    }

    fn start(reducer: &EncounterReducer) -> EncounterRuntime {
        let idle = EncounterRuntime::idle("session");
        reducer
            .reduce(
                &idle,
                ContextEvidence {
                    provider_session: "session".into(),
                    generation: 1,
                    sequence: 1,
                    monotonic_ms: 1,
                    source: EvidenceSource::Manual,
                    evidence: EvidenceKind::ConfirmedOpponent {
                        profile_id: EntityId::new(),
                        encounter_id: EntityId::new(),
                    },
                },
            )
            .expect("start")
            .runtime
    }

    #[test]
    fn ut_009_confirmed_candidate_starts_pre_match() {
        let reducer = EncounterReducer;
        let idle = EncounterRuntime::idle("session");
        let result = reducer
            .reduce(
                &idle,
                ContextEvidence {
                    provider_session: "session".into(),
                    generation: 1,
                    sequence: 1,
                    monotonic_ms: 1,
                    source: EvidenceSource::Manual,
                    evidence: EvidenceKind::ConfirmedOpponent {
                        profile_id: EntityId::new(),
                        encounter_id: EntityId::new(),
                    },
                },
            )
            .expect("reduce");
        assert_eq!(result.runtime.phase, InternalPhase::PreMatch);
        assert!(matches!(
            result.actions.as_slice(),
            [
                EncounterAction::ResolveProfile { .. },
                EncounterAction::StartEncounter { .. }
            ]
        ));
    }

    #[test]
    fn ut_010_unknown_possible_gameplay_fails_closed() {
        let reducer = EncounterReducer;
        let runtime = start(&reducer);
        let result = reducer
            .reduce(
                &runtime,
                evidence(&runtime, 2, EvidenceKind::UnknownPossibleGameplay),
            )
            .expect("reduce");
        assert_eq!(result.runtime.phase, InternalPhase::InGameRestricted);
    }

    #[test]
    fn ut_011_strong_gameplay_signal_restricts_immediately() {
        let reducer = EncounterReducer;
        let runtime = start(&reducer);
        let result = reducer
            .reduce(
                &runtime,
                evidence(&runtime, 2, EvidenceKind::StrongGameplay),
            )
            .expect("reduce");
        assert_eq!(result.runtime.phase, InternalPhase::InGameRestricted);
        assert_eq!(result.actions.len(), 1);
    }

    #[test]
    fn ut_012_unstable_ocr_cannot_leave_restricted() {
        let reducer = EncounterReducer;
        let mut runtime = start(&reducer);
        runtime.phase = InternalPhase::InGameRestricted;
        let mut next = evidence(
            &runtime,
            2,
            EvidenceKind::TrustedPhase {
                phase: InternalPhase::BetweenGames,
                stable_for_ms: OCR_STABLE_DURATION_MS - 1,
            },
        );
        next.source = EvidenceSource::Ocr;
        let result = reducer.reduce(&runtime, next).expect("reduce");
        assert_eq!(result.runtime.phase, InternalPhase::InGameRestricted);
        assert!(result.actions.is_empty());
    }

    #[test]
    fn ut_013_new_opponent_finishes_before_start_in_one_undo_group() {
        let reducer = EncounterReducer;
        let runtime = start(&reducer);
        let result = reducer
            .reduce(
                &runtime,
                ContextEvidence {
                    provider_session: runtime.provider_session.clone(),
                    generation: 2,
                    sequence: 1,
                    monotonic_ms: 20,
                    source: EvidenceSource::Manual,
                    evidence: EvidenceKind::ConfirmedOpponent {
                        profile_id: EntityId::new(),
                        encounter_id: EntityId::new(),
                    },
                },
            )
            .expect("reduce");
        let (finish_group, start_group) = match result.actions.as_slice() {
            [
                EncounterAction::FinishEncounter { undo_group, .. },
                EncounterAction::ResolveProfile { .. },
                EncounterAction::StartEncounter {
                    undo_group: start_group,
                    ..
                },
            ] => (undo_group, start_group),
            other => panic!("unexpected actions: {other:?}"),
        };
        assert!(finish_group.is_some());
        assert_eq!(finish_group, start_group);
    }

    #[test]
    fn ut_014_repeated_end_is_idempotent() {
        let reducer = EncounterReducer;
        let runtime = start(&reducer);
        let finished = reducer
            .reduce(&runtime, evidence(&runtime, 2, EvidenceKind::End))
            .expect("finish")
            .runtime;
        let replay = reducer
            .reduce(&finished, evidence(&finished, 3, EvidenceKind::End))
            .expect("replay");
        assert!(replay.actions.is_empty());
        assert_eq!(replay.runtime.phase, InternalPhase::Finished);
    }

    #[test]
    fn ut_015_older_generation_is_ignored() {
        let reducer = EncounterReducer;
        let runtime = start(&reducer);
        let stale = ContextEvidence {
            provider_session: runtime.provider_session.clone(),
            generation: 0,
            sequence: 100,
            monotonic_ms: 100,
            source: EvidenceSource::TrustedUia,
            evidence: EvidenceKind::StrongGameplay,
        };
        let result = reducer.reduce(&runtime, stale).expect("ignore stale");
        assert_eq!(result.runtime, runtime);
        assert!(result.actions.is_empty());
    }

    #[test]
    fn ut_016_recovered_active_encounter_starts_restricted() {
        let recovered = EncounterRuntime::recover(
            "new-session",
            4,
            ActiveEncounter {
                id: EntityId::new(),
                profile_id: EntityId::new(),
                status: EncounterStatus::Active,
                unconfirmed_deck_present: false,
            },
        );
        assert_eq!(recovered.phase, InternalPhase::InGameRestricted);
        assert!(recovered.active.is_some());
    }

    #[test]
    fn ut_017_finish_without_attached_encounter_is_invalid() {
        let reducer = EncounterReducer;
        let idle = EncounterRuntime::idle("session");
        let error = reducer
            .reduce(&idle, evidence(&idle, 1, EvidenceKind::End))
            .expect_err("invalid");
        assert_eq!(error, RepoError::InvalidTransition);
    }

    #[test]
    fn ut_018_ignored_completion_becomes_incomplete() {
        let reducer = EncounterReducer;
        let mut runtime = start(&reducer);
        runtime
            .active
            .as_mut()
            .expect("active")
            .unconfirmed_deck_present = true;
        let result = reducer
            .reduce(
                &runtime,
                evidence(&runtime, 2, EvidenceKind::CompletionIgnored),
            )
            .expect("reduce");
        assert_eq!(result.runtime.phase, InternalPhase::Incomplete);
        assert!(matches!(
            result.actions.as_slice(),
            [EncounterAction::MarkIncomplete {
                excluded_unconfirmed_deck: true,
                ..
            }]
        ));
    }

    #[test]
    fn ut_019_reopening_history_does_not_displace_active() {
        let reducer = EncounterReducer;
        let runtime = start(&reducer);
        let active_id = runtime.active.as_ref().expect("active").id.clone();
        let result = reducer
            .reduce(
                &runtime,
                evidence(
                    &runtime,
                    2,
                    EvidenceKind::Reopen {
                        encounter_id: EntityId::new(),
                    },
                ),
            )
            .expect("reduce");
        assert_eq!(result.runtime.active.expect("active").id, active_id);
        assert!(matches!(
            result.actions.as_slice(),
            [EncounterAction::OpenHistoricalEditor { .. }]
        ));
    }

    #[test]
    fn ut_020_event_interleavings_never_create_two_active_encounters() {
        let reducer = EncounterReducer;
        for order in 0..128_u64 {
            let mut runtime = EncounterRuntime::idle("session");
            for generation in 1..=8 {
                let sequence = if order & (1 << (generation - 1)) == 0 {
                    1
                } else {
                    2
                };
                let result = reducer
                    .reduce(
                        &runtime,
                        ContextEvidence {
                            provider_session: "session".into(),
                            generation,
                            sequence,
                            monotonic_ms: generation,
                            source: EvidenceSource::Manual,
                            evidence: EvidenceKind::ConfirmedOpponent {
                                profile_id: EntityId::new(),
                                encounter_id: EntityId::new(),
                            },
                        },
                    )
                    .expect("reduce");
                runtime = result.runtime;
                assert!(usize::from(runtime.active.is_some()) <= 1);
            }
        }
    }
}
