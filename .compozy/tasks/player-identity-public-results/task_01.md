---
status: completed
title: Establish the Player Bounded-Context Persistence Foundation
type: backend
complexity: critical
---

# Task 01: Establish the Player Bounded-Context Persistence Foundation

## Overview

Create the complete durable foundation for Player identity and public-result evidence inside the encrypted notebook. This task fixes the independent ownership, canonical identity, migration, transaction, and replay contracts that every later source, lifecycle, portability, and UI slice consumes.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- Migration v3 MUST create the complete Player-owned table graph, indexes, checks, and intra-Player foreign keys in dependency order through the existing checksummed migration manager.
- Player records MUST use dedicated tables; no Player FK or mutation may target opponent profiles, encounters, deck records/revisions, public snapshots, opponent classification runs, or generic opponent consent.
- The notebook MUST enforce zero or one stable Player identity and repository edits MUST use optimistic revision checks without aliasing or rekeying historical evidence.
- Source keys, request/source/preview digests, nickname normalization, and typed payload canonicalization MUST be deterministic, versioned, bounded, and ambiguity-safe.
- Imported source statements MUST be insert-only: identical key/digest resolves the existing record, changed digest links a new immutable version, and equal digests under different keys remain distinct.
- Player mutations and their durable operation receipts MUST commit atomically; exact replay returns the original result and changed inputs under one key return `invalid_request`.
- Durable receipts MUST contain no nickname, URL, source key/digest, preview token, payload, cards, provider text, Service ID, or runtime configuration.
- Sessions, previews, cooldowns, content-free audit, transient replay, provider configuration, and secrets MUST remain outside the durable Player schema.
- Existing SQLCipher, migration, integrity, rollback, UUIDv7, revision, and timestamp primitives SHOULD be reused without broadening opponent-domain ownership.
</requirements>

## Subtasks

- [x] 1.1 Define the isolated Player module and closed identity, evidence, provenance, source-key, digest, selection, empty-outcome, tombstone, classification, and receipt types.
- [x] 1.2 Establish bounded nickname validation and deterministic canonical serialization/digest behavior for all later trusted-host inputs.
- [x] 1.3 Add migration v3 with the full Player table graph, constraints, indexes, and Player-only foreign keys.
- [x] 1.4 Register v3 in the checksummed forward migration chain and preserve rollback, foreign-key, and integrity guarantees.
- [x] 1.5 Deliver singleton Player identity reads/writes and optimistic revision behavior through a bounded repository adapter.
- [x] 1.6 Deliver immutable evidence, card, selection, empty-outcome, classification-run, tombstone, and paging persistence primitives required by later services.
- [x] 1.7 Deliver content-free durable receipt binding and exact replay/mismatched-input behavior inside mutation transactions.
- [x] 1.8 Prove encrypted migration, reopen, transaction, idempotency, revision, and opponent-isolation behavior with real temporary notebooks.

## Implementation Details

Implement the TechSpec sections “Domain Types,” “Database Schema and Migration,” and the durable parts of “Session, Preview, and Idempotency Binding.” Task 01 supplies repository primitives for later imports, selections, empty outcomes, classifications, tombstones, and consent, but it does not implement provider/runtime orchestration, public commands, deletion flows, or portability.

### Relevant Files

- `src-tauri/src/player/mod.rs` — new bounded-context module boundary.
- `src-tauri/src/player/models.rs` — Player IDs, canonical types, validation, source keys, and digests.
- `src-tauri/src/player/repository.rs` — `PlayerStore`, immutable queries/mutations, paging, and durable receipts.
- `src-tauri/src/notebook/schema.rs` — schema version and v3 Player graph.
- `src-tauri/src/notebook/migrations/mod.rs` — checksummed forward migration registration and rollback path.
- `src-tauri/src/notebook/repository.rs` — existing encrypted transaction/read boundaries consumed by the Player adapter.
- `src-tauri/src/domain/ids.rs` — UUIDv7, revision, and operation identity primitives.
- `src-tauri/src/domain/models.rs` — shared timestamp/error primitives where Player-specific types cannot remain local.
- `src-tauri/src/notebook/tests.rs` — encrypted v2-to-v3 success/failure/reopen fixtures.
- `src-tauri/src/player/tests.rs` — focused Player model/repository contract tests.

### Dependent Files

- `src-tauri/src/player/runtime.rs` and `service.rs` — later tasks consume identity, consent, receipt, evidence, and transaction contracts.
- `src-tauri/src/player/classification.rs` and `deletion.rs` — later tasks consume dedicated classification/tombstone tables.
- `src-tauri/src/portability/records.rs` and `restore.rs` — later register and merge the canonical Player graph.
- `src-tauri/src/commands/player.rs` and `src/lib/ipc/player.ts` — later expose projections only after repository contracts stabilize.
- Existing opponent services/tables — regression boundary that MUST remain unchanged.

### Related ADRs

- [ADR-001: Keep the Player Workspace Optional and Additive](adrs/adr-001.md) — establishes separate singleton ownership.
- [ADR-003: Preserve Immutable Player-Owned Public Result Evidence](adrs/adr-003.md) — defines canonical evidence/version semantics.
- [ADR-004: Use Dedicated Player Persistence and Trusted-Host Runtime](adrs/adr-004.md) — owns the v3 isolated table graph and durable/transient split.
- [ADR-005: Persist Player Classification Runs Independently](adrs/adr-005.md) — requires the Player-owned run table.
- [ADR-006: Keep Census Configuration Host-Only and Disabled by Default](adrs/adr-006.md) — forbids provider configuration/secrets in the schema.

## Deliverables

- Dedicated Player domain types and deterministic canonicalization/digest contracts.
- Checksummed v3 encrypted-notebook migration with full Player-owned schema.
- Transactional Player repository with singleton identity, immutable evidence/version primitives, optimistic selections, tombstones, classification rows, and receipts.
- Real SQLCipher migration/reopen/atomicity/isolation verification.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [ ] UT-001–UT-007 — nickname validation plus canonical source keys and source/preview digests.
- [ ] UT-013–UT-014 — durable receipt exact replay and changed-input rejection.
- [ ] UT-051–UT-055 — singleton identity, optimistic revision, immutable evidence deduplication/versioning.
- [ ] IT-001–IT-003 — v2-to-v3 migration, rollback, and encrypted reopen.
- [ ] IT-004–IT-007 — singleton, atomic Player row graph/receipt, restart replay, and concurrent revisions.

## Success Criteria

- Every assigned test case implemented and passing.
- A v2 encrypted notebook migrates to v3 and reopens without changing existing opponent data.
- Later tasks can implement source, evidence, deletion, portability, and UI behavior without adding another foundation migration or bypassing `PlayerStore`.
- Player receipt and canonicalization tests prove no content/secret leakage or ambiguous replay binding.
