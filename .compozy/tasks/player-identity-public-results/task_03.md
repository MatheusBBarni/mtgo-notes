---
status: completed
title: Deliver the Immutable Evidence Lifecycle and Classification
type: backend
complexity: critical
---

# Task 03: Deliver the Immutable Evidence Lifecycle and Classification

## Overview

Complete the durable Player evidence path from a trusted provider or manual statement through preview, selective atomic import, refresh/version reconciliation, retained-field revision, and local classification. This task turns the foundation and runtime contracts into the immutable Player-owned results that later deletion, portability, and UI slices consume.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- Player evidence MUST remain isolated from opponent profiles, encounters, deck records/revisions, public snapshots, and opponent classification runs.
- Manual official-source preview MUST validate a closed bounded schema and exact approved URL locally with zero network, DNS, preflight, embedded-browser, parse, or browser side effect.
- Source keys and canonical source/preview statements MUST be deterministic; source digest identifies content and MUST NOT substitute for source identity.
- Import MUST revalidate token, session, identity, source key/digest, preview digest, expiry, and selected fields in the host before any mutation.
- Import MUST discard unselected preview values and commit evidence, selected payload/cards, first selection revisions, and durable receipt atomically.
- Identical key/digest MUST resolve as already imported; changed digest MUST create a linked immutable version; different keys MUST remain distinct even with equal digests.
- Selection changes MUST append revisions, preserve mandatory source identity/attribution, reject stale revisions, and never edit public-source content.
- Only complete format-valid official decklist evidence MAY classify, only after evidence commit, and only into Player-owned classification records.
- Classifier unavailable, unsupported, or failed outcomes MUST preserve the imported evidence and expose a truthful unclassified state.
- Task 02 caller/phase/consent/session/replay fences MUST remain authoritative for every extended command.
</requirements>

## Subtasks

- [x] 3.1 Finalize canonical typed evidence envelopes, payloads, retained-field manifests, source/version rules, and classification eligibility.
- [x] 3.2 Deliver pure official artifact URL and bounded manual-result/deck validation with no external I/O.
- [x] 3.3 Extend runtime preview storage for manual statements and complete binding/expiry semantics.
- [x] 3.4 Deliver verified atomic batch import, selected payload/card retention, first selection revision, receipt, and rollback.
- [x] 3.5 Deliver duplicate detection, changed-source immutable version linking, and distinct-source behavior.
- [x] 3.6 Deliver explicit refresh reconciliation and scoped-empty coexistence without overwriting prior evidence.
- [x] 3.7 Deliver append-only retained-field revisions, optimistic concurrency, replay safety, and bounded evidence paging.
- [x] 3.8 Deliver post-commit complete-deck classification into Player-owned runs plus honest unclassified results.
- [x] 3.9 Extend the existing main-only Player service/command projection seam and prove opponent isolation.

## Implementation Details

Implement the TechSpec sections “Manual Evidence and Browser Routes” (manual half), “Session, Preview, and Idempotency Binding,” “Classification Design,” and the evidence/import/selection/refresh portions of the command surface. Extend Task 02's runtime/service/commands; do not introduce a renderer-authoritative repository path.

The existing opponent `DeckService::confirm_public_snapshot` may inform pure classifier invocation but MUST NOT be copied as a persistence design: it assumes opponent deck revisions and classification rows. Persist only selected values plus mandatory provenance/digests; never hide a complete rejected preview payload in `payload_json`.

### Relevant Files

- `src-tauri/src/player/models.rs` — typed evidence/manual/selection/version/classification models.
- `src-tauri/src/player/repository.rs` — immutable batch import, selection, empty, paging, and classification queries.
- `src-tauri/src/player/service.rs` — manual preview, import, selection, refresh, and evidence projections.
- `src-tauri/src/player/runtime.rs` — bound manual previews and session/identity invalidation.
- `src-tauri/src/player/routes.rs` — pure official artifact validation/canonicalization.
- `src-tauri/src/player/classification.rs` — complete-evidence adapter and Player-owned result persistence.
- `src-tauri/src/commands/player.rs` — manual preview, import, selection, refresh, and evidence command handlers.
- `src-tauri/src/classifier/mod.rs` — existing pure signed classifier engine and types.
- `src-tauri/src/commands/classifier.rs` — separate Player reclassification discovery integration if required.
- `src-tauri/src/ipc/error.rs` — closed safe import/manual/classification errors.

### Dependent Files

- `src-tauri/src/notebook/schema.rs` and migrations — Task 01 already owns the table graph; no redesign here.
- `src-tauri/src/player/census.rs` — Task 02 supplies trusted candidate statements.
- `src-tauri/src/services/decks.rs` — opponent-only reference; MUST remain unchanged by Player persistence.
- `src-tauri/src/portability/` and `src-tauri/src/player/deletion.rs` — Task 04 consumes completed canonical rows.
- `src/lib/ipc/player.ts` and `src/features/player/` — Task 05 consumes bounded views and handlers.

### Related ADRs

- [ADR-001: Keep the Player Workspace Optional and Additive](adrs/adr-001.md) — preserves opponent workflow independence.
- [ADR-002: Use Explicit Conditional and Manual Public Source Routes](adrs/adr-002.md) — constrains manual official-source behavior.
- [ADR-003: Preserve Immutable Player-Owned Public Result Evidence](adrs/adr-003.md) — primary evidence/version/selection contract.
- [ADR-004: Use Dedicated Player Persistence and Trusted-Host Runtime](adrs/adr-004.md) — trusted preview/import authority.
- [ADR-005: Persist Player Classification Runs Independently](adrs/adr-005.md) — owns classifier persistence separation.

## Deliverables

- Pure bounded manual official-result/deck preview and canonical attribution.
- Bound provider/manual previews plus transactional selective import and receipts.
- Immutable dedupe/version/refresh behavior and append-only selection history.
- Player-owned classification integration with import-safe unclassified fallback.
- Bounded evidence projections/paging and opponent-isolation regression evidence.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [x] UT-008–UT-010 — complete/reference deck and retained-field validation.
- [x] UT-016–UT-017 — source-version links and classification eligibility.
- [x] UT-022–UT-023, UT-049 — manual URL/field bounds and zero-I/O validation.
- [x] UT-038 — preview/session/identity/source/digest import binding.
- [x] UT-056–UT-062 — append-only selections, atomic imports, empty replay, Player classification, and paging.
- [x] IT-008, IT-010 — separate Player classification and opponent logical isolation.
- [x] IT-024 — no-fetch manual preview reference/complete/invalid flows.
- [x] IT-026–IT-035 — binding rejection, rollback, dedupe/version/refresh, selection, classification, identity fencing, and restart lifecycle.

## Success Criteria

- Every assigned test case implemented and passing.
- Every durable evidence record can be traced to one frozen source statement and selected-field revision chain without retained unselected content.
- Duplicate, changed, manual, refresh, classifier failure, replay, and restart paths preserve atomicity and immutable history.
- Player evidence/classification actions leave opponent data and consent unchanged.
