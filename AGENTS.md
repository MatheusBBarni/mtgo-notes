# Repository Guidelines

## Project Structure & Module Organization

The active app is the unpackaged WinUI 3 host at the repo root:

- `MTGONotes.Core` — domain, disclosure, session
- `MTGONotes.Data` — SQLCipher + DPAPI
- `MTGONotes.Live` — read-only MTGO attach abstractions
- `MTGONotes.App` — WinUI 3 shell
- `tests/` — xUnit suites

The previous Tauri 2 / React / Rust tree is frozen in `__oldversion__/`. Do not
treat it as the default build.

## Build, Test, and Development Commands

- `dotnet test tests/MTGONotes.Core.Tests/MTGONotes.Core.Tests.csproj`
- `dotnet test tests/MTGONotes.Data.Tests/MTGONotes.Data.Tests.csproj`
- `dotnet test tests/MTGONotes.Live.Tests/MTGONotes.Live.Tests.csproj`
- Windows only: `dotnet build MTGONotes.App/MTGONotes.App.csproj -r win-x64`

Legacy Tauri (only inside `__oldversion__/`): `npm ci` then `npm run verify`.

## Coding Style & Naming Conventions

C# uses file-scoped namespaces, nullable enabled, and warnings as errors.
Sensitive storage, MTGO attach, and portability stay out of the XAML views.
Overlay and capture talk only to session facades.

## Testing Guidelines

Portable tests are required before claiming Core/Data/Live behavior. Windows
overlay focus, live attach, DPAPI, and installer claims need
`tests/release/`. macOS results do not substitute for packaged-Windows proof.

## Commit & Pull Request Guidelines

Conventional Commits: `feat:`, `fix:`, `test:`, `ci:`, or `chore:` plus an
imperative summary. Call out missing Windows manual evidence explicitly.
