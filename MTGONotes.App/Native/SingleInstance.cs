namespace MTGONotes.App.Native;

internal sealed class SingleInstance : IDisposable
{
    private const string MutexName = @"Local\MTGONotes.App";
    private const string ShowEventName = @"Local\MTGONotes.App.Show";

    private readonly Mutex _mutex;
    private readonly EventWaitHandle _show;
    private readonly bool _owned;

    private SingleInstance(Mutex mutex, EventWaitHandle show, bool owned)
    {
        _mutex = mutex;
        _show = show;
        _owned = owned;
    }

    public EventWaitHandle ShowSignal => _show;

    public static SingleInstance Claim()
    {
        var mutex = new Mutex(true, MutexName, out var owned);
        var show = new EventWaitHandle(false, EventResetMode.AutoReset, ShowEventName);
        if (!owned)
        {
            show.Set();
        }

        return new SingleInstance(mutex, show, owned);
    }

    public bool IsOwner => _owned;

    public void Dispose()
    {
        if (_owned)
        {
            _mutex.ReleaseMutex();
        }

        _mutex.Dispose();
        _show.Dispose();
    }
}
