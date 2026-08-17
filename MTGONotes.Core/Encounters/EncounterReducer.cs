using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Encounters;

public enum EvidenceSource
{
    TrustedUia,
    Ocr,
    Manual,
    System,
    Mtgosdk,
}

public abstract record EvidenceKind
{
    public sealed record ConfirmedOpponent(EntityId ProfileId, EntityId EncounterId) : EvidenceKind;

    public sealed record TrustedPhase(InternalPhase Phase, ulong StableForMs) : EvidenceKind;

    public sealed record UnknownPossibleGameplay : EvidenceKind;

    public sealed record StrongGameplay : EvidenceKind;

    public sealed record End : EvidenceKind;

    public sealed record CompletionIgnored : EvidenceKind;

    public sealed record Reopen(EntityId EncounterId) : EvidenceKind;
}

public sealed record ContextEvidence(
    string ProviderSession,
    ulong Generation,
    ulong Sequence,
    ulong MonotonicMs,
    EvidenceSource Source,
    EvidenceKind Evidence);

public sealed record ActiveEncounter(
    EntityId Id,
    EntityId ProfileId,
    EncounterStatus Status,
    bool UnconfirmedDeckPresent);

public sealed class EncounterRuntime
{
    public required string ProviderSession { get; set; }

    public ulong Generation { get; set; }

    public ulong LastSequence { get; set; }

    public InternalPhase Phase { get; set; }

    public ActiveEncounter? Active { get; set; }

    public static EncounterRuntime Idle(string providerSession) =>
        new()
        {
            ProviderSession = providerSession,
            Generation = 0,
            LastSequence = 0,
            Phase = InternalPhase.Idle,
            Active = null,
        };

    public static EncounterRuntime Recover(
        string providerSession,
        ulong generation,
        ActiveEncounter active) =>
        new()
        {
            ProviderSession = providerSession,
            Generation = generation,
            LastSequence = 0,
            Phase = InternalPhase.InGameRestricted,
            Active = active,
        };

    public EncounterRuntime Clone() =>
        new()
        {
            ProviderSession = ProviderSession,
            Generation = Generation,
            LastSequence = LastSequence,
            Phase = Phase,
            Active = Active,
        };
}

public abstract record EncounterAction
{
    public sealed record ResolveProfile(EntityId ProfileId) : EncounterAction;

    public sealed record StartEncounter(
        EntityId EncounterId,
        EntityId ProfileId,
        EntityId? UndoGroup) : EncounterAction;

    public sealed record FinishEncounter(EntityId EncounterId, EntityId? UndoGroup) : EncounterAction;

    public sealed record ChangePhase(EntityId EncounterId, InternalPhase From, InternalPhase To)
        : EncounterAction;

    public sealed record MarkIncomplete(EntityId EncounterId, bool ExcludedUnconfirmedDeck)
        : EncounterAction;

    public sealed record OpenHistoricalEditor(EntityId EncounterId) : EncounterAction;
}

public sealed record Reduction(EncounterRuntime Runtime, IReadOnlyList<EncounterAction> Actions);

public sealed class EncounterReducer
{
    public const ulong OcrStableDurationMs = 1_500;

    public Result<Reduction> Reduce(EncounterRuntime current, ContextEvidence evidence)
    {
        if (evidence.ProviderSession != current.ProviderSession
            || evidence.Generation < current.Generation
            || (evidence.Generation == current.Generation
                && evidence.Sequence <= current.LastSequence))
        {
            return Result<Reduction>.Ok(new Reduction(current.Clone(), []));
        }

        var startsGeneration = evidence.Evidence is EvidenceKind.ConfirmedOpponent;
        if (evidence.Generation > current.Generation && !startsGeneration)
        {
            return Result<Reduction>.Ok(new Reduction(current.Clone(), []));
        }

        var runtime = current.Clone();
        runtime.LastSequence = evidence.Sequence;
        var actions = new List<EncounterAction>();

        switch (evidence.Evidence)
        {
            case EvidenceKind.ConfirmedOpponent confirmed:
                if (runtime.Active?.ProfileId.Equals(confirmed.ProfileId) == true)
                {
                    runtime.Generation = evidence.Generation;
                    return Result<Reduction>.Ok(new Reduction(runtime, actions));
                }

                EntityId? undoGroup = runtime.Active is null ? null : EntityId.New();
                if (runtime.Active is { } previous)
                {
                    actions.Add(new EncounterAction.FinishEncounter(previous.Id, undoGroup));
                    runtime.Active = null;
                }

                actions.Add(new EncounterAction.ResolveProfile(confirmed.ProfileId));
                actions.Add(
                    new EncounterAction.StartEncounter(
                        confirmed.EncounterId,
                        confirmed.ProfileId,
                        undoGroup));
                runtime.Generation = evidence.Generation;
                runtime.Phase = InternalPhase.PreMatch;
                runtime.Active = new ActiveEncounter(
                    confirmed.EncounterId,
                    confirmed.ProfileId,
                    EncounterStatus.Active,
                    false);
                break;

            case EvidenceKind.UnknownPossibleGameplay:
            case EvidenceKind.StrongGameplay:
            {
                var changed = ChangePhase(runtime, InternalPhase.InGameRestricted, actions);
                if (!changed.IsSuccess)
                {
                    return Result<Reduction>.Fail(changed.Error!.Value);
                }

                break;
            }

            case EvidenceKind.TrustedPhase trusted:
                if (runtime.Phase == InternalPhase.InGameRestricted
                    && evidence.Source == EvidenceSource.Ocr
                    && trusted.StableForMs < OcrStableDurationMs)
                {
                    return Result<Reduction>.Ok(new Reduction(runtime, actions));
                }

                {
                    var changed = ChangePhase(runtime, trusted.Phase, actions);
                    if (!changed.IsSuccess)
                    {
                        return Result<Reduction>.Fail(changed.Error!.Value);
                    }
                }

                break;

            case EvidenceKind.End:
                if (runtime.Active is not { } ending)
                {
                    return runtime.Phase == InternalPhase.Finished
                        ? Result<Reduction>.Ok(new Reduction(runtime, actions))
                        : Result<Reduction>.Fail(RepoError.InvalidTransition);
                }

                runtime.Active = null;
                actions.Add(new EncounterAction.FinishEncounter(ending.Id, null));
                runtime.Phase = InternalPhase.Finished;
                break;

            case EvidenceKind.CompletionIgnored:
                if (runtime.Active is not { } ignored)
                {
                    return Result<Reduction>.Fail(RepoError.InvalidTransition);
                }

                runtime.Active = null;
                actions.Add(
                    new EncounterAction.MarkIncomplete(ignored.Id, ignored.UnconfirmedDeckPresent));
                runtime.Phase = InternalPhase.Incomplete;
                break;

            case EvidenceKind.Reopen reopen:
                actions.Add(new EncounterAction.OpenHistoricalEditor(reopen.EncounterId));
                break;
        }

        return Result<Reduction>.Ok(new Reduction(runtime, actions));
    }

    private static Result ChangePhase(
        EncounterRuntime runtime,
        InternalPhase phase,
        List<EncounterAction> actions)
    {
        if (runtime.Active is not { } active)
        {
            return Result.Fail(RepoError.InvalidTransition);
        }

        if (runtime.Phase == phase)
        {
            return Result.Ok();
        }

        actions.Add(new EncounterAction.ChangePhase(active.Id, runtime.Phase, phase));
        runtime.Phase = phase;
        return Result.Ok();
    }
}
