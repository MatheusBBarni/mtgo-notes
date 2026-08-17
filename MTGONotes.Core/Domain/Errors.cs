namespace MTGONotes.Core.Domain;

public enum RepoError
{
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

public enum ErrorCode
{
    AcknowledgementRequired,
    AlreadyOpen,
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
    InternalError,
}

public sealed record AppError(ErrorCode Code, string Message, bool Retryable, string? Field = null);

public sealed class DomainException : Exception
{
    public DomainException(RepoError error)
        : base(error.ToString())
    {
        Error = error;
    }

    public RepoError Error { get; }
}

public static class RepoErrorMapping
{
    public static bool IsRetryableProviderError(this RepoError error) =>
        error == RepoError.ProviderUnavailable;

    public static AppError ToAppError(this RepoError error) =>
        error switch
        {
            RepoError.AcknowledgementRequired => new AppError(
                ErrorCode.AcknowledgementRequired,
                "Acknowledge that a forgotten backup passphrase cannot be recovered.",
                false,
                "passphraseAcknowledged"),
            RepoError.AlreadyOpen => new AppError(
                ErrorCode.AlreadyOpen,
                "Quick capture is already open.",
                false),
            RepoError.EmptyPortabilityConfirmationRequired => new AppError(
                ErrorCode.AcknowledgementRequired,
                "Confirm that the selected scope contains no active profiles.",
                false,
                "confirmEmpty"),
            RepoError.AssetsInvalid => new AppError(
                ErrorCode.AssetsInvalid,
                "Classifier assets failed signature, schema, or digest validation.",
                false),
            RepoError.BlankObservation => new AppError(
                ErrorCode.BlankObservation,
                "An observation requires non-whitespace text.",
                false,
                "text"),
            RepoError.CandidateStale => new AppError(
                ErrorCode.CandidateStale,
                "The opponent candidate was replaced by newer evidence.",
                false),
            RepoError.CancelUnsafe => new AppError(
                ErrorCode.CancelUnsafe,
                "This operation passed its safe cancellation point and must finish recovery.",
                false),
            RepoError.ConsentRequired => new AppError(
                ErrorCode.ConsentRequired,
                "Review and grant official deck provider consent before lookup.",
                false),
            RepoError.DeckIncomplete => new AppError(
                ErrorCode.DeckIncomplete,
                "A complete decklist is required for classification.",
                false),
            RepoError.DestinationUnwritable => new AppError(
                ErrorCode.DestinationUnwritable,
                "The selected destination cannot be written.",
                false,
                "destination"),
            RepoError.DisclosureRestricted => new AppError(
                ErrorCode.DisclosureRestricted,
                "Historical notebook data is unavailable during possible gameplay.",
                false),
            RepoError.ExplicitCorrectionRequired => new AppError(
                ErrorCode.ExplicitCorrectionRequired,
                "An encounter is already active. Use the explicit opponent correction flow.",
                false),
            RepoError.FormatUnsupported => new AppError(
                ErrorCode.FormatUnsupported,
                "This format has no bundled classifier coverage.",
                false),
            RepoError.IdentityConflict => new AppError(
                ErrorCode.IdentityConflict,
                "That handle is already assigned to another active profile.",
                false),
            RepoError.InvalidCard => new AppError(
                ErrorCode.InvalidCard,
                "A card entry has an invalid name, quantity, or certainty.",
                false,
                "cards"),
            RepoError.InvalidCursor => new AppError(
                ErrorCode.InvalidCursor,
                "The page cursor is invalid.",
                false,
                "cursor"),
            RepoError.InvalidHandle => new AppError(
                ErrorCode.InvalidHandle,
                "Enter a valid opponent handle.",
                false,
                "handle"),
            RepoError.InvalidTag => new AppError(
                ErrorCode.InvalidRequest,
                "A tendency tag has an invalid label.",
                false,
                "tags"),
            RepoError.InvalidBackup => new AppError(
                ErrorCode.InvalidBackup,
                "The backup is malformed, damaged, or incompatible.",
                false),
            RepoError.InvalidRequest => new AppError(
                ErrorCode.InvalidRequest,
                "The request is invalid.",
                false),
            RepoError.InvalidTransition => new AppError(
                ErrorCode.InvalidTransition,
                "The encounter transition is not valid.",
                false),
            RepoError.JobBusy => new AppError(
                ErrorCode.JobBusy,
                "A classifier job is already running.",
                true),
            RepoError.KeyUnavailable => new AppError(
                ErrorCode.KeyUnavailable,
                "The notebook key is unavailable for this Windows user.",
                false),
            RepoError.MergeConflict => new AppError(
                ErrorCode.MergeConflict,
                "The selected profiles cannot be merged with this preview.",
                false),
            RepoError.MigrationFailed => new AppError(
                ErrorCode.MigrationFailed,
                "The notebook upgrade failed. Rollback status: restored or recovery required.",
                false),
            RepoError.NoActiveEncounter => new AppError(
                ErrorCode.NoActiveEncounter,
                "Select or confirm an active encounter before capturing a note.",
                false),
            RepoError.NotebookInvalid => new AppError(
                ErrorCode.NotebookInvalid,
                "The encrypted notebook could not be validated.",
                false),
            RepoError.NotFound => new AppError(
                ErrorCode.NotFound,
                "The requested record was not found.",
                false),
            RepoError.OperationBusy => new AppError(
                ErrorCode.OperationBusy,
                "A conflicting notebook operation is already running.",
                true),
            RepoError.OcrLanguageMissing => new AppError(
                ErrorCode.OcrLanguageMissing,
                "The configured Windows OCR language is unavailable. UI Automation and manual entry remain available.",
                false),
            RepoError.OverlayUnavailable => new AppError(
                ErrorCode.OverlayUnavailable,
                "The overlay could not be shown above the selected window. Continue in the main window.",
                true),
            RepoError.PlaintextAcknowledgementRequired => new AppError(
                ErrorCode.AcknowledgementRequired,
                "Acknowledge that the exported text file is unencrypted.",
                false,
                "plaintextAcknowledged"),
            RepoError.ProviderInvalidResponse => new AppError(
                ErrorCode.ProviderInvalidResponse,
                "The official deck result is malformed, oversized, or incompatible.",
                false),
            RepoError.ProviderUnavailable => new AppError(
                ErrorCode.ProviderUnavailable,
                "Official deck enrichment is temporarily unavailable. Continue manually.",
                true),
            RepoError.RevisionConflict => new AppError(
                ErrorCode.RevisionConflict,
                "The record changed after this view was loaded.",
                true),
            RepoError.SaveFailed => new AppError(
                ErrorCode.SaveFailed,
                "The notebook change could not be saved. Your input is preserved.",
                true),
            RepoError.ScopeMismatch => new AppError(
                ErrorCode.ScopeMismatch,
                "The deletion scope changed after confirmation was shown.",
                false),
            RepoError.StaleProviderResult => new AppError(
                ErrorCode.StaleProviderResult,
                "This official result no longer matches the active encounter and format.",
                false),
            RepoError.UndoExpired => new AppError(
                ErrorCode.UndoExpired,
                "The undo period has ended.",
                false),
            RepoError.UnsafeSource => new AppError(
                ErrorCode.UnsafeSource,
                "Only HTTPS links on the official MTGO host are accepted.",
                false,
                "sourceUrl"),
            RepoError.WindowNotFound => new AppError(
                ErrorCode.WindowNotFound,
                "The selected MTGO window is no longer visible. Select it again or continue manually.",
                true),
            RepoError.WrongPassphrase => new AppError(
                ErrorCode.WrongPassphrase,
                "The backup passphrase is incorrect.",
                false,
                "passphrase"),
            _ => new AppError(ErrorCode.InternalError, "An unexpected error occurred.", false),
        };
}
