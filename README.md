# MTGO Opponent Notes

Private, local-first Windows companion scaffold built with Tauri 2, React 19,
TypeScript, and Rust.

## Toolchains

- Node.js `22.23.1` with npm `10.9.8`
- Rust `1.95.0`
- Windows target `x86_64-pc-windows-msvc`

Install frontend dependencies with `npm ci`. Run the complete local validation
gate with `npm run verify`. Windows packaging uses `npm run build:windows`.

The three renderer entrypoints are capability-isolated. Sensitive storage,
filesystem, process, shell, OCR, updater-installation, and network authority
remain in the Rust host.
