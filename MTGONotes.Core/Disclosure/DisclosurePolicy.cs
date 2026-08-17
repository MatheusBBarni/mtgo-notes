using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Disclosure;

public enum QueryKind
{
    SearchHistory,
    GetProfile,
    GetEncounter,
    GetDeckDetails,
}

public sealed record ObservationView(string Id, string Text, bool Editable);

public sealed record PublicSnapshotView(
    string Label,
    string Format,
    long PublishedAt,
    string SourceText,
    bool Available);

public sealed record NotebookState(
    InternalPhase Phase,
    string? ConfirmedHandle,
    bool ActiveProfileDeleted,
    IReadOnlyList<ObservationView> CurrentObservations,
    IReadOnlyList<ObservationView> HistoricalObservations,
    PublicSnapshotView? PublicSnapshot);

public sealed record OverlayView(
    InternalPhase Phase,
    string? ConfirmedHandle,
    IReadOnlyList<ObservationView> CurrentObservations,
    IReadOnlyList<ObservationView> HistoricalObservations,
    PublicSnapshotView? PublicSnapshot,
    bool HistoryEditable,
    bool NeedsIdentityResolution);

public abstract record DisclosureEmission
{
    public sealed record Replacement(OverlayView View) : DisclosureEmission;

    public sealed record Notification(string Message) : DisclosureEmission;
}

public sealed class DisclosurePolicy
{
    public static OverlayView Neutral { get; } =
        new(
            InternalPhase.Idle,
            null,
            [],
            [],
            null,
            false,
            false);

    public Result Authorize(QueryKind query, InternalPhase phase)
    {
        _ = query;
        return phase.IsDisclosureRestricted()
            ? Result.Fail(RepoError.DisclosureRestricted)
            : Result.Ok();
    }

    public OverlayView Overlay(NotebookState state)
    {
        if (state.ActiveProfileDeleted)
        {
            return new OverlayView(
                state.Phase,
                null,
                [],
                [],
                null,
                false,
                true);
        }

        if (state.ConfirmedHandle is null)
        {
            return new OverlayView(
                state.Phase,
                null,
                [],
                [],
                null,
                false,
                false);
        }

        var restricted = state.Phase.IsDisclosureRestricted();
        var finished = state.Phase == InternalPhase.Finished;
        return new OverlayView(
            state.Phase,
            state.ConfirmedHandle,
            state.CurrentObservations.Select(observation => observation with { Editable = finished }).ToArray(),
            restricted
                ? []
                : state.HistoricalObservations.Select(observation => observation with { Editable = finished }).ToArray(),
            restricted ? null : Sanitize(state.PublicSnapshot),
            finished,
            false);
    }

    public IReadOnlyList<DisclosureEmission> TransitionEmissions(
        NotebookState state,
        string notification) =>
        [
            new DisclosureEmission.Replacement(Overlay(state)),
            new DisclosureEmission.Notification(notification),
        ];

    private static PublicSnapshotView? Sanitize(PublicSnapshotView? snapshot)
    {
        if (snapshot is null)
        {
            return null;
        }

        if (snapshot.SourceText.Contains('<') || snapshot.SourceText.Contains('>'))
        {
            return snapshot with { SourceText = "Source unavailable", Available = false };
        }

        return snapshot;
    }
}

public readonly record struct Result
{
    private Result(RepoError? error)
    {
        Error = error;
    }

    public RepoError? Error { get; }

    public bool IsSuccess => Error is null;

    public static Result Ok() => new(null);

    public static Result Fail(RepoError error) => new(error);
}

public readonly record struct Result<T>
{
    private Result(T? value, RepoError? error)
    {
        Value = value;
        Error = error;
    }

    public T? Value { get; }

    public RepoError? Error { get; }

    public bool IsSuccess => Error is null;

    public static Result<T> Ok(T value) => new(value, null);

    public static Result<T> Fail(RepoError error) => new(default, error);
}
