use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::{InternalPhase, Revision, UtcMillis};
use crate::ipc::CallerIdentity;

use super::census::{
    ApprovedCensusScope, CensusProviderMode, PlayerProviderError, ValidatedLeaderboard,
    exact_matches,
};
use super::models::{
    PlayerId, PlayerOperationKey, PlayerPreviewToken, PlayerSourceRoute, canonical_digest,
    census_source_key,
};

pub const SESSION_TTL_MILLIS: i64 = 15 * 60 * 1_000;
pub const MAX_COMMAND_PAYLOAD_BYTES: usize = 256 * 1024;
pub const MAX_AUDIT_ENTRIES: usize = 100;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerErrorCode {
    ConsentRequired,
    ProviderDisabled,
    ProviderConfigurationInvalid,
    ProviderConfigurationExpired,
    DisclosureRestricted,
    CapabilityDenied,
    PlayerIdentityRequired,
    IdentityRevisionConflict,
    LookupInProgress,
    LookupCooldown,
    InvalidRequest,
    PayloadTooLarge,
    LookupTimeout,
    ProviderRateLimited,
    ProviderUnavailable,
    ProviderInvalidResponse,
    ResponseTooLarge,
    UnsafeSource,
    ManualEvidenceInvalid,
    LookupSessionStale,
    PreviewExpired,
    PreviewMismatch,
    BrowserOpenFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerRecovery {
    None,
    Retry,
    ReviewConsent,
    SaveIdentity,
    Wait,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerError {
    pub code: PlayerErrorCode,
    pub recovery: PlayerRecovery,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_at: Option<UtcMillis>,
}

impl PlayerError {
    pub fn new(code: PlayerErrorCode, recovery: PlayerRecovery) -> Self {
        Self {
            code,
            recovery,
            retry_at: None,
        }
    }

    pub fn with_retry_at(mut self, retry_at: UtcMillis) -> Self {
        self.retry_at = Some(retry_at);
        self
    }
}

impl From<PlayerProviderError> for PlayerError {
    fn from(error: PlayerProviderError) -> Self {
        let (code, recovery) = match error {
            PlayerProviderError::ProviderDisabled => {
                (PlayerErrorCode::ProviderDisabled, PlayerRecovery::None)
            }
            PlayerProviderError::ConfigurationInvalid => (
                PlayerErrorCode::ProviderConfigurationInvalid,
                PlayerRecovery::ReviewConsent,
            ),
            PlayerProviderError::ConfigurationExpired => (
                PlayerErrorCode::ProviderConfigurationExpired,
                PlayerRecovery::None,
            ),
            PlayerProviderError::InvalidResponse => (
                PlayerErrorCode::ProviderInvalidResponse,
                PlayerRecovery::Retry,
            ),
            PlayerProviderError::ResponseTooLarge => {
                (PlayerErrorCode::ResponseTooLarge, PlayerRecovery::Retry)
            }
            PlayerProviderError::Timeout => (PlayerErrorCode::LookupTimeout, PlayerRecovery::Retry),
            PlayerProviderError::RateLimited => {
                (PlayerErrorCode::ProviderRateLimited, PlayerRecovery::Wait)
            }
            PlayerProviderError::Unavailable => {
                (PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
            }
        };
        Self::new(code, recovery)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAvailability {
    Disabled,
    Ready,
    Invalid,
    Expired,
    Cooldown,
    Busy,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProviderStatus {
    pub route: PlayerSourceRoute,
    pub availability: ProviderAvailability,
    pub consent_granted: bool,
    pub disclosure_version: Option<String>,
    pub retry_at: Option<UtcMillis>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerCommandKind {
    Status,
    Cancel,
    RevokeConsent,
    GrantConsent,
    Lookup,
    Refresh,
    ManualPreview,
    Import,
    Selection,
    BrowserHandoff,
}

impl PlayerCommandKind {
    fn requires_outside_gameplay(self) -> bool {
        !matches!(self, Self::Status | Self::Cancel | Self::RevokeConsent)
    }
}

pub fn authorize_command(
    caller: CallerIdentity,
    phase: InternalPhase,
    command: PlayerCommandKind,
    payload_bytes: usize,
) -> Result<(), PlayerError> {
    if caller != CallerIdentity::Main {
        return Err(PlayerError::new(
            PlayerErrorCode::CapabilityDenied,
            PlayerRecovery::None,
        ));
    }
    if payload_bytes > MAX_COMMAND_PAYLOAD_BYTES {
        return Err(PlayerError::new(
            PlayerErrorCode::PayloadTooLarge,
            PlayerRecovery::None,
        ));
    }
    if command.requires_outside_gameplay()
        && !matches!(
            phase,
            InternalPhase::Idle
                | InternalPhase::PreMatch
                | InternalPhase::BetweenGames
                | InternalPhase::Finished
        )
    {
        return Err(PlayerError::new(
            PlayerErrorCode::DisclosureRestricted,
            PlayerRecovery::None,
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsentBinding {
    pub route: PlayerSourceRoute,
    pub disclosure_version: String,
    pub fields_digest: String,
    pub epoch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupSession {
    pub player_identity_id: PlayerId,
    pub identity_revision: Revision,
    pub nickname_snapshot: String,
    pub consent_epoch: u64,
    pub configuration_fingerprint: Option<String>,
    pub operation_key: PlayerOperationKey,
    pub generation: u64,
    pub expires_at: UtcMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidatePreview {
    pub source_key: String,
    pub source_digest: String,
    pub preview_digest: String,
    pub lookup_nickname: String,
    pub source_nickname: String,
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManualPreviewBinding {
    pub token: PlayerPreviewToken,
    pub player_identity_id: PlayerId,
    pub identity_revision: Revision,
    pub source_key: String,
    pub source_digest: String,
    pub preview_digest: String,
    pub operation_key: PlayerOperationKey,
    pub generation: u64,
    pub expires_at: UtcMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmptyLookupResult {
    pub provider_id: String,
    pub lookup_nickname: String,
    pub exact_match_rule: String,
    pub scope: ApprovedCensusScope,
    pub provider_configuration_version: String,
    pub completed_at: UtcMillis,
    pub operation_key: PlayerOperationKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LookupOutcome {
    Candidates(Vec<CandidatePreview>),
    Empty(EmptyLookupResult),
    Cancelled,
    Degraded(PlayerError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupAuditSummary {
    pub command: PlayerCommandKind,
    pub provider: Option<String>,
    pub configuration_version: Option<String>,
    pub consent_epoch: u64,
    pub generation: u64,
    pub started_at: UtcMillis,
    pub completed_at: UtcMillis,
    pub outcome_code: String,
    pub bytes: usize,
    pub rows: usize,
    pub previews: usize,
}

#[derive(Default)]
struct RuntimeState {
    provider_mode: CensusProviderMode,
    configuration_version: Option<String>,
    configuration_fingerprint: Option<String>,
    configuration_expires_at: Option<UtcMillis>,
    scope: Option<ApprovedCensusScope>,
    consents: BTreeMap<PlayerSourceRoute, ConsentBinding>,
    active: Option<LookupSession>,
    generation: u64,
    last_lookup_at: Option<UtcMillis>,
    provider_retry_at: Option<UtcMillis>,
    audit: VecDeque<LookupAuditSummary>,
    ephemeral_replay: BTreeMap<String, LookupOutcome>,
    manual_previews: BTreeMap<PlayerPreviewToken, ManualPreviewBinding>,
}

pub struct PlayerPublicResultsRuntime {
    state: Mutex<RuntimeState>,
}

impl Default for PlayerPublicResultsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerPublicResultsRuntime {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(RuntimeState {
                provider_mode: CensusProviderMode::Disabled,
                ..RuntimeState::default()
            }),
        }
    }

    pub fn set_provider_mode(
        &self,
        mode: CensusProviderMode,
        version: Option<String>,
        fingerprint: Option<String>,
        expires_at: Option<UtcMillis>,
        scope: Option<ApprovedCensusScope>,
    ) -> Result<(), PlayerError> {
        let mut state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        state.provider_mode = mode;
        state.configuration_version = version;
        state.configuration_fingerprint = fingerprint;
        state.configuration_expires_at = expires_at;
        state.scope = scope;
        state.generation = state.generation.saturating_add(1);
        state.active = None;
        state.ephemeral_replay.clear();
        state.manual_previews.clear();
        Ok(())
    }

    pub fn grant_consent(
        &self,
        route: PlayerSourceRoute,
        disclosure_version: String,
        fields_digest: String,
    ) -> Result<(), PlayerError> {
        if disclosure_version.trim().is_empty() || !is_digest(&fields_digest) {
            return Err(PlayerError::new(
                PlayerErrorCode::InvalidRequest,
                PlayerRecovery::None,
            ));
        }
        let mut state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        let epoch = state
            .consents
            .get(&route)
            .map_or(1, |binding| binding.epoch.saturating_add(1));
        state.consents.insert(
            route.clone(),
            ConsentBinding {
                route,
                disclosure_version,
                fields_digest,
                epoch,
            },
        );
        Ok(())
    }

    pub fn revoke_consent(&self, route: &PlayerSourceRoute) -> Result<(), PlayerError> {
        let mut state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        state.consents.remove(route);
        state.generation = state.generation.saturating_add(1);
        state.active = None;
        state.ephemeral_replay.clear();
        state.manual_previews.clear();
        Ok(())
    }

    pub fn consent_status(
        &self,
        route: PlayerSourceRoute,
    ) -> Result<PlayerProviderStatus, PlayerError> {
        let state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        let binding = state.consents.get(&route);
        let availability = if route == PlayerSourceRoute::CensusMocs {
            match state.provider_mode {
                CensusProviderMode::Disabled => ProviderAvailability::Disabled,
                CensusProviderMode::Synthetic | CensusProviderMode::Live => {
                    if state
                        .configuration_expires_at
                        .is_some_and(|expires| expires <= UtcMillis::now())
                    {
                        ProviderAvailability::Expired
                    } else if state.configuration_fingerprint.is_none() {
                        ProviderAvailability::Invalid
                    } else if state.active.is_some() {
                        ProviderAvailability::Busy
                    } else if state
                        .last_lookup_at
                        .zip(state.provider_retry_at)
                        .is_some_and(|(last, retry)| {
                            UtcMillis::now().get() < last.get() + 60_000 || UtcMillis::now() < retry
                        })
                    {
                        ProviderAvailability::Cooldown
                    } else {
                        ProviderAvailability::Ready
                    }
                }
            }
        } else {
            ProviderAvailability::Ready
        };
        Ok(PlayerProviderStatus {
            route,
            availability,
            consent_granted: binding.is_some(),
            disclosure_version: binding.map(|value| value.disclosure_version.clone()),
            retry_at: state.provider_retry_at,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn begin_lookup(
        &self,
        caller: CallerIdentity,
        phase: InternalPhase,
        payload_bytes: usize,
        identity_id: PlayerId,
        identity_revision: Revision,
        nickname: String,
        consent_version: &str,
        consent_fields_digest: &str,
        operation_key: PlayerOperationKey,
        now: UtcMillis,
    ) -> Result<LookupSession, PlayerError> {
        authorize_command(caller, phase, PlayerCommandKind::Lookup, payload_bytes)?;
        if nickname.trim().is_empty() {
            return Err(PlayerError::new(
                PlayerErrorCode::PlayerIdentityRequired,
                PlayerRecovery::SaveIdentity,
            ));
        }
        let mut state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        if state.active.is_some() {
            return Err(PlayerError::new(
                PlayerErrorCode::LookupInProgress,
                PlayerRecovery::Wait,
            ));
        }
        if state.provider_mode == CensusProviderMode::Disabled {
            return Err(PlayerError::new(
                PlayerErrorCode::ProviderDisabled,
                PlayerRecovery::None,
            ));
        }
        if state
            .configuration_expires_at
            .is_some_and(|expires| expires <= now)
        {
            return Err(PlayerError::new(
                PlayerErrorCode::ProviderConfigurationExpired,
                PlayerRecovery::None,
            ));
        }
        if state.configuration_fingerprint.is_none() || state.scope.is_none() {
            return Err(PlayerError::new(
                PlayerErrorCode::ProviderConfigurationInvalid,
                PlayerRecovery::None,
            ));
        }
        if state
            .last_lookup_at
            .is_some_and(|last| now.get() < last.get() + 60_000)
            || state.provider_retry_at.is_some_and(|retry| now < retry)
        {
            let retry_at = state.provider_retry_at.or_else(|| {
                state
                    .last_lookup_at
                    .and_then(|last| UtcMillis::new(last.get() + 60_000).ok())
            });
            return Err(
                PlayerError::new(PlayerErrorCode::LookupCooldown, PlayerRecovery::Wait)
                    .with_retry_at(retry_at.unwrap_or(now)),
            );
        }
        let consent_epoch = {
            let consent = state
                .consents
                .get(&PlayerSourceRoute::CensusMocs)
                .ok_or_else(|| {
                    PlayerError::new(
                        PlayerErrorCode::ConsentRequired,
                        PlayerRecovery::ReviewConsent,
                    )
                })?;
            if consent.disclosure_version != consent_version
                || consent.fields_digest != consent_fields_digest
            {
                return Err(PlayerError::new(
                    PlayerErrorCode::ConsentRequired,
                    PlayerRecovery::ReviewConsent,
                ));
            }
            consent.epoch
        };
        state.generation = state.generation.saturating_add(1);
        let session = LookupSession {
            player_identity_id: identity_id,
            identity_revision,
            nickname_snapshot: nickname,
            consent_epoch,
            configuration_fingerprint: state.configuration_fingerprint.clone(),
            operation_key,
            generation: state.generation,
            expires_at: UtcMillis::new(now.get() + SESSION_TTL_MILLIS).map_err(|_| {
                PlayerError::new(PlayerErrorCode::InvalidRequest, PlayerRecovery::None)
            })?,
        };
        state.last_lookup_at = Some(now);
        state.active = Some(session.clone());
        Ok(session)
    }

    pub fn complete_lookup(
        &self,
        session: &LookupSession,
        response: Result<ValidatedLeaderboard, PlayerProviderError>,
        now: UtcMillis,
    ) -> Result<LookupOutcome, PlayerError> {
        let mut state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        let active = state.active.as_ref().ok_or_else(|| {
            PlayerError::new(PlayerErrorCode::LookupSessionStale, PlayerRecovery::Retry)
        })?;
        if active != session || now >= session.expires_at || state.generation != session.generation
        {
            return Err(PlayerError::new(
                PlayerErrorCode::LookupSessionStale,
                PlayerRecovery::Retry,
            ));
        }
        let consent = state.consents.get(&PlayerSourceRoute::CensusMocs);
        if consent.is_none_or(|binding| binding.epoch != session.consent_epoch) {
            state.active = None;
            return Err(PlayerError::new(
                PlayerErrorCode::LookupSessionStale,
                PlayerRecovery::ReviewConsent,
            ));
        }
        let outcome = match response {
            Err(error) => LookupOutcome::Degraded(error.into()),
            Ok(response) => {
                let matches = exact_matches(&response, &session.nickname_snapshot)
                    .map_err(PlayerError::from)?;
                if matches.is_empty() {
                    let scope = state.scope.clone().ok_or_else(|| {
                        PlayerError::new(
                            PlayerErrorCode::ProviderConfigurationInvalid,
                            PlayerRecovery::None,
                        )
                    })?;
                    LookupOutcome::Empty(EmptyLookupResult {
                        provider_id: "census_mocs".into(),
                        lookup_nickname: session.nickname_snapshot.clone(),
                        exact_match_rule: "case_insensitive_full_string".into(),
                        scope,
                        provider_configuration_version: state
                            .configuration_version
                            .clone()
                            .unwrap_or_else(|| "unknown".into()),
                        completed_at: now,
                        operation_key: session.operation_key.clone(),
                    })
                } else {
                    let scope = state.scope.clone().ok_or_else(|| {
                        PlayerError::new(
                            PlayerErrorCode::ProviderConfigurationInvalid,
                            PlayerRecovery::None,
                        )
                    })?;
                    let mut candidates = Vec::with_capacity(matches.len());
                    for row in matches {
                        let source_key = census_source_key(
                            &scope.catalog_id,
                            &scope.start_date,
                            &scope.as_of_date,
                            &row.nickname,
                        )
                        .map_err(|_| {
                            PlayerError::new(PlayerErrorCode::InvalidRequest, PlayerRecovery::None)
                        })?;
                        let source_digest = canonical_digest(&row.payload).map_err(|_| {
                            PlayerError::new(
                                PlayerErrorCode::ProviderInvalidResponse,
                                PlayerRecovery::Retry,
                            )
                        })?;
                        let preview_digest = canonical_digest(&json!({
                            "sourceKey": source_key,
                            "sourceDigest": source_digest,
                            "lookupNickname": session.nickname_snapshot,
                            "observedAt": now.get(),
                            "payload": row.payload,
                        }))
                        .map_err(|_| {
                            PlayerError::new(
                                PlayerErrorCode::ProviderInvalidResponse,
                                PlayerRecovery::Retry,
                            )
                        })?;
                        candidates.push(CandidatePreview {
                            source_key,
                            source_digest,
                            preview_digest,
                            lookup_nickname: session.nickname_snapshot.clone(),
                            source_nickname: row.nickname.clone(),
                            payload: row.payload.clone(),
                        });
                    }
                    LookupOutcome::Candidates(candidates)
                }
            }
        };
        state.active = None;
        state
            .ephemeral_replay
            .insert(session.operation_key.as_str().to_owned(), outcome.clone());
        Ok(outcome)
    }

    pub fn cancel_lookup(
        &self,
        caller: CallerIdentity,
        phase: InternalPhase,
        operation_key: &PlayerOperationKey,
    ) -> Result<LookupOutcome, PlayerError> {
        authorize_command(caller, phase, PlayerCommandKind::Cancel, 0)?;
        let mut state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        let session = state.active.take().ok_or_else(|| {
            PlayerError::new(PlayerErrorCode::LookupSessionStale, PlayerRecovery::None)
        })?;
        if session.operation_key != *operation_key {
            state.active = Some(session);
            return Err(PlayerError::new(
                PlayerErrorCode::LookupSessionStale,
                PlayerRecovery::Retry,
            ));
        }
        state.generation = state.generation.saturating_add(1);
        let outcome = LookupOutcome::Cancelled;
        state
            .ephemeral_replay
            .insert(operation_key.as_str().to_owned(), outcome.clone());
        Ok(outcome)
    }

    pub fn replay(
        &self,
        operation_key: &PlayerOperationKey,
    ) -> Result<Option<LookupOutcome>, PlayerError> {
        let state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        Ok(state.ephemeral_replay.get(operation_key.as_str()).cloned())
    }

    /// Bind a pure manual preview to the trusted identity/session fence.  The
    /// token is opaque to the renderer and expires at the exact TTL boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn bind_manual_preview(
        &self,
        token: PlayerPreviewToken,
        player_identity_id: PlayerId,
        identity_revision: Revision,
        source_key: String,
        source_digest: String,
        preview_digest: String,
        operation_key: PlayerOperationKey,
        now: UtcMillis,
    ) -> Result<ManualPreviewBinding, PlayerError> {
        if !is_digest(&source_digest) || !is_digest(&preview_digest) {
            return Err(PlayerError::new(
                PlayerErrorCode::InvalidRequest,
                PlayerRecovery::None,
            ));
        }
        let mut state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        state.generation = state.generation.saturating_add(1);
        let binding = ManualPreviewBinding {
            token: token.clone(),
            player_identity_id,
            identity_revision,
            source_key,
            source_digest,
            preview_digest,
            operation_key,
            generation: state.generation,
            expires_at: UtcMillis::new(now.get() + SESSION_TTL_MILLIS).map_err(|_| {
                PlayerError::new(PlayerErrorCode::InvalidRequest, PlayerRecovery::None)
            })?,
        };
        state.manual_previews.insert(token, binding.clone());
        Ok(binding)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn verify_manual_preview(
        &self,
        token: &PlayerPreviewToken,
        player_identity_id: &PlayerId,
        identity_revision: Revision,
        source_key: &str,
        source_digest: &str,
        preview_digest: &str,
        now: UtcMillis,
    ) -> Result<ManualPreviewBinding, PlayerError> {
        let state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        let binding = state.manual_previews.get(token).ok_or_else(|| {
            PlayerError::new(PlayerErrorCode::PreviewMismatch, PlayerRecovery::Retry)
        })?;
        if now >= binding.expires_at {
            return Err(PlayerError::new(
                PlayerErrorCode::PreviewExpired,
                PlayerRecovery::Retry,
            ));
        }
        if binding.player_identity_id != *player_identity_id
            || binding.identity_revision != identity_revision
            || binding.source_key != source_key
            || binding.source_digest != source_digest
            || binding.preview_digest != preview_digest
        {
            return Err(PlayerError::new(
                PlayerErrorCode::PreviewMismatch,
                PlayerRecovery::Retry,
            ));
        }
        Ok(binding.clone())
    }

    pub fn invalidate_identity(&self, player_identity_id: &PlayerId) -> Result<(), PlayerError> {
        let mut state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        state
            .manual_previews
            .retain(|_, binding| binding.player_identity_id != *player_identity_id);
        if state
            .active
            .as_ref()
            .is_some_and(|session| session.player_identity_id == *player_identity_id)
        {
            state.active = None;
            state.generation = state.generation.saturating_add(1);
        }
        Ok(())
    }

    pub fn audit_snapshot(&self) -> Result<Vec<LookupAuditSummary>, PlayerError> {
        let state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        Ok(state.audit.iter().cloned().collect())
    }

    pub fn restart(&self) -> Result<(), PlayerError> {
        let mut state = self.state.lock().map_err(|_| {
            PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
        })?;
        state.active = None;
        state.ephemeral_replay.clear();
        state.manual_previews.clear();
        state.audit.clear();
        state.generation = state.generation.saturating_add(1);
        Ok(())
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::player::census::{ApprovedCensusScope, SyntheticProvider};

    fn runtime() -> PlayerPublicResultsRuntime {
        let runtime = PlayerPublicResultsRuntime::new();
        runtime
            .set_provider_mode(
                CensusProviderMode::Synthetic,
                Some("v1".into()),
                Some("a".repeat(64)),
                Some(UtcMillis::new(100_000).expect("time")),
                Some(ApprovedCensusScope {
                    catalog_id: "mocs".into(),
                    start_date: "2026-01-01".into(),
                    as_of_date: "2026-01-02".into(),
                }),
            )
            .expect("mode");
        runtime
            .grant_consent(
                PlayerSourceRoute::CensusMocs,
                "disclosure-v1".into(),
                "b".repeat(64),
            )
            .expect("consent");
        runtime
    }

    fn session(runtime: &PlayerPublicResultsRuntime) -> LookupSession {
        runtime
            .begin_lookup(
                CallerIdentity::Main,
                InternalPhase::Idle,
                10,
                PlayerId::new(),
                Revision::INITIAL,
                "Teichou_Aisu".into(),
                "disclosure-v1",
                &"b".repeat(64),
                PlayerOperationKey::new(),
                UtcMillis::new(1).expect("time"),
            )
            .expect("session")
    }

    #[test]
    fn ut_011_and_ut_035_empty_is_only_valid_zero_match() {
        let runtime = runtime();
        let session = session(&runtime);
        let provider =
            SyntheticProvider::from_response(&json!({"rows": []}), 10).expect("provider");
        assert!(matches!(
            runtime.complete_lookup(
                &session,
                provider.lookup(),
                UtcMillis::new(2).expect("time")
            ),
            Ok(LookupOutcome::Empty(_))
        ));
        let runtime_again = self::runtime();
        let session_again = self::session(&runtime_again);
        assert!(matches!(
            runtime_again.complete_lookup(
                &session_again,
                Err(PlayerProviderError::Unavailable),
                UtcMillis::new(2).expect("time")
            ),
            Ok(LookupOutcome::Degraded(_))
        ));
    }

    #[test]
    fn ut_018_to_021_status_consent_phase_and_payload_admission() {
        let runtime = PlayerPublicResultsRuntime::new();
        assert_eq!(
            runtime
                .consent_status(PlayerSourceRoute::CensusMocs)
                .expect("status")
                .availability,
            ProviderAvailability::Disabled
        );
        assert!(
            authorize_command(
                CallerIdentity::Overlay,
                InternalPhase::Idle,
                PlayerCommandKind::Lookup,
                1
            )
            .is_err()
        );
        assert!(
            authorize_command(
                CallerIdentity::Main,
                InternalPhase::InGameRestricted,
                PlayerCommandKind::Lookup,
                1
            )
            .is_err()
        );
        assert!(
            authorize_command(
                CallerIdentity::Main,
                InternalPhase::Idle,
                PlayerCommandKind::Status,
                MAX_COMMAND_PAYLOAD_BYTES + 1
            )
            .is_err()
        );
    }

    #[test]
    fn ut_039_to_050_fencing_lease_replay_and_redacted_error() {
        let runtime = runtime();
        let first = session(&runtime);
        assert!(matches!(
            runtime.begin_lookup(
                CallerIdentity::Main,
                InternalPhase::Idle,
                1,
                first.player_identity_id.clone(),
                Revision::INITIAL,
                "other".into(),
                "disclosure-v1",
                &"b".repeat(64),
                PlayerOperationKey::new(),
                UtcMillis::new(2).expect("time")
            ),
            Err(PlayerError {
                code: PlayerErrorCode::LookupInProgress,
                ..
            })
        ));
        let cancelled = runtime
            .cancel_lookup(
                CallerIdentity::Main,
                InternalPhase::InGameRestricted,
                &first.operation_key,
            )
            .expect("cancel");
        assert_eq!(cancelled, LookupOutcome::Cancelled);
        assert_eq!(
            runtime.replay(&first.operation_key).expect("replay"),
            Some(LookupOutcome::Cancelled)
        );
        let serialized = serde_json::to_string(&PlayerError::new(
            PlayerErrorCode::ProviderInvalidResponse,
            PlayerRecovery::Retry,
        ))
        .expect("error");
        assert!(!serialized.contains("nickname"));
        assert!(!serialized.contains("token"));
    }
}
