//! Closed, serializable domain types for the local Player bounded context.
//!
//! The module deliberately does not reuse opponent profile/deck identifiers or
//! payloads.  A Player nickname is a local screen name and public evidence is a
//! source-scoped immutable statement, never an account claim.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

use crate::domain::{EntityId, RepoError, Revision, UtcMillis};

pub const EVIDENCE_SCHEMA_VERSION: u32 = 1;
pub const CANONICALIZATION_VERSION: &str = "player-canonical-v1";
pub const MAX_NICKNAME_SCALARS: usize = 128;
pub const MAX_DECK_ROWS: usize = 500;
pub const MAX_CARD_QUANTITY: u16 = 250;

macro_rules! player_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(EntityId);

        impl $name {
            pub fn new() -> Self {
                Self(EntityId::new())
            }

            pub fn parse(value: impl Into<String>) -> Result<Self, RepoError> {
                EntityId::parse(value).map(Self)
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn into_entity_id(self) -> EntityId {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

player_id!(PlayerId);
player_id!(PlayerEvidenceId);
player_id!(PlayerSelectionId);
player_id!(PlayerEmptyOutcomeId);
player_id!(PlayerClassificationRunId);
player_id!(PlayerOperationKey);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedNickname {
    pub display: String,
    pub normalized: String,
}

/// Trim, bound, reject controls, and produce the deterministic case-folded key
/// used by source identity and exact matching.  NFKC is intentionally applied
/// to the key only; historical display spelling remains untouched apart from
/// outer whitespace.
pub fn normalize_player_nickname(value: &str) -> Result<NormalizedNickname, RepoError> {
    let display = value.trim().to_owned();
    if display.is_empty()
        || display.chars().count() > MAX_NICKNAME_SCALARS
        || display.chars().any(char::is_control)
    {
        return Err(RepoError::InvalidRequest);
    }
    let normalized = display.nfkc().case_fold().collect::<String>();
    if normalized.is_empty() || normalized.chars().any(char::is_control) {
        return Err(RepoError::InvalidRequest);
    }
    Ok(NormalizedNickname {
        display,
        normalized,
    })
}

pub fn normalize_player_nickname_key(value: &str) -> Result<String, RepoError> {
    normalize_player_nickname(value).map(|identity| identity.normalized)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    MocsLeaderboardEntry,
    OfficialPublishedDecklist,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceProvenance {
    ProviderObserved,
    UserAttestedOfficialSource,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckContentsKind {
    ReferenceOnly,
    CompleteDeck,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerSourceRoute {
    CensusMocs,
    OfficialMtgoBrowser,
    MtgTop8Browser,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationMethod {
    Signature,
    Knn,
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerIdentity {
    pub id: PlayerId,
    pub display_nickname: String,
    pub normalized_nickname: String,
    pub created_at: UtcMillis,
    pub updated_at: UtcMillis,
    pub revision: Revision,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerConsent {
    pub player_identity_id: PlayerId,
    pub route: PlayerSourceRoute,
    pub disclosure_version: String,
    pub outbound_fields: Value,
    pub fields_digest: String,
    pub granted_at: UtcMillis,
    pub revision: Revision,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerCard {
    pub oracle_id: String,
    pub display_name: String,
    pub zone: String,
    pub quantity: u16,
    pub basic_land: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerEvidence {
    pub id: PlayerEvidenceId,
    pub player_identity_id: PlayerId,
    pub evidence_schema_version: u32,
    pub kind: EvidenceKind,
    pub provenance_mode: EvidenceProvenance,
    pub provider_id: String,
    pub attribution_url: String,
    pub canonical_source_url: Option<String>,
    pub lookup_nickname: String,
    pub source_nickname: String,
    pub exact_match_rule: String,
    pub scope: Value,
    pub observed_at: UtcMillis,
    pub imported_at: UtcMillis,
    pub source_key: String,
    pub source_digest: String,
    pub preview_digest: String,
    pub payload: Value,
    pub selected_fields: Value,
    pub supersedes_evidence_id: Option<PlayerEvidenceId>,
    pub cards: Vec<PlayerCard>,
}

impl PlayerEvidence {
    pub fn is_complete_official_deck(&self) -> bool {
        matches!(self.kind, EvidenceKind::OfficialPublishedDecklist)
            && self.payload.get("contents").and_then(Value::as_str) == Some("complete_deck")
            && !self.cards.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSelectionRevision {
    pub id: PlayerSelectionId,
    pub evidence_id: PlayerEvidenceId,
    pub revision_number: Revision,
    pub selected_fields: Value,
    pub created_at: UtcMillis,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerEmptyOutcome {
    pub id: PlayerEmptyOutcomeId,
    pub player_identity_id: PlayerId,
    pub provider_id: String,
    pub lookup_nickname: String,
    pub exact_match_rule: String,
    pub scope: Value,
    pub provider_configuration_version: String,
    pub completed_at: UtcMillis,
    pub operation_key: PlayerOperationKey,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerClassificationRun {
    pub id: PlayerClassificationRunId,
    pub evidence_id: PlayerEvidenceId,
    pub classifier_version: String,
    pub classifier_digest: String,
    pub result_id: String,
    pub result_name: String,
    pub method: ClassificationMethod,
    pub confidence: f64,
    pub created_at: UtcMillis,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerOperationReceipt {
    pub operation_key: PlayerOperationKey,
    pub command_kind: String,
    pub player_identity_id: PlayerId,
    pub request_digest: String,
    pub result_code: String,
    pub result_locator: Option<String>,
    pub created_at: UtcMillis,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerTombstone {
    pub entity_kind: String,
    pub entity_id: String,
    pub player_identity_id: PlayerId,
    pub deleted_at: UtcMillis,
}

/// Encode source identity as length-prefixed segments.  Delimiters alone are
/// ambiguous (for example `a|bc` versus `a|b|c`); the prefix makes every key
/// injective while retaining a human-auditable version marker.
pub fn canonical_source_key(parts: &[&str]) -> Result<String, RepoError> {
    if parts.is_empty() || parts.iter().any(|part| part.is_empty()) {
        return Err(RepoError::InvalidRequest);
    }
    let mut key = String::from("source-v1:");
    for part in parts {
        let normalized = part.nfkc().collect::<String>();
        let length = normalized.chars().count();
        key.push_str(&length.to_string());
        key.push(':');
        key.push_str(&normalized);
    }
    Ok(key)
}

pub fn census_source_key(
    catalog_id: &str,
    start_date: &str,
    as_of_date: &str,
    source_nickname: &str,
) -> Result<String, RepoError> {
    let nickname = normalize_player_nickname_key(source_nickname)?;
    canonical_source_key(&["census_mocs", catalog_id, start_date, as_of_date, &nickname])
}

pub fn official_source_key(
    canonical_url: &str,
    source_nickname: &str,
) -> Result<String, RepoError> {
    let nickname = normalize_player_nickname_key(source_nickname)?;
    canonical_source_key(&["official_published_decklist", canonical_url, &nickname])
}

pub fn canonical_json(value: &Value) -> Result<String, RepoError> {
    let normalized = canonical_value(value);
    serde_json::to_string(&normalized).map_err(|_| RepoError::InvalidRequest)
}

pub fn canonical_digest(value: &Value) -> Result<String, RepoError> {
    let mut hasher = Sha256::new();
    hasher.update(CANONICALIZATION_VERSION.as_bytes());
    hasher.update([0]);
    hasher.update(canonical_json(value)?.as_bytes());
    Ok(hex_digest(hasher.finalize().as_slice()))
}

pub fn source_digest(payload: &Value) -> Result<String, RepoError> {
    canonical_digest(payload)
}

pub fn preview_digest(envelope: &Value) -> Result<String, RepoError> {
    canonical_digest(envelope)
}

fn canonical_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted = map
                .iter()
                .map(|(key, value)| (key.clone(), canonical_value(value)))
                .collect::<BTreeMap<_, _>>();
            let mut output = Map::new();
            for (key, value) in sorted {
                output.insert(key, value);
            }
            Value::Object(output)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_value).collect()),
        Value::String(text) => Value::String(text.nfc().collect()),
        _ => value.clone(),
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn validate_cards(cards: &[PlayerCard], complete: bool) -> Result<(), RepoError> {
    if !complete && !cards.is_empty() {
        return Err(RepoError::InvalidRequest);
    }
    if cards.len() > MAX_DECK_ROWS {
        return Err(RepoError::InvalidRequest);
    }
    let mut keys = std::collections::BTreeSet::new();
    for card in cards {
        if card.oracle_id.trim().is_empty()
            || card.display_name.trim().is_empty()
            || card.quantity == 0
            || card.quantity > MAX_CARD_QUANTITY
            || card.zone.trim().is_empty()
            || card
                .oracle_id
                .chars()
                .chain(card.display_name.chars())
                .any(char::is_control)
        {
            return Err(RepoError::InvalidRequest);
        }
        let key = (
            card.oracle_id.nfkc().case_fold().collect::<String>(),
            card.zone.clone(),
        );
        if !keys.insert(key) {
            return Err(RepoError::InvalidRequest);
        }
    }
    if complete && cards.is_empty() {
        return Err(RepoError::InvalidRequest);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ut_001_to_003_nickname_normalization_and_bounds() {
        let normalized = normalize_player_nickname("  Teichou_Aisu  ").expect("valid");
        assert_eq!(normalized.display, "Teichou_Aisu");
        assert_eq!(normalized.normalized, "teichou_aisu");
        assert!(normalize_player_nickname(&"a".repeat(129)).is_err());
        assert!(normalize_player_nickname("\0").is_err());
    }

    #[test]
    fn ut_004_to_005_source_keys_are_deterministic_and_unambiguous() {
        let first = canonical_source_key(&["a", "bc"]).expect("key");
        let second = canonical_source_key(&["ab", "c"]).expect("key");
        assert_ne!(first, second);
        assert_eq!(
            census_source_key("catalog", "2026-01-01", "2026-01-02", "Teichou_Aisu").expect("key"),
            census_source_key("catalog", "2026-01-01", "2026-01-02", "teichou_aisu").expect("key")
        );
    }

    #[test]
    fn ut_006_to_007_digests_are_order_independent_but_content_sensitive() {
        let one = json!({"b": 2, "a": 1});
        let two = json!({"a": 1, "b": 2});
        assert_eq!(
            source_digest(&one).expect("digest"),
            source_digest(&two).expect("digest")
        );
        assert_ne!(
            source_digest(&one).expect("digest"),
            preview_digest(&json!({"a": 1, "b": 3})).expect("digest")
        );
    }

    #[test]
    fn complete_deck_bounds_and_reference_only_contract() {
        let card = PlayerCard {
            oracle_id: "oracle".into(),
            display_name: "Card".into(),
            zone: "main".into(),
            quantity: 250,
            basic_land: false,
        };
        assert!(validate_cards(std::slice::from_ref(&card), true).is_ok());
        assert!(validate_cards(&[card.clone(), card.clone()], true).is_err());
        assert!(validate_cards(&[], false).is_ok());
        assert!(
            validate_cards(
                &[PlayerCard {
                    quantity: 1,
                    ..card
                }],
                false
            )
            .is_err()
        );
    }
}
