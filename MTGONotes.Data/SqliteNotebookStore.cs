using Microsoft.Data.Sqlite;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;
using MTGONotes.Core.Notebook;

namespace MTGONotes.Data;

public sealed class SqliteNotebookStore : INotebookStore, IDisposable
{
    private readonly EncryptedConnection _connection;
    private readonly object _gate = new();

    private SqliteNotebookStore(EncryptedConnection connection) => _connection = connection;

    public static Result<SqliteNotebookStore> Open(string databasePath, DatabaseKey key)
    {
        var opened = EncryptedConnection.Open(databasePath, key);
        return opened.IsSuccess
            ? Result<SqliteNotebookStore>.Ok(new SqliteNotebookStore(opened.Value!))
            : Result<SqliteNotebookStore>.Fail(opened.Error!.Value);
    }

    public ConnectionSecurity Security => _connection.Security;

    public Result<EntityId?> FindProfileByNormalizedHandle(string normalizedHandle)
    {
        lock (_gate)
        {
            try
            {
                using var command = _connection.Connection.CreateCommand();
                command.CommandText =
                    """
                    SELECT profile.id
                    FROM opponent_profiles profile
                    WHERE profile.normalized_handle = $h AND profile.deleted_at IS NULL
                    UNION ALL
                    SELECT profile.id
                    FROM opponent_aliases alias
                    JOIN opponent_profiles profile ON profile.id = alias.profile_id
                    WHERE alias.normalized_handle = $h AND profile.deleted_at IS NULL
                    LIMIT 1
                    """;
                command.Parameters.AddWithValue("$h", normalizedHandle);
                var value = command.ExecuteScalar() as string;
                if (value is null)
                {
                    return Result<EntityId?>.Ok(null);
                }

                return Result<EntityId?>.Ok(EntityId.Parse(value));
            }
            catch (Exception ex) when (ex is SqliteException or DomainException)
            {
                return Result<EntityId?>.Fail(RepoError.NotebookInvalid);
            }
        }
    }

    public Result CreateProfile(
        EntityId id,
        string displayHandle,
        string normalizedHandle,
        UtcMillis createdAt)
    {
        lock (_gate)
        {
            try
            {
                using var transaction = _connection.Connection.BeginTransaction();
                if (HandleTaken(transaction, normalizedHandle, null))
                {
                    return Result.Fail(RepoError.IdentityConflict);
                }

                using var insert = _connection.Connection.CreateCommand();
                insert.Transaction = transaction;
                insert.CommandText =
                    """
                    INSERT INTO opponent_profiles(
                        id, primary_handle, normalized_handle, created_at, revision
                    ) VALUES ($id, $display, $key, $created, 1)
                    """;
                insert.Parameters.AddWithValue("$id", id.AsString());
                insert.Parameters.AddWithValue("$display", displayHandle);
                insert.Parameters.AddWithValue("$key", normalizedHandle);
                insert.Parameters.AddWithValue("$created", createdAt.Value);
                insert.ExecuteNonQuery();
                transaction.Commit();
                return Result.Ok();
            }
            catch (SqliteException)
            {
                return Result.Fail(RepoError.SaveFailed);
            }
        }
    }

    public Result StartEncounter(
        EntityId encounterId,
        EntityId profileId,
        UtcMillis startedAt,
        ulong generation,
        string source)
    {
        if (source is not ("manual" or "uia" or "ocr" or "mtgosdk"))
        {
            return Result.Fail(RepoError.InvalidRequest);
        }

        lock (_gate)
        {
            try
            {
                using var command = _connection.Connection.CreateCommand();
                command.CommandText =
                    """
                    INSERT INTO encounters(
                        id, profile_id, format, started_at, status, phase,
                        source, generation, revision
                    ) VALUES ($id, $profile, 'Modern', $started, 'active', 'pre_match', $source, $generation, 1)
                    """;
                command.Parameters.AddWithValue("$id", encounterId.AsString());
                command.Parameters.AddWithValue("$profile", profileId.AsString());
                command.Parameters.AddWithValue("$started", startedAt.Value);
                command.Parameters.AddWithValue("$source", source);
                command.Parameters.AddWithValue("$generation", checked((long)generation));
                command.ExecuteNonQuery();
                return Result.Ok();
            }
            catch (SqliteException)
            {
                return Result.Fail(RepoError.SaveFailed);
            }
        }
    }

