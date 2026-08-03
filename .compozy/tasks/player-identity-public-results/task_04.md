---
status: completed
title: Extend Player Deletion, Portability, and Export
type: backend
complexity: critical
---

# Task 04: Extend Player Deletion, Portability, and Export

## Overview

Complete the Player data lifecycle with explicitly confirmed deletion, canonical encrypted portability, identity-safe restore, and precise plaintext export scopes. This task extends mature generic operation/file-safety mechanisms while keeping Player deletion and merge semantics independent from opponent undo and identity behavior.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- Player deletion MUST use a dedicated service and `player_tombstones`; it MUST NOT reuse or alter the opponent reversible-deletion protocol.
- Deletion confirmation MUST bind token, digest, exact target, current revision/counts, and 15-minute expiry and MUST reject changed or expired scope before mutation.
- Whole-identity deletion MUST remove only the Player graph/consent/runtime state, retain non-content no-resurrection tombstones, and leave opponent data/consent byte-for-byte unchanged.
- Encrypted archives MUST include exactly the seven canonical Player tables in FK-safe order and MUST exclude consent, receipts, runtime state, previews, audit, cooldowns, configuration, Service IDs, and machine-bound secrets.
- Restore merge MUST accept absent or identical Player IDs and MUST hard-block the whole merge for a different ID before any otherwise-mergeable opponent mutation.
- Restore MUST apply Player tombstones before records, report deterministic Player diff counts, and recheck identity compatibility at apply time.
- Explicit Replace MUST remain available for different IDs, while Merge is unavailable; every restore MUST start Player consent/provider/runtime disabled.
- Complete-notebook plaintext export MUST include human-readable Player identity/evidence/version/selection/classification/empty attribution and MUST exclude forbidden operational/secret state.
- Selected-opponent export MUST remain Player-free and on its existing opponent-only query path.
- Existing streaming, cancellation, partial-file, durable flush/sync, atomic publication, staging, rollback, and recovery guarantees MUST remain intact.
</requirements>

## Subtasks

- [x] 4.1 Define closed individual-evidence, empty-outcome, and whole-identity deletion preview/confirm contracts.
- [x] 4.2 Deliver atomic Player-only cascade deletion, content-free tombstones, receipts, consent removal, and runtime fencing.
- [x] 4.3 Expose main-only deletion commands/errors/projections without changing opponent deletion authority.
- [x] 4.4 Register the seven canonical Player tables and provenance/counts in encrypted archive processing.
- [x] 4.5 Deliver absent/same/different Player-ID restore preflight, allowed-mode projection, and defensive apply-time recheck.
- [x] 4.6 Deliver merge/replace/rollback/no-resurrection behavior with deterministic Player diffs and consent-off runtime reset.
- [x] 4.7 Extend complete-notebook text export with Player data and preserve selected-opponent exclusion.
- [x] 4.8 Prove deletion, interruption, restore, export, replay, and opponent-isolation behavior end to end.

## Implementation Details

Implement the TechSpec sections “Deletion Design” and “Portability and Restore.” Reuse the operation coordinator, staging database, encrypted rollback, streaming archive/export, and atomic file publication patterns, but keep Player target/revision/tombstone logic in `PlayerDeletionService`.

`RestorePreview.allowed_modes` currently uses a fixed two-element array and must become a bounded variable collection or equivalent representation so a different Player ID can expose Replace only. Extend Player diff counts explicitly rather than inferring them from opponent-oriented counters.

### Relevant Files

- `src-tauri/src/player/deletion.rs` — bound preview, cascade transaction, tombstones, receipts, runtime fence.
- `src-tauri/src/portability/records.rs` — Player archive registry/order/status/tombstone suppression.
- `src-tauri/src/portability/archive.rs` — Player manifest counts/checksum/provenance.
- `src-tauri/src/portability/restore.rs` — ID preflight, allowed modes, diffs, merge/apply recheck, runtime reset.
- `src-tauri/src/portability/export.rs` — complete-notebook Player rendering and selected-opponent exclusion.
- `src-tauri/src/portability/mod.rs` and `commands/portability.rs` — existing operation/cancellation/staging/rollback seams.
- `src-tauri/src/commands/player.rs` — preview/confirm Player deletion handlers.
- `src-tauri/src/portability/tests.rs` — archive/restore/export lifecycle coverage.
- `src/lib/ipc/player.ts` and `src/lib/ipc/portability.ts` — deletion and variable restore/diff types consumed later.

### Dependent Files

- `src-tauri/src/player/repository.rs` — canonical rows, receipts, tombstones, and Player-only transaction APIs.
- `src-tauri/src/player/runtime.rs` — whole-delete/restore fencing and disabled reset.
- `src-tauri/src/services/deletion.rs` and `commands/privacy.rs` — opponent behavior reference only; not extended.
- `src/features/backup/BackupPanel.tsx`, `restore/RestorePanel.tsx`, and `export/ExportPanel.tsx` — consume enriched existing portability projections where required.
- `src/features/player/PlayerDeletionDialog.tsx` — Task 05 consumes the deletion projection.
- Existing opponent archive/export paths — regression boundary.

### Related ADRs

- [ADR-001: Keep the Player Workspace Optional and Additive](adrs/adr-001.md) — deletion/export cannot affect opponent workflows.
- [ADR-003: Preserve Immutable Player-Owned Public Result Evidence](adrs/adr-003.md) — defines durable records and version history.
- [ADR-004: Use Dedicated Player Persistence and Trusted-Host Runtime](adrs/adr-004.md) — owns Player tombstone and consent/runtime separation.
- [ADR-005: Persist Player Classification Runs Independently](adrs/adr-005.md) — requires Player classification archive/delete/export handling.

## Deliverables

- Dedicated bound Player deletion service and main-only typed command contract.
- Canonical encrypted Player archive/restore integration and deterministic diff/mode behavior.
- Whole-merge different-ID conflict and tombstone no-resurrection guarantees.
- Complete-notebook Player plaintext section plus selected-opponent Player exclusion.
- Failure-injection/replay/opponent-isolation evidence for destructive and portable operations.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [x] UT-015 — non-content Player tombstone representation.
- [x] UT-066–UT-075 — deletion binding, archive inclusion/exclusion, merge/replace identity, disabled restore, diff, and export scope.
- [x] IT-009 — Player evidence cascade and tombstone persistence.
- [x] IT-036–IT-045 — scoped/cascade deletion, recovery, archive/restore modes, no resurrection, disabled restore, and export boundaries.
- [x] E2E-014 — complete mixed-notebook portability, conflict, replace, and plaintext-scope journey.

## Success Criteria

- Every assigned test case implemented and passing.
- Player deletion and restore failures always recover to one complete state and never mutate opponent ownership or consent.
- Different Player identities cannot be merged or partially reassigned, and deleted Player content cannot resurrect through merge.
- Backup/restore/export contain exactly the approved Player data while authorization and operational state remain non-portable.
