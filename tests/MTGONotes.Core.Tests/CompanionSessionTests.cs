using MTGONotes.Core.Domain;
using MTGONotes.Core.Live;
using MTGONotes.Core.Session;

namespace MTGONotes.Core.Tests;

public sealed class CompanionSessionTests
{
    [Fact]
    public void Manual_entry_starts_pre_match_and_accepts_a_note()
    {
        var session = new CompanionSession();
        Assert.True(session.EnterOpponent("VidereBot1").IsSuccess);
        Assert.Equal(InternalPhase.PreMatch, session.CurrentView.Phase);
        Assert.Equal("VidereBot1", session.CurrentView.ConfirmedHandle);
        Assert.True(session.OpenCapture().IsSuccess);
        Assert.True(session.SaveObservation("Mull to 5").IsSuccess);
        Assert.Equal("Mull to 5", session.CurrentView.CurrentObservations[0].Text);
    }

    [Fact]
    public void Stale_candidate_cannot_be_confirmed()
    {
        var session = new CompanionSession();
        session.ApplySnapshot(
            new LiveSnapshot(
                "sdk",
                1,
                1,
                10,
                new OpponentCandidate("Alice", "alice", 99, 1, 1, "sdk"),
                InternalPhase.PreMatch,
                "Modern",
                1,
                null));
        var stale = new OpponentCandidate("Alice", "alice", 99, 1, 0, "sdk");
        Assert.Equal(RepoError.CandidateStale, session.ConfirmOpponent(stale).Error);
    }

    [Fact]
    public void In_game_phase_hides_history_queries()
    {
        var session = new CompanionSession();
        Assert.True(session.EnterOpponent("Bob").IsSuccess);
        Assert.True(session.CorrectPhase(InternalPhase.InGameRestricted).IsSuccess);
        Assert.Equal(RepoError.DisclosureRestricted, session.AuthorizeHistory().Error);
        Assert.Empty(session.CurrentView.HistoricalObservations);
        Assert.Null(session.CurrentView.PublicSnapshot);
    }

    [Fact]
    public void Capture_without_encounter_fails_closed()
    {
        var session = new CompanionSession();
        Assert.Equal(RepoError.NoActiveEncounter, session.OpenCapture().Error);
        Assert.Equal(RepoError.NoActiveEncounter, session.SaveObservation("note").Error);
        Assert.True(session.EnterOpponent("Bob").IsSuccess);
        Assert.Equal(RepoError.BlankObservation, session.SaveObservation("   ").Error);
    }
}
