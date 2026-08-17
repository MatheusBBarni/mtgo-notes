using System.Diagnostics;
using MTGONotes.Core.Identity;
using MTGONotes.Core.Live;

namespace MTGONotes.Live;

public sealed class LiveAttachSource : IContextSource, IDisposable
{
    public const string SourceId = "mtgosdk";

    private readonly IMtgoClient _client;
    private readonly TimeSpan _interval;
    private readonly object _gate = new();
    private CancellationTokenSource? _run;
    private Task? _loop;
    private string _providerSession = NewSession();
    private ulong _generation;
    private ulong _sequence;
    private bool _attached;
    private LiveSnapshot? _last;

    public LiveAttachSource(IMtgoClient client, TimeSpan? interval = null)
    {
        _client = client;
        _interval = interval ?? TimeSpan.FromMilliseconds(500);
    }

    public string Id => SourceId;

    public bool IsAttached
    {
        get
        {
            lock (_gate)
            {
                return _attached;
            }
        }
    }

    public string ProviderSession
    {
        get
        {
            lock (_gate)
            {
                return _providerSession;
            }
        }
    }

    public event EventHandler<LiveSnapshot>? SnapshotChanged;

    public Task StartAsync(CancellationToken cancellationToken)
    {
        lock (_gate)
        {
            if (_loop is not null)
            {
                return _loop;
            }

            _run = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            _loop = PollAsync(_run.Token);
            return _loop;
        }
    }

    public async Task StopAsync()
    {
        Task? loop;
        lock (_gate)
        {
            _run?.Cancel();
            loop = _loop;
        }

        if (loop is not null)
        {
            try
            {
                await loop.ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
            }
        }

        lock (_gate)
        {
            _run?.Dispose();
            _run = null;
            _loop = null;
            _attached = false;
        }
    }

    public void PollOnce() => Publish(_client.Read());

    public void Dispose()
    {
        _run?.Cancel();
        _client.Dispose();
    }

    private async Task PollAsync(CancellationToken cancellationToken)
    {
        using var timer = new PeriodicTimer(_interval);
        Publish(_client.Read());
        while (await timer.WaitForNextTickAsync(cancellationToken).ConfigureAwait(false))
        {
            Publish(_client.Read());
        }
    }

    private void Publish(Core.Disclosure.Result<MtgoMatchReading> read)
    {
        LiveSnapshot? snapshot = null;
        lock (_gate)
        {
            if (!read.IsSuccess
                || read.Value is not { } data
                || !data.ProcessAvailable
                || !data.IsLoggedIn)
            {
                _attached = false;
                return;
            }

            if (!_attached)
            {
                _providerSession = NewSession();
                _generation++;
                _sequence = 0;
                _last = null;
            }

            _attached = true;
            var phase = PhaseMapper.FromLive(MatchSignalMapper.FromReading(data));
            OpponentCandidate? opponent = null;
            if (data.Opponent is { } remote)
            {
                var normalized = HandleNormalization.NormalizeHandle(remote.Name);
                var identity = normalized.Value;
                var selfKey = data.CurrentUser is null
                    ? null
                    : HandleNormalization.NormalizeHandle(data.CurrentUser.Name).Value?.Key;
                if (normalized.IsSuccess
                    && identity is not null
                    && (selfKey is null
                        || !string.Equals(identity.Key, selfKey, StringComparison.Ordinal)))
                {
                    opponent = new OpponentCandidate(
                        identity.Display,
                        identity.Key,
                        remote.Id,
                        _generation,
                        _sequence + 1,
                        _providerSession);
                }
            }

            var next = new LiveSnapshot(
                _providerSession,
                _generation,
                _sequence + 1,
                (ulong)Stopwatch.GetTimestamp(),
                opponent,
                phase,
                data.Format,
                data.GameCount == 0 ? null : data.GameCount,
                data.IsComplete ? "complete" : null);

            if (_last is not null
                && _last.ProviderSession == next.ProviderSession
                && _last.SuggestedPhase == next.SuggestedPhase
                && _last.Format == next.Format
                && _last.GameNumber == next.GameNumber
                && _last.Result == next.Result
                && _last.Opponent?.NormalizedHandle == next.Opponent?.NormalizedHandle
                && _last.Opponent?.MtgoUserId == next.Opponent?.MtgoUserId)
            {
                return;
            }

            _sequence = next.Sequence;
            if (next.Opponent is not null)
            {
                next = next with
                {
                    Opponent = next.Opponent with { Sequence = _sequence },
                };
            }

            _last = next;
            snapshot = next;
        }

        if (snapshot is not null)
        {
            SnapshotChanged?.Invoke(this, snapshot);
        }
    }

    private static string NewSession() => "mtgosdk-" + Guid.NewGuid().ToString("N");
}
