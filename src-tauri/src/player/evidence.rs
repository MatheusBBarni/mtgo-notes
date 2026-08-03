//! Pure manual official-source evidence validation and canonical preview data.
//!
//! This module intentionally has no filesystem, DNS, HTTP, browser, or parser
//! dependency.  It turns a closed input into a deterministic statement which a
//! trusted runtime may bind to a short-lived preview token.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::domain::{RepoError, UtcMillis};

use super::models::{
    DeckContentsKind, EvidenceKind, EvidenceProvenance, MAX_MANUAL_FIELD_SCALARS,
    MAX_MANUAL_PAYLOAD_BYTES, MAX_MANUAL_TITLE_SCALARS, PlayerCard, PlayerEvidence,
    PlayerEvidenceId, PlayerId, PlayerPreviewToken, canonical_digest, official_source_key,
    preview_digest, retain_selected_payload, validate_cards,
};
use super::routes::validate_official_artifact_url;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManualEvidenceInput {
    pub event_title: String,
    pub event_date: String,
    pub format: String,
    #[serde(default)]
    pub placement: Option<String>,
    #[serde(default)]
    pub record: Option<String>,
    pub source_nickname: String,
    pub attribution_url: String,
    pub contents: DeckContentsKind,
    #[serde(default)]
    pub cards: Vec<PlayerCard>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualEvidencePreview {
    pub token: PlayerPreviewToken,
    pub player_identity_id: PlayerId,
    pub identity_revision: u64,
    pub evidence: PlayerEvidence,
    pub approved_fields: Vec<String>,
}

/// Validate manual fields without doing any external work.
pub fn validate_manual_evidence(input: &ManualEvidenceInput) -> Result<String, RepoError> {
    bounded_required(&input.event_title, MAX_MANUAL_TITLE_SCALARS)?;
    bounded_required(&input.format, MAX_MANUAL_FIELD_SCALARS)?;
    bounded_required(&input.source_nickname, 128)?;
    bounded_optional(&input.placement, MAX_MANUAL_FIELD_SCALARS)?;
    bounded_optional(&input.record, MAX_MANUAL_FIELD_SCALARS)?;
    validate_iso_date(&input.event_date)?;
    let canonical_url = validate_official_artifact_url(&input.attribution_url)?;
    let complete = matches!(input.contents, DeckContentsKind::CompleteDeck);
    validate_cards(&input.cards, complete)?;
    let payload = canonical_payload(input)?;
    let encoded = serde_json::to_vec(&payload).map_err(|_| RepoError::InvalidRequest)?;
    if encoded.len() > MAX_MANUAL_PAYLOAD_BYTES {
        return Err(RepoError::InvalidRequest);
    }
    Ok(canonical_url)
}

/// Build a complete immutable statement and its source/preview digests.  The
/// source digest covers only source content; observation/identity metadata is
/// included in the preview digest and therefore cannot silently change during
/// import.
pub fn manual_preview(
    player_identity_id: PlayerId,
    identity_revision: u64,
    input: &ManualEvidenceInput,
    observed_at: UtcMillis,
) -> Result<ManualEvidencePreview, RepoError> {
    let canonical_url = validate_manual_evidence(input)?;
    let source_key = official_source_key(&canonical_url, &input.source_nickname)?;
    let payload = canonical_payload(input)?;
    let source_digest = canonical_digest(&payload)?;
    let envelope = json!({
        "canonicalizationVersion": super::models::CANONICALIZATION_VERSION,
        "sourceKey": source_key,
        "sourceDigest": source_digest,
        "identityId": player_identity_id,
        "identityRevision": identity_revision,
        "observedAt": observed_at.get(),
        "payload": payload,
    });
    let preview_digest = preview_digest(&envelope)?;
    let selected_fields = default_selected_fields();
    let evidence = PlayerEvidence {
        id: PlayerEvidenceId::new(),
        player_identity_id: player_identity_id.clone(),
        evidence_schema_version: super::models::EVIDENCE_SCHEMA_VERSION,
        kind: EvidenceKind::OfficialPublishedDecklist,
        provenance_mode: EvidenceProvenance::UserAttestedOfficialSource,
        provider_id: "official_mtgo".into(),
        attribution_url: input.attribution_url.clone(),
        canonical_source_url: Some(canonical_url),
        lookup_nickname: input.source_nickname.trim().to_owned(),
        source_nickname: input.source_nickname.trim().to_owned(),
        exact_match_rule: "case_insensitive_full_string".into(),
        scope: json!({
            "eventDate": input.event_date,
            "format": input.format.trim(),
        }),
        observed_at,
        imported_at: observed_at,
        source_key,
        source_digest,
        preview_digest,
        payload,
        selected_fields,
        supersedes_evidence_id: None,
        cards: input.cards.clone(),
    };
    Ok(ManualEvidencePreview {
        token: PlayerPreviewToken::new(),
        player_identity_id,
        identity_revision,
        evidence,
        approved_fields: approved_fields()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    })
}

pub fn default_selected_fields() -> Value {
    let mut fields = serde_json::Map::new();
    for field in approved_fields() {
        fields.insert(field.to_owned(), Value::Bool(true));
    }
    Value::Object(fields)
}

pub fn approved_fields() -> BTreeSet<&'static str> {
    [
        "source_nickname",
        "attribution_url",
        "event_title",
        "event_date",
        "format",
        "placement",
        "record",
        "contents",
    ]
    .into_iter()
    .collect()
}

