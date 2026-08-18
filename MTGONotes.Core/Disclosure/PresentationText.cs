using System.Globalization;
using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Disclosure;

public static class PresentationText
{
    public static string FormatPhase(InternalPhase phase) =>
        phase switch
        {
            InternalPhase.Idle => "Idle",
            InternalPhase.Candidate => "Candidate",
            InternalPhase.PreMatch => "Pre-match",
            InternalPhase.InGameRestricted => "In game",
            InternalPhase.BetweenGames => "Between games",
            InternalPhase.CompletionPending => "Completing",
            InternalPhase.Finished => "Finished",
            InternalPhase.Incomplete => "Incomplete",
            _ => phase.ToString(),
        };

    public static string FormatEncounterHeading(InternalPhase phase, string? handle) =>
        $"{FormatPhase(phase)} — {handle ?? "No confirmed opponent"}";

    public static string FormatOverlayHeading(InternalPhase phase, string? handle) =>
        $"{FormatPhase(phase)} — {handle ?? "unconfirmed"}";

    public static string FormatTimestamp(long unixMs) =>
        DateTimeOffset.FromUnixTimeMilliseconds(unixMs)
            .ToLocalTime()
            .ToString("g", CultureInfo.CurrentCulture);
}
