# MTGO Notes — WinUI 3 host

Unpackaged Windows App SDK rewrite. The Tauri tree in `src/` and `src-tauri/` stays as the frozen reference until cutover.

## On macOS

Only `MTGONotes.Core` builds here:

```bash
export PATH="$HOME/.dotnet:$PATH"
dotnet test winui/tests/MTGONotes.Core.Tests/MTGONotes.Core.Tests.csproj
```

Core currently has 46 tests: reducer, disclosure, identity, phase mapping, and the in-memory companion session.

## On Windows

Open `winui/MTGONotes.sln` in Visual Studio 2026 or:

```powershell
dotnet build winui/MTGONotes.App/MTGONotes.App.csproj -c Debug
```

The app is unpackaged (`WindowsPackageType=None`). Install the Windows App SDK runtime if the bootstrapper asks.

Live MTGOSDK attach is not wired in this first slice. Overlay HWND click-through is.
