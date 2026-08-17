using Microsoft.UI.Xaml;
using MTGONotes.App.Host;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

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
        Bind(host.Session.CurrentView);
    }

    public void Bind(OverlayView view)
    {
        var handle = view.ConfirmedHandle ?? "No confirmed opponent";
        PhaseText.Text = $"{view.Phase.ToString()} — {handle}";
        LiveText.Text = _host.Live.IsAttached
            ? $"Live attach: connected ({_host.Live.ProviderSession[..Math.Min(20, _host.Live.ProviderSession.Length)]}…)"
            : "Live attach: waiting for a logged-in MTGO client";
        NotesList.ItemsSource = view.CurrentObservations.Select(note => note.Text).ToArray();
    }

    private void OnConfirmClick(object sender, RoutedEventArgs e)
    {
        var result = _host.Session.EnterOpponent(HandleBox.Text);
        StatusText.Text = result.IsSuccess ? "Opponent confirmed." : result.Error!.Value.ToAppError().Message;
        Bind(_host.Session.CurrentView);
    }

    private void OnPauseLiveClick(object sender, RoutedEventArgs e)
    {
        var paused = !_host.Session.DetectionPaused;
        _ = _host.Session.PauseDetection(paused);
        PauseLiveButton.Content = paused ? "Resume live attach" : "Pause live attach";
        StatusText.Text = paused ? "Live attach paused." : "Live attach resumed.";
    }

    private void OnFinishClick(object sender, RoutedEventArgs e)
    {
        var result = _host.Session.FinishEncounter();
        StatusText.Text = result.IsSuccess ? "Encounter finished." : result.Error!.Value.ToAppError().Message;
        Bind(_host.Session.CurrentView);
    }
}
