using System.Diagnostics;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using MTGONotes.App.Host;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;
using MTGONotes.Core.Portability;
using MTGONotes.Core.Providers;
using MTGONotes.Core.Settings;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace MTGONotes.App.Windows;

public sealed partial class MainWindow : Window
{
    private readonly AppHost _host;

    public MainWindow(AppHost host)
    {
        _host = host;
        InitializeComponent();
        Title = "MTGO Opponent Notes";
        AppWindow.Resize(new Windows.Graphics.SizeInt32(1200, 800));
        LoadSettings();
        Bind(host.Session.CurrentView);
    }

    public void Bind(OverlayView view)
    {
        var handle = view.ConfirmedHandle ?? "No confirmed opponent";
        PhaseText.Text = $"{view.Phase} — {handle}";
        LiveText.Text = _host.Live.IsAttached
            ? "Live attach: connected"
            : "Live attach: waiting for a logged-in MTGO client";
        NotesList.ItemsSource = view.CurrentObservations.Select(note => note.Text).ToArray();
        if (!_host.Session.AuthorizeHistory().IsSuccess)
        {
            HistoryList.ItemsSource = new[] { "History hidden during possible gameplay." };
        }
    }

    public async Task MaybeOnboardAsync()
    {
        if (_host.Settings.LiveAttachAcknowledged)
        {
            return;
        }

        var dialog = new ContentDialog
        {
            Title = "Unofficial MTGO attach",
            Content =
                "This companion can read the already-logged-in MTGO process to detect opponents and match phase. It never logs on, never stores your password, and never writes into MTGO. You can pause or disable it at any time. This is unofficial and is not tournament-approved.",
            PrimaryButtonText = "Enable live attach",
            CloseButtonText = "Use manual entry only",
            XamlRoot = Content.XamlRoot,
        };
        var result = await dialog.ShowAsync();
        _host.Settings.LiveAttachAcknowledged = true;
        _host.Settings.LiveAttachEnabled = result == ContentDialogResult.Primary;
        _ = _host.SaveSettings();
        LoadSettings();
    }

    private void LoadSettings()
    {
        LiveConsentBox.IsChecked = _host.Settings.LiveAttachEnabled;
        DeckConsentBox.IsChecked = _host.Settings.OfficialDeckConsent;
        OverlayBox.IsChecked = _host.Settings.OverlayEnabled;
        TrayBox.IsChecked = _host.Settings.TrayEnabled;
        AutostartBox.IsChecked = _host.Settings.LaunchWithWindows;
    }

    private void OnEncounterTab(object sender, RoutedEventArgs e) => Show("encounter");

    private void OnHistoryTab(object sender, RoutedEventArgs e) => Show("history");

    private void OnSettingsTab(object sender, RoutedEventArgs e) => Show("settings");

    private void Show(string panel)
    {
        EncounterPanel.Visibility = panel == "encounter" ? Visibility.Visible : Visibility.Collapsed;
        HistoryPanel.Visibility = panel == "history" ? Visibility.Visible : Visibility.Collapsed;
        SettingsPanel.Visibility = panel == "settings" ? Visibility.Visible : Visibility.Collapsed;
        if (panel == "history")
        {
            RefreshHistory();
        }
    }

    private void OnConfirmClick(object sender, RoutedEventArgs e)
    {
        var result = _host.Session.EnterOpponent(HandleBox.Text);
        StatusText.Text = result.IsSuccess ? "Opponent confirmed." : result.Error!.Value.ToAppError().Message;
        Bind(_host.Session.CurrentView);
    }

    private void OnFinishClick(object sender, RoutedEventArgs e)
    {
        var result = _host.Session.FinishEncounter();
        StatusText.Text = result.IsSuccess ? "Encounter finished." : result.Error!.Value.ToAppError().Message;
        Bind(_host.Session.CurrentView);
    }

    private void OnPauseLiveClick(object sender, RoutedEventArgs e)
    {
        var paused = !_host.Session.DetectionPaused;
        _ = _host.Session.PauseDetection(paused);
        PauseLiveButton.Content = paused ? "Resume live attach" : "Pause live attach";
        StatusText.Text = paused ? "Live attach paused." : "Live attach resumed.";
    }

