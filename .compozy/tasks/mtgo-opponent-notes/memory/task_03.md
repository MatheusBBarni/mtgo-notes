# Task Memory: task_03.md

Keep only task-local execution context here. Do not duplicate facts that are obvious from the repository, task file, PRD documents, or git history.

## Objective Snapshot

- Deliver Task 03's authorized visible-window detector, conservative encounter lifecycle, tray/overlay shell, and draft-safe rapid capture against the canonical assigned `_tests.md` contracts.

## Important Decisions

- `_tests.md` owns each assigned test ID. The task prose mislabels several ranges: IT-021–030 are manual-entry cases, IT-194–206 are command success contracts, IT-236–246 are command failures, IT-265–269 are replacement events, and UIA/OCR boundaries are IT-273–277.
- Keep all automatic detection scoped to one explicitly selected visible top-level MTGO window. UIA is primary; OCR is a bounded transient fallback; raw pixels and OCR strings never enter persistence, renderer IPC, or diagnostics.
- Preserve the heavily dirty shared worktree and edit only Task 03 surfaces or narrow integration seams required to register them.
- Bind opponent confirmation to the exact provider session, generation, and sequence; an obsolete candidate cannot mutate the active encounter.
- Use one host-wide monotonic revision stream for renderer replacement events. Entity revisions remain persistence concurrency tokens and must not be reused as event ordering tokens.
- A provider interruption or selected-window generation change persists the active encounter as restricted before publishing provider or overlay replacements.
- Confirming a different opponent resolves or creates the profile and performs the previous-incomplete/new-active rollover in one repository transaction and one reversible undo group.
- Keep rapid capture intentionally text-first; structured cards and tags remain available in the main editor instead of slowing the global-shortcut path.

## Learnings

- The repository has no local `AGENTS.md` or `CLAUDE.md`; the run-provided `AGENTS.md` instructions and `/Users/matheusbbarni/.codex/RTK.md` are the applicable guidance.
- The spec corpus has no `analysis/` or `handoffs/` directory.
- Task 02's implementation is locally available, but its workflow memory records an outstanding Windows-only packaged DPAPI/SQLCipher evidence gate.
- The local repository gate passes on macOS, but the Windows-specific module cannot be compiled from this host with the current dependency path: `openssl-sys v0.9.117` invokes `perl ./Configure ... VC-WIN64A`, and the Darwin Perl reports that it cannot produce Windows-style paths.
- A static `windows-rs` implementation can enumerate and validate an explicitly selected visible MTGO HWND and initialize UI Automation, but the current implementation does not yet subscribe to UIA property/structure events or run actual cropped Windows Graphics Capture plus OCR.
- Deterministic provider fixtures and host/unit integration tests cover ordering, confidence, privacy, rollover, restricted replacement, shell, overlay, and draft recovery, but they are not substitutes for the assigned packaged-Windows accessibility, focus, performance, and end-to-end evidence.

## Files / Surfaces

- Implemented Task 03 surfaces: `src-tauri/src/detection/`, `src-tauri/resources/detection/`, provider/encounter/capture/note commands, notebook encounter/draft persistence, event revisions, `src-tauri/src/shell/`, Tauri setup/config/capabilities, `src/features/onboarding/`, `src/features/encounter/`, `src/overlay/`, `src/capture/`, typed IPC contracts, detection fixtures, unit tests, and integration tests.
- Detection evidence carries provider session, generation, sequence, monotonic time, confidence, and provenance; OCR text uses zeroizing transient storage and only bounded metadata enters diagnostics.
- The overlay receives complete policy-authorized replacements, clears stale restricted fields, remains click-through when collapsed, and enters an explicit interactive state.
- Quick capture is host-singleton, keyboard-first, and preserves encrypted recoverable draft text before a failed or stale save.

## Errors / Corrections

- Initial skill lookup incorrectly targeted `~/.codex/skills`; installed workflow skills are repository-local under `.agents/skills`.
- Pre-change inventory confirmed the detection resources, onboarding/encounter features, and detection fixtures did not exist, and assigned Task 03 test markers were absent.
- Task prose test-range descriptions drift from `_tests.md`; implementation and evidence must continue to use the catalog definitions, not infer meaning from numeric ranges.

## Ready for Next Run

- Task remains `pending`; do not mark `task_03.md` or `_tasks.md` complete.
- Fresh `rtk npm run verify` passed on 2026-07-28: Prettier and Cargo formatting, ESLint with zero warnings, Clippy with `-D warnings`, TypeScript typecheck, 3 capability manifests, 29 frontend unit tests, 10 frontend integration tests, 218 Rust library tests, 8 Rust contract tests, Vite production build, and Rust debug build.
- Remaining completion blockers: implement and verify real event-driven Windows UIA evidence delivery; implement bounded cropped Windows capture/OCR and ephemeral cleanup on the supported packaged build; connect those sources to the encounter evidence adapters; implement or evidence every assigned `_tests.md` case; and run packaged-Windows E2E, accessibility, focus/click-through, DPI/multi-monitor, latency, and resource-budget validation.
- No automatic commit was created. The shared worktree remains heavily dirty and largely untracked, so a future run must continue to preserve unrelated state and stage narrowly if explicitly requested.
