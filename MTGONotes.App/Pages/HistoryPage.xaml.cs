using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using MTGONotes.App.ViewModels;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace MTGONotes.App.Pages;

public sealed partial class HistoryPage : Page
{
    public HistoryPage(HistoryViewModel vm)
    {
        Vm = vm;
        InitializeComponent();
        Vm.Refresh();
    }

    public HistoryViewModel Vm { get; }

    private void OnQuerySubmitted(AutoSuggestBox sender, AutoSuggestBoxQuerySubmittedEventArgs args) =>
        Vm.Refresh();

    private void OnFocusSearch(KeyboardAccelerator sender, KeyboardAcceleratorInvokedEventArgs args)
    {
        HistorySearch.Focus(FocusState.Programmatic);
        args.Handled = true;
    }

    private async void OnExportText(object sender, RoutedEventArgs e)
    {
        var path = await PickSaveAsync("notes.txt");
        if (path is null)
        {
            return;
        }

        await Vm.ExportToAsync(path);
    }

    private async void OnBackup(object sender, RoutedEventArgs e)
    {
        var path = await PickSaveAsync("notebook.mtgonotes");
        if (path is null)
        {
            return;
        }

        var box = new PasswordBox { PlaceholderText = "Cannot be recovered" };
        var dialog = new ContentDialog
        {
            Title = "Backup passphrase",
            Content = box,
            PrimaryButtonText = "Create backup",
            CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Primary,
            XamlRoot = XamlRoot,
        };
        if (await dialog.ShowAsync() != ContentDialogResult.Primary)
        {
            return;
        }

        Vm.WriteBackup(path, box.Password);
    }

    private async Task<string?> PickSaveAsync(string name)
    {
        var picker = new FileSavePicker();
        if (Application.Current is App { MainWindow: { } window })
        {
            InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(window));
        }

        picker.SuggestedFileName = name;
        picker.FileTypeChoices.Add("Export", [Path.GetExtension(name)]);
        var file = await picker.PickSaveFileAsync();
        return file?.Path;
    }
}
