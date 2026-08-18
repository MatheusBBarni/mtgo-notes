using System.Globalization;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Tests;

public sealed class PresentationTextTests
{
    [Theory]
    [InlineData(InternalPhase.Idle, "Idle")]
    [InlineData(InternalPhase.Candidate, "Candidate")]
    [InlineData(InternalPhase.PreMatch, "Pre-match")]
    [InlineData(InternalPhase.InGameRestricted, "In game")]
    [InlineData(InternalPhase.BetweenGames, "Between games")]
    [InlineData(InternalPhase.CompletionPending, "Completing")]
    [InlineData(InternalPhase.Finished, "Finished")]
    [InlineData(InternalPhase.Incomplete, "Incomplete")]
    public void FormatPhase_uses_stable_labels(InternalPhase phase, string expected)
    {
        Assert.Equal(expected, PresentationText.FormatPhase(phase));
    }

    [Fact]
    public void Encounter_heading_falls_back_when_unconfirmed()
    {
        Assert.Equal(
            "Idle — No confirmed opponent",
            PresentationText.FormatEncounterHeading(InternalPhase.Idle, null));
        Assert.Equal(
            "Pre-match — Alice",
            PresentationText.FormatEncounterHeading(InternalPhase.PreMatch, "Alice"));
    }

    [Fact]
    public void Overlay_heading_stays_compact_when_unconfirmed()
    {
        Assert.Equal(
            "In game — unconfirmed",
            PresentationText.FormatOverlayHeading(InternalPhase.InGameRestricted, null));
    }

    [Fact]
    public void Timestamp_uses_current_culture()
    {
        var original = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("en-US");
            var stamp = DateTimeOffset.FromUnixTimeMilliseconds(1_700_000_000_000);
            var expected = stamp.ToLocalTime().ToString("g", CultureInfo.CurrentCulture);
            Assert.Equal(expected, PresentationText.FormatTimestamp(1_700_000_000_000));
        }
        finally
        {
            CultureInfo.CurrentCulture = original;
        }
    }
}
