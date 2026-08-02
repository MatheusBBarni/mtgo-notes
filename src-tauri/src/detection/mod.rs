use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::domain::{InternalPhase, RepoError};
use crate::services::profiles::normalize_handle;

const EVIDENCE_METADATA_CAPACITY: usize = 128;
const OCR_ACTIVE_INTERVAL_MS: u64 = 1_000;
const OCR_BACKOFF_INTERVAL_MS: u64 = 5_000;
const OCR_BACKOFF_AFTER_MISSES: u8 = 3;
const DETECTION_PROFILE_PUBLIC_KEY: &str = "KO6QZrLSSGfo4sAonBmg/LR/sO94aiEW8JWxORUxjGM=";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceProvenance {
    Ocr,
    Uia,
    Manual,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextField {
    Opponent,
    Phase,
    Format,
    Game,
    Result,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceClass {
    Ineligible,
    Candidate,
    Trusted,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEvidence {
    pub provider_session: String,
    pub generation: u64,
    pub sequence: u64,
    pub monotonic_ms: u64,
    pub field: ContextField,
    pub normalized_value: String,
    pub display_value: String,
    pub confidence: f32,
    pub confidence_class: ConfidenceClass,
    pub provenance: EvidenceProvenance,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceInput {
    pub provider_session: String,
    pub generation: u64,
    pub sequence: u64,
    pub monotonic_ms: u64,
    pub field: ContextField,
    pub visible_text: String,
    pub confidence: f32,
    pub provenance: EvidenceProvenance,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedWindow {
    pub native_handle: u64,
    pub class_name: String,
    pub visible_title: String,
    pub selected_at: i64,
    pub visible: bool,
    pub minimized: bool,
    pub usable_bounds: bool,
}

impl AuthorizedWindow {
    pub fn capture_allowed(&self) -> bool {
        self.native_handle != 0 && self.visible && !self.minimized && self.usable_bounds
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionProfile {
    pub schema_version: u16,
    pub profile_version: String,
    pub supported_client_versions: Vec<String>,
    pub language: String,
    pub opponent_confidence_threshold_millis: u16,
    pub semantic_locators: BTreeMap<String, Vec<String>>,
    pub ocr_regions: Vec<OcrRegion>,
    pub signature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrRegion {
    pub field: ContextField,
    pub x_millis: u16,
    pub y_millis: u16,
    pub width_millis: u16,
    pub height_millis: u16,
}

impl DetectionProfile {
    pub fn bundled() -> Result<Self, RepoError> {
        let profile: Self = serde_json::from_str(include_str!(
            "../../resources/detection/mtgo-visible-v1.json"
        ))
        .map_err(|_| RepoError::ProviderUnavailable)?;
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), RepoError> {
        if self.language.trim().is_empty() {
            return Err(RepoError::OcrLanguageMissing);
        }
        let region_valid = self.ocr_regions.iter().all(|region| {
            region.width_millis > 0
                && region.height_millis > 0
                && u32::from(region.x_millis) + u32::from(region.width_millis) <= 1_000
                && u32::from(region.y_millis) + u32::from(region.height_millis) <= 1_000
        });
        if self.schema_version != 1
            || self.profile_version.trim().is_empty()
            || self.semantic_locators.is_empty()
            || !region_valid
        {
            return Err(RepoError::ProviderUnavailable);
        }
        let public_key: [u8; 32] = STANDARD
            .decode(DETECTION_PROFILE_PUBLIC_KEY)
            .map_err(|_| RepoError::ProviderUnavailable)?
            .try_into()
            .map_err(|_| RepoError::ProviderUnavailable)?;
        let verifying_key =
            VerifyingKey::from_bytes(&public_key).map_err(|_| RepoError::ProviderUnavailable)?;
        let signature = self
            .signature
            .strip_prefix("ed25519:")
            .ok_or(RepoError::ProviderUnavailable)
            .and_then(|encoded| {
                STANDARD
                    .decode(encoded)
                    .map_err(|_| RepoError::ProviderUnavailable)
            })
            .and_then(|bytes| {
                Signature::from_slice(&bytes).map_err(|_| RepoError::ProviderUnavailable)
            })?;
        let mut unsigned = self.clone();
        unsigned.signature.clear();
        let payload = serde_json::to_vec(&unsigned).map_err(|_| RepoError::ProviderUnavailable)?;
        verifying_key
            .verify(&payload, &signature)
            .map_err(|_| RepoError::ProviderUnavailable)?;
        Ok(())
    }

    pub fn opponent_threshold(&self) -> f32 {
        f32::from(self.opponent_confidence_threshold_millis) / 1_000.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EvidenceMetadata {
    generation: u64,
    sequence: u64,
    field: ContextField,
    confidence_class: ConfidenceClass,
    provenance: EvidenceProvenance,
}

#[derive(Clone, Debug)]
pub struct DetectionEngine {
    profile: DetectionProfile,
    consent_granted: bool,
    disclosed_fields: Vec<String>,
    paused: bool,
    selected_window: Option<AuthorizedWindow>,
    generation: u64,
    last_sequence: BTreeMap<(String, u64), u64>,
    accepted: BTreeMap<ContextField, ContextEvidence>,
    metadata: VecDeque<EvidenceMetadata>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DetectionWorkerContext {
    pub native_handle: u64,
    pub generation: u64,
}

impl DetectionEngine {
    pub fn new(profile: DetectionProfile) -> Self {
        Self {
            profile,
            consent_granted: false,
            disclosed_fields: Vec::new(),
            paused: true,
            selected_window: None,
            generation: 0,
            last_sequence: BTreeMap::new(),
            accepted: BTreeMap::new(),
            metadata: VecDeque::with_capacity(EVIDENCE_METADATA_CAPACITY),
        }
    }

    pub fn set_consent(&mut self, granted: bool, disclosed_fields: Vec<String>) {
        self.consent_granted = granted;
        self.disclosed_fields = if granted {
            disclosed_fields
        } else {
            Vec::new()
        };
        if !granted {
            self.paused = true;
            self.selected_window = None;
            self.generation = self.generation.saturating_add(1);
            self.accepted.clear();
        }
    }

    pub fn select_window(&mut self, window: AuthorizedWindow) -> Result<u64, RepoError> {
        if !self.consent_granted {
            return Err(RepoError::ConsentRequired);
        }
        if !window.capture_allowed() {
            return Err(RepoError::WindowNotFound);
        }
        self.generation = self.generation.saturating_add(1);
        self.selected_window = Some(window);
        self.paused = false;
        self.accepted.clear();
        Ok(self.generation)
    }

    pub fn pause(&mut self, paused: bool) -> Result<(), RepoError> {
        if !paused
            && (!self.consent_granted
                || !self
                    .selected_window
                    .as_ref()
                    .is_some_and(AuthorizedWindow::capture_allowed))
        {
            return Err(RepoError::ProviderUnavailable);
        }
        self.paused = paused;
        Ok(())
    }

    pub fn revoke_window(&mut self) {
        self.selected_window = None;
        self.paused = true;
        self.generation = self.generation.saturating_add(1);
        self.accepted.clear();
    }

    pub fn update_window_visibility(
        &mut self,
        visible: bool,
        minimized: bool,
        usable_bounds: bool,
    ) -> Result<(), RepoError> {
        let Some(window) = self.selected_window.as_mut() else {
            return Err(RepoError::WindowNotFound);
        };
        window.visible = visible;
        window.minimized = minimized;
        window.usable_bounds = usable_bounds;
        if !window.capture_allowed() {
            self.accepted.insert(
                ContextField::Phase,
                restricted_system_evidence(self.generation),
            );
            return Err(RepoError::ProviderUnavailable);
        }
        Ok(())
    }

    pub fn ingest(&mut self, input: EvidenceInput) -> Result<Option<ContextEvidence>, RepoError> {
        if !self.consent_granted
            || self.paused
            || !self
                .selected_window
                .as_ref()
                .is_some_and(AuthorizedWindow::capture_allowed)
        {
            return Err(RepoError::ProviderUnavailable);
        }
        if input.generation != self.generation {
            return Ok(None);
        }
        if !self.field_authorized(input.field) {
            return Err(RepoError::ConsentRequired);
        }
        let sequence_key = (input.provider_session.clone(), input.generation);
        let last_sequence = self.last_sequence.get(&sequence_key).copied().unwrap_or(0);
        if input.sequence <= last_sequence {
            return Ok(None);
        }
        self.last_sequence.insert(sequence_key, input.sequence);

        let (display_value, normalized_value) =
            normalize_evidence_value(input.field, &input.visible_text)?;
        let threshold = if input.field == ContextField::Opponent
            && input.provenance == EvidenceProvenance::Ocr
        {
            self.profile.opponent_threshold()
        } else {
            0.5
        };
        let confidence_class = if input.confidence < threshold {
            ConfidenceClass::Ineligible
        } else if input.provenance == EvidenceProvenance::Uia || input.confidence >= 0.9 {
            ConfidenceClass::Trusted
        } else {
            ConfidenceClass::Candidate
        };
        let evidence = ContextEvidence {
            provider_session: input.provider_session,
            generation: input.generation,
            sequence: input.sequence,
            monotonic_ms: input.monotonic_ms,
            field: input.field,
            normalized_value,
            display_value,
            confidence: input.confidence,
            confidence_class,
            provenance: input.provenance,
        };
        self.record_metadata(&evidence);
        if confidence_class == ConfidenceClass::Ineligible {
            return Ok(None);
        }
        if let Some(previous) = self.accepted.get(&evidence.field)
            && (previous.monotonic_ms > evidence.monotonic_ms
                || (previous.monotonic_ms == evidence.monotonic_ms
                    && previous.provenance > evidence.provenance))
        {
            return Ok(None);
        }
        self.accepted.insert(evidence.field, evidence.clone());
        Ok(Some(evidence))
    }

    pub fn status(&self) -> ProviderStatus {
        ProviderStatus {
            provider_id: "windows_visible_mtgo".into(),
            disclosure_version: 1,
            disclosed_fields: self.disclosed_fields.clone(),
            consent_granted: self.consent_granted,
            available: self
                .selected_window
                .as_ref()
                .is_some_and(AuthorizedWindow::capture_allowed),
            paused: self.paused,
            generation: self.generation,
            selected_window: self
                .selected_window
                .as_ref()
                .map(|window| SelectedWindowStatus {
                    authorized: true,
                    visible: window.visible,
                    minimized: window.minimized,
                }),
            manual_available: true,
        }
    }

    pub fn worker_context(&self) -> Option<DetectionWorkerContext> {
        if !self.consent_granted || self.paused {
            return None;
        }
        self.selected_window
            .as_ref()
            .filter(|window| window.capture_allowed())
            .map(|window| DetectionWorkerContext {
                native_handle: window.native_handle,
                generation: self.generation,
            })
    }

    pub fn semantic_locator_fields(&self) -> BTreeMap<String, ContextField> {
        let mut fields = BTreeMap::new();
        for (name, field) in [
            ("opponent", ContextField::Opponent),
            ("phase", ContextField::Phase),
            ("format", ContextField::Format),
            ("game", ContextField::Game),
            ("result", ContextField::Result),
        ] {
            if let Some(automation_ids) = self.profile.semantic_locators.get(name) {
                fields.extend(
                    automation_ids
                        .iter()
                        .cloned()
                        .map(|automation_id| (automation_id, field)),
                );
            }
        }
        fields
    }

    fn record_metadata(&mut self, evidence: &ContextEvidence) {
        if self.metadata.len() == EVIDENCE_METADATA_CAPACITY {
            self.metadata.pop_front();
        }
        self.metadata.push_back(EvidenceMetadata {
            generation: evidence.generation,
            sequence: evidence.sequence,
            field: evidence.field,
            confidence_class: evidence.confidence_class,
            provenance: evidence.provenance,
        });
    }

    fn field_authorized(&self, field: ContextField) -> bool {
        let required = match field {
            ContextField::Opponent => "visible opponent handle",
            ContextField::Phase => "visible match phase",
            ContextField::Format | ContextField::Game | ContextField::Result => {
                "visible format, game, and result labels"
            }
        };
        self.disclosed_fields.iter().any(|field| field == required)
    }
}

impl Default for DetectionEngine {
    fn default() -> Self {
        Self::new(DetectionProfile::bundled().expect("bundled detection profile must be valid"))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedWindowStatus {
    pub authorized: bool,
    pub visible: bool,
    pub minimized: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub provider_id: String,
    pub disclosure_version: u16,
    pub disclosed_fields: Vec<String>,
    pub consent_granted: bool,
    pub available: bool,
    pub paused: bool,
    pub generation: u64,
    pub selected_window: Option<SelectedWindowStatus>,
    pub manual_available: bool,
}

pub struct DetectionRuntime {
    pub engine: Mutex<DetectionEngine>,
    pub revision: Mutex<u64>,
    pub applied_idempotency_keys: Mutex<VecDeque<String>>,
}

impl Default for DetectionRuntime {
    fn default() -> Self {
        Self {
            engine: Mutex::new(DetectionEngine::default()),
            revision: Mutex::new(1),
            applied_idempotency_keys: Mutex::new(VecDeque::with_capacity(64)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OcrScheduler {
    next_allowed_ms: u64,
    consecutive_misses: u8,
}

impl OcrScheduler {
    pub fn new() -> Self {
        Self {
            next_allowed_ms: 0,
            consecutive_misses: 0,
        }
    }

    pub fn may_capture(&self, monotonic_ms: u64, window: &AuthorizedWindow, paused: bool) -> bool {
        !paused && window.capture_allowed() && monotonic_ms >= self.next_allowed_ms
    }

    pub fn record_attempt(&mut self, monotonic_ms: u64, found_text: bool) {
        if found_text {
            self.consecutive_misses = 0;
        } else {
            self.consecutive_misses = self.consecutive_misses.saturating_add(1);
        }
        let interval = if self.consecutive_misses >= OCR_BACKOFF_AFTER_MISSES {
            OCR_BACKOFF_INTERVAL_MS
        } else {
            OCR_ACTIVE_INTERVAL_MS
        };
        self.next_allowed_ms = monotonic_ms.saturating_add(interval);
    }
}

impl Default for OcrScheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EphemeralOcrText(Vec<u8>);

impl EphemeralOcrText {
    pub fn new(value: String) -> Self {
        Self(value.into_bytes())
    }

    pub fn expose(&self) -> Result<&str, RepoError> {
        std::str::from_utf8(&self.0).map_err(|_| RepoError::InvalidHandle)
    }
}

impl Drop for EphemeralOcrText {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

pub fn logical_crop(
    region: &OcrRegion,
    window_width: u32,
    window_height: u32,
) -> (u32, u32, u32, u32) {
    (
        window_width.saturating_mul(u32::from(region.x_millis)) / 1_000,
        window_height.saturating_mul(u32::from(region.y_millis)) / 1_000,
        window_width.saturating_mul(u32::from(region.width_millis)) / 1_000,
        window_height.saturating_mul(u32::from(region.height_millis)) / 1_000,
    )
}

pub fn phase_from_visible_text(value: &str) -> InternalPhase {
    match value.trim().to_ascii_lowercase().as_str() {
        "sideboarding" | "between games" => InternalPhase::BetweenGames,
        "match complete" | "results" => InternalPhase::CompletionPending,
        "pairings" | "game starting" => InternalPhase::PreMatch,
        _ => InternalPhase::InGameRestricted,
    }
}

fn normalize_evidence_value(
    field: ContextField,
    value: &str,
) -> Result<(String, String), RepoError> {
    match field {
        ContextField::Opponent => {
            let normalized = normalize_handle(value)?;
            Ok((normalized.display, normalized.key))
        }
        ContextField::Phase => {
            let phase = phase_from_visible_text(value);
            let serialized = serde_json::to_value(phase)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .ok_or(RepoError::InvalidRequest)?;
            Ok((serialized.clone(), serialized))
        }
        _ => {
            let display = value.trim();
            if display.is_empty()
                || display.chars().any(char::is_control)
                || display.chars().count() > 128
            {
                return Err(RepoError::InvalidRequest);
            }
            Ok((display.to_owned(), display.to_ascii_lowercase()))
        }
    }
}

fn restricted_system_evidence(generation: u64) -> ContextEvidence {
    ContextEvidence {
        provider_session: "window-visibility".into(),
        generation,
        sequence: 0,
        monotonic_ms: 0,
        field: ContextField::Phase,
        normalized_value: "in_game_restricted".into(),
        display_value: "in_game_restricted".into(),
        confidence: 1.0,
        confidence_class: ConfidenceClass::Trusted,
        provenance: EvidenceProvenance::Manual,
    }
}

#[cfg(windows)]
pub mod windows;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::UtcMillis;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureWindow {
        class_name: String,
        visible_title: String,
        visible: bool,
        minimized: bool,
    }

    #[derive(Deserialize)]
    struct FixtureEvent {
        sequence: u64,
        field: ContextField,
        value: String,
        confidence: f32,
    }

    #[derive(Deserialize)]
    struct SupportedFixture {
        window: FixtureWindow,
        events: Vec<FixtureEvent>,
    }

    #[derive(Deserialize)]
    struct OcrFixture {
        window: FixtureWindow,
        #[serde(rename = "uiaMissingFields")]
        uia_missing_fields: Vec<String>,
        ocr: FixtureOcr,
    }

    #[derive(Deserialize)]
    struct FixtureOcr {
        region: String,
        value: String,
        confidence: f32,
        ephemeral: bool,
    }

    #[derive(Deserialize)]
    struct ReorderedFixture {
        generation: u64,
        events: Vec<ReorderedEvent>,
    }

    #[derive(Deserialize)]
    struct ReorderedEvent {
        generation: u64,
        sequence: u64,
        field: ContextField,
        value: String,
    }

    fn window() -> AuthorizedWindow {
        AuthorizedWindow {
            native_handle: 42,
            class_name: "SyntheticMtgoWindow".into(),
            visible_title: "Magic Online".into(),
            selected_at: UtcMillis::now().get(),
            visible: true,
            minimized: false,
            usable_bounds: true,
        }
    }

    fn input(
        generation: u64,
        sequence: u64,
        field: ContextField,
        provenance: EvidenceProvenance,
        visible_text: &str,
        confidence: f32,
    ) -> EvidenceInput {
        EvidenceInput {
            provider_session: "session".into(),
            generation,
            sequence,
            monotonic_ms: sequence * 10,
            field,
            visible_text: visible_text.into(),
            confidence,
            provenance,
        }
    }

    fn engine() -> DetectionEngine {
        let mut engine = DetectionEngine::default();
        engine.set_consent(
            true,
            vec![
                "visible opponent handle".into(),
                "visible match phase".into(),
                "visible format, game, and result labels".into(),
            ],
        );
        engine.select_window(window()).expect("window");
        engine
    }

    #[test]
    fn ut_001_nfkc_handle_normalization() {
        let normalized = normalize_handle("  ＧＰＴ_42  ").expect("normalize");
        assert_eq!(normalized.display, "ＧＰＴ_42");
        assert_eq!(normalized.key, "gpt_42");
    }

    #[test]
    fn ut_002_control_only_handle_is_invalid() {
        assert_eq!(normalize_handle("\0"), Err(RepoError::InvalidHandle));
    }

    #[test]
    fn ut_003_trusted_uia_creates_candidate_without_ocr() {
        let mut engine = engine();
        let generation = engine.generation;
        let evidence = engine
            .ingest(input(
                generation,
                1,
                ContextField::Opponent,
                EvidenceProvenance::Uia,
                "Opponent_42",
                1.0,
            ))
            .expect("ingest")
            .expect("evidence");
        assert_eq!(evidence.confidence_class, ConfidenceClass::Trusted);
        assert_eq!(evidence.provenance, EvidenceProvenance::Uia);
    }

    #[test]
    fn ut_004_newer_uia_supersedes_older_ocr() {
        let mut engine = engine();
        let generation = engine.generation;
        let mut ocr = input(
            generation,
            1,
            ContextField::Opponent,
            EvidenceProvenance::Ocr,
            "Opponent_A",
            0.9,
        );
        ocr.monotonic_ms = 10;
        engine.ingest(ocr).expect("ocr");
        let mut uia = input(
            generation,
            2,
            ContextField::Opponent,
            EvidenceProvenance::Uia,
            "Opponent_B",
            1.0,
        );
        uia.monotonic_ms = 20;
        assert_eq!(
            engine
                .ingest(uia)
                .expect("uia")
                .expect("evidence")
                .display_value,
            "Opponent_B"
        );
    }

    #[test]
    fn ut_005_ocr_threshold_is_inclusive() {
        let mut engine = engine();
        let generation = engine.generation;
        let threshold = engine.profile.opponent_threshold();
        assert!(
            engine
                .ingest(input(
                    generation,
                    1,
                    ContextField::Opponent,
                    EvidenceProvenance::Ocr,
                    "Eligible",
                    threshold,
                ))
                .expect("threshold")
                .is_some()
        );
        assert!(
            engine
                .ingest(input(
                    generation,
                    2,
                    ContextField::Opponent,
                    EvidenceProvenance::Ocr,
                    "Ineligible",
                    f32::from_bits(threshold.to_bits() - 1),
                ))
                .expect("below")
                .is_none()
        );
    }

    #[test]
    fn ut_006_minimized_or_unselected_stops_capture() {
        let mut engine = engine();
        assert_eq!(
            engine.update_window_visibility(true, true, true),
            Err(RepoError::ProviderUnavailable)
        );
        engine.revoke_window();
        assert_eq!(engine.pause(false), Err(RepoError::ProviderUnavailable));
    }

    #[test]
    fn ut_007_duplicate_sequences_emit_nothing() {
        let mut engine = engine();
        let generation = engine.generation;
        let first = input(
            generation,
            1,
            ContextField::Opponent,
            EvidenceProvenance::Uia,
            "Opponent",
            1.0,
        );
        assert!(engine.ingest(first.clone()).expect("first").is_some());
        assert!(engine.ingest(first).expect("duplicate").is_none());
    }

    #[test]
    fn ut_008_missing_ocr_language_preserves_uia_and_manual_paths() {
        let mut profile = DetectionProfile::bundled().expect("profile");
        profile.language.clear();
        assert_eq!(profile.validate(), Err(RepoError::OcrLanguageMissing));
        let engine = engine();
        assert!(engine.status().manual_available);
    }

    #[test]
    fn signed_detection_profile_rejects_tampering() {
        let mut profile = DetectionProfile::bundled().expect("profile");
        profile.ocr_regions[0].width_millis -= 1;
        assert_eq!(profile.validate(), Err(RepoError::ProviderUnavailable));
    }

    #[test]
    fn bundled_semantic_locators_map_to_authorized_fields() {
        let fields = DetectionEngine::default().semantic_locator_fields();
        assert_eq!(fields.get("OpponentName"), Some(&ContextField::Opponent));
        assert_eq!(fields.get("SideboardingScene"), Some(&ContextField::Phase));
        assert_eq!(fields.get("EventFormat"), Some(&ContextField::Format));
        assert!(!fields.contains_key("PrivateChatTranscript"));
    }

    #[test]
    fn undisclosed_visible_field_is_rejected_before_normalization() {
        let mut engine = DetectionEngine::default();
        engine.set_consent(true, vec!["visible match phase".into()]);
        engine.select_window(window()).expect("window");
        let generation = engine.generation;
        assert_eq!(
            engine.ingest(input(
                generation,
                1,
                ContextField::Opponent,
                EvidenceProvenance::Uia,
                "MustNotEnterEvidence",
                1.0,
            )),
            Err(RepoError::ConsentRequired)
        );
    }

    #[test]
    fn deterministic_uia_fixture_yields_expected_ordered_evidence() {
        let fixture: SupportedFixture = serde_json::from_str(include_str!(
            "../../../tests/fixtures/detection/supported-uia.json"
        ))
        .expect("fixture");
        let mut engine = DetectionEngine::default();
        engine.set_consent(
            true,
            vec![
                "visible opponent handle".into(),
                "visible match phase".into(),
            ],
        );
        engine
            .select_window(AuthorizedWindow {
                native_handle: 42,
                class_name: fixture.window.class_name,
                visible_title: fixture.window.visible_title,
                selected_at: UtcMillis::now().get(),
                visible: fixture.window.visible,
                minimized: fixture.window.minimized,
                usable_bounds: true,
            })
            .expect("select");
        let generation = engine.generation;
        let evidence = fixture
            .events
            .into_iter()
            .filter_map(|event| {
                engine
                    .ingest(input(
                        generation,
                        event.sequence,
                        event.field,
                        EvidenceProvenance::Uia,
                        &event.value,
                        event.confidence,
                    ))
                    .expect("ingest")
            })
            .collect::<Vec<_>>();
        assert_eq!(evidence.len(), 2);
        assert_eq!(evidence[0].display_value, "Opponent_42");
        assert_eq!(evidence[1].normalized_value, "pre_match");
    }

    #[test]
    fn deterministic_ocr_fixture_is_crop_scoped_and_metadata_only() {
        let fixture: OcrFixture = serde_json::from_str(include_str!(
            "../../../tests/fixtures/detection/degraded-ocr.json"
        ))
        .expect("fixture");
        assert_eq!(fixture.uia_missing_fields, ["opponent"]);
        assert_eq!(fixture.ocr.region, "opponent");
        assert!(fixture.ocr.ephemeral);
        let mut engine = DetectionEngine::default();
        engine.set_consent(true, vec!["visible opponent handle".into()]);
        engine
            .select_window(AuthorizedWindow {
                native_handle: 42,
                class_name: fixture.window.class_name,
                visible_title: fixture.window.visible_title,
                selected_at: UtcMillis::now().get(),
                visible: fixture.window.visible,
                minimized: fixture.window.minimized,
                usable_bounds: true,
            })
            .expect("select");
        let raw = EphemeralOcrText::new(fixture.ocr.value);
        let generation = engine.generation;
        let evidence = engine
            .ingest(input(
                generation,
                1,
                ContextField::Opponent,
                EvidenceProvenance::Ocr,
                raw.expose().expect("text"),
                fixture.ocr.confidence,
            ))
            .expect("ingest")
            .expect("evidence");
        drop(raw);
        assert_eq!(evidence.normalized_value, "ocr_opponent");
        assert_eq!(engine.metadata.len(), 1);
    }

    #[test]
    fn deterministic_reordered_fixture_ignores_old_generation_and_duplicate_sequence() {
        let fixture: ReorderedFixture = serde_json::from_str(include_str!(
            "../../../tests/fixtures/detection/stale-conflicting.json"
        ))
        .expect("fixture");
        let mut engine = engine();
        engine.generation = fixture.generation;
        let accepted = fixture
            .events
            .into_iter()
            .filter_map(|event| {
                engine
                    .ingest(input(
                        event.generation,
                        event.sequence,
                        event.field,
                        EvidenceProvenance::Uia,
                        &event.value,
                        1.0,
                    ))
                    .expect("ingest")
            })
            .collect::<Vec<_>>();
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].display_value, "Current");
    }

    #[test]
    fn ocr_scheduler_is_visible_bounded_and_backs_off() {
        let mut scheduler = OcrScheduler::new();
        let window = window();
        assert!(scheduler.may_capture(0, &window, false));
        scheduler.record_attempt(0, false);
        assert!(!scheduler.may_capture(999, &window, false));
        scheduler.record_attempt(1_000, false);
        scheduler.record_attempt(2_000, false);
        assert!(!scheduler.may_capture(6_999, &window, false));
        assert!(scheduler.may_capture(7_000, &window, false));
    }

    #[test]
    fn dpi_independent_crop_targets_same_logical_region() {
        let region = &DetectionProfile::bundled().expect("profile").ocr_regions[0];
        let at_100 = logical_crop(region, 1_000, 1_000);
        let at_200 = logical_crop(region, 2_000, 2_000);
        assert_eq!(
            at_200,
            (at_100.0 * 2, at_100.1 * 2, at_100.2 * 2, at_100.3 * 2)
        );
    }
}
