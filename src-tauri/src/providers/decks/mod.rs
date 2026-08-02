use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::domain::{RepoError, UtcMillis};

pub const PROVIDER_ID: &str = "official_mtgo";
pub const OFFICIAL_DECKLIST_URL: &str = "https://www.mtgo.com/decklists";
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_AUTOMATIC_ATTEMPTS: u8 = 3;
pub const ACCESS_SPIKE: &str = include_str!("OFFICIAL_ACCESS_SPIKE.md");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    InteractiveRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConsent {
    pub granted: bool,
    pub disclosed_fields: Vec<String>,
}

impl ProviderConsent {
    pub fn official_decks(granted: bool) -> Self {
        Self {
            granted,
            disclosed_fields: vec!["confirmed_handle".into(), "format".into()],
        }
    }

    fn permits_lookup(&self) -> bool {
        self.granted
            && self.disclosed_fields == ["confirmed_handle".to_owned(), "format".to_owned()]
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestBinding {
    pub encounter_generation: u64,
    pub request_token: String,
    pub confirmed_handle: String,
    pub format: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum LookupOutcome {
    #[serde(rename = "interactive_required")]
    InteractiveRequired {
        access_mode: AccessMode,
        official_url: String,
        binding: RequestBinding,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckZone {
    Main,
    Sideboard,
}

impl DeckZone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Sideboard => "sideboard",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckCard {
    pub oracle_id: String,
    pub display_name: String,
    pub zone: DeckZone,
    pub quantity: u16,
    #[serde(default)]
    pub basic_land: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckCandidate {
    pub provider: String,
    pub event: String,
    pub format: String,
    pub publication_date: UtcMillis,
    pub source_url: String,
    pub provider_label: Option<String>,
    pub response_token: String,
    pub encounter_generation: u64,
    pub cards: Vec<DeckCard>,
}

impl DeckCandidate {
    pub fn is_complete(&self) -> bool {
        let main = self
            .cards
            .iter()
            .filter(|card| card.zone == DeckZone::Main)
            .map(|card| usize::from(card.quantity))
            .sum::<usize>();
        let sideboard = self
            .cards
            .iter()
            .filter(|card| card.zone == DeckZone::Sideboard)
            .map(|card| usize::from(card.quantity))
            .sum::<usize>();
        (60..=250).contains(&main) && sideboard <= 15
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderResponse {
    pub content_type: String,
    pub body_size: usize,
    pub candidates: Vec<DeckCandidate>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetrySummary {
    pub attempts: u8,
    pub manual_retry_available: bool,
}

#[derive(Default)]
pub struct OfficialDeckProvider {
    pending: Mutex<HashMap<String, RequestBinding>>,
}

impl OfficialDeckProvider {
    pub fn access_mode(&self) -> AccessMode {
        AccessMode::InteractiveRequired
    }

    pub fn lookup(
        &self,
        consent: &ProviderConsent,
        confirmed_handle: &str,
        format: &str,
        encounter_generation: u64,
        request_token: &str,
    ) -> Result<LookupOutcome, RepoError> {
        if !consent.permits_lookup() {
            return Err(RepoError::ConsentRequired);
        }
        let confirmed_handle = confirmed_handle.trim();
        let format = format.trim();
        if confirmed_handle.is_empty() || format.is_empty() || request_token.trim().is_empty() {
            return Err(RepoError::InvalidRequest);
        }

        let mut official_url =
            Url::parse(OFFICIAL_DECKLIST_URL).map_err(|_| RepoError::ProviderInvalidResponse)?;
        official_url
            .query_pairs_mut()
            .append_pair("player", confirmed_handle)
            .append_pair("format", format);
        let binding = RequestBinding {
            encounter_generation,
            request_token: request_token.to_owned(),
            confirmed_handle: confirmed_handle.to_owned(),
            format: format.to_owned(),
        };
        self.pending
            .lock()
            .map_err(|_| RepoError::ProviderUnavailable)?
            .insert(request_token.to_owned(), binding.clone());

        Ok(LookupOutcome::InteractiveRequired {
            access_mode: AccessMode::InteractiveRequired,
            official_url: official_url.into(),
            binding,
        })
    }

    pub fn validate_confirmation(
        &self,
        candidate: &DeckCandidate,
        active_generation: u64,
        active_format: &str,
    ) -> Result<RequestBinding, RepoError> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| RepoError::ProviderUnavailable)?;
        let binding = pending
            .get(&candidate.response_token)
            .ok_or(RepoError::StaleProviderResult)?;
        if binding.encounter_generation != active_generation
            || candidate.encounter_generation != active_generation
            || !binding.format.eq_ignore_ascii_case(active_format)
            || !candidate.format.eq_ignore_ascii_case(active_format)
        {
            return Err(RepoError::StaleProviderResult);
        }
        validate_candidate(candidate, active_format)?;
        Ok(binding.clone())
    }

    pub fn invalidate_before(&self, active_generation: u64) -> Result<(), RepoError> {
        self.pending
            .lock()
            .map_err(|_| RepoError::ProviderUnavailable)?
            .retain(|_, binding| binding.encounter_generation >= active_generation);
        Ok(())
    }
}

pub fn validate_official_url(value: &str) -> Result<Url, RepoError> {
    let url = Url::parse(value).map_err(|_| RepoError::UnsafeSource)?;
    let host = url.host_str().ok_or(RepoError::UnsafeSource)?;
    if url.scheme() != "https"
        || !matches!(host, "mtgo.com" | "www.mtgo.com")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(RepoError::UnsafeSource);
    }
    Ok(url)
}

pub fn validate_candidate(
    candidate: &DeckCandidate,
    expected_format: &str,
) -> Result<(), RepoError> {
    if candidate.provider != PROVIDER_ID
        || candidate.event.trim().is_empty()
        || candidate.response_token.trim().is_empty()
        || !candidate.format.eq_ignore_ascii_case(expected_format)
    {
        return Err(RepoError::ProviderInvalidResponse);
    }
    validate_official_url(&candidate.source_url)?;
    if candidate.cards.is_empty()
        || candidate.cards.len() > 500
        || candidate.cards.iter().any(|card| {
            card.oracle_id.trim().is_empty()
                || card.display_name.trim().is_empty()
                || card.quantity == 0
                || card.quantity > 250
        })
    {
        return Err(RepoError::ProviderInvalidResponse);
    }
    if !candidate.is_complete() {
        return Err(RepoError::DeckIncomplete);
    }
    Ok(())
}

pub fn validate_response(
    response: &ProviderResponse,
    expected_format: &str,
) -> Result<(), RepoError> {
    if response.body_size > MAX_RESPONSE_BYTES
        || response.content_type != "application/json"
        || response.candidates.len() > 100
    {
        return Err(RepoError::ProviderInvalidResponse);
    }
    response
        .candidates
        .iter()
        .try_for_each(|candidate| validate_candidate(candidate, expected_format))
}

pub fn select_candidates(
    mut candidates: Vec<DeckCandidate>,
    expected_format: &str,
) -> Vec<DeckCandidate> {
    candidates.sort_by(|left, right| {
        let left_exact = left.format.eq_ignore_ascii_case(expected_format);
        let right_exact = right.format.eq_ignore_ascii_case(expected_format);
        right_exact
            .cmp(&left_exact)
            .then_with(|| right.publication_date.cmp(&left.publication_date))
            .then_with(|| left.response_token.cmp(&right.response_token))
    });
    candidates
}

pub fn execute_retry<T>(
    mut attempt: impl FnMut(u8) -> Result<T, RepoError>,
) -> Result<(T, RetrySummary), (RepoError, RetrySummary)> {
    for number in 1..=MAX_AUTOMATIC_ATTEMPTS {
        match attempt(number) {
            Ok(value) => {
                return Ok((
                    value,
                    RetrySummary {
                        attempts: number,
                        manual_retry_available: false,
                    },
                ));
            }
            Err(error) if error.retryable_provider_error() && number < MAX_AUTOMATIC_ATTEMPTS => {}
            Err(error) => {
                return Err((
                    error,
                    RetrySummary {
                        attempts: number,
                        manual_retry_available: true,
                    },
                ));
            }
        }
    }
    unreachable!("retry loop always returns")
}

pub fn compare_similarity(left: f64, right: f64) -> Ordering {
    right.total_cmp(&left)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding_token() -> String {
        crate::domain::EntityId::new().to_string()
    }

    fn cards() -> Vec<DeckCard> {
        vec![DeckCard {
            oracle_id: "oracle-card".into(),
            display_name: "Fixture Card".into(),
            zone: DeckZone::Main,
            quantity: 60,
            basic_land: false,
        }]
    }

    fn candidate(token: &str, generation: u64, publication_date: i64) -> DeckCandidate {
        DeckCandidate {
            provider: PROVIDER_ID.into(),
            event: "Fixture Challenge".into(),
            format: "Modern".into(),
            publication_date: UtcMillis::new(publication_date).expect("date"),
            source_url: OFFICIAL_DECKLIST_URL.into(),
            provider_label: Some("Fixture label".into()),
            response_token: token.into(),
            encounter_generation: generation,
            cards: cards(),
        }
    }

    #[test]
    fn ut_046_official_request_is_minimized_allowlisted_and_bound() {
        let provider = OfficialDeckProvider::default();
        let token = binding_token();
        let outcome = provider
            .lookup(
                &ProviderConsent::official_decks(true),
                "Opponent_42",
                "Modern",
                7,
                &token,
            )
            .expect("lookup");
        let LookupOutcome::InteractiveRequired {
            official_url,
            binding,
            ..
        } = outcome;
        let url = validate_official_url(&official_url).expect("official URL");
        assert_eq!(url.host_str(), Some("www.mtgo.com"));
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            vec![
                ("player".into(), "Opponent_42".into()),
                ("format".into(), "Modern".into())
            ]
        );
        assert_eq!(binding.encounter_generation, 7);
        assert_eq!(binding.request_token, token);
    }

    #[test]
    fn ut_047_and_it_094_missing_consent_stops_before_lookup() {
        let provider = OfficialDeckProvider::default();
        assert_eq!(
            provider.lookup(
                &ProviderConsent::official_decks(false),
                "Opponent",
                "Modern",
                1,
                &binding_token(),
            ),
            Err(RepoError::ConsentRequired)
        );
        assert!(provider.pending.lock().expect("pending").is_empty());
    }

    #[test]
    fn ut_048_and_it_091_reject_unsafe_or_malformed_sources() {
        for url in [
            "http://www.mtgo.com/decklists",
            "https://evil.example/decklists",
            "not a URL",
        ] {
            assert_eq!(validate_official_url(url), Err(RepoError::UnsafeSource));
        }
        let mut malformed = candidate(&binding_token(), 1, 10);
        malformed.event.clear();
        assert_eq!(
            validate_candidate(&malformed, "Modern"),
            Err(RepoError::ProviderInvalidResponse)
        );
    }

    #[test]
    fn ut_049_and_it_095_ignore_stale_generation_or_token() {
        let provider = OfficialDeckProvider::default();
        let token = binding_token();
        provider
            .lookup(
                &ProviderConsent::official_decks(true),
                "Opponent",
                "Modern",
                2,
                &token,
            )
            .expect("lookup");
        assert_eq!(
            provider.validate_confirmation(&candidate(&token, 1, 10), 2, "Modern"),
            Err(RepoError::StaleProviderResult)
        );
        assert_eq!(
            provider.validate_confirmation(&candidate(&binding_token(), 2, 10), 2, "Modern"),
            Err(RepoError::StaleProviderResult)
        );
    }

    #[test]
    fn ut_050_retry_stops_after_three_and_exposes_manual_retry() {
        let mut calls = 0;
        let result = execute_retry::<()>(|_| {
            calls += 1;
            Err(RepoError::ProviderUnavailable)
        });
        assert_eq!(calls, 3);
        assert_eq!(
            result,
            Err((
                RepoError::ProviderUnavailable,
                RetrySummary {
                    attempts: 3,
                    manual_retry_available: true
                }
            ))
        );
    }

    #[test]
    fn ut_051_it_092_and_it_093_select_newest_exact_format_with_provenance() {
        assert!(select_candidates(Vec::new(), "Modern").is_empty());
        let mut legacy = candidate("legacy", 1, 30);
        legacy.format = "Legacy".into();
        let older = candidate("older", 1, 10);
        let newer = candidate("newer", 1, 20);
        let selected = select_candidates(vec![legacy, older, newer], "Modern");
        assert_eq!(
            selected
                .iter()
                .map(|item| item.response_token.as_str())
                .collect::<Vec<_>>(),
            ["newer", "older", "legacy"]
        );
        assert!(selected.iter().all(|item| !item.event.is_empty()));
    }

    #[test]
    fn ut_052_rejects_size_shape_and_content_type() {
        let valid = candidate(&binding_token(), 1, 10);
        for response in [
            ProviderResponse {
                content_type: "text/html".into(),
                body_size: 100,
                candidates: vec![valid.clone()],
            },
            ProviderResponse {
                content_type: "application/json".into(),
                body_size: MAX_RESPONSE_BYTES + 1,
                candidates: vec![valid.clone()],
            },
        ] {
            assert_eq!(
                validate_response(&response, "Modern"),
                Err(RepoError::ProviderInvalidResponse)
            );
        }
    }

    #[test]
    fn ut_053_interactive_fallback_uses_official_site_only() {
        let provider = OfficialDeckProvider::default();
        assert_eq!(provider.access_mode(), AccessMode::InteractiveRequired);
        assert!(ACCESS_SPIKE.contains("Automatic access is disabled"));
        let LookupOutcome::InteractiveRequired { official_url, .. } = provider
            .lookup(
                &ProviderConsent::official_decks(true),
                "Opponent",
                "Modern",
                1,
                &binding_token(),
            )
            .expect("interactive");
        assert!(validate_official_url(&official_url).is_ok());
    }

    #[test]
    fn it_096_received_candidate_remains_confirmable_after_disconnect() {
        let provider = OfficialDeckProvider::default();
        let token = binding_token();
        provider
            .lookup(
                &ProviderConsent::official_decks(true),
                "Opponent",
                "Modern",
                1,
                &token,
            )
            .expect("lookup");
        assert!(
            provider
                .validate_confirmation(&candidate(&token, 1, 10), 1, "Modern")
                .is_ok()
        );
    }

    #[test]
    fn it_098_format_or_generation_change_invalidates_old_candidate() {
        let provider = OfficialDeckProvider::default();
        let token = binding_token();
        provider
            .lookup(
                &ProviderConsent::official_decks(true),
                "Opponent",
                "Modern",
                1,
                &token,
            )
            .expect("lookup");
        provider.invalidate_before(2).expect("invalidate");
        assert_eq!(
            provider.validate_confirmation(&candidate(&token, 1, 10), 2, "Legacy"),
            Err(RepoError::StaleProviderResult)
        );
    }

    #[test]
    fn it_280_local_fixture_covers_minimization_allowlist_retry_and_validation() {
        ut_046_official_request_is_minimized_allowlisted_and_bound();
        ut_048_and_it_091_reject_unsafe_or_malformed_sources();
        ut_050_retry_stops_after_three_and_exposes_manual_retry();
        ut_052_rejects_size_shape_and_content_type();
    }
}
