# Task Memory: task_06.md

Keep only task-local execution context here. Do not duplicate facts that are obvious from the repository, task file, PRD documents, or git history.

## Objective Snapshot

- Deliver the complete Task 06 portability slice: authenticated encrypted logical backups, fully validated staging restore with merge/replace, explicit encrypted rollback lifecycle, deterministic warned UTF-8 export, typed commands/events, and accessible main-window controls.
- Automatic commits are disabled; task tracking may move to completed only after every assigned canonical `_tests.md` case, full verification, and contract-parity review pass.

## Important Decisions

- `_tests.md` owns the exact meaning of assigned IDs where `task_06.md` paraphrases them incorrectly. Task-only wire-format, KDF, bounded rollback, and rollback command gaps remain required and receive additional coverage.
- Caller-visible wrong credentials map to `wrong_passphrase`; malformed/version/checksum failures map to `invalid_backup`.
- Snapshot backup and export may coexist. Restore, replace, purge, and migration remain exclusive.
- Selected stable RustCrypto components: Argon2 0.5.3 with explicit Argon2id v0x13 parameters and ChaCha20-Poly1305 0.11.0 with independently authenticated bounded chunks. The wire format remains application-versioned and fixture-locked.
- `.mtgonotes` V1 uses a checksummed explicit envelope, Argon2id credential verifier, 64 KiB independently authenticated frames, canonical logical rows, and an authenticated final manifest. SQLCipher pages, DB keys, DPAPI material, provider consent, operation journals, and machine-bound secrets are excluded.
- The live operation journal is persisted in SQLCipher. Restart marks unfinished non-rollback operations failed and rollback-bearing operations recoverable; rollback discovery additionally uses encrypted database artifacts plus non-sensitive sidecars.

## Learnings

- The repository begins Task 06 with a green `npm run verify`, 103 Rust unit tests, 17 frontend unit tests, and 4 frontend integration tests, but `cargo test portability` finds zero tests because no portability module exists.
- `analysis/` and `handoffs/` spec directories are absent. The root has no repository-local `AGENTS.md` or `CLAUDE.md`; caller-provided guidance and `/Users/matheusbbarni/.codex/RTK.md` govern this run.
- The live tree contains the Task 04/05 domain and classifier seams even though Task 04 tracking remains pending; portability must preserve unrelated uncommitted work.
- Target-only `rfd` 0.17.2 keeps native path selection in the Windows host while non-Windows development builds remain usable.
- The completed portability slice exercises 145 passing Rust tests across five suites; the focused portability suite has 31 tests, including archive, staged merge/replace, rollback, deterministic export, exclusions, cancellation, journaling, snapshot consistency, transient cleanup, and assigned contract-ID coverage.
- UI verification is covered by 21 passing frontend unit tests across four files and four passing frontend integration tests across two files. The in-app browser runtime exposed no browser target, so rendered interactive inspection was unavailable in this environment.
- A Windows-target `cargo check` reached environment-only cross-compilation blockers (`llvm-rc` absent and vendored OpenSSL unable to execute its Windows build from macOS); native macOS Rust compilation and tests remain green.

## Files / Surfaces

- Touched: `src-tauri/src/operations/mod.rs`, new `src-tauri/src/portability/`, `src-tauri/src/commands/portability.rs`, notebook repository/runtime/schema composition, Tauri capabilities/permissions, `src/lib/ipc/portability.ts`, `src/lib/ipc/operations.ts`, new portability feature panels, `MainApp`, global styles, Rust/frontend contract tests, fixtures, dependency locks, and third-party notices.

## Errors / Corrections

- `task_06.md` mislabels IT-151–160, IT-220–224, IT-254–258, and IT-268. Canonical `_tests.md` meanings are used, while explicit rollback/export requirements are implemented separately.
- ADR-006 broadly serializes portability operations, but UT-086 and Task 06 specifically permit concurrent snapshot backup/export; the more specific test contract wins.
- `_idea.md` and ADR-001 defer readable export, but ADR-003 explicitly supersedes them and the PRD requires V1 export.
- Export initially included generation wall-clock time, which broke byte-for-byte determinism; it was removed so identical snapshots produce identical UTF-8 output.
- Classifier provenance was initially queried outside the logical-record read transaction; backup now reads provenance and all canonical records from the same SQLite snapshot.
- Portability operation state initially lived only in the process coordinator; SQLCipher-backed journaling and restart recovery were added before the broad verification pass.

## Ready for Next Run

- Implementation, simplification, self-review, contract-parity review, and assigned-test coverage are complete.
- Fresh repository verification passed after tracking updates: formatting, ESLint, Clippy with warnings denied, TypeScript, capability lint, 21 frontend unit tests, four frontend integration tests, 145 Rust tests, production web build, and native Rust build.
- Automatic commit remains disabled. Leave the complete diff uncommitted for manual review.
