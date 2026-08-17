using System.Runtime.InteropServices;
using Microsoft.UI;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using WinRT.Interop;

namespace MTGONotes.App.Native;

internal static partial class OverlayHwnd
{
    private const int GwlExstyle = -20;
    private const int WsExLayered = 0x00080000;
    private const int WsExTransparent = 0x00000020;
    private const int WsExToolwindow = 0x00000080;
    private const int WsExNoactivate = 0x08000000;
    private const int SwShownoactivate = 4;
    private const uint SwpNosize = 0x0001;
    private const uint SwpNomove = 0x0002;
    private const uint SwpNoactivate = 0x0010;
    private const uint SwpShowwindow = 0x0040;

    public static nint Handle(Window window) => WindowNative.GetWindowHandle(window);

    public static AppWindow AppWindowFor(Window window)
    {
        var hwnd = Handle(window);
        return AppWindow.GetFromWindowId(Win32Interop.GetWindowIdFromWindow(hwnd));
    }

    public static void ConfigureChrome(Window window, int width, int height)
    {
        var appWindow = AppWindowFor(window);
        if (appWindow.Presenter is OverlappedPresenter presenter)
        {
            presenter.IsAlwaysOnTop = true;
            presenter.IsResizable = true;
            presenter.IsMinimizable = false;
            presenter.IsMaximizable = false;
            presenter.SetBorderAndTitleBar(false, false);
        }

        appWindow.IsShownInSwitchers = false;
        appWindow.Resize(new Windows.Graphics.SizeInt32(width, height));
    }

    public static void ShowWithoutActivating(Window window)
    {
        var hwnd = Handle(window);
        _ = ShowWindow(hwnd, SwShownoactivate);
        _ = SetWindowPos(
            hwnd,
            nint.Zero,
            0,
            0,
            0,
            0,
            SwpNomove | SwpNosize | SwpNoactivate | SwpShowwindow);
    }

    public static void SetClickThrough(Window window, bool clickThrough)
    {
        var hwnd = Handle(window);
        var style = GetWindowLong(hwnd);
        style |= WsExLayered | WsExToolwindow;
        if (clickThrough)
        {
            style |= WsExTransparent | WsExNoactivate;
        }
        else
        {
            style &= ~WsExTransparent;
            style &= ~WsExNoactivate;
        }

        _ = SetWindowLong(hwnd, style);
    }

    private static int GetWindowLong(nint hwnd) => unchecked((int)GetWindowLongPtr(hwnd, GwlExstyle));

    private static nint SetWindowLong(nint hwnd, int value) =>
        SetWindowLongPtr(hwnd, GwlExstyle, value);

    [LibraryImport("user32.dll", EntryPoint = "GetWindowLongPtrW", SetLastError = true)]
    private static partial nint GetWindowLongPtr(nint hWnd, int nIndex);

    [LibraryImport("user32.dll", EntryPoint = "SetWindowLongPtrW", SetLastError = true)]
    private static partial nint SetWindowLongPtr(nint hWnd, int nIndex, nint dwNewLong);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool ShowWindow(nint hWnd, int nCmdShow);

    [LibraryImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetWindowPos(
        nint hWnd,
        nint hWndInsertAfter,
        int x,
        int y,
        int cx,
        int cy,
        uint uFlags);
}
