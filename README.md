# MTGO Opponent Notes

A private, local-first Windows companion for remembering MTGO opponents and
reviewing verifiable public context. It is not an MTGO client, not a board
logger, and not a Videre clone.

**Download:** [GitHub Releases](https://github.com/MatheusBBarni/mtgo-notes/releases)  
**Site:** https://matheusbarni.github.io/mtgo-notes

Unofficial. Not affiliated with Wizards of the Coast or Daybreak Game Company.
Live attach is optional, read-only, and not legal advice. Daybreak may still
terminate accounts under their EULA.

## What it does

- Confirm an opponent before anything is saved
- Capture a short note between games
- Recall history when you are not playing
- Optional read-only attach to an already-logged-in MTGO process (no `LogOn`,
  password, chat, queue, or concede)

Notes stay on your machine. There is no signup and no telemetry.

## Install

1. Download `MTGONotes-<version>-win-x64.zip` from
   [Releases](https://github.com/MatheusBBarni/mtgo-notes/releases).
2. Unzip and run `MTGONotes.App.exe`.
3. Requires Windows 10 22H2 or Windows 11, x64.
4. If you want live attach, log into MTGO before launching the companion.

There is no auto-updater. Grab a newer zip from Releases when you want one.

## Develop the Windows app

Needs the .NET 10 SDK. The WinUI host builds on Windows only. Portable tests
run on macOS and Linux.

```bash
dotnet test tests/MTGONotes.Core.Tests/MTGONotes.Core.Tests.csproj
dotnet test tests/MTGONotes.Data.Tests/MTGONotes.Data.Tests.csproj
dotnet test tests/MTGONotes.Live.Tests/MTGONotes.Live.Tests.csproj
```

On Windows, open `MTGONotes.slnx` and run `MTGONotes.App`.

```bash
dotnet build MTGONotes.App/MTGONotes.App.csproj -r win-x64
```

## Develop the website

Static Astro site in `web/`, published to GitHub Pages from `main`.

```bash
cd web
npm ci
npm test
npm run test:build
npm run preview
```

Local preview: http://127.0.0.1:4321/mtgo-notes

## Repository layout

- `MTGONotes.App` — unpackaged WinUI 3 shell
- `MTGONotes.Core` — domain, disclosure, session
- `MTGONotes.Data` — local SQLCipher notebook
- `MTGONotes.Live` — read-only MTGO attach abstractions
- `web/` — brochure site
- `tests/` — portable xUnit suites
- `__oldversion__/` — frozen Tauri tree; not the default build

## Release

```bash
git tag v0.2.1
git push origin v0.2.1
```

The **Release Windows app** workflow publishes
`MTGONotes-<version>-win-x64.zip` on GitHub Releases. The site Download button
opens that page.

## License

[Apache License 2.0](LICENSE)

Live attach uses [MTGOSDK](https://github.com/videre-project/MTGOSDK)
(Apache-2.0). This project is not affiliated with the Videre Project.
