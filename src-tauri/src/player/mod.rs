pub mod census;
pub mod classification;
pub mod deletion;
pub mod evidence;
pub mod models;
pub mod repository;
pub mod routes;
pub mod runtime;
pub mod service;

pub use classification::{
    PlayerClassificationEligibility, PlayerClassificationOutcome, PlayerClassificationReason,
    PlayerClassificationService, classification_eligibility, is_classification_eligible,
};
pub use deletion::{
    PlayerDeletionCounts, PlayerDeletionOutcome, PlayerDeletionPreview, PlayerDeletionService,
    PlayerDeletionTarget,
};
pub use evidence::{
    ManualEvidenceInput, ManualEvidencePreview, approved_fields, default_selected_fields,
    manual_preview, select_preview_fields, validate_manual_evidence,
};
pub use models::*;
pub use repository::{
    AppendSelectionInput, EmptyOutcomeInput, EvidencePage, ImportOutcome, PlayerStore,
    ReceiptReplay, SaveIdentityInput, VerifiedImportBatch,
};
pub use runtime::{
    CandidatePreview, EmptyLookupResult, LookupOutcome, LookupSession, ManualPreviewBinding,
    PlayerError, PlayerErrorCode, PlayerProviderStatus, PlayerPublicResultsRuntime, PlayerRecovery,
    ProviderAvailability, authorize_command,
};
pub use service::PlayerPublicResultsService;

#[cfg(test)]
mod tests;
