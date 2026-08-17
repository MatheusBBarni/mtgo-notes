using Microsoft.UI.Xaml;
using MTGONotes.App.Live;
using MTGONotes.App.Native;
using MTGONotes.App.Windows;
using MTGONotes.Core.Portability;
using MTGONotes.Core.Session;
using MTGONotes.Core.Settings;
using MTGONotes.Data;
using MTGONotes.Live;

namespace MTGONotes.App.Host;

public sealed class AppHost
{
    private MainWindow? _main;
    private OverlayWindow? _overlay;
    private CaptureWindow? _capture;
    private HotkeyService? _hotkey;
    private WindowMessages? _messages;
    private TrayIconService? _tray;
    private CancellationTokenSource? _liveRun;
    private bool _quitRequested;

    public CompanionSession Session { get; }

    public LiveAttachSource Live { get; }

    public AppSettings Settings => SettingsStore.Current;

    public SettingsStore SettingsStore { get; }

    public OperationCoordinator Operations { get; } = new();

    public AppHost()
    {
        var root = DataRoot();
        SettingsStore = new SettingsStore(Path.Combine(root, "settings.json"));
        _ = SettingsStore.Load();
        Session = CreateSession(root);
        Live = new LiveAttachSource(new SdkMtgoClient());
    }

    public void Start()
    {
        Session.OverlayChanged += (_, view) => DispatchBind(view);
        Live.SnapshotChanged += (_, snapshot) =>
        {
            if (Settings.LiveAttachEnabled && !Session.DetectionPaused)
            {
                Session.ApplySnapshot(snapshot);
            }

            DispatchBind(Session.CurrentView);
        };

        _main = new MainWindow(this);
        _overlay = new OverlayWindow(this);
        _capture = new CaptureWindow(this);
        _hotkey = new HotkeyService(_main);
        _messages = new WindowMessages(_main);
        _messages.Install();
        _messages.HotkeyPressed += id =>
        {
            if (id == HotkeyService.CaptureHotkeyId)
            {
                OpenCapture();
            }
        };
        _messages.TrayMessage += message =>
        {
            if (message is WindowMessages.WmRButtonUp)
            {
                HandleTrayCommand(_tray?.ShowMenu() ?? 0);
            }
            else if (message is WindowMessages.WmLButtonDblClk)
            {
                ShowMain();
            }
        };
        _ = _hotkey.RegisterCaptureShortcut();
        ApplyChrome();

        _main.AppWindow.Closing += (_, args) =>
        {
            if (_quitRequested || !Settings.TrayEnabled)
            {
                return;
            }

            args.Cancel = true;
            _main.AppWindow.Hide();
        };

        _ = new Thread(() =>
        {
            while (true)
            {
                ((App)Application.Current).SingleInstance?.ShowSignal.WaitOne();
                _main?.DispatcherQueue.TryEnqueue(ShowMain);
            }
        })
        {
            IsBackground = true,
        }.Start();

        _liveRun = new CancellationTokenSource();
        if (Settings.LiveAttachEnabled)
        {
            _ = Live.StartAsync(_liveRun.Token);
        }

        _main.Activate();
        if (Settings.OverlayEnabled)
        {
            _overlay.ShowPassive();
        }

        _ = _main.MaybeOnboardAsync();
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

    public void SaveSettings()
    {
        _ = SettingsStore.Save(Settings);
        Autostart.Apply(Settings.LaunchWithWindows);
        ApplyChrome();
        if (!Settings.LiveAttachEnabled)
        {
            _ = Session.PauseDetection(true);
        }
    }

    public void Quit()
    {
        _quitRequested = true;
        _liveRun?.Cancel();
        Live.Dispose();
        _hotkey?.Dispose();
        _messages?.Dispose();
        _tray?.Dispose();
        _overlay?.Close();
        _capture?.Close();
        _main?.Close();
    }

    private void ApplyChrome()
    {
        if (_main is null)
        {
            return;
        }

        if (Settings.TrayEnabled && _tray is null)
        {
            _tray = new TrayIconService(OverlayHwnd.Handle(_main));
            _tray.Add("MTGO Opponent Notes");
        }
        else if (!Settings.TrayEnabled)
        {
            _tray?.Dispose();
            _tray = null;
        }

        if (!Settings.OverlayEnabled)
        {
            _overlay?.AppWindow.Hide();
        }
        else
        {
            _overlay?.ShowPassive();
        }
    }

    private void HandleTrayCommand(int command)
    {
        switch (command)
        {
            case TrayIconService.CommandOpen:
                ShowMain();
                break;
            case TrayIconService.CommandOverlay:
                ToggleOverlay();
                break;
            case TrayIconService.CommandPause:
                _ = Session.PauseDetection(!Session.DetectionPaused);
                break;
            case TrayIconService.CommandQuit:
                Quit();
                break;
        }
    }

    private void ShowMain() => _main?.Activate();

    private void DispatchBind(Core.Disclosure.OverlayView view)
    {
        _main?.DispatcherQueue.TryEnqueue(() => _main.Bind(view));
        _overlay?.DispatcherQueue.TryEnqueue(() => _overlay.Bind(view));
    }

    private static CompanionSession CreateSession(string root)
    {
        var opened = NotebookBootstrap.Initialize(
            Path.Combine(root, "notebook.db"),
            Path.Combine(root, "notebook.key"),
            new CurrentUserDpapi());
        return opened.IsSuccess ? new CompanionSession(opened.Value) : new CompanionSession();
    }

    private static string DataRoot()
    {
        var root = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "MTGONotes");
        Directory.CreateDirectory(root);
        return root;
    }
}
