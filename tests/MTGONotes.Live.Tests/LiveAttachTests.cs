using MTGONotes.Core.Domain;
using MTGONotes.Core.Live;
using MTGONotes.Core.Session;
using MTGONotes.Live;

namespace MTGONotes.Live.Tests;

public sealed class LiveAttachTests
{
    [Theory]
    [InlineData(false, false, 0, false, false, LiveMatchSignal.None)]
    [InlineData(true, true, MtgoMatchFlags.Joined, false, false, LiveMatchSignal.Pairings)]
    [InlineData(true, true, MtgoMatchFlags.GameStarted, true, false, LiveMatchSignal.InGame)]
    [InlineData(true, true, MtgoMatchFlags.Sideboarding, false, false, LiveMatchSignal.Sideboarding)]
    [InlineData(true, true, MtgoMatchFlags.MatchCompleted, false, true, LiveMatchSignal.Finished)]
    [InlineData(true, true, 0x8, false, false, LiveMatchSignal.Unknown)]
    public void Match_flags_map_fail_closed(
        bool process,
        bool loggedIn,
        long flags,
        bool hasGame,
        bool complete,
        LiveMatchSignal expected)
    {
        var reading = new MtgoMatchReading(
            process,
            loggedIn,
            "1.0",
            new MtgoUser("Me", 1),
            process && loggedIn && expected != LiveMatchSignal.Unknown
                ? new MtgoUser("Them", 2)
                : null,
            "Modern",
            flags,
            hasGame,
            hasGame ? 1 : 0,
            complete);
        Assert.Equal(expected, MatchSignalMapper.FromReading(reading));
        Assert.Equal(PhaseMapper.FromLive(expected), PhaseMapper.FromLive(MatchSignalMapper.FromReading(reading)));
    }

    [Fact]
    public void Attach_emits_opponent_and_never_the_current_user()
    {
        var client = new FakeMtgoClient();
        client.AttachLoggedIn("Me", 1);
        client.SetMatch("Alice", 99, MtgoMatchFlags.Joined);
        using var source = new LiveAttachSource(client);
        LiveSnapshot? snapshot = null;
        source.SnapshotChanged += (_, value) => snapshot = value;
        source.PollOnce();

        Assert.True(source.IsAttached);
        Assert.NotNull(snapshot);
        Assert.Equal("Alice", snapshot!.Opponent!.DisplayHandle);
        Assert.Equal(99, snapshot.Opponent.MtgoUserId);
        Assert.Equal(InternalPhase.PreMatch, snapshot.SuggestedPhase);
        Assert.NotEqual("me", snapshot.Opponent.NormalizedHandle);
    }

    [Fact]
    public void Reconnect_starts_a_new_provider_session()
    {
        var client = new FakeMtgoClient();
        client.AttachLoggedIn("Me", 1);
        client.SetMatch("Bob", 2, MtgoMatchFlags.GameStarted, hasCurrentGame: true, gameCount: 1);
        using var source = new LiveAttachSource(client);
        source.PollOnce();
        var first = source.ProviderSession;

        client.Disconnect();
        source.PollOnce();
        Assert.False(source.IsAttached);

        client.AttachLoggedIn("Me", 1);
        client.SetMatch("Carol", 3, MtgoMatchFlags.Joined);
        LiveSnapshot? after = null;
        source.SnapshotChanged += (_, value) => after = value;
        source.PollOnce();

        Assert.True(source.IsAttached);
        Assert.NotEqual(first, source.ProviderSession);
        Assert.NotNull(after);
        Assert.True(after!.Generation > 1);
        Assert.Equal("Carol", after.Opponent!.DisplayHandle);
    }

    [Fact]
    public void Snapshot_drives_session_candidate_then_confirm()
    {
        var client = new FakeMtgoClient();
        client.AttachLoggedIn("Me", 1);
        client.SetMatch("Dana", 4, MtgoMatchFlags.Joined, format: "Pioneer");
        using var source = new LiveAttachSource(client);
        var session = new CompanionSession();
        source.SnapshotChanged += (_, snapshot) => session.ApplySnapshot(snapshot);
        source.PollOnce();

        Assert.Equal("Dana", session.Candidate!.DisplayHandle);
        Assert.True(session.ConfirmOpponent(session.Candidate).IsSuccess);
        Assert.Equal("Dana", session.CurrentView.ConfirmedHandle);
        Assert.Equal(InternalPhase.PreMatch, session.CurrentView.Phase);
    }

    [Fact]
    public void Unavailable_client_does_not_attach()
    {
        using var source = new LiveAttachSource(new UnavailableMtgoClient());
        var emitted = false;
        source.SnapshotChanged += (_, _) => emitted = true;
        source.PollOnce();
        Assert.False(source.IsAttached);
        Assert.False(emitted);
    }

    [Fact]
    public void Source_tree_never_calls_client_log_on()
    {
        var roots = new[]
        {
            Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "MTGONotes.Live")),
            Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "MTGONotes.App")),
        };
        foreach (var root in roots.Where(Directory.Exists))
        {
            foreach (var file in Directory.EnumerateFiles(root, "*.cs", SearchOption.AllDirectories))
            {
                var text = File.ReadAllText(file);
                Assert.DoesNotContain("Client.LogOn", text, StringComparison.Ordinal);
                Assert.DoesNotContain(".LogOn(", text, StringComparison.Ordinal);
            }
        }
    }
}
