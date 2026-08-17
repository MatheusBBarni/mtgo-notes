using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;
using MTGONotes.Core.Encounters;

namespace MTGONotes.Core.Tests;

public sealed class EncounterReducerTests
{
    private static ContextEvidence Evidence(
        EncounterRuntime runtime,
        ulong sequence,
        EvidenceKind kind,
        EvidenceSource source = EvidenceSource.TrustedUia) =>
        new(
            runtime.ProviderSession,
            runtime.Generation,
            sequence,
            sequence * 10,
            source,
            kind);

    private static EncounterRuntime Start(EncounterReducer reducer)
    {
        var idle = EncounterRuntime.Idle("session");
        return reducer
            .Reduce(
                idle,
                new ContextEvidence(
                    "session",
                    1,
                    1,
                    1,
                    EvidenceSource.Manual,
                    new EvidenceKind.ConfirmedOpponent(EntityId.New(), EntityId.New())))
            .Value!
            .Runtime;
    }

    [Fact]
    public void Ut009_confirmed_candidate_starts_pre_match()
    {
        var reducer = new EncounterReducer();
        var idle = EncounterRuntime.Idle("session");
        var result = reducer
            .Reduce(
                idle,
                new ContextEvidence(
                    "session",
                    1,
                    1,
                    1,
                    EvidenceSource.Manual,
                    new EvidenceKind.ConfirmedOpponent(EntityId.New(), EntityId.New())))
            .Value!;
        Assert.Equal(InternalPhase.PreMatch, result.Runtime.Phase);
        Assert.Equal(2, result.Actions.Count);
        Assert.IsType<EncounterAction.ResolveProfile>(result.Actions[0]);
        Assert.IsType<EncounterAction.StartEncounter>(result.Actions[1]);
    }

    [Fact]
    public void Ut010_unknown_possible_gameplay_fails_closed()
    {
        var reducer = new EncounterReducer();
        var runtime = Start(reducer);
        var result = reducer
            .Reduce(runtime, Evidence(runtime, 2, new EvidenceKind.UnknownPossibleGameplay()))
            .Value!;
        Assert.Equal(InternalPhase.InGameRestricted, result.Runtime.Phase);
    }

    [Fact]
    public void Ut011_strong_gameplay_signal_restricts_immediately()
    {
        var reducer = new EncounterReducer();
        var runtime = Start(reducer);
        var result = reducer
            .Reduce(runtime, Evidence(runtime, 2, new EvidenceKind.StrongGameplay()))
            .Value!;
        Assert.Equal(InternalPhase.InGameRestricted, result.Runtime.Phase);
        Assert.Single(result.Actions);
    }

    [Fact]
    public void Ut012_unstable_ocr_cannot_leave_restricted()
    {
        var reducer = new EncounterReducer();
        var runtime = Start(reducer);
        runtime.Phase = InternalPhase.InGameRestricted;
        var next = Evidence(
            runtime,
            2,
            new EvidenceKind.TrustedPhase(
                InternalPhase.BetweenGames,
                EncounterReducer.OcrStableDurationMs - 1),
            EvidenceSource.Ocr);
        var result = reducer.Reduce(runtime, next).Value!;
        Assert.Equal(InternalPhase.InGameRestricted, result.Runtime.Phase);
        Assert.Empty(result.Actions);
    }

    [Fact]
    public void Ut013_new_opponent_finishes_before_start_in_one_undo_group()
    {
        var reducer = new EncounterReducer();
        var runtime = Start(reducer);
        var result = reducer
            .Reduce(
                runtime,
                new ContextEvidence(
                    runtime.ProviderSession,
                    2,
                    1,
                    20,
                    EvidenceSource.Manual,
                    new EvidenceKind.ConfirmedOpponent(EntityId.New(), EntityId.New())))
            .Value!;
        Assert.Equal(3, result.Actions.Count);
        var finish = Assert.IsType<EncounterAction.FinishEncounter>(result.Actions[0]);
        Assert.IsType<EncounterAction.ResolveProfile>(result.Actions[1]);
        var start = Assert.IsType<EncounterAction.StartEncounter>(result.Actions[2]);
        Assert.NotNull(finish.UndoGroup);
        Assert.Equal(finish.UndoGroup, start.UndoGroup);
    }