    public Result FinishEncounter(EntityId encounterId, UtcMillis endedAt)
    {
        lock (_gate)
        {
            try
            {
                using var command = _connection.Connection.CreateCommand();
                command.CommandText =
                    """
                    UPDATE encounters
                    SET ended_at = $ended, status = 'finished', phase = 'finished',
                        revision = revision + 1
                    WHERE id = $id AND status = 'active' AND deleted_at IS NULL
                    """;
                command.Parameters.AddWithValue("$ended", endedAt.Value);
                command.Parameters.AddWithValue("$id", encounterId.AsString());
                return command.ExecuteNonQuery() == 1
                    ? Result.Ok()
                    : Result.Fail(RepoError.InvalidTransition);
            }
            catch (SqliteException)
            {
                return Result.Fail(RepoError.SaveFailed);
            }
        }
    }

    public Result ChangePhase(EntityId encounterId, InternalPhase phase)
    {
        var sqlPhase = phase.ToSql();
        if (sqlPhase is "idle" or "candidate")
        {
            return Result.Fail(RepoError.InvalidTransition);
        }

        lock (_gate)
        {
            try
            {
                using var command = _connection.Connection.CreateCommand();
                command.CommandText =
                    """
                    UPDATE encounters
                    SET phase = $phase, revision = revision + 1
                    WHERE id = $id AND deleted_at IS NULL
                    """;
                command.Parameters.AddWithValue("$phase", sqlPhase);
                command.Parameters.AddWithValue("$id", encounterId.AsString());
                return command.ExecuteNonQuery() == 1
                    ? Result.Ok()
                    : Result.Fail(RepoError.NotFound);
            }
            catch (SqliteException)
            {
                return Result.Fail(RepoError.SaveFailed);
            }
        }
    }

    public Result MarkIncomplete(EntityId encounterId, string reason)
    {
        lock (_gate)
        {
            try
            {
                using var command = _connection.Connection.CreateCommand();
                command.CommandText =
                    """
                    UPDATE encounters
                    SET status = 'incomplete', phase = 'incomplete', incomplete_reason = $reason,
                        revision = revision + 1
                    WHERE id = $id AND status = 'active'
                    """;
                command.Parameters.AddWithValue("$reason", reason);
                command.Parameters.AddWithValue("$id", encounterId.AsString());
                return command.ExecuteNonQuery() == 1
                    ? Result.Ok()
                    : Result.Fail(RepoError.InvalidTransition);
            }
            catch (SqliteException)
            {
                return Result.Fail(RepoError.SaveFailed);
            }
        }
    }

    public Result SaveObservation(
        EntityId observationId,
        EntityId encounterId,
        string text,
        UtcMillis createdAt)
    {
        lock (_gate)
        {
            try
            {
                using var command = _connection.Connection.CreateCommand();
                command.CommandText =
                    """
                    INSERT INTO observations(
                        id, encounter_id, text, created_at, revision, searchable
                    ) VALUES ($id, $encounter, $text, $created, 1, 1)
                    """;
                command.Parameters.AddWithValue("$id", observationId.AsString());
                command.Parameters.AddWithValue("$encounter", encounterId.AsString());
                command.Parameters.AddWithValue("$text", text);
                command.Parameters.AddWithValue("$created", createdAt.Value);
                command.ExecuteNonQuery();
                return Result.Ok();
            }
            catch (SqliteException)
            {
                return Result.Fail(RepoError.SaveFailed);
            }
        }
    }

