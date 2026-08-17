using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;
using MTGONotes.Data;

namespace MTGONotes.Data.Tests;

internal sealed class ScopedProtector : IKeyProtector
{
    private readonly byte _scope;
    private readonly bool _failUnprotect;

    public ScopedProtector(byte scope, bool failUnprotect = false)
    {
        _scope = scope;
        _failUnprotect = failUnprotect;
    }

    public Result<byte[]> Protect(ReadOnlySpan<byte> plaintext)
    {
        var protectedBytes = new byte[plaintext.Length + 1];
        protectedBytes[0] = _scope;
        for (var index = 0; index < plaintext.Length; index++)
        {
            protectedBytes[index + 1] = (byte)(plaintext[index] ^ _scope);
        }

        return Result<byte[]>.Ok(protectedBytes);
    }

    public Result<byte[]> Unprotect(ReadOnlySpan<byte> ciphertext)
    {
        if (_failUnprotect || ciphertext.Length == 0 || ciphertext[0] != _scope)
        {
            return Result<byte[]>.Fail(RepoError.KeyUnavailable);
        }

        var plaintext = new byte[ciphertext.Length - 1];
        for (var index = 0; index < plaintext.Length; index++)
        {
            plaintext[index] = (byte)(ciphertext[index + 1] ^ _scope);
        }

        return Result<byte[]>.Ok(plaintext);
    }
}
