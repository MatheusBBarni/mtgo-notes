using Microsoft.UI.Xaml.Controls;
using MTGONotes.App.ViewModels;

namespace MTGONotes.App.Pages;

public sealed partial class SettingsPage : Page
{
    public SettingsPage(SettingsViewModel vm)
    {
        Vm = vm;
        InitializeComponent();
    }

    public SettingsViewModel Vm { get; }
}
