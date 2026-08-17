use serde::{Deserialize, Serialize};

use crate::domain::{EntityId, Revision, UtcMillis};
use crate::ipc::{AppError, ErrorCode};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalPhase {
    Idle,
    Candidate,
    PreMatch,
    InGameRestricted,
    BetweenGames,
    CompletionPending,
    Finished,
    Incomplete,
}

impl InternalPhase {
    pub fn is_disclosure_restricted(self) -> bool {
        matches!(
            self,
            Self::Candidate | Self::InGameRestricted | Self::CompletionPending | Self::Incomplete
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EncounterStatus {
    Active,
    Finished,
    Incomplete,
    Deleted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardCertainty {
    Observed,
    Suspected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpponentProfile {
    pub id: EntityId,
    pub primary_handle: String,
    pub normalized_handle: String,
    pub created_at: UtcMillis,
    pub revision: Revision,
    pub deleted_at: Option<UtcMillis>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpponentAlias {
    pub id: EntityId,
    pub profile_id: EntityId,
    pub display_handle: String,
    pub normalized_handle: String,
    pub provenance: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Encounter {
    pub id: EntityId,
    pub profile_id: EntityId,
    pub format: String,
    pub started_at: UtcMillis,
    pub ended_at: Option<UtcMillis>,
    pub status: EncounterStatus,
    pub phase: InternalPhase,
    pub source: String,
    pub revision: Revision,
    pub incomplete_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub id: EntityId,
    pub encounter_id: EntityId,
    pub text: String,
    pub created_at: UtcMillis,
    pub edited_at: Option<UtcMillis>,
    pub revision: Revision,
    pub deletion_deadline: Option<UtcMillis>,
    pub deleted_at: Option<UtcMillis>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepoError {
    AcknowledgementRequired,
    AlreadyOpen,
    EmptyPortabilityConfirmationRequired,
    AssetsInvalid,
    BlankObservation,
    CandidateStale,
    CancelUnsafe,
    ConsentRequired,
    DeckIncomplete,
    DestinationUnwritable,
    DisclosureRestricted,
    ExplicitCorrectionRequired,
    FormatUnsupported,
    IdentityConflict,
    InvalidCard,
    InvalidCursor,
    InvalidHandle,
    InvalidTag,
    InvalidBackup,
    InvalidRequest,
    InvalidTransition,
    JobBusy,
    KeyUnavailable,
    MergeConflict,
    MigrationFailed,
    NoActiveEncounter,
    NotebookInvalid,
    NotFound,
    OperationBusy,
    OcrLanguageMissing,
    OverlayUnavailable,
    PlaintextAcknowledgementRequired,
    ProviderInvalidResponse,
    ProviderUnavailable,
    RevisionConflict,
    SaveFailed,
    ScopeMismatch,
    StaleProviderResult,
    UndoExpired,
    UnsafeSource,
    WindowNotFound,
    WrongPassphrase,
}

impl RepoError {
    pub fn retryable_provider_error(self) -> bool {
        matches!(self, Self::ProviderUnavailable)
    }

    pub fn to_app_error(self) -> AppError {
        match self {
            Self::AcknowledgementRequired => AppError::new(
                ErrorCode::AcknowledgementRequired,
                "Acknowledge that a forgotten backup passphrase cannot be recovered.",
                false,
            )
            .with_field("passphraseAcknowledged"),
            Self::AlreadyOpen => AppError::new(
                ErrorCode::AlreadyOpen,
                "Quick capture is already open.",
                false,
            ),
            Self::EmptyPortabilityConfirmationRequired => AppError::new(
                ErrorCode::AcknowledgementRequired,
                "Confirm that the selected scope contains no active profiles.",
                false,
            )
            .with_field("confirmEmpty"),
            Self::AssetsInvalid => AppError::new(
                ErrorCode::AssetsInvalid,
                "Classifier assets failed signature, schema, or digest validation.",
                false,
            ),
            Self::BlankObservation => AppError::new(
                ErrorCode::BlankObservation,
                "An observation requires non-whitespace text.",
                false,
            )
            .with_field("text"),
            Self::CandidateStale => AppError::new(
                ErrorCode::CandidateStale,
                "The opponent candidate was replaced by newer evidence.",
                false,
            ),
            Self::CancelUnsafe => AppError::new(
                ErrorCode::CancelUnsafe,
                "This operation passed its safe cancellation point and must finish recovery.",
                false,
            ),
            Self::ConsentRequired => AppError::new(
                ErrorCode::ConsentRequired,
                "Review and grant official deck provider consent before lookup.",
                false,
            ),
            Self::DeckIncomplete => AppError::new(
                ErrorCode::DeckIncomplete,
                "A complete decklist is required for classification.",
                false,
            ),
            Self::DestinationUnwritable => AppError::new(
                ErrorCode::DestinationUnwritable,
                "The selected destination cannot be written.",
                false,
            )
            .with_field("destination"),
            Self::DisclosureRestricted => AppError::new(
                ErrorCode::DisclosureRestricted,
                "Historical notebook data is unavailable during possible gameplay.",
                false,
            ),
            Self::ExplicitCorrectionRequired => AppError::new(
                ErrorCode::ExplicitCorrectionRequired,
                "An encounter is already active. Use the explicit opponent correction flow.",
                false,
            ),
            Self::FormatUnsupported => AppError::new(
                ErrorCode::FormatUnsupported,
                "This format has no bundled classifier coverage.",
                false,
            ),
            Self::IdentityConflict => AppError::new(
                ErrorCode::IdentityConflict,
                "That handle is already assigned to another active profile.",
                false,
            ),
            Self::InvalidCard => AppError::new(
                ErrorCode::InvalidCard,
                "A card entry has an invalid name, quantity, or certainty.",
                false,
            )
            .with_field("cards"),
            Self::InvalidCursor => AppError::new(
                ErrorCode::InvalidCursor,
                "The page cursor is invalid.",
                false,
            )
            .with_field("cursor"),
            Self::InvalidHandle => AppError::new(
                ErrorCode::InvalidHandle,
                "Enter a valid opponent handle.",
                false,
            )
            .with_field("handle"),
            Self::InvalidTag => AppError::new(
                ErrorCode::InvalidRequest,
                "A tendency tag has an invalid label.",
                false,
            )
            .with_field("tags"),
            Self::InvalidBackup => AppError::new(
                ErrorCode::InvalidBackup,
                "The backup is malformed, damaged, or incompatible.",
                false,
            ),
            Self::InvalidRequest => {
                AppError::new(ErrorCode::InvalidRequest, "The request is invalid.", false)
            }
            Self::InvalidTransition => AppError::new(
                ErrorCode::InvalidTransition,
                "The encounter transition is not valid.",
                false,
            ),
            Self::JobBusy => AppError::new(
                ErrorCode::JobBusy,
                "A classifier job is already running.",
                true,
            ),
            Self::KeyUnavailable => AppError::new(
                ErrorCode::KeyUnavailable,
                "The notebook key is unavailable for this Windows user.",
                false,
            ),
            Self::MergeConflict => AppError::new(
                ErrorCode::MergeConflict,
                "The selected profiles cannot be merged with this preview.",
                false,
            ),
            Self::MigrationFailed => AppError::new(
                ErrorCode::MigrationFailed,
                "The notebook upgrade failed. Rollback status: restored or recovery required.",
                false,
            ),
            Self::NoActiveEncounter => AppError::new(
                ErrorCode::NoActiveEncounter,
                "Select or confirm an active encounter before capturing a note.",
                false,
            ),
            Self::NotebookInvalid => AppError::new(
                ErrorCode::NotebookInvalid,
                "The encrypted notebook could not be validated.",
                false,
            ),
            Self::NotFound => AppError::new(
                ErrorCode::NotFound,
                "The requested record was not found.",
                false,
            ),
            Self::OperationBusy => AppError::new(
                ErrorCode::OperationBusy,
                "A conflicting notebook operation is already running.",
                true,
            ),
            Self::OcrLanguageMissing => AppError::new(
                ErrorCode::OcrLanguageMissing,
                "The configured Windows OCR language is unavailable. UI Automation and manual entry remain available.",
                false,
            ),
            Self::OverlayUnavailable => AppError::new(
                ErrorCode::OverlayUnavailable,
                "The overlay could not be shown above the selected window. Continue in the main window.",
                true,
            ),
            Self::PlaintextAcknowledgementRequired => AppError::new(
                ErrorCode::AcknowledgementRequired,
                "Acknowledge that the exported text file is unencrypted.",
                false,
            )
            .with_field("plaintextAcknowledged"),
            Self::ProviderInvalidResponse => AppError::new(
                ErrorCode::ProviderInvalidResponse,
                "The official deck result is malformed, oversized, or incompatible.",
                false,
            ),
            Self::ProviderUnavailable => AppError::new(
                ErrorCode::ProviderUnavailable,
                "Official deck enrichment is temporarily unavailable. Continue manually.",
                true,
            ),
            Self::RevisionConflict => AppError::new(
                ErrorCode::RevisionConflict,
                "The record changed after this view was loaded.",
                true,
            ),
            Self::SaveFailed => AppError::new(
                ErrorCode::SaveFailed,
                "The notebook change could not be saved. Your input is preserved.",
                true,
            ),
            Self::ScopeMismatch => AppError::new(
                ErrorCode::ScopeMismatch,
                "The deletion scope changed after confirmation was shown.",
                false,
            ),
            Self::StaleProviderResult => AppError::new(
                ErrorCode::StaleProviderResult,
                "This official result no longer matches the active encounter and format.",
                false,
            ),
            Self::UndoExpired => {
                AppError::new(ErrorCode::UndoExpired, "The undo period has ended.", false)
            }
            Self::UnsafeSource => AppError::new(
                ErrorCode::UnsafeSource,
                "Only HTTPS links on the official MTGO host are accepted.",
                false,
            )
            .with_field("sourceUrl"),
            Self::WindowNotFound => AppError::new(
                ErrorCode::WindowNotFound,
                "The selected MTGO window is no longer visible. Select it again or continue manually.",
                true,
            ),
            Self::WrongPassphrase => AppError::new(
                ErrorCode::WrongPassphrase,
                "The backup passphrase is incorrect.",
                false,
            )
            .with_field("passphrase"),
        }
    }
}
