using System.Runtime.InteropServices;
using Microsoft.UI.Xaml;

namespace MTGONotes.App.Native;

internal sealed class WindowMessages : IDisposable
{
    public const uint WmHotkey = 0x0312;
    public const uint WmAppTray = 0x8001;
    public const uint WmRButtonUp = 0x0205;
    public const uint WmLButtonDblClk = 0x0203;

    private readonly nint _hwnd;
    private readonly SubclassProc _proc;
    private bool _installed;

    public WindowMessages(Window window)
    {
        _hwnd = OverlayHwnd.Handle(window);
        _proc = Hook;
    }

    public event Action<int>? HotkeyPressed;

    public event Action<uint>? TrayMessage;

    public void Install()
    {
        if (_installed)
        {
            return;
        }

        _installed = SetWindowSubclass(_hwnd, _proc, 1, 0);
    }

    public void Dispose()
    {
        if (_installed)
        {
            _ = RemoveWindowSubclass(_hwnd, _proc, 1);
            _installed = false;
        }
    }

    private nint Hook(nint hWnd, uint msg, nint wParam, nint lParam, nuint id, nint data)
    {
        if (msg == WmHotkey)
        {
            HotkeyPressed?.Invoke((int)wParam);
            return 0;
        }

        if (msg == WmAppTray)
        {
            TrayMessage?.Invoke((uint)(lParam & 0xFFFF));
            return 0;
        }

        return DefSubclassProc(hWnd, msg, wParam, lParam);
    }

    private delegate nint SubclassProc(nint hWnd, uint msg, nint wParam, nint lParam, nuint uIdSubclass, nint dwRefData);

    [DllImport("comctl32.dll", SetLastError = true)]
    private static extern bool SetWindowSubclass(
        nint hWnd,
        SubclassProc pfnSubclass,
        nuint uIdSubclass,
        nuint dwRefData);

    [DllImport("comctl32.dll", SetLastError = true)]
    private static extern bool RemoveWindowSubclass(nint hWnd, SubclassProc pfnSubclass, nuint uIdSubclass);

    [DllImport("comctl32.dll")]
    private static extern nint DefSubclassProc(nint hWnd, uint uMsg, nint wParam, nint lParam);
}
