using System.Runtime.InteropServices;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;

namespace MTGONotes.App.Native;

internal static class AppIcon
{
    public static string IcoPath { get; } =
        Path.Combine(AppContext.BaseDirectory, "Assets", "mtgo-notes.ico");

    public static string PngPath { get; } =
        Path.Combine(AppContext.BaseDirectory, "Assets", "icon.png");

    public static void Apply(Window window)
    {
        if (File.Exists(IcoPath))
        {
            OverlayHwnd.AppWindowFor(window).SetIcon(IcoPath);
        }
    }

    public static ImageSource? Image()
    {
        if (!File.Exists(PngPath))
        {
            return null;
        }

        return new BitmapImage(new Uri(PngPath));
    }

    public static nint LoadSmallHandle(out bool owns)
    {
        if (File.Exists(IcoPath))
        {
            var handle = LoadImageW(nint.Zero, IcoPath, ImageIcon, 16, 16, LrLoadFromFile);
            if (handle != nint.Zero)
            {
                owns = true;
                return handle;
            }
        }

        owns = false;
        return LoadIconW(nint.Zero, IdiApplication);
    }

    private const uint ImageIcon = 1;
    private const uint LrLoadFromFile = 0x0010;
    private const nint IdiApplication = 32512;

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern nint LoadImageW(
        nint hInst,
        string name,
        uint type,
        int cx,
        int cy,
        uint fuLoad);

    [DllImport("user32.dll", EntryPoint = "LoadIconW")]
    private static extern nint LoadIconW(nint hInstance, nint lpIconName);
}
