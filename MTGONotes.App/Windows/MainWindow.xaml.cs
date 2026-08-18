using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using MTGONotes.App.Helpers;
using MTGONotes.App.Host;
using MTGONotes.App.Native;
using MTGONotes.App.Pages;
using MTGONotes.App.Themes;
using MTGONotes.App.ViewModels;
using MTGONotes.Core.Disclosure;

namespace MTGONotes.App.Windows;

public sealed partial class MainWindow : Window
{
    private readonly AppHost _host;

    public MainWindow(AppHost host)
    {
        _host = host;
        Vm = new MainShellViewModel(host);
        InitializeComponent();
        Title = "MTGO Opponent Notes";
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(AppTitleBar);
        if (AppIcon.Image() is { } image)
        {
            TitleBarIcon.ImageSource = image;
        }

        ThemeService.Apply(this, host.Settings.Theme);
        WindowSizing.Resize(this, WindowSizing.MainWidthDip, WindowSizing.MainHeightDip);
        WindowSizing.SetMinimum(this, WindowSizing.MainMinWidthDip, WindowSizing.MainMinHeightDip);
        Vm.Apply(host.Session.CurrentView);
        ShowPage("encounter");
    }

    public MainShellViewModel Vm { get; }

    public void Apply(OverlayView view) => Vm.Apply(view);

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
            DefaultButton = ContentDialogButton.Primary,
            XamlRoot = Content.XamlRoot,
        };
        var result = await dialog.ShowAsync();
        _host.Settings.LiveAttachAcknowledged = true;
        _host.Settings.LiveAttachEnabled = result == ContentDialogResult.Primary;
        _host.SaveSettings();
        Vm.Settings.Reload();
        Vm.Apply(_host.Session.CurrentView);
    }

    private void OnPaneToggleRequested(TitleBar sender, object args) =>
        RootNav.IsPaneOpen = !RootNav.IsPaneOpen;

    private void OnRootSizeChanged(object sender, SizeChangedEventArgs e)
    {
        if (e.NewSize.Width < WindowSizing.PhoneWidth)
        {
            RootNav.PaneDisplayMode = NavigationViewPaneDisplayMode.LeftMinimal;
            RootNav.IsPaneOpen = false;
            return;
        }

        if (e.NewSize.Width < WindowSizing.CompactWidth)
        {
            RootNav.PaneDisplayMode = NavigationViewPaneDisplayMode.LeftCompact;
            RootNav.IsPaneOpen = false;
            return;
        }

        RootNav.PaneDisplayMode = NavigationViewPaneDisplayMode.Left;
        RootNav.IsPaneOpen = true;
    }

    private void OnNavSelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        if (args.SelectedItem is NavigationViewItem { Tag: string tag })
        {
            ShowPage(tag);
        }
    }

    private void OnGoEncounter(KeyboardAccelerator sender, KeyboardAcceleratorInvokedEventArgs args)
    {
        RootNav.SelectedItem = EncounterNavItem;
        args.Handled = true;
    }

    private void OnGoHistory(KeyboardAccelerator sender, KeyboardAcceleratorInvokedEventArgs args)
    {
        RootNav.SelectedItem = HistoryNavItem;
        args.Handled = true;
    }

    private void OnGoSettings(KeyboardAccelerator sender, KeyboardAcceleratorInvokedEventArgs args)
    {
        RootNav.SelectedItem = SettingsNavItem;
        args.Handled = true;
    }

    private void ShowPage(string tag)
    {
        object page = tag switch
        {
            "history" => ContentFrame.Content is HistoryPage ? ContentFrame.Content : new HistoryPage(Vm.History),
            "settings" => ContentFrame.Content is SettingsPage ? ContentFrame.Content : new SettingsPage(Vm.Settings),
            _ => ContentFrame.Content is EncounterPage ? ContentFrame.Content : new EncounterPage(Vm.Encounter),
        };

        if (!ReferenceEquals(ContentFrame.Content, page))
        {
            ContentFrame.Content = page;
        }

        if (RootNav.DisplayMode is NavigationViewDisplayMode.Minimal or NavigationViewDisplayMode.Compact)
        {
            RootNav.IsPaneOpen = false;
        }
    }
}
