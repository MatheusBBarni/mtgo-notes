# Task Memory: task_02.md

Keep only task-local execution context here. Do not duplicate facts that are obvious from the repository, task file, PRD documents, or git history.

## Objective Snapshot

- Audit and complete the encrypted notebook, encounter reducer, disclosure policy, repository, migrations, FTS, operation coordination, and typed bootstrap contracts for canonical tests UT-009–UT-045 plus IT-233, IT-234, IT-278, and IT-279.

## Important Decisions

- `_tests.md` owns the canonical test meanings: UT-009–020 are encounter tests, UT-021–030 disclosure tests, and UT-031–045 notebook tests despite the task file's shifted category labels.
- Treat `candidate` and `completion_pending` as disclosure-restricted internal phases. Only confirmed pre-match, between-games, and finished states may authorize historical queries; unresolved or not-yet-confirmed completion stays fail-closed.
- Keep the Windows-only IT-279 test compiled under `cfg(windows)` and execute it on both packaged-release Windows runners; macOS local verification cannot satisfy that OS-specific evidence gate.
- Persist confirmed-opponent replacement through one repository transaction: finish the previous active encounter, start the new encounter, and record both transitions under one undo group. A failed new encounter insert rolls the whole replacement back.

## Learnings

- The live tree already contained a broad untracked implementation and 181 passing Rust tests. A source-only inventory initially missed the canonical IT-233/234/278/279 suite under `src-tauri/tests/notebook_contract.rs`.
- The spec corpus has no `analysis/` or `handoffs/` directory and no repository-local `AGENTS.md` or `CLAUDE.md`; the execution-context guidance is the available repository instruction source.

## Files / Surfaces

- `src-tauri/src/domain/models.rs`, `src-tauri/src/disclosure/mod.rs`
- `src-tauri/src/notebook/repository.rs`, `src-tauri/src/notebook/tests.rs`
- `.github/workflows/windows.yml`

## Errors / Corrections

- A first targeted test command supplied two Cargo test filters, which Cargo rejects. Use one filter or the complete library test suite.
- The initial IT-233 test used `expect_err`, but the success type does not implement `Debug`; match the `Result` explicitly instead.
- Self-review found that added IT-233/234 unit tests duplicated the stronger existing integration contracts. Remove the duplicates; retain the Windows-only current-user DPAPI IT-279 extension and packaged-runner step.

## Ready for Next Run

- Local `npm run verify` passes after the final code state: 25 frontend unit tests, 6 frontend integration tests, 183 Rust unit tests, 8 Rust integration tests, production frontend build, and Rust build.
- Local contract parity is complete for `_prd.md`, `_techspec.md`, `_user_stories.md`, `_tests.md`, and ADR-001–007.
- Task remains pending only for the Windows-specific portion of IT-279. Run the named test on both packaged-release Windows runners; if both pass, check 2.10 and the integration-test group, change task status to `completed`, and leave `_tasks.md` unchanged.
