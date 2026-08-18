using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using MTGONotes.App.ViewModels;
using Windows.System;

namespace MTGONotes.App.Pages;

public sealed partial class EncounterPage : Page
{
    public EncounterPage(EncounterViewModel vm)
    {
        Vm = vm;
        InitializeComponent();
    }

    public EncounterViewModel Vm { get; }

    private void OnHandleKeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != VirtualKey.Enter || !Vm.ConfirmOpponentCommand.CanExecute(null))
        {
            return;
        }

        Vm.ConfirmOpponentCommand.Execute(null);
        e.Handled = true;
    }

    private async void OnFinishClick(object sender, RoutedEventArgs e)
    {
        if (!Vm.FinishEncounterCommand.CanExecute(null))
        {
            return;
        }

        var identity = string.IsNullOrWhiteSpace(Vm.ConfirmedHandle)
            ? "the current encounter"
            : Vm.ConfirmedHandle;
        var dialog = new ContentDialog
        {
            Title = "Finish encounter",
            Content = $"End the encounter with {identity}? This closes the active notebook session.",
            PrimaryButtonText = "Finish encounter",
            CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = XamlRoot,
        };
        if (await dialog.ShowAsync() != ContentDialogResult.Primary)
        {
            return;
        }

        Vm.FinishEncounterCommand.Execute(null);
    }
}
