# Task Memory: task_04.md

Keep only task-local execution context here. Do not duplicate facts that are obvious from the repository, task file, PRD documents, or git history.

## Objective Snapshot

- Deliver Task 04 end to end: personal profiles/aliases, structured observations, offline history/search, reversible identity correction, and tombstone-based deletion with typed caller-safe UI projections.
- Automatic commits are disabled; leave the final diff ready for manual review.

## Important Decisions

- `_tests.md` owns the exact assigned-case behavior where Task 04's prose labels disagree with the catalog.
- Existing Task 01-03 work in the dirty workspace is preserved; Task 04 changes remain scoped to new service, command, IPC, feature, and test surfaces plus narrowly required repository extensions.
- History/profile/search authorization remains host-owned through `DisclosurePolicy`; renderer state never grants access.
- Deletion requests and due purges share the portability `OperationCoordinator` as exclusive purge work; completed purges append a non-content operation record before startup continues.
- Invalid card and tendency-tag validation remains typed and now identifies the `cards` or `tags` field without exposing rejected content.
- Combined observation saves validate every optional enrichment before creating the free-text row, preventing malformed cards or tags from leaving a hidden partial observation behind.
- Encounter detail keeps its SQL projection in `EncounterSummary` field order; merged and unmerged encounters failed closed when `profile_id` occupied the observation-count slot.

## Learnings

- The baseline has 15 frontend unit, 4 frontend integration, 46 Rust unit, and 8 Rust integration tests passing, but no Task 04 service or feature modules exist.
- Task 02 and Task 03 tracking remain pending even though their core repository, disclosure, encounter, and basic capture seams are present in the working tree.
- Reconciliation on 2026-07-28 found substantial Task 04-shaped service, command, IPC, and notebook UI code already present in the untracked working tree. The fresh baseline is 29 frontend unit, 10 frontend integration, 218 Rust unit, and 8 Rust integration tests passing; the remaining gap is assigned contract coverage plus missing visible edit/detail/pagination/unmerge/purge workflows.
- The 2026-07-29 continuation baseline remains green at the same counts, but only UT-068–UT-076 have explicit Task 04 test markers. Every assigned IT/E2E contract still needs explicit coverage and fresh full verification.
- Explicit executable coverage now exists for every Task 04-assigned UT, IT, and E2E identifier. Focused Rust Task 04 coverage is green at 72 tests, and the three visible React journeys for E2E-008, E2E-012, and E2E-013 pass.
- A focused identity/history run exposed a real `get_encounter` projection defect after merge/unmerge: `map_encounter_summary` read `profile_id` as `observation_count`. Reordering the select fixed the valid encounter detail path; all 34 focused history/identity cases then passed.
- The refinement pass caught an existing-notebook compatibility hazard: adding `retired_at` directly to schema version 1 changed its checksum. The column now arrives through schema migration 2, and migration tests prove fresh application and rollback behavior.
- Fresh final verification on 2026-07-29 passes the complete local repository gate: formatting, ESLint, Clippy with warnings denied, TypeScript, capability policy, 33 frontend unit tests, 10 frontend integration tests, 290 Rust library tests, 4 bootstrap contract tests, 4 notebook contract tests, Vite build, and Cargo build.
- The Windows contract cannot be completed on this macOS host. `npm run test:windows` exits 101 while configuring vendored OpenSSL because Darwin Perl cannot produce Windows paths for `VC-WIN64A`; packaged Windows E2E plus native focus/window evidence still requires a Windows runner.

## Files / Surfaces

- Present and under review: `src-tauri/src/services/{profiles,observations,history,identity,deletion}.rs`, Task 04 command modules, `src/lib/ipc/notebook.ts`, `src/features/notebook/NotebookWorkspace.tsx`, capture integration, and Task 04 unit/integration/E2E contracts.
- Newly corrected: `src-tauri/src/domain/models.rs`, `src-tauri/src/commands/privacy.rs`, and startup composition in `src-tauri/src/lib.rs`.
- Added contract suites: `src-tauri/src/services/task04_observation_tests.rs`, `task04_history_identity_tests.rs`, `task04_deletion_command_tests.rs`, plus `tests/unit/notebook-workflows.test.tsx`.
- Additional corrected surfaces: `src-tauri/src/services/history.rs`, `deletion.rs`, `observations.rs`, `profiles.rs`, `src-tauri/src/notebook/schema.rs`, and `src-tauri/src/services/mod.rs`.

## Errors / Corrections

- Corrected task prose interpretation: IT-071-080 are structured-observation cases, IT-081-090 are edit/delete lifecycle cases, and E2E-013 is identity merge/unmerge.
- The earlier “no Task 04 modules exist” baseline is stale and must not be used for completion decisions; current code exists but still needs contract-parity verification.
- Assigned contract markers and the full local repository gate are complete. Task tracking remains pending solely because the PRD release contract requires packaged Windows E2E and native focus/window evidence that this macOS host cannot produce.

## Ready for Next Run

- Run `npm run test:windows` and the packaged E2E/native focus probe on a Windows runner with the MSVC/OpenSSL build environment.
- If that evidence passes, perform the final tracking update for `task_04.md`; do not create an automatic commit.
