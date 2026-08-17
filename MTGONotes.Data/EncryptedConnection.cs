using Microsoft.Data.Sqlite;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Data;

public sealed class ConnectionSecurity
{
    public required bool CipherActive { get; init; }

    public required string CipherVersion { get; init; }

    public required bool ForeignKeys { get; init; }

    public required bool Wal { get; init; }

    public required bool SecureDelete { get; init; }

    public required long BusyTimeoutMs { get; init; }
}

public sealed class EncryptedConnection : IDisposable
{
    public const long BusyTimeoutMs = 5_000;

    private EncryptedConnection(SqliteConnection connection, string path, ConnectionSecurity security)
    {
        Connection = connection;
        Path = path;
        Security = security;
    }

    public SqliteConnection Connection { get; }

    public string Path { get; }

    public ConnectionSecurity Security { get; }

    public static Result<EncryptedConnection> Open(string path, DatabaseKey key)
    {
        SqliteEngine.EnsureInitialized();
        Directory.CreateDirectory(System.IO.Path.GetDirectoryName(System.IO.Path.GetFullPath(path))!);
        var connection = new SqliteConnection(new SqliteConnectionStringBuilder
        {
            DataSource = path,
            Mode = SqliteOpenMode.ReadWriteCreate,
            Pooling = false,
        }.ToString());

        try
        {
            connection.Open();
            Execute(connection, $"PRAGMA key = \"x'{key.ToSqlCipherHex()}'\";");
            var cipherVersion = QueryString(connection, "PRAGMA cipher_version");
            if (string.IsNullOrWhiteSpace(cipherVersion))
            {
                connection.Dispose();
                return Result<EncryptedConnection>.Fail(RepoError.NotebookInvalid);
            }

            _ = QueryInt64(connection, "SELECT count(*) FROM sqlite_master");
            Execute(connection, $"PRAGMA busy_timeout = {BusyTimeoutMs};");
            Execute(connection, "PRAGMA foreign_keys = ON;");
            Execute(connection, "PRAGMA journal_mode = WAL;");
            Execute(connection, "PRAGMA synchronous = FULL;");
            Execute(connection, "PRAGMA secure_delete = ON;");

            var foreignKeys = QueryInt64(connection, "PRAGMA foreign_keys") == 1;
            var journalMode = QueryString(connection, "PRAGMA journal_mode");
            var secureDelete = QueryInt64(connection, "PRAGMA secure_delete") == 1;
            if (!foreignKeys || !journalMode.Equals("wal", StringComparison.OrdinalIgnoreCase) || !secureDelete)
            {
                connection.Dispose();
                return Result<EncryptedConnection>.Fail(RepoError.NotebookInvalid);
            }

            return Result<EncryptedConnection>.Ok(
                new EncryptedConnection(
                    connection,
                    path,
                    new ConnectionSecurity
                    {
                        CipherActive = true,
                        CipherVersion = cipherVersion,
                        ForeignKeys = true,
                        Wal = true,
                        SecureDelete = true,
                        BusyTimeoutMs = BusyTimeoutMs,
                    }));
        }
        catch (SqliteException)
        {
            connection.Dispose();
            return Result<EncryptedConnection>.Fail(RepoError.NotebookInvalid);
        }
    }

    public Result IntegrityCheck()
    {
        try
        {
            return QueryString(Connection, "PRAGMA integrity_check") == "ok"
                ? Result.Ok()
                : Result.Fail(RepoError.NotebookInvalid);
        }
        catch (SqliteException)
        {
            return Result.Fail(RepoError.NotebookInvalid);
        }
    }

    public Result BackupTo(string destination, DatabaseKey key)
    {
        if (File.Exists(destination))
        {
            File.Delete(destination);
        }

        var target = Open(destination, key);
        if (!target.IsSuccess)
        {
            return Result.Fail(target.Error!.Value);
        }

        using var opened = target.Value!;
        try
        {
            Connection.BackupDatabase(opened.Connection);
            return opened.IntegrityCheck();
        }
        catch (SqliteException)
        {
            return Result.Fail(RepoError.MigrationFailed);
        }
    }

    public void Dispose() => Connection.Dispose();

    internal static void Execute(SqliteConnection connection, string sql)
    {
        using var command = connection.CreateCommand();
        command.CommandText = sql;
        command.ExecuteNonQuery();
    }

    internal static void ExecuteBatch(SqliteConnection connection, string sql)
    {
        var result = SQLitePCL.raw.sqlite3_exec(connection.Handle, sql);
        if (result != SQLitePCL.raw.SQLITE_OK)
        {
            throw new SqliteException("SQL batch failed", result);
        }
    }

    internal static long QueryInt64(SqliteConnection connection, string sql)
    {
        using var command = connection.CreateCommand();
        command.CommandText = sql;
        return Convert.ToInt64(command.ExecuteScalar());
    }

    internal static string QueryString(SqliteConnection connection, string sql)
    {
        using var command = connection.CreateCommand();
        command.CommandText = sql;
        return Convert.ToString(command.ExecuteScalar()) ?? string.Empty;
    }
}

internal static class SqliteEngine
{
    private static readonly object Gate = new();
    private static bool _initialized;

    public static void EnsureInitialized()
    {
        lock (Gate)
        {
            if (_initialized)
            {
                return;
            }

            SQLitePCL.Batteries_V2.Init();
            _initialized = true;
        }
    }
}
