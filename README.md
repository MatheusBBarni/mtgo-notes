# MTGO Opponent Notes

Private, local-first Windows companion. The shipping host is moving from Tauri 2
to an unpackaged WinUI 3 / .NET 10 app on `feat/winui-rewrite`. The Tauri tree
in `src/` and `src-tauri/` remains as a frozen reference until cutover.

## WinUI host (active rewrite)

- .NET 10 SDK
- Windows 10 22H2 / Windows 11 x64 for the app and MTGOSDK attach
- Portable tests run on macOS/Linux

```bash
export PATH="$HOME/.dotnet:$PATH"
dotnet test winui/tests/MTGONotes.Core.Tests/MTGONotes.Core.Tests.csproj
dotnet test winui/tests/MTGONotes.Data.Tests/MTGONotes.Data.Tests.csproj
dotnet test winui/tests/MTGONotes.Live.Tests/MTGONotes.Live.Tests.csproj
```

On Windows, open `winui/MTGONotes.slnx` and run `MTGONotes.App`. Log into MTGO
first. Live attach is read-only and never calls `Client.LogOn`.

## Legacy Tauri host

- Node.js `22.23.1` with npm `10.9.8`
- Rust `1.95.0`
- `npm ci` then `npm run verify`
- Windows package: `npm run build:windows`

Keep `npm run verify` for the frozen Tauri tree. Do not treat macOS results as
packaged-Windows evidence.
