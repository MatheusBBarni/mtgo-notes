using MTGOSDK.API;
using MTGOSDK.API.Play;
using MTGOSDK.API.Users;
using MTGONotes.Core.Disclosure;
using MTGONotes.Live;

namespace MTGONotes.App.Live;

public sealed class SdkMtgoClient : IMtgoClient
{
    private Client? _client;

    public Result<MtgoMatchReading> Read()
    {
        try
        {
            _client ??= new Client();
            var version = Client.Version?.ToString();
            if (!_client.IsLoggedIn || _client.CurrentUser is null)
            {
                return Result<MtgoMatchReading>.Ok(
                    new MtgoMatchReading(
                        true,
                        false,
                        version,
                        null,
                        null,
                        null,
                        MtgoMatchFlags.Invalid,
                        false,
                        0,
                        false));
            }

            var self = new MtgoUser(_client.CurrentUser.Name, _client.CurrentUser.Id);
            Match? match = null;
            foreach (var joined in EventManager.JoinedEvents)
            {
                if (joined is Match candidate)
                {
                    match = candidate;
                    if (!candidate.IsComplete)
                    {
                        break;
                    }
                }
            }

            if (match is null)
            {
                return Result<MtgoMatchReading>.Ok(
                    new MtgoMatchReading(
                        true,
                        true,
                        version,
                        self,
                        null,
                        null,
                        MtgoMatchFlags.Invalid,
                        false,
                        0,
                        false));
            }

            MtgoUser? opponent = null;
            foreach (var player in match.Players)
            {
                if (player.Id != self.Id
                    && !string.Equals(player.Name, self.Name, StringComparison.OrdinalIgnoreCase))
                {
                    opponent = new MtgoUser(player.Name, player.Id);
                    break;
                }
            }

            return Result<MtgoMatchReading>.Ok(
                new MtgoMatchReading(
                    true,
                    true,
                    version,
                    self,
                    opponent,
                    match.Format?.Name,
                    (long)match.State,
                    match.CurrentGame is not null,
                    match.Games.Count,
                    match.IsComplete));
        }
        catch (Exception)
        {
            DisposeClient();
            return Result<MtgoMatchReading>.Ok(
                new MtgoMatchReading(
                    false,
                    false,
                    null,
                    null,
                    null,
                    null,
                    MtgoMatchFlags.Invalid,
                    false,
                    0,
                    false));
        }
    }

    public void Dispose() => DisposeClient();

    private void DisposeClient()
    {
        try
        {
            _client?.Dispose();
        }
        catch (Exception)
        {
        }

        _client = null;
    }
}
