using System.Runtime.InteropServices;

namespace MTGONotes.App.Native;

internal sealed class TrayIconService : IDisposable
{
    private const uint NimAdd = 0;
    private const uint NimDelete = 2;
    private const uint NifMessage = 0x01;
    private const uint NifIcon = 0x02;
    private const uint NifTip = 0x04;
    private const uint MfString = 0;
    private const uint TpmLeftAlign = 0;
    private const uint TpmBottomAlign = 0x0020;
    private const uint TpmRightButton = 0x0002;
    private const uint TpmReturnCmd = 0x0100;

    public const int CommandOpen = 1;
    public const int CommandOverlay = 2;
    public const int CommandPause = 3;
    public const int CommandQuit = 4;

    private readonly nint _hwnd;
    private bool _added;

    public TrayIconService(nint hwnd) => _hwnd = hwnd;

    public void Add(string tip)
    {
        var data = CreateData(tip);
        _added = Shell_NotifyIcon(NimAdd, ref data);
    }

    public int ShowMenu()
    {
        var menu = CreatePopupMenu();
        _ = AppendMenu(menu, MfString, CommandOpen, "Open MTGO Opponent Notes");
        _ = AppendMenu(menu, MfString, CommandOverlay, "Show/Hide Overlay");
        _ = AppendMenu(menu, MfString, CommandPause, "Toggle Live Attach Pause");
        _ = AppendMenu(menu, MfString, CommandQuit, "Quit");
        GetCursorPos(out var point);
        SetForegroundWindow(_hwnd);
        var command = TrackPopupMenu(
            menu,
            TpmLeftAlign | TpmBottomAlign | TpmRightButton | TpmReturnCmd,
            point.X,
            point.Y,
            0,
            _hwnd,
            nint.Zero);
        DestroyMenu(menu);
        return command;
    }

    public void Dispose()
    {
        if (!_added)
        {
            return;
        }

        var data = CreateData(string.Empty);
        _ = Shell_NotifyIcon(NimDelete, ref data);
        _added = false;
    }

    private NotifyIconData CreateData(string tip)
    {
        return new NotifyIconData
        {
            cbSize = Marshal.SizeOf<NotifyIconData>(),
            hWnd = _hwnd,
            uID = 1,
            uFlags = NifMessage | NifIcon | NifTip,
            uCallbackMessage = WindowMessages.WmAppTray,
            hIcon = LoadIcon(nint.Zero, 32512),
            szTip = tip,
        };
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct NotifyIconData
    {
        public int cbSize;
        public nint hWnd;
        public uint uID;
        public uint uFlags;
        public uint uCallbackMessage;
        public nint hIcon;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)]
        public string szTip;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Point
    {
        public int X;
        public int Y;
    }

    [DllImport("shell32.dll", EntryPoint = "Shell_NotifyIconW")]
    private static extern bool Shell_NotifyIcon(uint dwMessage, ref NotifyIconData lpData);

    [DllImport("user32.dll", EntryPoint = "LoadIconW")]
    private static extern nint LoadIcon(nint hInstance, nint lpIconName);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern nint CreatePopupMenu();

    [DllImport("user32.dll", EntryPoint = "AppendMenuW", CharSet = CharSet.Unicode)]
    private static extern bool AppendMenu(nint hMenu, uint uFlags, nuint uIDNewItem, string lpNewItem);

    [DllImport("user32.dll")]
    private static extern bool DestroyMenu(nint hMenu);

    [DllImport("user32.dll")]
    private static extern bool GetCursorPos(out Point lpPoint);

    [DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(nint hWnd);

    [DllImport("user32.dll")]
    private static extern int TrackPopupMenu(
        nint hMenu,
        uint uFlags,
        int x,
        int y,
        int nReserved,
        nint hWnd,
        nint prcRect);
}
