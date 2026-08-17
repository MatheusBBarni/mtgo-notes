using System.Reflection;
using System.Security.Cryptography;
using System.Text;

namespace MTGONotes.Data;

public static class SchemaSql
{
    public const long SchemaVersion = 2;

    public static string Initial { get; } = Load("v1_initial.sql");

    public static string RetiredTags { get; } = Load("v2_retired_tags.sql");

    public static string Checksum(string sql)
    {
        var hash = SHA256.HashData(Encoding.UTF8.GetBytes(sql));
        return Convert.ToHexString(hash).ToLowerInvariant();
    }

    private static string Load(string name)
    {
        var assembly = typeof(SchemaSql).Assembly;
        var resource = assembly
            .GetManifestResourceNames()
            .Single(item => item.EndsWith(name, StringComparison.Ordinal));
        using var stream = assembly.GetManifestResourceStream(resource)
            ?? throw new InvalidOperationException($"Missing SQL resource {name}.");
        using var reader = new StreamReader(stream, Encoding.UTF8, detectEncodingFromByteOrderMarks: false);
        return reader.ReadToEnd();
    }
}
