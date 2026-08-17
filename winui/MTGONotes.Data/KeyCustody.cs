using System.Security.Cryptography;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Data;

public sealed class DatabaseKey : IDisposable
{
    public const int Size = 32;

    private readonly byte[] _bytes;

    private DatabaseKey(byte[] bytes) => _bytes = bytes;

    public static DatabaseKey Generate()
    {
        var bytes = new byte[Size];
        RandomNumberGenerator.Fill(bytes);
        return new DatabaseKey(bytes);
    }

    public static Result<DatabaseKey> FromBytes(byte[] bytes)
    {
        if (bytes.Length != Size)
        {
            return Result<DatabaseKey>.Fail(RepoError.KeyUnavailable);
        }

        var copy = new byte[Size];
        Buffer.BlockCopy(bytes, 0, copy, 0, Size);
        return Result<DatabaseKey>.Ok(new DatabaseKey(copy));
    }

    public ReadOnlySpan<byte> Expose() => _bytes;

    public string ToSqlCipherHex() => Convert.ToHexString(_bytes).ToLowerInvariant();

    public byte[] ToArray()
    {
        var copy = new byte[Size];
        Buffer.BlockCopy(_bytes, 0, copy, 0, Size);
        return copy;
    }

    public void Dispose() => CryptographicOperations.ZeroMemory(_bytes);
}

public interface IKeyProtector
{
    Result<byte[]> Protect(ReadOnlySpan<byte> plaintext);

    Result<byte[]> Unprotect(ReadOnlySpan<byte> ciphertext);
}

public sealed class CurrentUserDpapi : IKeyProtector
{
    public Result<byte[]> Protect(ReadOnlySpan<byte> plaintext)
    {
        if (!OperatingSystem.IsWindows())
        {
            return Result<byte[]>.Fail(RepoError.KeyUnavailable);
        }

        try
        {
            return Result<byte[]>.Ok(
                ProtectedData.Protect(plaintext.ToArray(), null, DataProtectionScope.CurrentUser));
        }
        catch (CryptographicException)
        {
            return Result<byte[]>.Fail(RepoError.KeyUnavailable);
        }
    }

    public Result<byte[]> Unprotect(ReadOnlySpan<byte> ciphertext)
    {
        if (!OperatingSystem.IsWindows())
        {
            return Result<byte[]>.Fail(RepoError.KeyUnavailable);
        }

        try
        {
            return Result<byte[]>.Ok(
                ProtectedData.Unprotect(ciphertext.ToArray(), null, DataProtectionScope.CurrentUser));
        }
        catch (CryptographicException)
        {
            return Result<byte[]>.Fail(RepoError.KeyUnavailable);
        }
    }
}

public sealed class KeyCustody
{
    private readonly string _sealedKeyPath;
    private readonly string _databasePath;
    private readonly IKeyProtector _protector;

    public KeyCustody(string sealedKeyPath, string databasePath, IKeyProtector protector)
    {
        _sealedKeyPath = sealedKeyPath;
        _databasePath = databasePath;
        _protector = protector;
    }

    public Result<DatabaseKey> LoadOrCreate()
    {
        if (File.Exists(_sealedKeyPath))
        {
            var sealedBytes = File.ReadAllBytes(_sealedKeyPath);
            var plaintext = _protector.Unprotect(sealedBytes);
            if (!plaintext.IsSuccess)
            {
                return Result<DatabaseKey>.Fail(plaintext.Error!.Value);
            }

            return DatabaseKey.FromBytes(plaintext.Value!);
        }

        if (File.Exists(_databasePath))
        {
            return Result<DatabaseKey>.Fail(RepoError.KeyUnavailable);
        }

        var key = DatabaseKey.Generate();
        var protectedBytes = _protector.Protect(key.Expose());
        if (!protectedBytes.IsSuccess)
        {
            key.Dispose();
            return Result<DatabaseKey>.Fail(protectedBytes.Error!.Value);
        }

        var written = WriteAtomicPrivate(_sealedKeyPath, protectedBytes.Value!);
        if (!written.IsSuccess)
        {
            key.Dispose();
            return Result<DatabaseKey>.Fail(written.Error!.Value);
        }

        return Result<DatabaseKey>.Ok(key);
    }

    private static Result WriteAtomicPrivate(string path, byte[] contents)
    {
        try
        {
            var directory = Path.GetDirectoryName(path);
            if (string.IsNullOrEmpty(directory))
            {
                return Result.Fail(RepoError.KeyUnavailable);
            }

            Directory.CreateDirectory(directory);
            var partial = path + ".partial";
            if (File.Exists(partial))
            {
                File.Delete(partial);
            }

            using (var file = new FileStream(
                       partial,
                       FileMode.CreateNew,
                       FileAccess.Write,
                       FileShare.None,
                       4096,
                       FileOptions.WriteThrough))
            {
                file.Write(contents);
                file.Flush(true);
            }

            if (OperatingSystem.IsMacOS() || OperatingSystem.IsLinux())
            {
                File.SetUnixFileMode(partial, UnixFileMode.UserRead | UnixFileMode.UserWrite);
            }

            File.Move(partial, path, overwrite: false);
            return Result.Ok();
        }
        catch (IOException)
        {
            return Result.Fail(RepoError.KeyUnavailable);
        }
    }
}
