using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Live;

public sealed class FakeMtgoClient : IMtgoClient
{
    public MtgoMatchReading Reading { get; set; } = new(
        false,
        false,
        null,
        null,
        null,
        null,
        MtgoMatchFlags.Invalid,
        false,
        0,
        false);

    public Result<MtgoMatchReading> Read() => Result<MtgoMatchReading>.Ok(Reading);

    public void Dispose()
    {
    }

    public void AttachLoggedIn(string selfName, long selfId, string version = "1.0.0")
    {
        Reading = Reading with
        {
            ProcessAvailable = true,
            IsLoggedIn = true,
            ClientVersion = version,
            CurrentUser = new MtgoUser(selfName, selfId),
        };
    }

    public void SetMatch(
        string opponentName,
        long opponentId,
        long flags,
        string? format = "Modern",
        bool hasCurrentGame = false,
        int gameCount = 0,
        bool complete = false)
    {
        Reading = Reading with
        {
            ProcessAvailable = true,
            IsLoggedIn = true,
            Opponent = new MtgoUser(opponentName, opponentId),
            MatchStateFlags = flags,
            Format = format,
            HasCurrentGame = hasCurrentGame,
            GameCount = gameCount,
            IsComplete = complete,
        };
    }

    public void Disconnect()
    {
        Reading = Reading with
        {
            ProcessAvailable = false,
            IsLoggedIn = false,
            Opponent = null,
            MatchStateFlags = MtgoMatchFlags.Invalid,
            HasCurrentGame = false,
            IsComplete = false,
        };
    }
}

public sealed class UnavailableMtgoClient : IMtgoClient
{
    public Result<MtgoMatchReading> Read() => Result<MtgoMatchReading>.Fail(RepoError.ProviderUnavailable);

    public void Dispose()
    {
    }
}