/// Apply an approved selection to an immutable preview without mutating its
/// source statement.  Cards are retained as a separate bounded relation only
/// when the complete-deck statement is selected/imported.
pub fn select_preview_fields(
    preview: &ManualEvidencePreview,
    selected_fields: &Value,
) -> Result<PlayerEvidence, RepoError> {
    let payload = &preview.evidence.payload;
    let retained = retain_selected_payload(payload, selected_fields)?;
    let mut evidence = preview.evidence.clone();
    evidence.payload = retained;
    evidence.selected_fields = selected_fields.clone();
    Ok(evidence)
}

fn canonical_payload(input: &ManualEvidenceInput) -> Result<Value, RepoError> {
    let mut cards = input.cards.clone();
    cards.sort_by(|left, right| {
        left.zone
            .cmp(&right.zone)
            .then_with(|| left.oracle_id.cmp(&right.oracle_id))
    });
    let contents = match input.contents {
        DeckContentsKind::ReferenceOnly => "reference_only",
        DeckContentsKind::CompleteDeck => "complete_deck",
    };
    Ok(json!({
        "event_title": input.event_title.trim(),
        "event_date": input.event_date,
        "format": input.format.trim(),
        "placement": input.placement.as_deref().map(str::trim),
        "record": input.record.as_deref().map(str::trim),
        "source_nickname": input.source_nickname.trim(),
        "attribution_url": input.attribution_url,
        "contents": contents,
        "cards": cards,
    }))
}

fn bounded_required(value: &str, max: usize) -> Result<(), RepoError> {
    if value.trim().is_empty() || value.chars().count() > max || value.chars().any(char::is_control)
    {
        Err(RepoError::InvalidRequest)
    } else {
        Ok(())
    }
}

fn bounded_optional(value: &Option<String>, max: usize) -> Result<(), RepoError> {
    if let Some(value) = value {
        bounded_required(value, max)?;
    }
    Ok(())
}

fn validate_iso_date(value: &str) -> Result<(), RepoError> {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(RepoError::InvalidRequest);
    }
    let year = value[0..4]
        .parse::<u32>()
        .map_err(|_| RepoError::InvalidRequest)?;
    let month = value[5..7]
        .parse::<u32>()
        .map_err(|_| RepoError::InvalidRequest)?;
    let day = value[8..10]
        .parse::<u32>()
        .map_err(|_| RepoError::InvalidRequest)?;
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > max_day {
        Err(RepoError::InvalidRequest)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(contents: DeckContentsKind) -> ManualEvidenceInput {
        ManualEvidenceInput {
            event_title: "Event".into(),
            event_date: "2026-08-03".into(),
            format: "Modern".into(),
            placement: Some("1".into()),
            record: Some("5-0".into()),
            source_nickname: "Teichou_Aisu".into(),
            attribution_url: "https://www.mtgo.com/decklists/event-1".into(),
            contents,
            cards: Vec::new(),
        }
    }

    #[test]
    fn ut_023_manual_bounds_and_zero_io_validator() {
        assert!(validate_manual_evidence(&input(DeckContentsKind::ReferenceOnly)).is_ok());
        let mut invalid = input(DeckContentsKind::ReferenceOnly);
        invalid.event_title = "x".repeat(MAX_MANUAL_TITLE_SCALARS + 1);
        assert!(validate_manual_evidence(&invalid).is_err());
        invalid = input(DeckContentsKind::ReferenceOnly);
        invalid.format = "x".repeat(MAX_MANUAL_FIELD_SCALARS + 1);
        assert!(validate_manual_evidence(&invalid).is_err());
    }

    #[test]
    fn ut_008_to_010_reference_and_complete_contracts() {
        let mut reference = input(DeckContentsKind::ReferenceOnly);
        reference.cards.push(PlayerCard {
            oracle_id: "one".into(),
            display_name: "One".into(),
            zone: "main".into(),
            quantity: 1,
            basic_land: false,
        });
        assert!(validate_manual_evidence(&reference).is_err());
        let complete = input(DeckContentsKind::ReferenceOnly);
        let preview = manual_preview(
            PlayerId::new(),
            1,
            &complete,
            UtcMillis::new(1).expect("time"),
        )
        .expect("preview");
        assert!(!preview.evidence.is_complete_official_deck());
    }
}