    public Result<IReadOnlyList<ObservationView>> ListObservations(EntityId encounterId)
    {
        lock (_gate)
        {
            try
            {
                using var command = _connection.Connection.CreateCommand();
                command.CommandText =
                    """
                    SELECT id, text FROM observations
                    WHERE encounter_id = $id AND deleted_at IS NULL
                    ORDER BY created_at DESC, id DESC
                    """;
                command.Parameters.AddWithValue("$id", encounterId.AsString());
                using var reader = command.ExecuteReader();
                var notes = new List<ObservationView>();
                while (reader.Read())
                {
                    notes.Add(new ObservationView(reader.GetString(0), reader.GetString(1), false));
                }

                return Result<IReadOnlyList<ObservationView>>.Ok(notes);
            }
            catch (SqliteException)
            {
                return Result<IReadOnlyList<ObservationView>>.Fail(RepoError.NotebookInvalid);
            }
        }
    }

    public Result<HistoryPage> SearchHistory(string query, int pageSize = 50)
    {
        if (pageSize is <= 0 or > 100 || string.IsNullOrWhiteSpace(query))
        {
            return Result<HistoryPage>.Fail(RepoError.InvalidRequest);
        }

        lock (_gate)
        {
            try
            {
                var tokens = query.Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
                    .Select(token => token.Replace("\"", string.Empty))
                    .Where(token => token.Length > 0)
                    .Select(token => $"\"{token}\"");
                var match = string.Join(" AND ", tokens);
                if (match.Length == 0)
                {
                    return Result<HistoryPage>.Fail(RepoError.InvalidRequest);
                }

                using var command = _connection.Connection.CreateCommand();
                command.CommandText =
                    """
                    SELECT entity_type, entity_id, CAST(sort_ms AS INTEGER), content
                    FROM history_fts
                    WHERE history_fts MATCH $q
                    ORDER BY CAST(sort_ms AS INTEGER) DESC, entity_id DESC
                    LIMIT $limit
                    """;
                command.Parameters.AddWithValue("$q", match);
                command.Parameters.AddWithValue("$limit", pageSize + 1);
                using var reader = command.ExecuteReader();
                var items = new List<HistoryHit>();
                while (reader.Read())
                {
                    items.Add(new HistoryHit(
                        reader.GetString(0),
                        reader.GetString(1),
                        reader.GetInt64(2),
                        reader.GetString(3)));
                }

                var hasMore = items.Count > pageSize;
                if (hasMore)
                {
                    items.RemoveAt(items.Count - 1);
                }

                return Result<HistoryPage>.Ok(new HistoryPage(items, hasMore));
            }
            catch (SqliteException)
            {
                return Result<HistoryPage>.Fail(RepoError.InvalidRequest);
            }
        }
    }

    public Result<IReadOnlyList<EncounterSummary>> ListRecentEncounters(int limit = 50)
    {
        lock (_gate)
        {
            try
            {
                using var command = _connection.Connection.CreateCommand();
                command.CommandText =
                    """
                    SELECT e.id, e.profile_id, p.primary_handle, e.phase, e.status, e.started_at
                    FROM encounters e
                    JOIN opponent_profiles p ON p.id = e.profile_id
                    WHERE e.deleted_at IS NULL AND p.deleted_at IS NULL
                    ORDER BY e.started_at DESC, e.id DESC
                    LIMIT $limit
                    """;
                command.Parameters.AddWithValue("$limit", limit);
                using var reader = command.ExecuteReader();
                var rows = new List<EncounterSummary>();
                while (reader.Read())
                {
                    rows.Add(new EncounterSummary(
                        EntityId.Parse(reader.GetString(0)),
                        EntityId.Parse(reader.GetString(1)),
                        reader.GetString(2),
                        reader.GetString(3),
                        reader.GetString(4),
                        reader.GetInt64(5)));
                }

                return Result<IReadOnlyList<EncounterSummary>>.Ok(rows);
            }
            catch (Exception ex) when (ex is SqliteException or DomainException)
            {
                return Result<IReadOnlyList<EncounterSummary>>.Fail(RepoError.NotebookInvalid);
            }
        }
    }

