---
status: pending
title: Implement the encrypted notebook and policy core
type: backend
complexity: critical
---

# Task 02: Implement the encrypted notebook and policy core

## Overview

Implement the durable, encrypted domain foundation for encounters, disclosure, notebook data, migrations, and coordinated operations. This slice gives later detection and UI work a transactionally safe source of truth with stable invariants and fail-closed policy behavior.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- Domain identifiers MUST be UUIDv7 strings, timestamps MUST be UTC milliseconds, mutable aggregates MUST carry revisions, and mutations MUST support idempotency where the TechSpec requires it.
- The encounter reducer MUST enforce the documented internal state machine, monotonic evidence ordering, generation isolation, one-active-encounter invariant, reversible compound transitions, and resumable incomplete encounters.
- DisclosurePolicy MUST be the sole authorization source for phase-sensitive projections and queries and MUST fail closed on unknown, stale, conflicting, or uncertain gameplay evidence.
- SQLCipher MUST receive its DPAPI-unwrapped current-user key before any schema read, with no plaintext or substitute-key fallback.
- The repository MUST enable foreign keys, WAL, secure deletion, bounded busy handling, transactional FTS5 maintenance, integrity checks after unclean shutdown, and rollback-safe checksummed migrations.
- Expected failures MUST cross IPC as stable typed errors rather than panics, raw database errors, or untyped strings.
- Logs and diagnostics emitted by this layer MUST NOT contain handles, notes, card observations, deck contents, keys, OCR strings, or other notebook content.
</requirements>

## Subtasks

- [x] 2.1 Define the shared domain value objects, aggregate models, revisions, idempotency contracts, and stable error taxonomy.
- [x] 2.2 Implement the encounter state machine and ordered evidence reducer with reversible transition records.
- [x] 2.3 Implement disclosure authorization and complete phase-filtered projections for all renderer consumers.
- [x] 2.4 Establish DPAPI key custody, SQLCipher connection opening, security pragmas, and unclean-shutdown checks.
- [x] 2.5 Create the normalized schema, constraints, indexes, settings storage, operation records, and migration ledger.
- [x] 2.6 Implement transactional repository primitives that enforce identity, encounter, revision, and idempotency invariants.
- [x] 2.7 Implement paged notebook queries, FTS5 synchronization, tombstone filtering, and snapshot reads.
- [x] 2.8 Add checksummed migrations, transactional rollback, recovery reporting, and safe unsupported-version handling.
- [x] 2.9 Expose only the minimal typed host bootstrap and service seams required by dependent tasks.
- [ ] 2.10 Add exhaustive reducer, policy, encryption, schema, migration, concurrency, and repository tests.

## Implementation Details

Implement the TechSpec's “Encounter State Machine,” “Core Interfaces,” “Data Models,” “Database Schema,” “Migration Strategy,” and “Concurrency Model.” Keep renderer-facing projections separate from stored entities, and make the repository and disclosure policy independently testable before automatic detection is introduced.

### Relevant Files

- `src-tauri/src/domain/` — identifiers, timestamps, revisions, shared entities, and typed failures.
- `src-tauri/src/encounters/` — encounter runtime, reducer, transitions, undo groups, and completion rules.
- `src-tauri/src/disclosure/` — phase-aware authorization and safe projections.
- `src-tauri/src/notebook/key.rs` — DPAPI-sealed database-key custody.
- `src-tauri/src/notebook/connection.rs` — SQLCipher opening and security pragmas.
- `src-tauri/src/notebook/schema.rs` and `src-tauri/src/notebook/migrations/` — durable schema and migration lifecycle.
- `src-tauri/src/notebook/repository.rs` — transactional notebook persistence and read snapshots.
- `src-tauri/src/notebook/fts.rs` — transactional search indexing and paged search.
- `src-tauri/src/operations/` — durable operation identity, state, exclusion, and progress primitives.
- `src-tauri/src/commands/` — minimal typed bootstrap and policy-query command seams.

### Dependent Files

- `src-tauri/src/detection/` — later evidence producers feed the reducer.
- `src-tauri/src/services/` — later notebook workflows transact through the repository.
- `src-tauri/src/portability/` — later backup and restore consume snapshots and staging databases.
- `src/lib/ipc/` — shared command types mirror stable host projections and errors.

### Related ADRs

- [ADR-001](adrs/adr-001.md) — local-first trusted-host ownership.
- [ADR-002](adrs/adr-002.md) — conservative encounter and disclosure behavior.
- [ADR-003](adrs/adr-003.md) — encrypted persistence and key custody.
- [ADR-004](adrs/adr-004.md) — typed IPC and caller-aware authorization.
- [ADR-006](adrs/adr-006.md) — durable operation coordination and recovery.

## Deliverables

- A deterministic encounter engine and fail-closed disclosure policy.
- A DPAPI-keyed SQLCipher notebook with complete schema, migrations, integrity checks, and FTS5 search primitives.
- Transactional repositories enforcing revisions, idempotency, one-active-encounter, tombstone, and identity invariants.
- Durable operation-coordination primitives for later portability and maintenance flows.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [x] UT-009, UT-010, UT-011, UT-012, UT-013, UT-014, UT-015, UT-016, UT-017, UT-018 — encounter reducer states, evidence ordering, generation isolation, and completion transitions.
- [x] UT-019, UT-020, UT-021, UT-022, UT-023, UT-024, UT-025, UT-026, UT-027, UT-028 — disclosure authorization and conservative phase projections.
- [x] UT-029, UT-030, UT-031, UT-032, UT-033, UT-034, UT-035, UT-036, UT-037 — identifiers, revisions, idempotency, stable errors, and entity invariants.
- [x] UT-038, UT-039, UT-040, UT-041, UT-042, UT-043, UT-044, UT-045 — key custody, encrypted connection, migrations, integrity, and repository helpers.
- [ ] IT-233, IT-234, IT-278, IT-279 — encrypted bootstrap, fail-closed recovery, transaction rollback, and schema-version boundaries.

## Verification Note

- Local `npm run verify` passes, including IT-233, IT-234, IT-278, and the portable SQLCipher portion of IT-279.
- Task completion remains pending until the Windows-only current-user DPAPI and packaged SQLCipher IT-279 test runs on both supported Windows release runners.

## Success Criteria

- Every assigned test case implemented and passing
- The notebook cannot be read before successful current-user DPAPI key recovery and SQLCipher initialization.
- Encounter and disclosure invariants hold under duplicate, stale, conflicting, interrupted, and concurrent inputs.
- Migrations, repository mutations, FTS updates, and durable operation state are atomic and recoverable.
