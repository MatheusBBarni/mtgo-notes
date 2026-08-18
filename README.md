# MTGO Opponent Notes

A private, local-first Windows companion for remembering MTGO opponents and
reviewing verifiable public context. It is not an MTGO client, not a board
logger, and not a Videre clone.

**Download:** [GitHub Releases](https://github.com/MatheusBBarni/mtgo-notes/releases)  
**Site:** https://matheusbarni.github.io/mtgo-notes

Unofficial. Not affiliated with Wizards of the Coast or Daybreak Game Company.
Live attach is optional, read-only, and not legal advice. Daybreak may still
terminate accounts under their EULA.

## Features

**Local notebook**
Notes and opponent profiles live on your machine. There is no signup, no cloud
notebook, and no telemetry.

**Confirm before persist**
An opponent nickname is previewed first. Nothing is written to history until
you confirm it.

**Fast capture**
Write a short personal note between games. History stays hidden during possible
gameplay.

**Click-through overlay**
The overlay does not steal clicks from MTGO.

**Player workspace**
Save one local **Player identity** (your MTGO nickname). Lookups never start
just because you saved it.

**Public result lookup**
An explicit, consented search of an enabled public provider. Only an exact
nickname match is offered. You preview candidates before import. An imported
public result is a read-only snapshot with its source. It is not “their current
deck,” and a refresh never silently overwrites what you already imported.

**Live attach (optional)**
Off means manual notes only. On is a read-only attach to an already-logged-in
MTGO process. It does not call `LogOn` and does not send a password, chat,
queue, or concede. Confirmation is still required before history changes.

**Export and backups**
Backups are yours to make. Text export is unencrypted — treat those files as
plaintext.

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
git tag v0.3.0
git push origin v0.3.0
```

The **Release Windows app** workflow publishes
`MTGONotes-<version>-win-x64.zip` on GitHub Releases. The site Download button
opens that page.

## License

[Apache License 2.0](LICENSE)

Live attach uses [MTGOSDK](https://github.com/videre-project/MTGOSDK)
(Apache-2.0). This project is not affiliated with the Videre Project.
