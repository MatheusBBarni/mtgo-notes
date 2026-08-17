using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;
using MTGONotes.Core.Notebook;

namespace MTGONotes.Core.Portability;

public sealed class OperationCoordinator
{
    private readonly object _gate = new();
    private string? _busy;

    public Result Begin(string kind)
    {
        lock (_gate)
        {
            if (_busy is not null)
            {
                return Result.Fail(RepoError.OperationBusy);
            }

            _busy = kind;
            return Result.Ok();
        }
    }

    public void End()
    {
        lock (_gate)
        {
            _busy = null;
        }
    }

    public string? BusyKind
    {
        get
        {
            lock (_gate)
            {
                return _busy;
            }
        }
    }
}

public static class TextExporter
{
    public static string Render(NotebookDump dump)
    {
        var builder = new StringBuilder();
        builder.AppendLine("MTGO Opponent Notes export");
        builder.AppendLine("UNENCRYPTED. Do not share this file.");
        builder.AppendLine();
        foreach (var profile in dump.Profiles.OrderBy(item => item.Handle, StringComparer.OrdinalIgnoreCase))
        {
            builder.AppendLine($"# {profile.Handle}");
            foreach (var encounter in dump.Encounters.Where(item => item.ProfileId == profile.Id)
                         .OrderBy(item => item.StartedAt))
            {
                builder.AppendLine($"  Encounter {encounter.Id} ({encounter.Status}, {encounter.Phase})");
                foreach (var note in encounter.Observations)
                {
                    builder.AppendLine($"    - {note.Text}");
                }
            }

            builder.AppendLine();
        }

        return builder.ToString();
    }
}

public static class NotebookBackup
{
    private const string Magic = "MTGONOTES1";

    public static Result Write(string path, NotebookDump dump, string passphrase)
    {
        if (string.IsNullOrEmpty(passphrase))
        {
            return Result.Fail(RepoError.InvalidRequest);
        }

        try
        {
            var json = JsonSerializer.Serialize(dump);
            var salt = RandomNumberGenerator.GetBytes(16);
            var nonce = RandomNumberGenerator.GetBytes(12);
            var key = Rfc2898DeriveBytes.Pbkdf2(passphrase, salt, 200_000, HashAlgorithmName.SHA256, 32);
            var plaintext = Encoding.UTF8.GetBytes(json);
            var ciphertext = new byte[plaintext.Length];
            var tag = new byte[16];
            using var aes = new AesGcm(key, 16);
            aes.Encrypt(nonce, plaintext, ciphertext, tag);
            using var output = new FileStream(path + ".partial", FileMode.Create, FileAccess.Write, FileShare.None);
            output.Write(Encoding.ASCII.GetBytes(Magic));
            output.Write(salt);
            output.Write(nonce);
            output.Write(tag);
            output.Write(ciphertext);
            output.Flush(true);
            output.Dispose();
            File.Move(path + ".partial", path, overwrite: true);
            return Result.Ok();
        }
        catch (IOException)
        {
            return Result.Fail(RepoError.DestinationUnwritable);
        }
        catch (CryptographicException)
        {
            return Result.Fail(RepoError.SaveFailed);
        }
    }

    public static Result<NotebookDump> Read(string path, string passphrase)
    {
        try
        {
            var bytes = File.ReadAllBytes(path);
            var magic = Encoding.ASCII.GetBytes(Magic);
            if (bytes.Length < magic.Length + 16 + 12 + 16
                || !bytes.AsSpan(0, magic.Length).SequenceEqual(magic))
            {
                return Result<NotebookDump>.Fail(RepoError.InvalidBackup);
            }

            var salt = bytes.AsSpan(magic.Length, 16).ToArray();
            var nonce = bytes.AsSpan(magic.Length + 16, 12).ToArray();
            var tag = bytes.AsSpan(magic.Length + 28, 16).ToArray();
            var ciphertext = bytes[(magic.Length + 44)..];
            var key = Rfc2898DeriveBytes.Pbkdf2(passphrase, salt, 200_000, HashAlgorithmName.SHA256, 32);
            var plaintext = new byte[ciphertext.Length];
            using var aes = new AesGcm(key, 16);
            aes.Decrypt(nonce, ciphertext, tag, plaintext);
            var dump = JsonSerializer.Deserialize<NotebookDump>(Encoding.UTF8.GetString(plaintext));
            return dump is null
                ? Result<NotebookDump>.Fail(RepoError.InvalidBackup)
                : Result<NotebookDump>.Ok(dump);
        }
        catch (CryptographicException)
        {
            return Result<NotebookDump>.Fail(RepoError.WrongPassphrase);
        }
        catch (Exception ex) when (ex is IOException or JsonException)
        {
            return Result<NotebookDump>.Fail(RepoError.InvalidBackup);
        }
    }
}

public static class DiagnosticsBundle
{
    public static string Preview(bool liveAttached, string? busyKind, long schemaVersion) =>
        $"""
        liveAttached={liveAttached}
        operation={busyKind ?? "none"}
        schemaVersion={schemaVersion}
        """;
}
