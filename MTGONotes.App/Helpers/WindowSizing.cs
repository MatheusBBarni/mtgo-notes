using System.Runtime.InteropServices;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using MTGONotes.App.Native;
using Windows.Graphics;

namespace MTGONotes.App.Helpers;

internal static partial class WindowSizing
{
    public const int MainWidthDip = 1200;
    public const int MainHeightDip = 800;
    public const int MainMinWidthDip = 720;
    public const int MainMinHeightDip = 560;
    public const double CompactWidth = 1008;
    public const double PhoneWidth = 640;

    [LibraryImport("user32.dll")]
    private static partial uint GetDpiForWindow(nint hWnd);

    public static double Scale(Window window)
    {
        var dpi = GetDpiForWindow(OverlayHwnd.Handle(window));
        return dpi == 0 ? 1 : dpi / 96.0;
    }

    public static void Resize(Window window, int widthDip, int heightDip)
    {
        var scale = Scale(window);
        OverlayHwnd.AppWindowFor(window).Resize(
            new SizeInt32(
                (int)Math.Round(widthDip * scale),
                (int)Math.Round(heightDip * scale)));
    }

    public static void SetMinimum(Window window, int widthDip, int heightDip)
    {
        if (OverlayHwnd.AppWindowFor(window).Presenter is OverlappedPresenter presenter)
        {
            presenter.PreferredMinimumWidth = widthDip;
            presenter.PreferredMinimumHeight = heightDip;
        }
    }
}
