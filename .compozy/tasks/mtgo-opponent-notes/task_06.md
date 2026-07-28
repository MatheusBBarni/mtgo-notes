---
status: pending
title: Deliver encrypted backup, staged restore, and text export
type: backend
complexity: critical
---

# Task 06: Deliver encrypted backup, staged restore, and text export

## Overview

Deliver user-controlled portability without weakening the encrypted local notebook: authenticated encrypted backups, staged merge or replace restore, explicit rollback access, and one-way UTF-8 text export. Long-running operations must remain bounded, cancellable where safe, atomic at file boundaries, and recoverable after failure.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- Rust MUST own passphrases, cryptographic operations, archive validation, staging databases, filesystem access, atomic replacement, and rollback artifacts; webviews MUST receive only typed status and previews.
- Backups MUST be canonical logical authenticated archives containing no plaintext notebook data, database key, DPAPI blob, or machine-bound secret.
- Backup, restore, and export MUST stream with bounded memory below the TechSpec limit, write to an explicit partial path, flush durable output, and atomically publish only complete files.
- Restore MUST authenticate and validate the full archive before mutation, import into a staging SQLCipher database, and issue a bound expiring preview token before merge or replace confirmation.
- Merge restore MUST obey exact identity, immutable-source, classifier-provenance, tombstone, revision, and no-resurrection rules; replace restore MUST use an atomic live-database swap.
- Replace restore MUST create a bounded encrypted rollback artifact and expose typed confirm/apply/discard rollback lifecycle commands, closing the PRD requirement even where the initial command table is incomplete.
- Concurrent snapshot backup and text export MAY run when the specific test contract permits them, but migration, restore, purge, replace, and other destructive operations MUST obey the documented exclusion matrix.
- Wrong archive credentials MUST preserve the stable user-facing `wrong_passphrase` contract even if the internal authenticated-archive layer reports a broader authentication failure.
- Text export MUST be an explicit one-way UTF-8 action with clear plaintext disclosure and deterministic, human-readable opponent, encounter, observation, deck, and provenance sections.
- Progress MUST be monotonic; cancellation MUST leave live data unchanged and remove or recover partial artifacts according to operation state.
</requirements>

## Subtasks

- [ ] 6.1 Complete the durable operation lifecycle, exclusion matrix, progress, cancellation, restart, and priority behavior required by portability flows.
- [ ] 6.2 Define the versioned canonical archive manifest, authenticated encryption contract, KDF parameters, logical record ordering, and interoperability fixtures.
- [ ] 6.3 Deliver streaming encrypted backup with snapshot consistency, partial-file handling, durable publication, cancellation, and safe retry.
- [ ] 6.4 Deliver archive authentication, schema validation, staging import, integrity checks, restore diff calculation, and expiring preview tokens.
- [ ] 6.5 Deliver transactional merge restore with exact identity, duplicate, provenance, classifier, deletion, and no-resurrection rules.
- [ ] 6.6 Deliver atomic replace restore plus bounded encrypted rollback creation and typed rollback confirm, apply, and discard lifecycle.
- [ ] 6.7 Deliver deterministic UTF-8 plaintext text export with explicit disclosure, snapshot consistency, partial-file safety, and cancellation.
- [ ] 6.8 Complete caller-aware portability commands, safe native file selection, typed events, and host-owned secret handling.
- [ ] 6.9 Deliver accessible backup, preview, merge/replace, rollback, export, progress, cancellation, and recovery UI.
- [ ] 6.10 Add cryptographic fixtures, failure injection, bounded-memory checks, concurrency coverage, and full end-to-end portability journeys.

## Implementation Details

Implement the TechSpec's “Backup and Restore Format,” `PortabilityService`, and `OperationCoordinator` behavior. Select the concrete audited cryptographic crate only after supply-chain and interoperability verification; preserve the specified wire-format and stable error contracts independently of that choice.

The specific concurrency cases in `_tests.md` govern snapshot backup/export coexistence where they are more precise than broad ADR prose. Treat rollback access as required product scope and add the missing typed command lifecycle consistent with the PRD and TechSpec rather than silently omitting it.

### Relevant Files

