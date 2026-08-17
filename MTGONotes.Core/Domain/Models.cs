namespace MTGONotes.Core.Domain;

public enum InternalPhase
{
    Idle,
    Candidate,
    PreMatch,
    InGameRestricted,
    BetweenGames,
    CompletionPending,
    Finished,
    Incomplete,
}

public static class InternalPhaseExtensions
{
    public static bool IsDisclosureRestricted(this InternalPhase phase) =>
        phase is InternalPhase.Candidate
            or InternalPhase.InGameRestricted
            or InternalPhase.CompletionPending
            or InternalPhase.Incomplete;
}

public enum EncounterStatus
{
    Active,
    Finished,
    Incomplete,
    Deleted,
}

public enum CardCertainty
{
    Observed,
    Suspected,
}

public sealed record OpponentProfile(
    EntityId Id,
    string PrimaryHandle,
    string NormalizedHandle,
    UtcMillis CreatedAt,
    Revision Revision,
    UtcMillis? DeletedAt);

public sealed record OpponentAlias(
    EntityId Id,
    EntityId ProfileId,
    string DisplayHandle,
    string NormalizedHandle,
    string Provenance,
    long? MtgoUserId = null);

public sealed record Encounter(
    EntityId Id,
    EntityId ProfileId,
    string Format,
    UtcMillis StartedAt,
    UtcMillis? EndedAt,
    EncounterStatus Status,
    InternalPhase Phase,
    string Source,
    Revision Revision,
    string? IncompleteReason);

public sealed record Observation(
    EntityId Id,
    EntityId EncounterId,
    string Text,
    UtcMillis CreatedAt,
    UtcMillis? EditedAt,
    Revision Revision,
    UtcMillis? DeletionDeadline,
    UtcMillis? DeletedAt);
