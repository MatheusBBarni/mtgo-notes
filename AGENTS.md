# Repository Guidelines

## Project Structure & Module Organization

The active rewrite is `winui/`: `MTGONotes.Core` (domain, disclosure, session),
`MTGONotes.Data` (SQLCipher + DPAPI), `MTGONotes.Live` (read-only MTGO attach
abstractions), and `MTGONotes.App` (unpackaged WinUI 3 shell). Tests live in
`winui/tests/`.

The Tauri 2 host remains frozen in `src/` (React entrypoints) and `src-tauri/`
until a Windows install of the WinUI app opens a real notebook. Do not delete
it in this branch.

## Build, Test, and Development Commands

WinUI (preferred on this branch):

- `dotnet test winui/tests/MTGONotes.Core.Tests/MTGONotes.Core.Tests.csproj`
- `dotnet test winui/tests/MTGONotes.Data.Tests/MTGONotes.Data.Tests.csproj`
- `dotnet test winui/tests/MTGONotes.Live.Tests/MTGONotes.Live.Tests.csproj`
- Windows only: `dotnet build winui/MTGONotes.App/MTGONotes.App.csproj -r win-x64`

Legacy Tauri:

- `npm ci` then `npm run verify`
- `npm run build:windows` for the NSIS package

## Coding Style & Naming Conventions

C# uses file-scoped namespaces, nullable enabled, and warnings as errors.
React/TypeScript in the frozen tree still uses two-space indentation, double
quotes, and semicolons. Sensitive storage, MTGO attach, and portability stay
out of the XAML views. Overlay and capture talk only to session facades.

## Testing Guidelines

Portable tests are required before claiming Core/Data/Live behavior. Windows
UIA/OCR, DPAPI, overlay focus, live attach, and installer claims need the
checklists under `winui/tests/release/` and `tests/release/`. macOS results do
not substitute for packaged-Windows proof.

## Commit & Pull Request Guidelines

Conventional Commits: `feat:`, `fix:`, `test:`, `ci:`, or `chore:` plus an
imperative summary. Call out missing Windows manual evidence explicitly.
