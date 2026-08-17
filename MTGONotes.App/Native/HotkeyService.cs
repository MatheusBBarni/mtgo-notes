using System.Runtime.InteropServices;
using Microsoft.UI.Xaml;
using WinRT.Interop;

namespace MTGONotes.App.Native;

internal sealed partial class HotkeyService : IDisposable
{
    public const int CaptureHotkeyId = 0x4E01;
    private const uint ModControl = 0x0002;
    private const uint ModShift = 0x0004;
    private const uint VkN = 0x4E;

    private readonly nint _hwnd;
    private bool _registered;

    public HotkeyService(Window window)
    {
        _hwnd = WindowNative.GetWindowHandle(window);
    }

    public bool RegisterCaptureShortcut()
    {
        _registered = RegisterHotKey(_hwnd, CaptureHotkeyId, ModControl | ModShift, VkN);
        return _registered;
    }

    public void Dispose()
    {
        if (_registered)
        {
            _ = UnregisterHotKey(_hwnd, CaptureHotkeyId);
            _registered = false;
        }
    }

    [LibraryImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool RegisterHotKey(nint hWnd, int id, uint fsModifiers, uint vk);

    [LibraryImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool UnregisterHotKey(nint hWnd, int id);
}
