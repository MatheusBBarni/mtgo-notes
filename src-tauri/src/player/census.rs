use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::domain::UtcMillis;

use super::models::{MAX_NICKNAME_SCALARS, normalize_player_nickname_key};

pub const CENSUS_HOST: &str = "census.daybreakgames.com";
pub const CENSUS_PATH: &str = "/get/mg/leaderboard";
pub const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
pub const MAX_RESPONSE_ROWS: usize = 2_000;
pub const MAX_PREVIEWS: usize = 10;
pub const CONNECT_TIMEOUT_SECONDS: u64 = 5;
pub const TOTAL_TIMEOUT_SECONDS: u64 = 15;
pub const LOCAL_COOLDOWN_SECONDS: i64 = 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedCensusScope {
    pub catalog_id: String,
    pub start_date: String,
    pub as_of_date: String,
}

impl ApprovedCensusScope {
    pub fn validate(&self) -> Result<(), PlayerProviderError> {
        if self.catalog_id.trim().is_empty()
            || self.start_date.trim().is_empty()
            || self.as_of_date.trim().is_empty()
            || self.catalog_id.chars().count() > 128
            || self.start_date.chars().count() > 32
            || self.as_of_date.chars().count() > 32
            || self
                .catalog_id
                .chars()
                .chain(self.start_date.chars())
                .chain(self.as_of_date.chars())
                .any(char::is_control)
        {
            Err(PlayerProviderError::ConfigurationInvalid)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewedCensusConfiguration {
    pub service_id: String,
    pub scope: ApprovedCensusScope,
    pub version: String,
    pub expires_at: UtcMillis,
    pub reviewed: bool,
    pub fingerprint: String,
}

impl ReviewedCensusConfiguration {
    pub fn validate(&self, now: UtcMillis) -> Result<(), PlayerProviderError> {
        self.scope.validate()?;
        let service_id = self.service_id.trim();
        if !self.reviewed
            || service_id.is_empty()
            || service_id.eq_ignore_ascii_case("s:example")
            || service_id.to_ascii_lowercase().contains("placeholder")
            || service_id.to_ascii_lowercase().contains("shared")
            || self.version.trim().is_empty()
            || self.fingerprint.len() != 64
            || !self.fingerprint.chars().all(|c| c.is_ascii_hexdigit())
            || self.expires_at <= now
        {
            return Err(if self.expires_at <= now {
                PlayerProviderError::ConfigurationExpired
            } else {
                PlayerProviderError::ConfigurationInvalid
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CensusRequest {
    pub url: String,
    pub query: BTreeMap<String, String>,
    pub connect_timeout_seconds: u64,
    pub total_timeout_seconds: u64,
}

pub fn build_request(
    configuration: &ReviewedCensusConfiguration,
) -> Result<CensusRequest, PlayerProviderError> {
    configuration.scope.validate()?;
    let mut query: BTreeMap<String, String> = BTreeMap::new();
    query.insert("service_id".into(), configuration.service_id.clone());
    query.insert(
        "digitalobjectcatalogid".into(),
        configuration.scope.catalog_id.clone(),
    );
    query.insert("startdate".into(), configuration.scope.start_date.clone());
    query.insert("date".into(), configuration.scope.as_of_date.clone());
    let mut url = Url::parse(&format!("https://{CENSUS_HOST}{CENSUS_PATH}"))
        .map_err(|_| PlayerProviderError::ConfigurationInvalid)?;
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in &query {
            pairs.append_pair(key, value);
        }
    }
    Ok(CensusRequest {
        url: url.into(),
        query,
        connect_timeout_seconds: CONNECT_TIMEOUT_SECONDS,
        total_timeout_seconds: TOTAL_TIMEOUT_SECONDS,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaderboardRow {
    pub nickname: String,
    pub total_points: u64,
    pub top_eight_finishes: u32,
    pub best_score: u64,
    pub payload: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedLeaderboard {
    pub rows: Vec<LeaderboardRow>,
    pub response_bytes: usize,
}

pub fn validate_response(
    body: &Value,
    response_bytes: usize,
) -> Result<ValidatedLeaderboard, PlayerProviderError> {
    if response_bytes > MAX_RESPONSE_BYTES {
        return Err(PlayerProviderError::ResponseTooLarge);
    }
    let rows = body
        .get("rows")
        .or_else(|| body.get("data"))
        .and_then(Value::as_array)
        .ok_or(PlayerProviderError::InvalidResponse)?;
    if rows.len() > MAX_RESPONSE_ROWS {
        return Err(PlayerProviderError::ResponseTooLarge);
    }
    let mut validated = Vec::with_capacity(rows.len());
    for row in rows {
        let object = row
            .as_object()
            .ok_or(PlayerProviderError::InvalidResponse)?;
        let nickname = object
            .get("nickname")
            .or_else(|| object.get("name"))
            .and_then(Value::as_str)
            .ok_or(PlayerProviderError::InvalidResponse)?;
        let nickname = nickname.trim();
        if nickname.is_empty()
            || nickname.chars().count() > MAX_NICKNAME_SCALARS
            || nickname.chars().any(char::is_control)
        {
            return Err(PlayerProviderError::InvalidResponse);
        }
        let total_points = object
            .get("total_points")
            .or_else(|| object.get("points"))
            .and_then(Value::as_u64)
            .ok_or(PlayerProviderError::InvalidResponse)?;
        let top_eight_finishes = object
            .get("top_eight_finishes")
            .or_else(|| object.get("top8"))
            .and_then(Value::as_u64)
            .ok_or(PlayerProviderError::InvalidResponse)?;
        let best_score = object
            .get("best_score")
            .or_else(|| object.get("best"))
            .and_then(Value::as_u64)
            .ok_or(PlayerProviderError::InvalidResponse)?;
        validated.push(LeaderboardRow {
            nickname: nickname.to_owned(),
            total_points,
            top_eight_finishes: u32::try_from(top_eight_finishes)
                .map_err(|_| PlayerProviderError::InvalidResponse)?,
            best_score,
            payload: row.clone(),
        });
    }
    Ok(ValidatedLeaderboard {
        rows: validated,
        response_bytes,
    })
}

pub fn exact_matches<'a>(
    response: &'a ValidatedLeaderboard,
    nickname: &str,
) -> Result<Vec<&'a LeaderboardRow>, PlayerProviderError> {
    let expected = normalize_player_nickname_key(nickname)
        .map_err(|_| PlayerProviderError::InvalidResponse)?;
    let matches = response
        .rows
        .iter()
        .filter(|row| {
            normalize_player_nickname_key(&row.nickname).ok().as_deref() == Some(&expected)
        })
        .take(MAX_PREVIEWS + 1)
        .collect::<Vec<_>>();
    if matches.len() > MAX_PREVIEWS {
        return Err(PlayerProviderError::ResponseTooLarge);
    }
    Ok(matches)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CensusProviderMode {
    #[default]
    Disabled,
    Synthetic,
    Live,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerProviderError {
    ProviderDisabled,
    ConfigurationInvalid,
    ConfigurationExpired,
    InvalidResponse,
    ResponseTooLarge,
    Timeout,
    RateLimited,
    Unavailable,
}

impl PlayerProviderError {
    pub fn code(self) -> &'static str {
        match self {
            Self::ProviderDisabled => "provider_disabled",
            Self::ConfigurationInvalid => "provider_configuration_invalid",
            Self::ConfigurationExpired => "provider_configuration_expired",
            Self::InvalidResponse => "provider_invalid_response",
            Self::ResponseTooLarge => "response_too_large",
            Self::Timeout => "lookup_timeout",
            Self::RateLimited => "provider_rate_limited",
            Self::Unavailable => "provider_unavailable",
        }
    }
}

pub struct DisabledProvider;

impl DisabledProvider {
    pub fn lookup(&self) -> Result<ValidatedLeaderboard, PlayerProviderError> {
        Err(PlayerProviderError::ProviderDisabled)
    }
}

pub struct SyntheticProvider {
    response: Result<ValidatedLeaderboard, PlayerProviderError>,
}

impl SyntheticProvider {
    pub fn from_response(body: &Value, response_bytes: usize) -> Result<Self, PlayerProviderError> {
        Ok(Self {
            response: validate_response(body, response_bytes),
        })
    }

    pub fn lookup(&self) -> Result<ValidatedLeaderboard, PlayerProviderError> {
        self.response.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config() -> ReviewedCensusConfiguration {
        ReviewedCensusConfiguration {
            service_id: "s:approved".into(),
            scope: ApprovedCensusScope {
                catalog_id: "mocs".into(),
                start_date: "2026-01-01".into(),
                as_of_date: "2026-01-02".into(),
            },
            version: "v1".into(),
            expires_at: UtcMillis::new(10_000).expect("time"),
            reviewed: true,
            fingerprint: "a".repeat(64),
        }
    }

    #[test]
    fn ut_026_disabled_provider_makes_no_http_effect() {
        assert_eq!(
            DisabledProvider.lookup().expect_err("disabled").code(),
            "provider_disabled"
        );
    }

    #[test]
    fn ut_027_and_ut_028_live_config_and_request_are_fail_closed_and_minimal() {
        let mut invalid = config();
        invalid.service_id = "s:example".into();
        assert_eq!(
            invalid.validate(UtcMillis::new(1).expect("time")),
            Err(PlayerProviderError::ConfigurationInvalid)
        );
        let request = build_request(&config()).expect("request");
        assert!(
            request
                .url
                .starts_with("https://census.daybreakgames.com/get/mg/leaderboard?")
        );
        assert!(!request.url.contains("nickname"));
        assert_eq!(request.query.len(), 4);
    }

    #[test]
    fn ut_030_to_035_response_bounds_exact_match_and_degradation() {
        let body = json!({"rows": [
            {"nickname": "Teichou_Aisu", "points": 12, "top8": 1, "best": 12},
            {"nickname": "Teichou_Aisu_extra", "points": 1, "top8": 0, "best": 1}
        ]});
        let response = validate_response(&body, 100).expect("valid");
        assert_eq!(
            exact_matches(&response, "teichou_aisu")
                .expect("match")
                .len(),
            1
        );
        assert!(validate_response(&body, MAX_RESPONSE_BYTES + 1).is_err());
        assert!(validate_response(&json!({"rows": [{"nickname": "x"}]}), 10).is_err());
    }
}
