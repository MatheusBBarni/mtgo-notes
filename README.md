# MTGO Opponent Notes

Private, local-first Windows companion. The active host is an unpackaged
WinUI 3 / .NET 10 app. The previous Tauri 2 tree is frozen in `__oldversion__/`.

**License:** [Apache License 2.0](LICENSE)

**Site:** https://matheusbarni.github.io/mtgo-notes  
**Downloads:** https://github.com/MatheusBBarni/mtgo-notes/releases

This project is unofficial and not affiliated with Wizards of the Coast or
Daybreak Game Company. Live attach is optional, read-only, and not legal advice.

## Toolchain

- .NET 10 SDK
- Windows 10 22H2 / Windows 11 x64 for the app and MTGOSDK attach
- Node 22 for the brochure site in `web/`
- Portable Core/Data/Live tests run on macOS and Linux

```bash
export PATH="$HOME/.dotnet:$PATH"
dotnet test tests/MTGONotes.Core.Tests/MTGONotes.Core.Tests.csproj
dotnet test tests/MTGONotes.Data.Tests/MTGONotes.Data.Tests.csproj
dotnet test tests/MTGONotes.Live.Tests/MTGONotes.Live.Tests.csproj
```

On Windows, open `MTGONotes.slnx` and run `MTGONotes.App`. Log into MTGO first.
Live attach is read-only and never calls `Client.LogOn`.

## Website

Static Astro brochure in `web/`, published with GitHub Pages.

```bash
cd web
npm ci
npm test
npm run test:build
npm run preview
```

## Layout

- `MTGONotes.App` — unpackaged WinUI shell (overlay, capture, tray)
- `MTGONotes.Core` — encounter reducer, disclosure, session
- `MTGONotes.Data` — SQLCipher notebook
- `MTGONotes.Live` — read-only MTGO attach abstractions
- `web/` — Astro brochure (GitHub Pages)
- `tests/` — portable xUnit suites
- `__oldversion__/` — frozen Tauri/React/Rust tree

## Release

Push a version tag or run **Release Windows app** from Actions:

```bash
git tag v0.1.0
git push origin v0.1.0
```

That publishes a GitHub Release with `MTGONotes-<version>-win-x64.zip`.
The site Download button opens that Releases page.
