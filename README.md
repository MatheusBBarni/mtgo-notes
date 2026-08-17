# MTGO Opponent Notes

Private, local-first Windows companion. The active host is an unpackaged
WinUI 3 / .NET 10 app. The previous Tauri 2 tree is frozen in `__oldversion__/`.

## Toolchain

- .NET 10 SDK
- Windows 10 22H2 / Windows 11 x64 for the app and MTGOSDK attach
- Portable Core/Data/Live tests run on macOS and Linux

```bash
export PATH="$HOME/.dotnet:$PATH"
dotnet test tests/MTGONotes.Core.Tests/MTGONotes.Core.Tests.csproj
dotnet test tests/MTGONotes.Data.Tests/MTGONotes.Data.Tests.csproj
dotnet test tests/MTGONotes.Live.Tests/MTGONotes.Live.Tests.csproj
```

On Windows, open `MTGONotes.slnx` and run `MTGONotes.App`. Log into MTGO first.
Live attach is read-only and never calls `Client.LogOn`.

## Layout

- `MTGONotes.App` — unpackaged WinUI shell (overlay, capture, tray)
- `MTGONotes.Core` — encounter reducer, disclosure, session
- `MTGONotes.Data` — SQLCipher notebook
- `MTGONotes.Live` — read-only MTGO attach abstractions
- `tests/` — portable xUnit suites
- `__oldversion__/` — frozen Tauri/React/Rust tree
