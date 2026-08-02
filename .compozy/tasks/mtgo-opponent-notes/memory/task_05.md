# Task Memory: task_05.md

Keep only task-local execution context here. Do not duplicate facts that are obvious from the repository, task file, PRD documents, or git history.

## Objective Snapshot

- Deliver Task 05's consent-aware official MTGO deck enrichment, immutable deck/snapshot provenance, deterministic local classifier, typed commands/events, read-only UI, and every assigned canonical test contract.

## Important Decisions

- `_tests.md` owns each test ID's exact meaning. Task prose that misgroups UT-054–UT-056 or describes E2E-010 as classifier-update coverage is not authoritative.
- Automated official lookup remains disabled because no documented or explicitly permitted API contract has been validated. V1 uses `interactive_required` with an allowlisted official MTGO URL and user-confirmed official data.
- E2E-010 covers official snapshot confirmation and refresh. Full classifier-update/no-editor E2E-019 remains Task 07 scope.
- Task 02 and Task 03 implementation seams are absent even though Task 05 depends on Task 03. Keep this task self-contained behind task-owned repository/provider abstractions and do not absorb detection, encounter, SQLCipher/DPAPI, or updater delivery scope.
- Treat the existing untracked application scaffold as user-owned and preserve it while adding Task 05 surfaces.
- The built-in classifier resource uses a pinned Ed25519 release public key to verify the exact definitions, corpus, and golden-vector bytes; the manifest separately pins the corpus SHA-256 digest and release provenance.
- Unsupported formats produce a typed `format_unsupported` result for direct classification, but deck confirmation uses an auditable `Unclassified`/`unsupported` run so enrichment never blocks local work.
- Complete user decks can append immutable revisions through the same `save_complete_deck` contract by supplying `deckId`; earlier revision runs remain attached and queryable.
- Reclassification persists its classifier version and last completed deck revision in the job cursor, uses 25-item transactions, and recovers requested/running/paused jobs after process restart.

## Learnings

- The repository contains no `AGENTS.md`, `CLAUDE.md`, `analysis/`, or `handoffs/` artifacts; the caller-provided AGENTS instructions and RTK guidance are the available repository rules.
- The pre-task scaffold had no public-deck provider, classifier engine, deck/classifier commands, signed assets, or deck/classifier UI.
- The live tree had more Task 02/04 repository and notebook scaffolding than the initial memory snapshot recorded, including deck/snapshot/classification tables; Task 05 extended those seams without claiming the still-pending prerequisite tasks complete.
- The canonical corpus requires E2E-010 to stop at separate official snapshot refresh and user-label provenance; classifier-update E2E remains Task 07.

## Files / Surfaces

- Implemented: `src-tauri/src/providers/decks/`, `src-tauri/src/classifier/`, `src-tauri/src/services/decks.rs`, deck/classifier commands, notebook schema/repository consent seams, shell allowlist handoff, signed classifier resources, Tauri permissions, TypeScript IPC, read-only React features, and deterministic provider/classifier fixtures/tests.

## Errors / Corrections

- Pre-change `npm test` could not start because dependencies were not installed: `vitest: command not found`.
- Corpus survey correction: ADR-005 primarily owns UIA/OCR and only constrains the public-deck boundary; ADR-006 owns SQLCipher/DPAPI rather than reclassification orchestration.
- Initial deck persistence returned `NotebookInvalid` because canonical-digest serialization used a JSON map with tuple keys; changed the canonical representation to a stable ordered tuple list before hashing.
- Initial reclassification implementation resumed only in memory; self-review corrected it to restore persisted job/version/cursor progress after restart.

## Ready for Next Run

- Task 05 is implemented and contract-parity reviewed. Fresh `npm run verify` passed after all code changes: formatting, zero-warning ESLint/Clippy, typecheck, capability lint, 17 frontend unit tests, 4 frontend integration tests, 111 Rust tests, Vite build, and Rust build.
- Automatic official access remains intentionally disabled; future enablement requires the documented permission/semantics/limits/redistribution spike recorded in `OFFICIAL_ACCESS_SPIKE.md`.
- No automatic commit was created. The working tree remains ready for manual review alongside the pre-existing user-owned untracked scaffold.
