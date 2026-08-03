//! Adapter from immutable Player evidence to the shared pure classifier.
//!
//! The adapter never creates opponent deck records or opponent classification
//! rows.  A classifier failure is returned as an unclassified outcome while the
//! already committed evidence remains untouched.

use crate::classifier::{
    AssetRegistry, CanonicalCard, ClassificationResult, CompleteDeck, DeckClassifier,
};
use crate::domain::{RepoError, UtcMillis};

use super::models::{
    ClassificationMethod, PlayerClassificationRun, PlayerClassificationRunId, PlayerEvidence,
    validate_cards,
};
use super::repository::PlayerStore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlayerClassificationEligibility {
    Eligible,
    Ineligible,
}

pub fn classification_eligibility(evidence: &PlayerEvidence) -> PlayerClassificationEligibility {
    if evidence.is_complete_official_deck() {
        PlayerClassificationEligibility::Eligible
    } else {
        PlayerClassificationEligibility::Ineligible
    }
}

pub fn is_classification_eligible(evidence: &PlayerEvidence) -> bool {
    matches!(
        classification_eligibility(evidence),
        PlayerClassificationEligibility::Eligible
    )
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerClassificationOutcome {
    Classified(PlayerClassificationRun),
    Unclassified { reason: PlayerClassificationReason },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlayerClassificationReason {
    IneligibleEvidence,
    UnsupportedFormat,
    ClassifierUnavailable,
    ClassifierFailed,
}

pub struct PlayerClassificationService<'a> {
    store: &'a PlayerStore<'a>,
    assets: &'a AssetRegistry,
}

impl<'a> PlayerClassificationService<'a> {
    pub fn new(store: &'a PlayerStore<'a>, assets: &'a AssetRegistry) -> Self {
        Self { store, assets }
    }

    /// Classify only after the caller has committed the evidence.  The method
    /// is intentionally read-only with respect to evidence and writes solely
    /// to the Player-owned classification table.
    pub fn classify_and_persist(
        &self,
        evidence: &PlayerEvidence,
        now: UtcMillis,
    ) -> Result<PlayerClassificationOutcome, RepoError> {
        if !is_classification_eligible(evidence) {
            return Ok(PlayerClassificationOutcome::Unclassified {
                reason: PlayerClassificationReason::IneligibleEvidence,
            });
        }
        let format = evidence
            .payload
            .get("format")
            .and_then(|value| value.as_str())
            .ok_or(RepoError::InvalidRequest)?
            .to_owned();
        validate_cards(&evidence.cards, true)?;
        let deck = CompleteDeck {
            format,
            complete: true,
            cards: evidence
                .cards
                .iter()
                .map(|card| CanonicalCard {
                    oracle_id: card.oracle_id.clone(),
                    quantity: card.quantity,
                    basic_land: card.basic_land,
                })
                .collect(),
        };
        let assets = self
            .assets
            .current()
            .map_err(|_| RepoError::AssetsInvalid)?;
        let result = match DeckClassifier::classify_confirmable(&deck, &assets) {
            Ok(result) => result,
            Err(RepoError::FormatUnsupported) => {
                return Ok(PlayerClassificationOutcome::Unclassified {
                    reason: PlayerClassificationReason::UnsupportedFormat,
                });
            }
            Err(_) => {
                return Ok(PlayerClassificationOutcome::Unclassified {
                    reason: PlayerClassificationReason::ClassifierFailed,
                });
            }
        };
        let run = classification_run(evidence, &result, now);
        let run = self.store.insert_classification(run)?;
        Ok(PlayerClassificationOutcome::Classified(run))
    }
}

fn classification_run(
    evidence: &PlayerEvidence,
    result: &ClassificationResult,
    now: UtcMillis,
) -> PlayerClassificationRun {
    PlayerClassificationRun {
        id: PlayerClassificationRunId::new(),
        evidence_id: evidence.id.clone(),
        classifier_version: result.classifier_version.clone(),
        classifier_digest: result.classifier_digest.clone(),
        result_id: result.result_id.clone(),
        result_name: result.result_name.clone(),
        method: match result.method {
            crate::classifier::ClassificationMethod::Signature => ClassificationMethod::Signature,
            crate::classifier::ClassificationMethod::Knn => ClassificationMethod::Knn,
            crate::classifier::ClassificationMethod::Unsupported => {
                ClassificationMethod::Unsupported
            }
        },
        confidence: result.confidence,
        created_at: now,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::domain::UtcMillis;
    use crate::player::models::{
        EvidenceKind, EvidenceProvenance, PlayerCard, PlayerEvidenceId, PlayerId,
    };

    fn evidence(complete: bool) -> PlayerEvidence {
        PlayerEvidence {
            id: PlayerEvidenceId::new(),
            player_identity_id: PlayerId::new(),
            evidence_schema_version: 1,
            kind: EvidenceKind::OfficialPublishedDecklist,
            provenance_mode: EvidenceProvenance::UserAttestedOfficialSource,
            provider_id: "official_mtgo".into(),
            attribution_url: "https://www.mtgo.com/decklists/x".into(),
            canonical_source_url: Some("https://www.mtgo.com/decklists/x".into()),
            lookup_nickname: "Alpha".into(),
            source_nickname: "Alpha".into(),
            exact_match_rule: "case_insensitive_full_string".into(),
            scope: json!({"format":"Modern"}),
            observed_at: UtcMillis::new(1).expect("time"),
            imported_at: UtcMillis::new(1).expect("time"),
            source_key: "source".into(),
            source_digest: "a".repeat(64),
            preview_digest: "b".repeat(64),
            payload: json!({"format":"Modern", "contents": if complete {"complete_deck"} else {"reference_only"}}),
            selected_fields: json!({"source_nickname":true,"attribution_url":true}),
            supersedes_evidence_id: None,
            cards: if complete {
                vec![PlayerCard {
                    oracle_id: "one".into(),
                    display_name: "One".into(),
                    zone: "main".into(),
                    quantity: 1,
                    basic_land: false,
                }]
            } else {
                Vec::new()
            },
        }
    }

    #[test]
    fn ut_017_only_complete_official_evidence_is_eligible() {
        assert!(is_classification_eligible(&evidence(true)));
        assert!(!is_classification_eligible(&evidence(false)));
    }
}
