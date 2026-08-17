using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Providers;

public enum AccessMode
{
    InteractiveRequired,
}

public sealed record RequestBinding(
    ulong EncounterGeneration,
    string RequestToken,
    string ConfirmedHandle,
    string Format);

public sealed record OfficialLookup(
    AccessMode AccessMode,
    string OfficialUrl,
    RequestBinding Binding);

public static class OfficialDeckProvider
{
    public const string ProviderId = "official_mtgo";
    public const string OfficialDecklistUrl = "https://www.mtgo.com/decklists";

    public static Result<Uri> ValidateOfficialUrl(string value)
    {
        if (!Uri.TryCreate(value, UriKind.Absolute, out var url))
        {
            return Result<Uri>.Fail(RepoError.UnsafeSource);
        }

        if (url.Scheme != Uri.UriSchemeHttps
            || url.UserInfo.Length > 0
            || url.Host is not ("mtgo.com" or "www.mtgo.com"))
        {
            return Result<Uri>.Fail(RepoError.UnsafeSource);
        }

        return Result<Uri>.Ok(url);
    }

    public static Result<OfficialLookup> Lookup(
        bool consentGranted,
        string confirmedHandle,
        string format,
        ulong encounterGeneration)
    {
        if (!consentGranted)
        {
            return Result<OfficialLookup>.Fail(RepoError.ConsentRequired);
        }

        var handle = Identity.HandleNormalization.NormalizeHandle(confirmedHandle);
        if (!handle.IsSuccess)
        {
            return Result<OfficialLookup>.Fail(handle.Error!.Value);
        }

        if (string.IsNullOrWhiteSpace(format))
        {
            return Result<OfficialLookup>.Fail(RepoError.InvalidRequest);
        }

        return Result<OfficialLookup>.Ok(
            new OfficialLookup(
                AccessMode.InteractiveRequired,
                OfficialDecklistUrl,
                new RequestBinding(
                    encounterGeneration,
                    Guid.NewGuid().ToString("N"),
                    handle.Value!.Display,
                    format)));
    }
}