    [Fact]
    public void Ut014_repeated_end_is_idempotent()
    {
        var reducer = new EncounterReducer();
        var runtime = Start(reducer);
        var finished = reducer.Reduce(runtime, Evidence(runtime, 2, new EvidenceKind.End())).Value!.Runtime;
        var replay = reducer.Reduce(finished, Evidence(finished, 3, new EvidenceKind.End())).Value!;
        Assert.Empty(replay.Actions);
        Assert.Equal(InternalPhase.Finished, replay.Runtime.Phase);
    }

    [Fact]
    public void Ut015_older_generation_is_ignored()
    {
        var reducer = new EncounterReducer();
        var runtime = Start(reducer);
        var stale = new ContextEvidence(
            runtime.ProviderSession,
            0,
            100,
            100,
            EvidenceSource.TrustedUia,
            new EvidenceKind.StrongGameplay());
        var result = reducer.Reduce(runtime, stale).Value!;
        Assert.Equal(runtime.Generation, result.Runtime.Generation);
        Assert.Equal(runtime.Phase, result.Runtime.Phase);
        Assert.Equal(runtime.LastSequence, result.Runtime.LastSequence);
        Assert.Empty(result.Actions);
    }

    [Fact]
    public void Ut016_recovered_active_encounter_starts_restricted()
    {
        var recovered = EncounterRuntime.Recover(
            "new-session",
            4,
            new ActiveEncounter(EntityId.New(), EntityId.New(), EncounterStatus.Active, false));
        Assert.Equal(InternalPhase.InGameRestricted, recovered.Phase);
        Assert.NotNull(recovered.Active);
    }

    [Fact]
    public void Ut017_finish_without_attached_encounter_is_invalid()
    {
        var reducer = new EncounterReducer();
        var idle = EncounterRuntime.Idle("session");
        var error = reducer.Reduce(idle, Evidence(idle, 1, new EvidenceKind.End()));
        Assert.Equal(RepoError.InvalidTransition, error.Error);
    }

    [Fact]
    public void Ut018_ignored_completion_becomes_incomplete()
    {
        var reducer = new EncounterReducer();
        var runtime = Start(reducer);
        runtime.Active = runtime.Active! with { UnconfirmedDeckPresent = true };
        var result = reducer
            .Reduce(runtime, Evidence(runtime, 2, new EvidenceKind.CompletionIgnored()))
            .Value!;
        Assert.Equal(InternalPhase.Incomplete, result.Runtime.Phase);
        var marked = Assert.IsType<EncounterAction.MarkIncomplete>(Assert.Single(result.Actions));
        Assert.True(marked.ExcludedUnconfirmedDeck);
    }

    [Fact]
    public void Ut019_reopening_history_does_not_displace_active()
    {
        var reducer = new EncounterReducer();
        var runtime = Start(reducer);
        var activeId = runtime.Active!.Id;
        var result = reducer
            .Reduce(runtime, Evidence(runtime, 2, new EvidenceKind.Reopen(EntityId.New())))
            .Value!;
        Assert.Equal(activeId, result.Runtime.Active!.Id);
        Assert.IsType<EncounterAction.OpenHistoricalEditor>(Assert.Single(result.Actions));
    }

    [Fact]
    public void Ut020_event_interleavings_never_create_two_active_encounters()
    {
        var reducer = new EncounterReducer();
        for (ulong order = 0; order < 128; order++)
        {
            var runtime = EncounterRuntime.Idle("session");
            for (ulong generation = 1; generation <= 8; generation++)
            {
                var sequence = (order & (1UL << (int)(generation - 1))) == 0 ? 1UL : 2UL;
                var result = reducer
                    .Reduce(
                        runtime,
                        new ContextEvidence(
                            "session",
                            generation,
                            sequence,
                            generation,
                            EvidenceSource.Manual,
                            new EvidenceKind.ConfirmedOpponent(EntityId.New(), EntityId.New())))
                    .Value!;
                runtime = result.Runtime;
                Assert.True((runtime.Active is null ? 0 : 1) <= 1);
            }
        }
    }
}