    private void OnSearchHistory(object sender, RoutedEventArgs e)
    {
        var result = _host.Session.SearchHistory(HistoryQuery.Text);
        if (!result.IsSuccess)
        {
            HistoryList.ItemsSource = new[] { result.Error!.Value.ToAppError().Message };
            return;
        }

        HistoryList.ItemsSource = result.Value!.Items
            .Select(item => $"{item.EntityType}: {item.Content}")
            .ToArray();
    }

    private void RefreshHistory()
    {
        var result = _host.Session.ListRecentEncounters();
        if (!result.IsSuccess)
        {
            HistoryList.ItemsSource = new[] { result.Error!.Value.ToAppError().Message };
            return;
        }

        HistoryList.ItemsSource = result.Value!
            .Select(item => $"{item.Handle} — {item.Status} — {item.Phase}")
            .ToArray();
    }

    private async void OnExportText(object sender, RoutedEventArgs e)
    {
        var dump = _host.Session.ExportLogical();
        if (!dump.IsSuccess)
        {
            HistoryList.ItemsSource = new[] { dump.Error!.Value.ToAppError().Message };
            return;
        }

        var path = await PickSaveAsync("notes.txt");
        if (path is null)
        {
            return;
        }

        await File.WriteAllTextAsync(path, TextExporter.Render(dump.Value!));
    }

    private async void OnBackup(object sender, RoutedEventArgs e)
    {
        if (!_host.Operations.Begin("backup").IsSuccess)
        {
            HistoryList.ItemsSource = new[] { "Another notebook operation is running." };
            return;
        }

        try
        {
            var dump = _host.Session.ExportLogical();
            if (!dump.IsSuccess)
            {
                HistoryList.ItemsSource = new[] { dump.Error!.Value.ToAppError().Message };
                return;
            }

            var path = await PickSaveAsync("notebook.mtgonotes");
            if (path is null)
            {
                return;
            }

            var dialog = new ContentDialog
            {
                Title = "Backup passphrase",
                Content = new PasswordBox { Name = "Passphrase", PlaceholderText = "Cannot be recovered" },
                PrimaryButtonText = "Create backup",
                CloseButtonText = "Cancel",
                XamlRoot = Content.XamlRoot,
            };
            if (await dialog.ShowAsync() != ContentDialogResult.Primary
                || dialog.Content is not PasswordBox box)
            {
                return;
            }

            var written = NotebookBackup.Write(path, dump.Value!, box.Password);
            HistoryList.ItemsSource = new[]
            {
                written.IsSuccess ? "Backup written." : written.Error!.Value.ToAppError().Message,
            };
        }
        finally
        {
            _host.Operations.End();
        }
    }

    private void OnSettingsSave(object sender, RoutedEventArgs e)
    {
        _host.Settings.LiveAttachEnabled = LiveConsentBox.IsChecked == true;
        _host.Settings.OfficialDeckConsent = DeckConsentBox.IsChecked == true;
        _host.Settings.OverlayEnabled = OverlayBox.IsChecked == true;
        _host.Settings.TrayEnabled = TrayBox.IsChecked == true;
        _host.Settings.LaunchWithWindows = AutostartBox.IsChecked == true;
        _host.SaveSettings();
        SettingsStatus.Text = "Settings saved.";
    }

    private void OnOpenOfficialDecks(object sender, RoutedEventArgs e)
    {
        if (!_host.Settings.OfficialDeckConsent)
        {
            SettingsStatus.Text = RepoError.ConsentRequired.ToAppError().Message;
            return;
        }

        var url = OfficialDeckProvider.ValidateOfficialUrl(OfficialDeckProvider.OfficialDecklistUrl);
        if (!url.IsSuccess)
        {
            SettingsStatus.Text = url.Error!.Value.ToAppError().Message;
            return;
        }

        Process.Start(new ProcessStartInfo(url.Value!.ToString()) { UseShellExecute = true });
    }

    private async Task<string?> PickSaveAsync(string name)
    {
        var picker = new FileSavePicker();
        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(this));
        picker.SuggestedFileName = name;
        picker.FileTypeChoices.Add("Export", [Path.GetExtension(name)]);
        var file = await picker.PickSaveFileAsync();
        return file?.Path;
    }
}
