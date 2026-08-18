using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace MTGONotes.App.Helpers;

internal static class XamlConverters
{
    public static Visibility VisibleWhen(bool value) =>
        value ? Visibility.Visible : Visibility.Collapsed;

    public static Visibility CollapsedWhen(bool value) =>
        value ? Visibility.Collapsed : Visibility.Visible;

    public static bool Not(bool value) => !value;

    public static InfoBarSeverity StatusSeverity(bool isError) =>
        isError ? InfoBarSeverity.Error : InfoBarSeverity.Success;
}
