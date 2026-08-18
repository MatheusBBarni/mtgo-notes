using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using MTGONotes.App.Host;
using MTGONotes.App.Native;
using MTGONotes.App.Themes;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.App.Windows;

public sealed partial class OverlayWindow : Window
{
    public const int OverlayWidth = 360;
    public const int OverlayHeight = 248;
    public const int OverlayMinHeight = 48;

    private readonly AppHost _host;
    private bool _minimized;

    public OverlayWindow(AppHost host)
    {
        _host = host;
        InitializeComponent();
        Title = "Opponent overlay";
        BrandImage.Source = AppIcon.Image();
        ThemeService.Apply(this, host.Settings.Theme);
        OverlayHwnd.ConfigureChrome(this, OverlayWidth, OverlayHeight, resizable: false);
        _minimized = host.Settings.OverlayMinimized;
        ApplyMinimizedLayout();
        Bind(host.Session.CurrentView);
    }

    public void ShowPassive()
    {
        OverlayHwnd.RestorePosition(this, _host.Settings.OverlayX, _host.Settings.OverlayY);
        ApplyMinimizedLayout();
        OverlayHwnd.ShowWithoutActivating(this);
    }

    public void Bind(OverlayView view)
    {
        PhaseText.Text = PresentationText.FormatOverlayHeading(view.Phase, view.ConfirmedHandle);
        NotesList.ItemsSource = view.CurrentObservations.Select(note => note.Text).ToArray();

        var candidate = _host.Session.Candidate;
        if (candidate is not null)
        {
            CandidateText.Text = $"Detected opponent: {candidate.DisplayHandle}";
            CandidateText.Visibility = Visibility.Visible;
            ConfirmButton.Content = $"Confirm {candidate.DisplayHandle}";
            ConfirmButton.Visibility = Visibility.Visible;
        }
        else
        {
            CandidateText.Visibility = Visibility.Collapsed;
            ConfirmButton.Visibility = Visibility.Collapsed;
        }

        var restricted = view.Phase.IsDisclosureRestricted();
        RestrictionText.Visibility = restricted ? Visibility.Visible : Visibility.Collapsed;
        HintText.Visibility = view.ConfirmedHandle is null && !restricted
            ? Visibility.Visible
            : Visibility.Collapsed;
    }

    public void Collapse()
    {
        SetMinimized(true);
    }

    private void OnDragBarPressed(object sender, PointerRoutedEventArgs e)
    {
        if (!e.GetCurrentPoint((UIElement)sender).Properties.IsLeftButtonPressed)
        {
            return;
        }

        if (IsChromeButton(e.OriginalSource as DependencyObject))
        {
            return;
        }

        OverlayHwnd.DragMove(this);
        PersistPlacement();
    }

    private void OnMinimizeClick(object sender, RoutedEventArgs e) => SetMinimized(!_minimized);

    private void OnHideClick(object sender, RoutedEventArgs e)
    {
        PersistPlacement();
        AppWindow.Hide();
    }

    private void OnCaptureClick(object sender, RoutedEventArgs e) => _host.OpenCapture();

    private void OnConfirmClick(object sender, RoutedEventArgs e)
    {
        if (_host.Session.Candidate is { } candidate)
        {
            _ = _host.Session.ConfirmOpponent(candidate);
            Bind(_host.Session.CurrentView);
        }
    }

    private void SetMinimized(bool minimized)
    {
        _minimized = minimized;
        ApplyMinimizedLayout();
        PersistPlacement();
    }

    private void ApplyMinimizedLayout()
    {
        BodyPanel.Visibility = _minimized ? Visibility.Collapsed : Visibility.Visible;
        OverlayHwnd.Resize(this, OverlayWidth, _minimized ? OverlayMinHeight : OverlayHeight);
        MinimizeIcon.Glyph = _minimized ? "\uE923" : "\uE921";
        ToolTipService.SetToolTip(MinimizeButton, _minimized ? "Restore" : "Minimize");
        AutomationProperties.SetName(MinimizeButton, _minimized ? "Restore overlay" : "Minimize overlay");
    }

    private void PersistPlacement()
    {
        var position = OverlayHwnd.AppWindowFor(this).Position;
        _host.RememberOverlay(position.X, position.Y, _minimized);
    }

    private static bool IsChromeButton(DependencyObject? source)
    {
        while (source is not null)
        {
            if (source is Button)
            {
                return true;
            }

            source = Microsoft.UI.Xaml.Media.VisualTreeHelper.GetParent(source);
        }

        return false;
    }

}
