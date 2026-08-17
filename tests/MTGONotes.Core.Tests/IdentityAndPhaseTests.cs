using MTGONotes.Core.Domain;
using MTGONotes.Core.Identity;
using MTGONotes.Core.Live;

namespace MTGONotes.Core.Tests;

public sealed class IdentityAndPhaseTests
{
    [Fact]
    public void Normalize_handle_is_case_insensitive_and_rejects_markup()
    {
        var ok = HandleNormalization.NormalizeHandle("  VidereBot1  ");
        Assert.True(ok.IsSuccess);
        Assert.Equal("VidereBot1", ok.Value!.Display);
        Assert.Equal("viderebot1", ok.Value.Key);

        Assert.Equal(
            RepoError.InvalidHandle,
            HandleNormalization.NormalizeHandle("bad<name>").Error);
        Assert.Equal(RepoError.InvalidHandle, HandleNormalization.NormalizeHandle("   ").Error);
    }

    [Fact]
    public void Normalize_handle_strips_visual_separators()
    {
        var ok = HandleNormalization.NormalizeHandle("| Opponent_42 •");
        Assert.Equal("Opponent_42", ok.Value!.Display);
        Assert.Equal("opponent_42", ok.Value.Key);
    }

    [Theory]
    [InlineData(LiveMatchSignal.None, InternalPhase.Idle)]
    [InlineData(LiveMatchSignal.Pairings, InternalPhase.PreMatch)]
    [InlineData(LiveMatchSignal.InGame, InternalPhase.InGameRestricted)]
    [InlineData(LiveMatchSignal.Sideboarding, InternalPhase.BetweenGames)]
    [InlineData(LiveMatchSignal.Finished, InternalPhase.CompletionPending)]
    [InlineData(LiveMatchSignal.Unknown, InternalPhase.InGameRestricted)]
    public void Live_signals_map_to_fail_closed_phases(LiveMatchSignal signal, InternalPhase expected)
    {
        Assert.Equal(expected, PhaseMapper.FromLive(signal));
    }

    [Theory]
    [InlineData("sideboarding", InternalPhase.BetweenGames)]
    [InlineData("between games", InternalPhase.BetweenGames)]
    [InlineData("match complete", InternalPhase.CompletionPending)]
    [InlineData("results", InternalPhase.CompletionPending)]
    [InlineData("pairings", InternalPhase.PreMatch)]
    [InlineData("game starting", InternalPhase.PreMatch)]
    [InlineData("combat", InternalPhase.InGameRestricted)]
    public void Visible_text_maps_like_the_rust_detector(string text, InternalPhase expected)
    {
        Assert.Equal(expected, PhaseMapper.FromVisibleText(text));
    }
}
