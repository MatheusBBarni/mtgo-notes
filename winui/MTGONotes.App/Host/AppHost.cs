using Microsoft.UI.Xaml;
using MTGONotes.App.Live;
using MTGONotes.App.Native;
using MTGONotes.App.Windows;
using MTGONotes.Core.Session;
using MTGONotes.Data;
using MTGONotes.Live;

namespace MTGONotes.App.Host;

public sealed class AppHost
{
    private MainWindow? _main;
    private OverlayWindow? _overlay;
    private CaptureWindow? _capture;
    private HotkeyService? _hotkey;
    private CancellationTokenSource? _liveRun;

    public CompanionSession Session { get; }

    public LiveAttachSource Live { get; }

    public AppHost()
    {
        Session = CreateSession();
        Live = new LiveAttachSource(new SdkMtgoClient());
    }

    private static CompanionSession CreateSession()
    {
        var root = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "MTGONotes");
        Directory.CreateDirectory(root);
        var opened = NotebookBootstrap.Initialize(
            Path.Combine(root, "notebook.db"),
            Path.Combine(root, "notebook.key"),
            new CurrentUserDpapi());
        return opened.IsSuccess ? new CompanionSession(opened.Value) : new CompanionSession();
    }

    public void Start()
    {
        Session.OverlayChanged += (_, view) =>
        {
            _main?.DispatcherQueue.TryEnqueue(() => _main.Bind(view));
            _overlay?.DispatcherQueue.TryEnqueue(() => _overlay.Bind(view));
        };
        Live.SnapshotChanged += (_, snapshot) =>
        {
            Session.ApplySnapshot(snapshot);
            _main?.DispatcherQueue.TryEnqueue(() => _main.Bind(Session.CurrentView));
            _overlay?.DispatcherQueue.TryEnqueue(() => _overlay.Bind(Session.CurrentView));
        };

        _main = new MainWindow(this);
        _overlay = new OverlayWindow(this);
        _capture = new CaptureWindow(this);
        _hotkey = new HotkeyService(_main);
        _ = _hotkey.RegisterCaptureShortcut();

        _main.Closed += (_, _) =>
        {
            _liveRun?.Cancel();
            Live.Dispose();
            _hotkey?.Dispose();
            _overlay?.Close();
            _capture?.Close();
        };

        _liveRun = new CancellationTokenSource();
        _ = Live.StartAsync(_liveRun.Token);
        _main.Activate();
        _overlay.ShowPassive();
    }

    public void OpenCapture() => _capture?.Open();

    public void ToggleOverlay()
    {
        if (_overlay is null)
        {
            return;
        }

        if (_overlay.AppWindow.IsVisible)
        {
            _overlay.AppWindow.Hide();
        }
        else
        {
            _overlay.ShowPassive();
        }
    }
}
