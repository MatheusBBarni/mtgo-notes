using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Input;
using MTGONotes.App.Host;
using MTGONotes.App.Native;
using MTGONotes.App.Themes;
using MTGONotes.Core.Domain;
using Windows.System;

namespace MTGONotes.App.Windows;

public sealed partial class CaptureWindow : Window
{
    private readonly AppHost _host;

    public CaptureWindow(AppHost host)
    {
        _host = host;
        InitializeComponent();
        Title = "Quick capture";
        ThemeService.Apply(this, host.Settings.Theme);
        OverlayHwnd.ConfigureChrome(this, 420, 160);
    }

    public void Open()
    {
        var opened = _host.Session.OpenCapture();
        if (!opened.IsSuccess)
        {
            ErrorText.Text = opened.Error!.Value.ToAppError().Message;
        }
        else
        {
            ErrorText.Text = string.Empty;
        }

        OverlayHwnd.AppWindowFor(this).Show();
        NoteBox.Focus(FocusState.Programmatic);
        Activate();
    }

    private void OnNoteKeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key == VirtualKey.Escape)
        {
            _ = _host.Session.DiscardDraft();
            AppWindow.Hide();
            e.Handled = true;
            return;
        }

        if (e.Key != VirtualKey.Enter)
        {
            return;
        }

        var result = _host.Session.SaveObservation(NoteBox.Text);
        if (result.IsSuccess)
        {
            NoteBox.Text = string.Empty;
            ErrorText.Text = string.Empty;
            AppWindow.Hide();
        }
        else
        {
            ErrorText.Text = result.Error!.Value.ToAppError().Message;
        }

        e.Handled = true;
    }
}
