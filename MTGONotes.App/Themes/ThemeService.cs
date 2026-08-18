using Microsoft.UI;
using Microsoft.UI.Xaml;
using MTGONotes.App.Native;
using MTGONotes.Core.Settings;
using Windows.UI;
using Windows.UI.ViewManagement;

namespace MTGONotes.App.Themes;

internal static class ThemeService
{
    public static ElementTheme ToElementTheme(string? preference) =>
        AppTheme.Normalize(preference) switch
        {
            AppTheme.Light => ElementTheme.Light,
            AppTheme.Dark => ElementTheme.Dark,
            _ => ElementTheme.Default,
        };

    public static void Apply(Window window, string? preference)
    {
        var theme = ToElementTheme(preference);
        if (window.Content is FrameworkElement root)
        {
            root.RequestedTheme = theme;
        }

        ApplyTitleBar(window, theme);
    }

    public static bool IsDark(ElementTheme theme)
    {
        if (theme is ElementTheme.Dark)
        {
            return true;
        }

        if (theme is ElementTheme.Light)
        {
            return false;
        }

        var background = new UISettings().GetColorValue(UIColorType.Background);
        return background.R + background.G + background.B < 384;
    }

    private static void ApplyTitleBar(Window window, ElementTheme theme)
    {
        var titleBar = OverlayHwnd.AppWindowFor(window).TitleBar;
        if (new AccessibilitySettings().HighContrast)
        {
            return;
        }

        if (window.ExtendsContentIntoTitleBar)
        {
            titleBar.ButtonBackgroundColor = Colors.Transparent;
            titleBar.ButtonInactiveBackgroundColor = Colors.Transparent;
            var ink = IsDark(theme)
                ? Color.FromArgb(255, 0xF5, 0xF6, 0xF7)
                : Color.FromArgb(255, 0x18, 0x1D, 0x26);
            var hover = IsDark(theme)
                ? Color.FromArgb(255, 0x2D, 0x32, 0x3B)
                : Color.FromArgb(255, 0xE0, 0xE2, 0xE6);
            var pressed = IsDark(theme)
                ? Color.FromArgb(255, 0x3A, 0x40, 0x4A)
                : Color.FromArgb(255, 0xDD, 0xDD, 0xDD);
            titleBar.ButtonForegroundColor = ink;
            titleBar.ButtonHoverBackgroundColor = hover;
            titleBar.ButtonHoverForegroundColor = ink;
            titleBar.ButtonPressedBackgroundColor = pressed;
            titleBar.ButtonPressedForegroundColor = ink;
            titleBar.ButtonInactiveForegroundColor = IsDark(theme)
                ? Color.FromArgb(255, 0xB4, 0xB8, 0xC0)
                : Color.FromArgb(255, 0x41, 0x45, 0x4D);
            return;
        }

        if (IsDark(theme))
        {
            var canvas = Color.FromArgb(255, 0x18, 0x1D, 0x26);
            var hover = Color.FromArgb(255, 0x2D, 0x32, 0x3B);
            var pressed = Color.FromArgb(255, 0x3A, 0x40, 0x4A);
            var ink = Color.FromArgb(255, 0xF5, 0xF6, 0xF7);
            var muted = Color.FromArgb(255, 0xB4, 0xB8, 0xC0);
            PaintTitleBar(titleBar, canvas, ink, hover, pressed, muted);
            return;
        }

        PaintTitleBar(
            titleBar,
            Color.FromArgb(255, 255, 255, 255),
            Color.FromArgb(255, 0x18, 0x1D, 0x26),
            Color.FromArgb(255, 0xE0, 0xE2, 0xE6),
            Color.FromArgb(255, 0xDD, 0xDD, 0xDD),
            Color.FromArgb(255, 0x41, 0x45, 0x4D));
    }

    private static void PaintTitleBar(
        Microsoft.UI.Windowing.AppWindowTitleBar titleBar,
        Color canvas,
        Color ink,
        Color hover,
        Color pressed,
        Color muted)
    {
        titleBar.BackgroundColor = canvas;
        titleBar.ForegroundColor = ink;
        titleBar.InactiveBackgroundColor = canvas;
        titleBar.InactiveForegroundColor = muted;
        titleBar.ButtonBackgroundColor = canvas;
        titleBar.ButtonForegroundColor = ink;
        titleBar.ButtonHoverBackgroundColor = hover;
        titleBar.ButtonHoverForegroundColor = ink;
        titleBar.ButtonPressedBackgroundColor = pressed;
        titleBar.ButtonPressedForegroundColor = ink;
        titleBar.ButtonInactiveBackgroundColor = canvas;
        titleBar.ButtonInactiveForegroundColor = muted;
    }
}
