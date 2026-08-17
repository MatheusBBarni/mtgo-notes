using Microsoft.UI.Xaml;
using MTGONotes.App.Native;
using MTGONotes.App.Windows;
using MTGONotes.Core.Session;

namespace MTGONotes.App.Host;

public sealed class AppHost
{
    private MainWindow? _main;
    private OverlayWindow? _overlay;
    private CaptureWindow? _capture;
    private HotkeyService? _hotkey;

    public CompanionSession Session { get; } = new();

    public void Start()
    {
        Session.OverlayChanged += (_, view) =>
        {
            _main?.DispatcherQueue.TryEnqueue(() => _main.Bind(view));
            _overlay?.DispatcherQueue.TryEnqueue(() => _overlay.Bind(view));
        };

        _main = new MainWindow(this);
        _overlay = new OverlayWindow(this);
        _capture = new CaptureWindow(this);
        _hotkey = new HotkeyService(_main);
        _ = _hotkey.RegisterCaptureShortcut();

        _main.Closed += (_, _) =>
        {
            _hotkey?.Dispose();
            _overlay?.Close();
            _capture?.Close();
        };

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
