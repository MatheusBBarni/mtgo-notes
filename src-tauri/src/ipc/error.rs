use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    AcknowledgementRequired,
    AlreadyOpen,
    AssetsInvalid,
    BlankObservation,
    CancelUnsafe,
    CandidateStale,
    ConsentRequired,
    DeckIncomplete,
    DisclosureRestricted,
    DestinationUnwritable,
    ExplicitCorrectionRequired,
    FormatUnsupported,
    IdentityConflict,
    InternalError,
    InputTooLong,
    InteractiveRequired,
    InvalidBackup,
    InvalidCard,
    InvalidCursor,
    InvalidHandle,
    InvalidTransition,
    InvalidRequest,
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
    ProviderInvalidResponse,
    ProviderUnavailable,
    RedactionFailed,
    RevisionConflict,
    SaveFailed,
    ScopeMismatch,
    SignatureInvalid,
    StaleProviderResult,
    UndoExpired,
    UnauthorizedCaller,
    UnsafeSource,
    UpdateUnavailable,
    WindowNotFound,
    WrongPassphrase,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
            field: None,
        }
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn internal(correlation_code: &str) -> Self {
        Self::new(
            ErrorCode::InternalError,
            format!("An internal error occurred. Reference: {correlation_code}"),
            false,
        )
    }
}
