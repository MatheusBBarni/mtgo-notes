using System.Text.Json;
using MTGONotes.Core;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Tests;

public sealed class DisclosurePolicyTests
{
    private static ObservationView Observation(string id) =>
        new(id, $"note-{id}", false);

    private static NotebookState State(InternalPhase phase) =>
        new(
            phase,
            "Opponent_42",
            false,
            [Observation("current")],
            [Observation("history")],
            new PublicSnapshotView(
                "Deck label",
                "Modern",
                1_753_689_600_000,
                "Official MTGO",
                true));

    [Fact]
    public void Ut021_pre_match_includes_permitted_context()
    {
        var view = new DisclosurePolicy().Overlay(State(InternalPhase.PreMatch));
        Assert.Equal("Opponent_42", view.ConfirmedHandle);
        Assert.Single(view.HistoricalObservations);
        Assert.NotNull(view.PublicSnapshot);
    }

    [Fact]
    public void Ut022_in_game_contains_only_identity_and_current_observations()
    {
        var view = new DisclosurePolicy().Overlay(State(InternalPhase.InGameRestricted));
        Assert.NotNull(view.ConfirmedHandle);
        Assert.Single(view.CurrentObservations);
        Assert.Empty(view.HistoricalObservations);
        Assert.Null(view.PublicSnapshot);
    }

    [Fact]
    public void Ut023_incomplete_possible_gameplay_is_restricted()
    {
        var view = new DisclosurePolicy().Overlay(State(InternalPhase.Incomplete));
        Assert.Empty(view.HistoricalObservations);
        Assert.Null(view.PublicSnapshot);
    }

    [Fact]
    public void Ut024_finished_projection_allows_full_editing()
    {
        var view = new DisclosurePolicy().Overlay(State(InternalPhase.Finished));
        Assert.True(view.HistoryEditable);
        Assert.True(view.CurrentObservations[0].Editable);
        Assert.True(view.HistoricalObservations[0].Editable);
    }

    [Fact]
    public void Ut025_search_is_denied_during_gameplay()
    {
        var result = new DisclosurePolicy().Authorize(
            QueryKind.SearchHistory,
            InternalPhase.InGameRestricted);
        Assert.False(result.IsSuccess);
        Assert.Equal(RepoError.DisclosureRestricted, result.Error);
        Assert.Equal(ErrorCode.DisclosureRestricted, result.Error!.Value.ToAppError().Code);
    }

    [Fact]
    public void Ut026_unconfirmed_opponent_exposes_no_history_or_external_data()
    {
        var notebook = State(InternalPhase.PreMatch) with { ConfirmedHandle = null };
        var view = new DisclosurePolicy().Overlay(notebook);
        Assert.Null(view.ConfirmedHandle);
        Assert.Empty(view.CurrentObservations);
        Assert.Empty(view.HistoricalObservations);
        Assert.Null(view.PublicSnapshot);
    }

    [Fact]
    public void Ut027_deleted_active_profile_clears_stale_context()
    {
        var notebook = State(InternalPhase.BetweenGames) with { ActiveProfileDeleted = true };
        var view = new DisclosurePolicy().Overlay(notebook);
        Assert.True(view.NeedsIdentityResolution);
        Assert.Null(view.ConfirmedHandle);
        Assert.Empty(view.HistoricalObservations);
    }

    [Fact]
    public void Ut028_restricted_replacement_precedes_notification()
    {
        var emissions = new DisclosurePolicy().TransitionEmissions(
            State(InternalPhase.InGameRestricted),
            "phase changed");
        var replacement = Assert.IsType<DisclosureEmission.Replacement>(emissions[0]);
        Assert.Empty(replacement.View.HistoricalObservations);
        Assert.Null(replacement.View.PublicSnapshot);
        Assert.IsType<DisclosureEmission.Notification>(emissions[1]);
    }

    [Fact]
    public void Ut029_equivalent_states_serialize_byte_equivalently()
    {
        var policy = new DisclosurePolicy();
        var first = JsonSerializer.SerializeToUtf8Bytes(
            policy.Overlay(State(InternalPhase.PreMatch)),
            JsonUtil.Options);
        var second = JsonSerializer.SerializeToUtf8Bytes(
            policy.Overlay(State(InternalPhase.PreMatch)),
            JsonUtil.Options);
        Assert.Equal(first, second);
    }

    [Fact]
    public void Ut030_malformed_public_markup_is_plain_unavailable_text()
    {
        var snapshot = State(InternalPhase.PreMatch).PublicSnapshot! with
        {
            SourceText = "<script>alert(1)</script>",
        };
        var notebook = State(InternalPhase.PreMatch) with { PublicSnapshot = snapshot };
        var projected = new DisclosurePolicy().Overlay(notebook).PublicSnapshot;
        Assert.NotNull(projected);
        Assert.False(projected.Available);
        Assert.Equal("Source unavailable", projected.SourceText);
        Assert.DoesNotContain('<', projected.SourceText);
    }

    [Theory]
    [InlineData(InternalPhase.Candidate)]
    [InlineData(InternalPhase.CompletionPending)]
    public void Unresolved_and_unconfirmed_completion_phases_remain_restricted(InternalPhase phase)
    {
        var view = new DisclosurePolicy().Overlay(State(phase));
        Assert.Empty(view.HistoricalObservations);
        Assert.Null(view.PublicSnapshot);
        Assert.Equal(
            RepoError.DisclosureRestricted,
            new DisclosurePolicy().Authorize(QueryKind.SearchHistory, phase).Error);
    }
}
