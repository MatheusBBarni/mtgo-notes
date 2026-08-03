use serde::{Deserialize, Serialize};
use url::Url;

use crate::domain::RepoError;

pub const OFFICIAL_PLAYER_ROUTE: &str = "https://www.mtgo.com/decklists";
pub const MTGTOP8_PLAYER_ROUTE: &str = "https://www.mtgtop8.com/search";
pub const MAX_OFFICIAL_URL_CHARS: usize = 2_048;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserRoute {
    OfficialMtgo,
    MtgTop8,
}

impl BrowserRoute {
    pub fn build(&self, nickname: &str) -> Result<String, RepoError> {
        if nickname.trim().is_empty()
            || nickname.chars().count() > 128
            || nickname.chars().any(char::is_control)
        {
            return Err(RepoError::InvalidRequest);
        }
        let base = match self {
            Self::OfficialMtgo => OFFICIAL_PLAYER_ROUTE,
            Self::MtgTop8 => MTGTOP8_PLAYER_ROUTE,
        };
        let mut url = Url::parse(base).map_err(|_| RepoError::UnsafeSource)?;
        url.query_pairs_mut().append_pair("player", nickname.trim());
        Ok(url.into())
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OfficialMtgo => "official_mtgo_browser",
            Self::MtgTop8 => "mtg_top8_browser",
        }
    }
}

/// Validate and canonicalize a user-attested official artifact without any
/// DNS, HTTP, browser, or parser call.  The exact entered URL remains the
/// attribution; the canonical URL is used only for source identity.
pub fn validate_official_artifact_url(value: &str) -> Result<String, RepoError> {
    if value.chars().count() > MAX_OFFICIAL_URL_CHARS || value.chars().any(char::is_control) {
        return Err(RepoError::UnsafeSource);
    }
    let url = Url::parse(value).map_err(|_| RepoError::UnsafeSource)?;
    let host = url.host_str().ok_or(RepoError::UnsafeSource)?;
    if url.scheme() != "https"
        || !matches!(host, "mtgo.com" | "www.mtgo.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path().trim_matches('/').is_empty()
        || !url.path().to_ascii_lowercase().contains("deck")
    {
        return Err(RepoError::UnsafeSource);
    }
    let mut canonical = url;
    canonical.set_fragment(None);
    Ok(canonical.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ut_046_and_ut_047_build_closed_percent_encoded_routes() {
        let official = BrowserRoute::OfficialMtgo
            .build("A nickname")
            .expect("route");
        assert!(official.contains("player=A+nickname") || official.contains("player=A%20nickname"));
        assert!(
            BrowserRoute::MtgTop8
                .build("A nickname")
                .expect("route")
                .starts_with(MTGTOP8_PLAYER_ROUTE)
        );
    }

    #[test]
    fn ut_022_and_ut_048_manual_url_validation_is_closed() {
        assert!(validate_official_artifact_url("https://www.mtgo.com/decklists/abc").is_ok());
        assert!(validate_official_artifact_url("http://www.mtgo.com/decklists/abc").is_err());
        assert!(validate_official_artifact_url("https://evil.example/deck").is_err());
        assert!(
            validate_official_artifact_url("https://www.mtgo.com/decklists/abc#token").is_err()
        );
        assert!(
            validate_official_artifact_url(&format!("https://www.mtgo.com/{}", "d".repeat(2_050)))
                .is_err()
        );
    }
}
