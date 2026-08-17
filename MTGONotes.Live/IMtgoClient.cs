using MTGONotes.Core.Disclosure;

namespace MTGONotes.Live;

public sealed record MtgoUser(string Name, long Id);

public sealed record MtgoMatchReading(
    bool ProcessAvailable,
    bool IsLoggedIn,
    string? ClientVersion,
    MtgoUser? CurrentUser,
    MtgoUser? Opponent,
    string? Format,
    long MatchStateFlags,
    bool HasCurrentGame,
    int GameCount,
    bool IsComplete);

public interface IMtgoClient : IDisposable
{
    Result<MtgoMatchReading> Read();
}

public static class MtgoMatchFlags
{
    public const long Invalid = 0L;
    public const long Joined = 2L;
    public const long AwaitingPlayerStart = 0x200L;
    public const long AwaitingHostStart = 0x400L;
    public const long GameStarted = 0x1000L;
    public const long Sideboarding = 0x4000L;
    public const long MatchCompleted = 0x100000L;
    public const long GameClosed = 0x20000000000L;
}
