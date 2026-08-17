using System.Globalization;
using System.Text;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Identity;

public sealed record NormalizedIdentity(string Display, string Key);

public static class HandleNormalization
{
    public const int MaxHandleChars = 128;

    public static Result<NormalizedIdentity> NormalizeHandle(string value) =>
        Normalize(value, MaxHandleChars, RepoError.InvalidHandle);

    public static Result<NormalizedIdentity> NormalizeTag(string value) =>
        Normalize(value, 128, RepoError.InvalidTag);

    public static Result<NormalizedIdentity> NormalizeCardName(string value) =>
        Normalize(value, 256, RepoError.InvalidCard);

    private static Result<NormalizedIdentity> Normalize(
        string value,
        int maxChars,
        RepoError error)
    {
        var display = value.Trim().Trim('|', '•', '·').Trim();
        var valid = display.Length > 0
            && display.EnumerateRunes().Count() <= maxChars
            && !display.EnumerateRunes().Any(rune =>
                Rune.IsControl(rune) || rune.Value is '<' or '>');
        if (!valid)
        {
            return Result<NormalizedIdentity>.Fail(error);
        }

        var key = display.Normalize(NormalizationForm.FormKC).ToLower(CultureInfo.InvariantCulture);
        return key.Length == 0
            ? Result<NormalizedIdentity>.Fail(error)
            : Result<NormalizedIdentity>.Ok(new NormalizedIdentity(display, key));
    }
}
