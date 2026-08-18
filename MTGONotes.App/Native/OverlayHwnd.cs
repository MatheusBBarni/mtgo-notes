using System.Runtime.InteropServices;
using Microsoft.UI;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using MTGONotes.App.Helpers;
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
    private const uint WmSysCommand = 0x0112;
    private const nuint ScMoveCaption = 0xF012;
    private const int DwmwaWindowCornerPreference = 33;
    private const int DwmwcpRound = 2;

    public static nint Handle(Window window) => WindowNative.GetWindowHandle(window);

    public static AppWindow AppWindowFor(Window window)
    {
        var hwnd = Handle(window);
        return AppWindow.GetFromWindowId(Win32Interop.GetWindowIdFromWindow(hwnd));
    }

    public static void ConfigureChrome(Window window, int width, int height, bool resizable = true)
    {
        var appWindow = AppWindowFor(window);
        if (appWindow.Presenter is OverlappedPresenter presenter)
        {
            presenter.IsAlwaysOnTop = true;
            presenter.IsResizable = resizable;
            presenter.IsMinimizable = false;
            presenter.IsMaximizable = false;
            presenter.SetBorderAndTitleBar(false, false);
        }

        appWindow.IsShownInSwitchers = false;
        WindowSizing.Resize(window, width, height);
        RoundCorners(window);
        ApplyToolStyle(window, clickThrough: false);
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

    public static void DragMove(Window window)
    {
        var hwnd = Handle(window);
        _ = ReleaseCapture();
        _ = SendMessageW(hwnd, WmSysCommand, ScMoveCaption, 0);
    }

    public static void Resize(Window window, int widthDip, int heightDip) =>
        WindowSizing.Resize(window, widthDip, heightDip);

    public static void RestorePosition(Window window, int x, int y)
    {
        var appWindow = AppWindowFor(window);
        var display = DisplayArea.GetFromWindowId(appWindow.Id, DisplayAreaFallback.Primary);
        var work = display.WorkArea;
        var width = Math.Max(appWindow.Size.Width, 80);
        var height = Math.Max(appWindow.Size.Height, 40);
        var maxX = work.X + Math.Max(work.Width - width, 0);
        var maxY = work.Y + Math.Max(work.Height - height, 0);
        var clampedX = Math.Clamp(x, work.X, maxX);
        var clampedY = Math.Clamp(y, work.Y, maxY);
        appWindow.Move(new global::Windows.Graphics.PointInt32(clampedX, clampedY));
    }

    public static void SetClickThrough(Window window, bool clickThrough) =>
        ApplyToolStyle(window, clickThrough);

    private static void ApplyToolStyle(Window window, bool clickThrough)
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

    private static void RoundCorners(Window window)
    {
        var preference = DwmwcpRound;
        _ = DwmSetWindowAttribute(
            Handle(window),
            DwmwaWindowCornerPreference,
            ref preference,
            sizeof(int));
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

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool ReleaseCapture();

    [LibraryImport("user32.dll", EntryPoint = "SendMessageW")]
    private static partial nint SendMessageW(nint hWnd, uint msg, nuint wParam, nint lParam);

    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(
        nint hwnd,
        int dwAttribute,
        ref int pvAttribute,
        int cbAttribute);
}
