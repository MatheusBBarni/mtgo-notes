using Microsoft.Data.Sqlite;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Data;

public sealed record Migration(long Version, string Sql)
{
    public string Checksum() => SchemaSql.Checksum(Sql);
}

public enum RollbackStatus
{
    NotRequired,
    Created,
    Restored,
}

public sealed record MigrationReport(long PreviousVersion, long CurrentVersion, RollbackStatus RollbackStatus);

public sealed class MigrationManager
{
    private readonly IReadOnlyList<Migration> _migrations;

    public MigrationManager()
        : this(
            [
                new Migration(1, SchemaSql.Initial),
                new Migration(SchemaSql.SchemaVersion, SchemaSql.RetiredTags),
            ])
    {
    }

    public MigrationManager(IReadOnlyList<Migration> migrations)
    {
        _migrations = migrations.OrderBy(item => item.Version).ToArray();
        SupportedVersion = _migrations.Count == 0 ? 0 : _migrations[^1].Version;
    }

    public long SupportedVersion { get; }

    public Result<MigrationReport> Migrate(string databasePath, DatabaseKey key)
    {
        var opened = EncryptedConnection.Open(databasePath, key);
        if (!opened.IsSuccess)
        {
            return Result<MigrationReport>.Fail(opened.Error!.Value);
        }

        using var encrypted = opened.Value!;
        var previous = CurrentVersion(encrypted.Connection);
        if (!previous.IsSuccess)
        {
            return Result<MigrationReport>.Fail(previous.Error!.Value);
        }

        if (previous.Value > SupportedVersion)
        {
            return Result<MigrationReport>.Fail(RepoError.MigrationFailed);
        }

        var checksums = VerifyAppliedChecksums(encrypted.Connection);
        if (!checksums.IsSuccess)
        {
            return Result<MigrationReport>.Fail(checksums.Error!.Value);
        }

        var pending = _migrations.Where(item => item.Version > previous.Value).ToArray();
        if (pending.Length == 0)
        {
            return Result<MigrationReport>.Ok(
                new MigrationReport(previous.Value, previous.Value, RollbackStatus.NotRequired));
        }

        var rollbackPath = RollbackPath(databasePath);
        var backup = encrypted.BackupTo(rollbackPath, key);
        if (!backup.IsSuccess)
        {
            return Result<MigrationReport>.Fail(RepoError.MigrationFailed);
        }

        try
        {
            foreach (var migration in pending)
            {
                EncryptedConnection.ExecuteBatch(encrypted.Connection, "BEGIN IMMEDIATE;");
                try
                {
                    EncryptedConnection.ExecuteBatch(encrypted.Connection, migration.Sql);
                    using (var insert = encrypted.Connection.CreateCommand())
                    {
                        insert.CommandText =
                            "INSERT INTO schema_migrations(version, checksum, applied_at) VALUES ($v, $c, $t)";
                        insert.Parameters.AddWithValue("$v", migration.Version);
                        insert.Parameters.AddWithValue("$c", migration.Checksum());
                        insert.Parameters.AddWithValue("$t", UtcMillis.Now().Value);
                        insert.ExecuteNonQuery();
                    }

                    using (var fk = encrypted.Connection.CreateCommand())
                    {
                        fk.CommandText = "PRAGMA foreign_key_check";
                        using var reader = fk.ExecuteReader();
                        if (reader.Read())
                        {
                            throw new SqliteException("foreign key check failed", 19);
                        }
                    }

                    EncryptedConnection.ExecuteBatch(encrypted.Connection, "COMMIT;");
                }
                catch
                {
                    try
                    {
                        EncryptedConnection.ExecuteBatch(encrypted.Connection, "ROLLBACK;");
                    }
                    catch (SqliteException)
                    {
                        // The rollback copy is restored by the outer handler.
                    }

                    throw;
                }
            }

            var integrity = encrypted.IntegrityCheck();
            if (!integrity.IsSuccess)
            {
                throw new SqliteException("integrity check failed", 11);
            }
        }
        catch (SqliteException)
        {
            encrypted.Dispose();
            RestoreRollback(databasePath, rollbackPath);
            return Result<MigrationReport>.Fail(RepoError.MigrationFailed);
        }

        RemoveDatabaseFamily(rollbackPath);
        return Result<MigrationReport>.Ok(
            new MigrationReport(previous.Value, SupportedVersion, RollbackStatus.Created));
    }

    public static Result<long> CurrentVersion(SqliteConnection connection)
    {
        try
        {
            using var exists = connection.CreateCommand();
            exists.CommandText =
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations')";
            if (Convert.ToInt64(exists.ExecuteScalar()) != 1)
            {
                return Result<long>.Ok(0);
            }

            using var version = connection.CreateCommand();
            version.CommandText = "SELECT coalesce(max(version), 0) FROM schema_migrations";
            return Result<long>.Ok(Convert.ToInt64(version.ExecuteScalar()));
        }
        catch (SqliteException)
        {
            return Result<long>.Fail(RepoError.MigrationFailed);
        }
    }

    private Result VerifyAppliedChecksums(SqliteConnection connection)
    {
        var version = CurrentVersion(connection);
        if (!version.IsSuccess)
        {
            return Result.Fail(version.Error!.Value);
        }

        if (version.Value == 0)
        {
            return Result.Ok();
        }

        try
        {
            using var command = connection.CreateCommand();
            command.CommandText = "SELECT version, checksum FROM schema_migrations ORDER BY version";
            using var reader = command.ExecuteReader();
            while (reader.Read())
            {
                var appliedVersion = reader.GetInt64(0);
                var checksum = reader.GetString(1);
                var expected = _migrations.FirstOrDefault(item => item.Version == appliedVersion);
                if (expected is null || expected.Checksum() != checksum)
                {
                    return Result.Fail(RepoError.MigrationFailed);
                }
            }

            return Result.Ok();
        }
        catch (SqliteException)
        {
            return Result.Fail(RepoError.MigrationFailed);
        }
    }

    public static string RollbackPath(string databasePath) =>
        Path.ChangeExtension(databasePath, ".rollback");

    private static void RestoreRollback(string databasePath, string rollbackPath)
    {
        if (!File.Exists(rollbackPath))
        {
            return;
        }

        var failedPath = Path.ChangeExtension(databasePath, ".failed");
        RemoveDatabaseFamily(failedPath);
        if (File.Exists(databasePath))
        {
            File.Move(databasePath, failedPath, overwrite: true);
        }

        File.Move(rollbackPath, databasePath, overwrite: true);
        RemoveDatabaseFamily(failedPath);
        RemoveSidecars(databasePath);
    }

    private static void RemoveDatabaseFamily(string path)
    {
        if (File.Exists(path))
        {
            File.Delete(path);
        }

        RemoveSidecars(path);
    }

    private static void RemoveSidecars(string path)
    {
        foreach (var sidecar in new[] { path + "-wal", path + "-shm" })
        {
            if (File.Exists(sidecar))
            {
                File.Delete(sidecar);
            }
        }
    }
}
