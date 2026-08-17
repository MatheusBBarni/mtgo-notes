using MTGONotes.Core.Classifier;
using MTGONotes.Core.Detection;
using MTGONotes.Core.Domain;
using MTGONotes.Core.Live;
using MTGONotes.Core.Portability;
using MTGONotes.Core.Providers;
using MTGONotes.Core.Session;
using MTGONotes.Core.Settings;

namespace MTGONotes.Core.Tests;

public sealed class RemainingPhasesTests
{
    [Fact]
    public void Settings_round_trip_on_disk()
    {
        var path = Path.Combine(Path.GetTempPath(), "mtgo-settings-" + Guid.NewGuid().ToString("N") + ".json");
        try
        {
            var store = new SettingsStore(path);
            Assert.True(store.Save(new AppSettings { LiveAttachEnabled = false, OverlayEnabled = false }).IsSuccess);
            var loaded = new SettingsStore(path);
            Assert.True(loaded.Load().IsSuccess);
            Assert.False(loaded.Current.LiveAttachEnabled);
            Assert.False(loaded.Current.OverlayEnabled);
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Theory]
    [InlineData("https://www.mtgo.com/decklists", true)]
    [InlineData("https://mtgo.com/decklists", true)]
    [InlineData("http://www.mtgo.com/decklists", false)]
    [InlineData("https://example.com/decklists", false)]
    [InlineData("https://user@www.mtgo.com/decklists", false)]
    public void Official_urls_are_allowlisted(string url, bool ok)
    {
        Assert.Equal(ok, OfficialDeckProvider.ValidateOfficialUrl(url).IsSuccess);
    }

    [Fact]
    public void Official_lookup_requires_consent()
    {
        Assert.Equal(
            RepoError.ConsentRequired,
            OfficialDeckProvider.Lookup(false, "Alice", "Modern", 1).Error);
        Assert.True(OfficialDeckProvider.Lookup(true, "Alice", "Modern", 1).IsSuccess);
    }

    [Fact]
    public void Signature_classifier_matches_then_falls_back_to_knn()
    {
        var burn = new ArchetypeDefinition(
            "burn",
            "Burn",
            false,
            [new SignatureConstraint("bolt", "Lightning Bolt", 4, null)]);
        var assets = new ClassifierAssets(
            "test",
            "digest",
            [new FormatDefinition("Modern", 1, 0.3, [burn])],
            [
                new CorpusDeck("c1", "Modern", "burn", new Dictionary<string, int> { ["bolt"] = 4, ["land"] = 20 }),
            ]);
        var signed = DeckClassifier.Classify(
            new CompleteDeck("Modern", true, [new CanonicalCard("bolt", 4)]),
            assets);
        Assert.Equal(ClassificationMethod.Signature, signed.Value!.Method);
        Assert.Equal("burn", signed.Value.ResultId);

        var knn = DeckClassifier.Classify(
            new CompleteDeck("Modern", true, [new CanonicalCard("bolt", 3), new CanonicalCard("land", 20)]),
            assets);
        Assert.Equal(ClassificationMethod.Knn, knn.Value!.Method);
        Assert.True(knn.Value.Confidence > 0);
    }

    [Fact]
    public void Incomplete_deck_is_rejected()
    {
        var assets = new ClassifierAssets("test", "digest", [], []);
        Assert.Equal(
            RepoError.DeckIncomplete,
            DeckClassifier.Classify(new CompleteDeck("Modern", false, []), assets).Error);
    }

    [Fact]
    public void Backup_round_trips_and_rejects_wrong_passphrase()
    {
        var path = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N") + ".mtgonotes");
        try
        {
            var dump = new Notebook.NotebookDump(
                2,
                [new Notebook.LogicalProfile("p", "Alice", "alice", 1)],
                []);
            Assert.True(NotebookBackup.Write(path, dump, "secret-pass").IsSuccess);
            var read = NotebookBackup.Read(path, "secret-pass");
            Assert.True(read.IsSuccess);
            Assert.Equal("Alice", read.Value!.Profiles[0].Handle);
            Assert.Equal(RepoError.WrongPassphrase, NotebookBackup.Read(path, "nope").Error);
            Assert.Contains("UNENCRYPTED", TextExporter.Render(dump));
        }
        finally
        {
            if (File.Exists(path))
            {
                File.Delete(path);
            }
        }
    }

    [Fact]
    public void Coordinator_rejects_overlapping_operations()
    {
        var coordinator = new OperationCoordinator();
        Assert.True(coordinator.Begin("backup").IsSuccess);
        Assert.Equal(RepoError.OperationBusy, coordinator.Begin("export").Error);
        coordinator.End();
        Assert.True(coordinator.Begin("export").IsSuccess);
    }

    [Fact]
    public void History_search_is_blocked_in_game()
    {
        var session = new CompanionSession();
        Assert.True(session.EnterOpponent("Eve").IsSuccess);
        Assert.True(session.CorrectPhase(InternalPhase.InGameRestricted).IsSuccess);
        Assert.Equal(RepoError.DisclosureRestricted, session.SearchHistory("bolt").Error);
    }

    [Fact]
    public void Title_fallback_only_reacts_to_mtgo_windows()
    {
        var source = new TitleFallbackSource(new FakeWindows([new VisibleWindow("Notepad", 1)]));
        var emitted = false;
        source.SnapshotChanged += (_, _) => emitted = true;
        source.PollOnce();
        Assert.False(emitted);

        var mtgo = new TitleFallbackSource(new FakeWindows([new VisibleWindow("MTGO - Magic: The Gathering Online", 2)]));
        LiveSnapshot? snapshot = null;
        mtgo.SnapshotChanged += (_, value) => snapshot = value;
        mtgo.PollOnce();
        Assert.Equal(InternalPhase.InGameRestricted, snapshot!.SuggestedPhase);
    }

    private sealed class FakeWindows(IReadOnlyList<VisibleWindow> windows) : IWindowEnumerator
    {
        public IReadOnlyList<VisibleWindow> ListVisible() => windows;
    }
}
