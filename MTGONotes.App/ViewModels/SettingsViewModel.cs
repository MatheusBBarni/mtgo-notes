using System.Diagnostics;
using MTGONotes.App.Host;
using MTGONotes.Core.Domain;
using MTGONotes.Core.Providers;
using MTGONotes.Core.Settings;

namespace MTGONotes.App.ViewModels;

public sealed class SettingsViewModel : ViewModel
{
    private readonly AppHost _host;
    private bool _loading = true;
    private bool _liveAttachEnabled;
    private bool _officialDeckConsent;
    private bool _overlayEnabled;
    private bool _trayEnabled;
    private bool _launchWithWindows;
    private int _themeIndex;
    private string _statusMessage = string.Empty;
    private bool _isStatusOpen;
    private bool _isStatusError;

    public SettingsViewModel(AppHost host)
    {
        _host = host;
        OpenOfficialDecksCommand = new RelayCommand(OpenOfficialDecks);
        Reload();
    }

    public bool LiveAttachEnabled
    {
        get => _liveAttachEnabled;
        set => Persist(ref _liveAttachEnabled, value, () => _host.Settings.LiveAttachEnabled = value);
    }

    public bool OfficialDeckConsent
    {
        get => _officialDeckConsent;
        set => Persist(ref _officialDeckConsent, value, () => _host.Settings.OfficialDeckConsent = value);
    }

    public bool OverlayEnabled
    {
        get => _overlayEnabled;
        set => Persist(ref _overlayEnabled, value, () => _host.Settings.OverlayEnabled = value);
    }

    public bool TrayEnabled
    {
        get => _trayEnabled;
        set => Persist(ref _trayEnabled, value, () => _host.Settings.TrayEnabled = value);
    }

    public bool LaunchWithWindows
    {
        get => _launchWithWindows;
        set => Persist(ref _launchWithWindows, value, () => _host.Settings.LaunchWithWindows = value);
    }

    public int ThemeIndex
    {
        get => _themeIndex;
        set
        {
            if (value < 0 || !SetProperty(ref _themeIndex, value) || _loading)
            {
                return;
            }

            _host.Settings.Theme = AppTheme.FromIndex(value);
            _host.SaveSettings();
            SetStatus("Settings saved.", isError: false);
        }
    }

    public string StatusMessage
    {
        get => _statusMessage;
        set => SetProperty(ref _statusMessage, value);
    }

    public bool IsStatusOpen
    {
        get => _isStatusOpen;
        set => SetProperty(ref _isStatusOpen, value);
    }

    public bool IsStatusError
    {
        get => _isStatusError;
        set => SetProperty(ref _isStatusError, value);
    }

    public RelayCommand OpenOfficialDecksCommand { get; }

    public void Reload()
    {
        _loading = true;
        LiveAttachEnabled = _host.Settings.LiveAttachEnabled;
        OfficialDeckConsent = _host.Settings.OfficialDeckConsent;
        OverlayEnabled = _host.Settings.OverlayEnabled;
        TrayEnabled = _host.Settings.TrayEnabled;
        LaunchWithWindows = _host.Settings.LaunchWithWindows;
        ThemeIndex = AppTheme.IndexOf(_host.Settings.Theme);
        _loading = false;
    }

    private void OpenOfficialDecks()
    {
        if (!_host.Settings.OfficialDeckConsent)
        {
            SetStatus(RepoError.ConsentRequired.ToAppError().Message, isError: true);
            return;
        }

        var url = OfficialDeckProvider.ValidateOfficialUrl(OfficialDeckProvider.OfficialDecklistUrl);
        if (!url.IsSuccess)
        {
            SetStatus(url.Error!.Value.ToAppError().Message, isError: true);
            return;
        }

        Process.Start(new ProcessStartInfo(url.Value!.ToString()) { UseShellExecute = true });
        SetStatus("Opened official MTGO decklists.", isError: false);
    }

    private void Persist(ref bool field, bool value, Action assign)
    {
        if (!SetProperty(ref field, value) || _loading)
        {
            return;
        }

        assign();
        _host.SaveSettings();
        SetStatus("Settings saved.", isError: false);
    }

    private void SetStatus(string message, bool isError)
    {
        StatusMessage = message;
        IsStatusError = isError;
        IsStatusOpen = !string.IsNullOrWhiteSpace(message);
    }
}
