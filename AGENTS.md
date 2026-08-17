# Repository Guidelines

## Project Structure & Module Organization

This is a private, local-first Tauri 2 desktop application. React/TypeScript renderer code lives in `src/`: `main/`, `overlay/`, and `capture/` are separate capability-isolated entrypoints; reusable workflows belong in `features/`, IPC clients in `lib/ipc/`, and shared controls in `ui/`. The Rust host is under `src-tauri/src/`, organized by domain (`notebook/`, `detection/`, `portability/`, `providers/`) with Tauri handlers in `commands/`. Frontend unit and integration tests live in `tests/unit/` and `tests/integration/`; Rust integration contracts live in `src-tauri/tests/`. Static fonts and icons belong in `assets/`, while bundled native resources belong in `src-tauri/resources/`.

## Build, Test, and Development Commands

- `npm ci` installs the pinned dependency graph (Node `22.23.1`, npm `10.9.8`).
- `npm run dev` starts Vite on `127.0.0.1:1420`.
- `npm run tauri dev` runs the desktop shell in development.
- `npm run build` type-checks and builds all renderer entrypoints.
- `npm test` runs Vitest unit/integration suites and Rust tests.
- `npm run verify` is the required local gate: formatting, linting, types, capability checks, tests, and frontend/Rust builds.
- `npm run build:windows` produces the Windows x64 NSIS package.

## Coding Style & Naming Conventions

Use strict TypeScript, two-space indentation, double quotes, semicolons, and type-only imports where applicable. Prettier and ESLint are authoritative; Rust must pass `cargo fmt` and Clippy with warnings denied. Name React components and Rust types in `PascalCase`, functions/modules in `camelCase` or `snake_case` respectively, and constants in `SCREAMING_SNAKE_CASE`. Keep sensitive filesystem, network, OCR, updater, and storage authority in Rust; renderer code should use typed IPC contracts.

## Testing Guidelines

Name frontend tests `*.test.ts` or `*.test.tsx` in the matching suite directory. Place cross-boundary Rust contracts in `src-tauri/tests/`; colocate focused Rust unit tests with their module. There is no numeric coverage threshold, so every behavior change needs a regression test. Run targeted suites while iterating, then `npm run verify`. Windows UIA/OCR, DPAPI/SQLCipher, accessibility, signing, and installer claims require the evidence described in `tests/release/`; macOS results do not substitute for it.

## Commit & Pull Request Guidelines

Follow the repository's Conventional Commit style: `feat:`, `fix:`, `test:`, `ci:`, or `chore:` plus an imperative summary. Keep commits narrow. Pull requests should explain behavior and security-boundary changes, link the relevant issue or task, list verification commands, and include screenshots for visible UI changes. Call out any missing Windows manual evidence explicitly; never present portable checks as packaged-Windows proof.
