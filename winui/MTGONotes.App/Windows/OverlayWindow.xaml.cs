using Microsoft.UI.Xaml;
using MTGONotes.App.Host;
using MTGONotes.App.Native;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.App.Windows;

public sealed partial class OverlayWindow : Window
{
    private readonly AppHost _host;
    private bool _expanded;

    public OverlayWindow(AppHost host)
    {
        _host = host;
        InitializeComponent();
        Title = "Opponent overlay";
        OverlayHwnd.ConfigureChrome(this, 360, 220);
        OverlayHwnd.SetClickThrough(this, true);
        Bind(host.Session.CurrentView);
    }

    public void ShowPassive()
    {
        OverlayHwnd.ShowWithoutActivating(this);
        OverlayHwnd.SetClickThrough(this, !_expanded);
    }

    public void Bind(OverlayView view)
    {
        var handle = view.ConfirmedHandle ?? "unconfirmed";
        PhaseText.Text = $"{FormatPhase(view.Phase)} — {handle}";
        NotesList.ItemsSource = view.CurrentObservations.Select(note => note.Text).ToArray();

        var candidate = _host.Session.Candidate;
        if (candidate is not null)
        {
            CandidateText.Text = $"Detected opponent: {candidate.DisplayHandle}";
            CandidateText.Visibility = Visibility.Visible;
            ConfirmButton.Visibility = Visibility.Visible;
        }
        else
        {
            CandidateText.Visibility = Visibility.Collapsed;
            ConfirmButton.Visibility = Visibility.Collapsed;
        }
    }

    public void Collapse()
    {
        _expanded = false;
        OverlayHwnd.SetClickThrough(this, true);
        ExpandButton.Content = "Expand overlay";
    }

    private void OnExpandClick(object sender, RoutedEventArgs e)
    {
        _expanded = !_expanded;
        OverlayHwnd.SetClickThrough(this, !_expanded);
        ExpandButton.Content = _expanded ? "Collapse overlay" : "Expand overlay";
    }

    private void OnConfirmClick(object sender, RoutedEventArgs e)
    {
        if (_host.Session.Candidate is { } candidate)
        {
            _ = _host.Session.ConfirmOpponent(candidate);
            Bind(_host.Session.CurrentView);
        }
    }

    private static string FormatPhase(Core.Domain.InternalPhase phase) =>
        phase.ToString().Replace("InGameRestricted", "in game", StringComparison.Ordinal);
}