    public Result<NotebookDump> ExportLogical()
    {
        lock (_gate)
        {
            try
            {
                var profiles = new List<LogicalProfile>();
                using (var command = _connection.Connection.CreateCommand())
                {
                    command.CommandText =
                        "SELECT id, primary_handle, normalized_handle, created_at FROM opponent_profiles WHERE deleted_at IS NULL";
                    using var reader = command.ExecuteReader();
                    while (reader.Read())
                    {
                        profiles.Add(new LogicalProfile(
                            reader.GetString(0),
                            reader.GetString(1),
                            reader.GetString(2),
                            reader.GetInt64(3)));
                    }
                }

                var encounters = new List<LogicalEncounter>();
                using (var command = _connection.Connection.CreateCommand())
                {
                    command.CommandText =
                        """
                        SELECT e.id, e.profile_id, p.primary_handle, e.phase, e.status, e.started_at
                        FROM encounters e
                        JOIN opponent_profiles p ON p.id = e.profile_id
                        WHERE e.deleted_at IS NULL
                        ORDER BY e.started_at
                        """;
                    using var reader = command.ExecuteReader();
                    while (reader.Read())
                    {
                        var encounterId = EntityId.Parse(reader.GetString(0));
                        var notes = ListObservationsUnlocked(encounterId);
                        encounters.Add(new LogicalEncounter(
                            reader.GetString(0),
                            reader.GetString(1),
                            reader.GetString(2),
                            reader.GetString(3),
                            reader.GetString(4),
                            reader.GetInt64(5),
                            notes));
                    }
                }

                return Result<NotebookDump>.Ok(new NotebookDump(2, profiles, encounters));
            }
            catch (Exception ex) when (ex is SqliteException or DomainException)
            {
                return Result<NotebookDump>.Fail(RepoError.NotebookInvalid);
            }
        }
    }

    public Result<long> SchemaVersion()
    {
        lock (_gate)
        {
            return MigrationManager.CurrentVersion(_connection.Connection);
        }
    }

    public void Dispose() => _connection.Dispose();

    private IReadOnlyList<LogicalObservation> ListObservationsUnlocked(EntityId encounterId)
    {
        using var command = _connection.Connection.CreateCommand();
        command.CommandText =
            """
            SELECT id, encounter_id, text, created_at FROM observations
            WHERE encounter_id = $id AND deleted_at IS NULL
            ORDER BY created_at
            """;
        command.Parameters.AddWithValue("$id", encounterId.AsString());
        using var reader = command.ExecuteReader();
        var notes = new List<LogicalObservation>();
        while (reader.Read())
        {
            notes.Add(new LogicalObservation(
                reader.GetString(0),
                reader.GetString(1),
                reader.GetString(2),
                reader.GetInt64(3)));
        }

        return notes;
    }

    private bool HandleTaken(SqliteTransaction transaction, string normalizedHandle, string? exceptProfileId)
    {
        using var command = _connection.Connection.CreateCommand();
        command.Transaction = transaction;
        command.CommandText =
            """
            SELECT EXISTS(
                SELECT 1 FROM opponent_profiles
                WHERE normalized_handle = $h AND deleted_at IS NULL
                  AND ($except IS NULL OR id <> $except)
                UNION ALL
                SELECT 1 FROM opponent_aliases alias
                JOIN opponent_profiles profile ON profile.id = alias.profile_id
                WHERE alias.normalized_handle = $h AND profile.deleted_at IS NULL
                  AND ($except IS NULL OR alias.profile_id <> $except)
            )
            """;
        command.Parameters.AddWithValue("$h", normalizedHandle);
        command.Parameters.AddWithValue("$except", exceptProfileId is null ? DBNull.Value : exceptProfileId);
        return Convert.ToInt64(command.ExecuteScalar()) == 1;
    }
}

public sealed class NotebookBootstrap
{
    public static Result<SqliteNotebookStore> Initialize(
        string databasePath,
        string keyPath,
        IKeyProtector protector)
    {
        var custody = new KeyCustody(keyPath, databasePath, protector);
        var key = custody.LoadOrCreate();
        if (!key.IsSuccess)
        {
            return Result<SqliteNotebookStore>.Fail(key.Error!.Value);
        }

        using var ownedKey = key.Value!;
        var migrated = new MigrationManager().Migrate(databasePath, ownedKey);
        if (!migrated.IsSuccess)
        {
            return Result<SqliteNotebookStore>.Fail(migrated.Error!.Value);
        }

        return SqliteNotebookStore.Open(databasePath, ownedKey);
    }
}
