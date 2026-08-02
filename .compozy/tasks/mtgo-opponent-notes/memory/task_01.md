# Task Memory: task_01.md

Keep only task-local execution context here. Do not duplicate facts that are obvious from the repository, task file, PRD documents, or git history.

## Objective Snapshot

- Scaffold the greenfield Tauri 2, React 19, TypeScript, and Rust desktop foundation with exact three-window, IPC, design-system, Windows packaging, CI, and assigned-test contracts.

## Important Decisions

- `_tests.md` owns the assigned test definitions: UT-105–108 cover design/accessibility; UT-113–120 cover IPC/capabilities/events; IT-191–193 cover bootstrap/settings; IT-272 covers fail-closed unknown event recovery.
- `_techspec.md` exact window dimensions, lifecycle behavior, capability command lists, and versioned replacement-event behavior override the task file's broader shorthand.
- Implement a restrained desktop hover treatment because assigned UT-106 explicitly requires a distinguishable hover state; this is a test-contract exception to `DESIGN.md`'s extracted marketing-site no-hover guidance.
- Keep the Task 01 host registration surface limited to bootstrap/settings. Remove incomplete references to downstream deck/classifier/provider services while retaining reserved capability permission names for later commands.
- No repository-local `AGENTS.md` or `CLAUDE.md` exists; use the root instructions supplied by the execution context.
- Preserve the pre-existing untracked `.gitignore` and workflow-memory directory while keeping implementation edits scoped to Task 01.

## Learnings

- The spec corpus contains no `analysis/` directory or summary artifact.
- The repository is a planning-only greenfield baseline with no existing package, Rust, source, test, or CI workspace.
- A partial untracked scaffold was present at execution time. Its baseline failed formatting, TypeScript rejected the capture-field ref, both Vitest suites had no tests, and Rust referenced feature modules that do not exist.
- `npm run verify` is the complete local gate: Prettier, rustfmt, ESLint, Clippy with warnings denied, TypeScript, capability policy, frontend/Rust tests, three-entrypoint Vite build, and native Rust build.

## Files / Surfaces

- Root Node/npm, TypeScript, Vite, Vitest, ESLint, Prettier, Rust toolchain, lockfiles, and validation scripts.
- Independent `src/main/`, `src/overlay/`, and `src/capture/` entrypoints plus shared `src/lib/ipc/`, capability policy, and `src/ui/` tokens/primitives.
- Tauri host composition, bootstrap/settings commands, IPC serialization, window lifecycle/navigation, capability manifests, generated native icons, Windows packaging configuration, and Windows x64 CI.
- Assigned frontend/Rust unit and integration tests, bundled Inter attribution/license material, and project setup documentation.

## Errors / Corrections

- The task prose groups the assigned IDs inaccurately; implementation follows each ID's canonical `_tests.md` definition.
- Required repository guidance files were absent, so no local guidance content could be loaded.
- An optional macOS-to-MSVC `cargo check` reached Tauri's Windows resource step but could not run because `llvm-rc` is absent on the macOS host. Actual MSVC tests and NSIS packaging run in the checked-in `windows-latest` CI job.

## Ready for Next Run

- Task 01 is complete and locally verified. Task 02 can build on the caller-scoped bootstrap/settings seam, plain-data IPC envelopes, replacement events, window identities, and least-privilege manifests.
