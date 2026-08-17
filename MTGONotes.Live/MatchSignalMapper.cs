using MTGONotes.Core.Live;

namespace MTGONotes.Live;

public static class MatchSignalMapper
{
    public static LiveMatchSignal FromReading(MtgoMatchReading reading)
    {
        if (!reading.ProcessAvailable || !reading.IsLoggedIn)
        {
            return LiveMatchSignal.None;
        }

        if (reading.IsComplete || Has(reading.MatchStateFlags, MtgoMatchFlags.MatchCompleted))
        {
            return LiveMatchSignal.Finished;
        }

        if (Has(reading.MatchStateFlags, MtgoMatchFlags.Sideboarding))
        {
            return LiveMatchSignal.Sideboarding;
        }

        if (reading.HasCurrentGame || Has(reading.MatchStateFlags, MtgoMatchFlags.GameStarted))
        {
            return LiveMatchSignal.InGame;
        }

        if (reading.Opponent is not null
            || Has(reading.MatchStateFlags, MtgoMatchFlags.Joined)
            || Has(reading.MatchStateFlags, MtgoMatchFlags.AwaitingPlayerStart)
            || Has(reading.MatchStateFlags, MtgoMatchFlags.AwaitingHostStart))
        {
            return LiveMatchSignal.Pairings;
        }

        return reading.MatchStateFlags == MtgoMatchFlags.Invalid
            ? LiveMatchSignal.None
            : LiveMatchSignal.Unknown;
    }

    private static bool Has(long flags, long bit) => (flags & bit) == bit;
}
