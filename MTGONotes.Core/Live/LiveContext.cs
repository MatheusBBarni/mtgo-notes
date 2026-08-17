using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Live;

public enum LiveMatchSignal
{
    None,
    Pairings,
    InGame,
    Sideboarding,
    Finished,
    Unknown,
}

public sealed record OpponentCandidate(
    string DisplayHandle,
    string NormalizedHandle,
    long? MtgoUserId,
    ulong Generation,
    ulong Sequence,
    string ProviderSession);

public sealed record LiveSnapshot(
    string ProviderSession,
    ulong Generation,
    ulong Sequence,
    ulong MonotonicMs,
    OpponentCandidate? Opponent,
    InternalPhase SuggestedPhase,
    string? Format,
    int? GameNumber,
    string? Result);

public interface IContextSource
{
    string Id { get; }

    event EventHandler<LiveSnapshot>? SnapshotChanged;

    Task StartAsync(CancellationToken cancellationToken);

    Task StopAsync();
}

public static class PhaseMapper
{
    public static InternalPhase FromLive(LiveMatchSignal signal) =>
        signal switch
        {
            LiveMatchSignal.None => InternalPhase.Idle,
            LiveMatchSignal.Pairings => InternalPhase.PreMatch,
            LiveMatchSignal.InGame => InternalPhase.InGameRestricted,
            LiveMatchSignal.Sideboarding => InternalPhase.BetweenGames,
            LiveMatchSignal.Finished => InternalPhase.CompletionPending,
            _ => InternalPhase.InGameRestricted,
        };

    public static InternalPhase FromVisibleText(string value) =>
        value.Trim().ToLowerInvariant() switch
        {
            "sideboarding" or "between games" => InternalPhase.BetweenGames,
            "match complete" or "results" => InternalPhase.CompletionPending,
            "pairings" or "game starting" => InternalPhase.PreMatch,
            _ => InternalPhase.InGameRestricted,
        };
}
