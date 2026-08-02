# Task Memory: task_07.md

Keep only task-local execution context here. Do not duplicate facts that are obvious from the repository, task file, PRD documents, or git history.

## Objective Snapshot

- Deliver Task 07 end to end: private allowlisted diagnostics, independent versioned opt-ins, signed application and classifier update flows, offline degradation, caller-safe UI/IPC, and release-grade Windows automation/evidence.
- Automatic commits are disabled. Task status can change only after every assigned canonical `_tests.md` case, full verification, self-review, and contract-parity review pass.

## Important Decisions

- `_tests.md` owns each assigned ID's exact behavior. IT-101–IT-110 are offline-resilience cases, not settings/autostart cases; E2E-018 is diagnostics-only, while application updater coverage is IT-231, IT-232, IT-263, IT-264, IT-271, and IT-281.
- Reconcile the classifier-update opt-in with ADR-007 by keeping assets app-owned and read-only: the independent choice authorizes checks for publisher-signed classifier bundles and atomic activation only; it does not add an editor, import, or user-selected asset path.
- Preserve the existing dirty/untracked workspace and prior task tracking changes. Task 07 will touch only its diagnostics/settings/updater/offline/release surfaces plus narrowly required integration seams.
- No repository-local `AGENTS.md` or `CLAUDE.md` exists; use the caller-provided repository instructions and `/Users/matheusbbarni/.codex/RTK.md`.
- Keep Task 07 pending: local automated verification is green, but the production updater transports, native Windows package execution, production signing, and required reviewed Windows 10/11 manual evidence are unavailable in this workspace.
- Treat settings plus OS integration as one reversible operation: apply tray/autostart choices before persisting, roll them back on persistence failure, and do not replay side effects for a repeated idempotency key.

## Learnings

- The spec corpus has no `analysis/` or `handoffs/` directory.
- Task 05 already provides signed built-in classifier validation, atomic in-memory activation, last-known-good behavior on validation failure, and resumable 25-item reclassification; Task 07 must add the independent consented delivery/check boundary rather than replace those primitives.
- Task 06 already provides durable operation coordination, cancellation, progress, encrypted portability, and restart cleanup seams needed by diagnostics and offline release journeys.
- Tauri 2 tray support requires its `tray-icon` feature. The tray can be created or removed by stable ID, and left-click/open restores and focuses the main window.
- macOS cannot build this repository's vendored SQLCipher/OpenSSL dependency for `x86_64-pc-windows-msvc`: OpenSSL configuration rejects Darwin Perl because it does not produce Windows paths. The Windows test and NSIS gates must run on the native Windows CI/self-hosted runners.

## Files / Surfaces

- Implemented versioned local settings and fail-closed provider/update/classifier/diagnostics consent in `src-tauri/src/settings.rs`, `commands/settings.rs`, and `commands/decks.rs`.
- Implemented reversible per-user Windows autostart, runtime tray creation/removal, and tray-aware close behavior in `src-tauri/src/shell/autostart.rs` and `shell/windows.rs`.
- Implemented allowlisted rotating local diagnostics, retention cleanup, redacted preview, cancellation, preview-bound local bundle creation, and caller-aware commands in `src-tauri/src/diagnostics/` and `commands/diagnostics.rs`.
- Implemented minimal-metadata signed application update validation, explicit confirmation, interruption-safe pending state, signed classifier bundle validation/activation, and reclassification orchestration in `src-tauri/src/shell/updater.rs` and `commands/updates.rs`.
- Implemented offline/degraded state and all IT-101–IT-110 plus E2E-011 behavior in `src-tauri/src/resilience.rs`.
- Added typed renderer IPC and accessible privacy/update/diagnostics controls in `src/lib/ipc/`, `src/features/settings/OperationalSettings.tsx`, `src/main/MainApp.tsx`, and `src/ui/global.css`.
- Added least-privilege command grants, Windows 10/11 release workflow scaffolding, fail-closed evidence validation, explicit manual-evidence blockers, and release contract tests under `src-tauri/permissions/`, `src-tauri/capabilities/`, `.github/workflows/`, and `tests/release/`.

## Errors / Corrections

- Task 07's prose misgroups canonical test meanings; execution follows `_tests.md` and records the correction instead of naming tests after the paraphrase.
- The PRD/ADR-007 application-release wording and Task 07's independent classifier-update opt-in are both satisfied by an app-owned signed asset channel with separate consent and no runtime configuration surface.
- The first tray implementation compile exposed the missing Tauri `tray-icon` feature and `Manager` trait import; both were added and the fresh full gate passed.
- The release workflow initially omitted `workflow_dispatch` while gating its packaged matrix on that event, and its validator step label overstated the evidence it produced. The trigger and label were corrected.
- The initial global provider access choice was persisted and rendered but did not gate existing provider entry points. Both lookup and official-page commands now fail closed before an external path when the choice is disabled.

## Ready for Next Run

- `rtk npm run verify` passes from the final source state: formatting, ESLint, Clippy, typecheck, capability policy, 25 frontend unit tests, 6 frontend integration tests, 181 Rust library tests, 8 Rust contract tests, frontend production build, and Rust build.
- `rtk npm run test:windows` and `rtk npm run build:windows` both fail before application compilation because Darwin Perl cannot configure vendored OpenSSL for the MSVC target.
- Run the workflow-dispatch packaged matrix on native Windows 10 22H2 and Windows 11 x64 runners, provide production Authenticode/updater signing and delivery fixtures, capture reviewed UIA/OCR/tray/shortcut/overlay/accessibility/performance evidence, and only then re-run final verification and update Task 07 tracking.
- Automatic commit remains disabled; no commit was created.
