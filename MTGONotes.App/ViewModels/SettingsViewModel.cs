using System.Diagnostics;
using CommunityToolkit.Mvvm.Input;
using MTGONotes.App.Host;
using MTGONotes.Core.Domain;
using MTGONotes.Core.Providers;
using MTGONotes.Core.Settings;

namespace MTGONotes.App.ViewModels;

public sealed partial class SettingsViewModel : ViewModel
{
    private readonly AppHost _host;
    private bool _loading = true;

    public SettingsViewModel(AppHost host)
    {
        _host = host;
        Reload();
    }

    [ObservableProperty]
    public partial bool LiveAttachEnabled { get; set; }

    [ObservableProperty]
    public partial bool OfficialDeckConsent { get; set; }

    [ObservableProperty]
    public partial bool OverlayEnabled { get; set; }

    [ObservableProperty]
    public partial bool TrayEnabled { get; set; }

    [ObservableProperty]
    public partial bool LaunchWithWindows { get; set; }

    [ObservableProperty]
    public partial int ThemeIndex { get; set; }

    [ObservableProperty]
    public partial string StatusMessage { get; set; } = string.Empty;

    [ObservableProperty]
    public partial bool IsStatusOpen { get; set; }

    [ObservableProperty]
    public partial bool IsStatusError { get; set; }

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

    [RelayCommand]
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

    partial void OnLiveAttachEnabledChanged(bool value) => Persist(value, () => _host.Settings.LiveAttachEnabled = value);

    partial void OnOfficialDeckConsentChanged(bool value) => Persist(value, () => _host.Settings.OfficialDeckConsent = value);

    partial void OnOverlayEnabledChanged(bool value) => Persist(value, () => _host.Settings.OverlayEnabled = value);

    partial void OnTrayEnabledChanged(bool value) => Persist(value, () => _host.Settings.TrayEnabled = value);

    partial void OnLaunchWithWindowsChanged(bool value) => Persist(value, () => _host.Settings.LaunchWithWindows = value);

    partial void OnThemeIndexChanged(int value)
    {
        if (_loading || value < 0)
        {
            return;
        }

        _host.Settings.Theme = AppTheme.FromIndex(value);
        _host.SaveSettings();
        SetStatus("Settings saved.", isError: false);
    }

    private void Persist(bool _, Action assign)
    {
        if (_loading)
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
