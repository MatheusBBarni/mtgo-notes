using MTGONotes.Core.Domain;
using MTGONotes.Core.Session;
using MTGONotes.Data;

namespace MTGONotes.Data.Tests;

public sealed class NotebookPersistenceTests
{
    [Fact]
    public void Schema_checksums_are_stable()
    {
        Assert.Equal(
            "d58493c1967659c37f49729a3adf86c7d7ae90923185e9bac1214ebbd09f39bc",
            SchemaSql.Checksum(SchemaSql.Initial));
        Assert.Equal(
            "755f66e88de52eb5c853ec3dc0bb52295033a88937cfb75ef9a5a93372c19d6e",
            SchemaSql.Checksum(SchemaSql.RetiredTags));
    }

    [Fact]
    public void Ut032_unseal_failure_does_not_replace_database_or_key()
    {
        using var directory = new TempNotebook();
        File.WriteAllText(directory.DatabasePath, "existing-database");
        File.WriteAllText(directory.KeyPath, "existing-key");
        var custody = new KeyCustody(
            directory.KeyPath,
            directory.DatabasePath,
            new ScopedProtector(7, failUnprotect: true));

        Assert.Equal(RepoError.KeyUnavailable, custody.LoadOrCreate().Error);
        Assert.Equal("existing-database", File.ReadAllText(directory.DatabasePath));
        Assert.Equal("existing-key", File.ReadAllText(directory.KeyPath));
    }

    [Fact]
    public void Key_round_trips_for_same_scope_and_fails_for_foreign_scope()
    {
        using var directory = new TempNotebook();
        var owner = new KeyCustody(directory.KeyPath, directory.DatabasePath, new ScopedProtector(7));
        using var original = owner.LoadOrCreate().Value!;
        using var reopened = owner.LoadOrCreate().Value!;
        Assert.Equal(original.ToSqlCipherHex(), reopened.ToSqlCipherHex());

        var foreign = new KeyCustody(directory.KeyPath, directory.DatabasePath, new ScopedProtector(8));
        Assert.Equal(RepoError.KeyUnavailable, foreign.LoadOrCreate().Error);
    }

    [Fact]
    public void Initialize_creates_schema_v2_and_reopens_profiles()
    {
        using var directory = new TempNotebook();
        using var first = NotebookBootstrap
            .Initialize(directory.DatabasePath, directory.KeyPath, new ScopedProtector(3))
            .Value!;
        Assert.Equal(2, first.SchemaVersion().Value);
        Assert.False(string.IsNullOrWhiteSpace(first.Security.CipherVersion));

        var profileId = EntityId.New();
        Assert.True(
            first.CreateProfile(profileId, "VidereBot1", "viderebot1", UtcMillis.Now()).IsSuccess);
        first.Dispose();

        using var reopened = NotebookBootstrap
            .Initialize(directory.DatabasePath, directory.KeyPath, new ScopedProtector(3))
            .Value!;
        Assert.Equal(profileId, reopened.FindProfileByNormalizedHandle("viderebot1").Value);
    }

    [Fact]
    public void Missing_key_does_not_open_existing_database()
    {
        using var directory = new TempNotebook();
        using var created = NotebookBootstrap
            .Initialize(directory.DatabasePath, directory.KeyPath, new ScopedProtector(1))
            .Value!;
        created.Dispose();
        File.Delete(directory.KeyPath);
        Assert.Equal(
            RepoError.KeyUnavailable,
            NotebookBootstrap.Initialize(
                directory.DatabasePath,
                directory.KeyPath,
                new ScopedProtector(1)).Error);
    }

    [Fact]
    public void Session_persists_opponent_and_note_across_reopen()
    {
        using var directory = new TempNotebook();
        using var store = NotebookBootstrap
            .Initialize(directory.DatabasePath, directory.KeyPath, new ScopedProtector(4))
            .Value!;
        var session = new CompanionSession(store);
        Assert.True(session.EnterOpponent("Alice").IsSuccess);
        Assert.True(session.SaveObservation("Has leftover Surgical").IsSuccess);
        Assert.NotNull(session.ActiveEncounterId);
        var notes = store.ListObservations(session.ActiveEncounterId!.Value);
        Assert.True(notes.IsSuccess);
        Assert.Contains(notes.Value!, note => note.Text == "Has leftover Surgical");

        using var reopened = NotebookBootstrap
            .Initialize(directory.DatabasePath, directory.KeyPath, new ScopedProtector(4))
            .Value!;
        Assert.NotNull(reopened.FindProfileByNormalizedHandle("alice").Value);
    }

    [Fact]
    public void History_search_and_export_include_saved_notes()
    {
        using var directory = new TempNotebook();
        using var store = NotebookBootstrap
            .Initialize(directory.DatabasePath, directory.KeyPath, new ScopedProtector(6))
            .Value!;
        var session = new CompanionSession(store);
        Assert.True(session.EnterOpponent("SearchMe").IsSuccess);
        Assert.True(session.SaveObservation("Surgical leftover").IsSuccess);
        var hits = store.SearchHistory("Surgical");
        Assert.True(hits.IsSuccess);
        Assert.Contains(hits.Value!.Items, item => item.Content.Contains("Surgical"));
        var dump = store.ExportLogical();
        Assert.Contains(dump.Value!.Profiles, profile => profile.Handle == "SearchMe");
        Assert.Contains(
            dump.Value.Encounters.SelectMany(item => item.Observations),
            note => note.Text == "Surgical leftover");
    }

    [Fact]
    public void Duplicate_handle_is_an_identity_conflict()
    {
        using var directory = new TempNotebook();
        using var store = NotebookBootstrap
            .Initialize(directory.DatabasePath, directory.KeyPath, new ScopedProtector(5))
            .Value!;
        Assert.True(store.CreateProfile(EntityId.New(), "Bob", "bob", UtcMillis.Now()).IsSuccess);
        Assert.Equal(
            RepoError.IdentityConflict,
            store.CreateProfile(EntityId.New(), "bob", "bob", UtcMillis.Now()).Error);
    }
}

internal sealed class TempNotebook : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), "mtgonotes-" + Guid.NewGuid().ToString("N"));

    public TempNotebook()
    {
        Directory.CreateDirectory(_root);
        DatabasePath = Path.Combine(_root, "notebook.db");
        KeyPath = Path.Combine(_root, "notebook.key");
    }

    public string DatabasePath { get; }

    public string KeyPath { get; }

    public void Dispose()
    {
        try
        {
            Directory.Delete(_root, recursive: true);
        }
        catch (IOException)
        {
        }
    }
}