- `src-tauri/src/operations/` — durable lifecycle, exclusion matrix, snapshots, progress, priority, cancellation, and recovery.
- `src-tauri/src/portability/archive.rs` — canonical manifest, record encoding, authentication, and version checks.
- `src-tauri/src/portability/backup.rs` — streaming encrypted logical backup and atomic publication.
- `src-tauri/src/portability/restore.rs` — authentication, staging import, preview, merge, replace, and rollback.
- `src-tauri/src/portability/export.rs` — deterministic disclosed UTF-8 text export.
- `src-tauri/src/commands/portability.rs` — caller-aware backup, restore, rollback, and export commands.
- `src/lib/ipc/portability.ts` and `src/lib/ipc/operations.ts` — typed previews, operations, errors, progress, and cancellation.
- `src/features/backup/`, `src/features/restore/`, `src/features/export/`, and `src/features/operations/` — portability UI.
- `tests/fixtures/portability/` — versioned valid, invalid, interrupted, wrong-passphrase, and cross-version archives.

### Dependent Files

- `src-tauri/src/notebook/` — logical records, snapshots, staging SQLCipher databases, migrations, and atomic live swap.
- `src-tauri/src/services/identity.rs` and `deletion.rs` — merge, tombstone, and no-resurrection invariants.
- `src-tauri/src/classifier/` — immutable classifier provenance and reclassification scheduling after restore.
- `src-tauri/src/shell/` — native file selection without renderer filesystem authority.
- `src/features/settings/` — portability entrypoints and plaintext-export disclosure.

### Related ADRs

- [ADR-003](adrs/adr-003.md) — encrypted local persistence and portable-data protection.
- [ADR-004](adrs/adr-004.md) — host-owned secrets, filesystem access, and typed portability IPC.
- [ADR-006](adrs/adr-006.md) — coordinated durable long-running operations.

## Deliverables

- Versioned authenticated encrypted backup archives with bounded-memory streaming and atomic publication.
- Staged, previewed merge/replace restore with exact conflict semantics and no live mutation before confirmation.
- Atomic replacement plus discoverable, typed, encrypted rollback apply/discard lifecycle.
- Disclosed deterministic UTF-8 text export and accessible operation/recovery UI.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [ ] UT-077, UT-078, UT-079, UT-080, UT-081, UT-082 — archive manifests, canonical records, authentication, KDF, and version behavior.
- [ ] UT-083, UT-084, UT-085, UT-086, UT-087, UT-088 — restore diff, merge decisions, rollback state, text formatting, and operation transitions.
- [ ] IT-131, IT-132, IT-133, IT-134, IT-135, IT-136, IT-137, IT-138, IT-139, IT-140 — encrypted backup creation, snapshot consistency, bounded memory, progress, and cancellation.
- [ ] IT-141, IT-142, IT-143, IT-144, IT-145, IT-146, IT-147, IT-148, IT-149, IT-150 — archive authentication, wrong passphrase, validation, staging import, and restore preview.
- [ ] IT-151, IT-152, IT-153, IT-154, IT-155, IT-156, IT-157, IT-158, IT-159, IT-160 — merge/replace confirmation, atomicity, conflicts, tombstones, classifier provenance, and no resurrection.
- [ ] IT-220, IT-221, IT-222, IT-223, IT-224 — rollback discovery, confirmation, apply, discard, and retention lifecycle.
- [ ] IT-254, IT-255, IT-256, IT-257, IT-258 — disclosed UTF-8 export contents, ordering, atomic publication, and cancellation.
- [ ] IT-268 — bounded portability memory under the specified large-notebook workload.
- [ ] E2E-009, E2E-014, E2E-015, E2E-016, E2E-017 — deletion-safe export, encrypted backup, merge restore, replace/rollback, and portability recovery journeys.

## Success Criteria

- Every assigned test case implemented and passing
- Backup archives reveal no plaintext notebook content and restore never mutates live data before successful authentication, staging, preview, and confirmation.
- Interrupted, cancelled, corrupt, incompatible, or wrong-passphrase operations leave the current notebook usable and do not publish partial output.
- Users can explicitly recover or discard the encrypted rollback after replace restore and can export a deterministic disclosed text snapshot.

