---
status: pending
title: Deliver the personal notebook, history, identity, and deletion workflows
type: frontend
complexity: high
---

# Task 04: Deliver the personal notebook, history, identity, and deletion workflows

## Overview

Turn the encrypted core into a useful personal opponent notebook with profiles, aliases, observations, structured card knowledge, tendencies, searchable encounter history, identity correction, and privacy-safe deletion. This task owns the full user-facing notebook workflow while preserving provenance, reversibility, and phase-sensitive disclosure.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- Handles and tags MUST use the exact TechSpec normalization rules while preserving display values and MUST NOT be fuzzy-merged automatically.
- A free-text observation MUST be the only required note input; card observations, quantities, certainty, context, tags, and tendencies MUST remain optional structured enrichments with provenance.
- History and profile views MUST work offline, use stable cursor pagination, exclude tombstoned content, and expose the last deck seen with its user-entered or official-source provenance.
- Search and profile/history commands MUST be denied whenever DisclosurePolicy disallows them, regardless of renderer state or cached UI data.
- Merge previews MUST enumerate affected profiles, aliases, encounters, observations, decks, and conflicts before confirmation; merge and unmerge MUST be durable, transactional, and reversible where the PRD permits.
- Deletion MUST use tombstones, explicit confirmation, an undo window, durable purge coordination, FTS removal, and safeguards that prevent deleted data from resurfacing through merge, restore, search, or reclassification.
- The main, overlay, and capture surfaces MUST receive only caller-authorized typed projections and stable errors.
- All visible notebook workflows MUST follow `DESIGN.md` density, keyboard, focus, contrast, empty-state, and error-recovery requirements.
</requirements>

## Subtasks

- [ ] 4.1 Define notebook service contracts and renderer projections for profiles, aliases, observations, history, decks-seen summaries, identity actions, and deletion state.
- [ ] 4.2 Deliver profile creation, exact identity lookup, alias management, editing, revision conflict handling, and provenance display.
- [ ] 4.3 Deliver free-text observations plus optional card, certainty, context, tendency, and tag enrichment across main and capture flows.
- [ ] 4.4 Deliver offline paged encounter history, full-text search, filters, profile timelines, and last-deck-seen presentation.
- [ ] 4.5 Deliver merge and unmerge previews that expose reassignment plans, conflicts, irreversible consequences, and affected counts.
- [ ] 4.6 Implement transactional merge, reversal, alias reassignment, duplicate handling, revision checks, and undo records.
- [ ] 4.7 Deliver deletion confirmation, tombstones, undo, durable purge, search removal, and no-resurrection protections.
- [ ] 4.8 Complete caller-aware typed commands and replacement events for every notebook, identity, history, and privacy workflow.
- [ ] 4.9 Integrate live encounter, overlay, capture, classifier, restore, and export seams without duplicating their owning services.
- [ ] 4.10 Add unit, integration, accessibility, and end-to-end coverage for notebook creation, discovery, correction, deletion, and recovery.

## Implementation Details

Implement the TechSpec's `NotebookService`, profile and observation models, FTS-backed history, identity transaction rules, and deletion lifecycle. Keep all mutation orchestration in Rust services; React owns accessible presentation, local form state, confirmation surfaces, and complete projection replacement.

### Relevant Files

- `src-tauri/src/services/profiles.rs` — profile, alias, edit, and exact identity workflows.
- `src-tauri/src/services/observations.rs` — free-text and structured observation lifecycle.
- `src-tauri/src/services/history.rs` — cursor-paged history, search, filters, and last-deck-seen projections.
- `src-tauri/src/services/identity.rs` — merge preview, merge, unmerge, and conflict handling.
- `src-tauri/src/services/deletion.rs` — tombstone, undo, purge, and no-resurrection orchestration.
- `src-tauri/src/commands/notes.rs`, `history.rs`, `identity.rs`, and `privacy.rs` — caller-aware command surfaces.
- `src/features/profiles/`, `src/features/observations/`, `src/features/history/`, and `src/features/search/` — core notebook UI.
- `src/features/identity/` and `src/features/privacy/` — merge, correction, deletion, undo, and purge UI.
- `src/lib/ipc/` — typed notebook, identity, history, and privacy contracts.

### Dependent Files

- `src-tauri/src/notebook/` — transactional persistence, FTS, tombstones, and cursor reads.
- `src-tauri/src/disclosure/` — phase-sensitive command authorization and projections.
- `src/capture/` and `src/overlay/` — limited notebook inputs and safe current-context views.
- `src-tauri/src/classifier/` — later classification records feed last-deck-seen summaries.
- `src-tauri/src/portability/` — later restore and export must honor merge/deletion invariants.

### Related ADRs

- [ADR-001](adrs/adr-001.md) — encrypted local notebook ownership.
- [ADR-002](adrs/adr-002.md) — phase-sensitive disclosure restrictions.
- [ADR-003](adrs/adr-003.md) — durable encrypted records and tombstones.
- [ADR-004](adrs/adr-004.md) — typed, caller-scoped command projections.
- [ADR-006](adrs/adr-006.md) — purge and recovery operation coordination.
- [ADR-007](adrs/adr-007.md) — downstream immutable archetype provenance shown by notebook views.

## Deliverables

- Complete profile, alias, observation, structured note, tag, and tendency workflows.
- Offline paged history, search, profile timelines, and provenance-aware last-deck-seen views.
- Previewed and reversible identity correction with conflict-safe merge/unmerge.
- Tombstone-based deletion, undo, durable purge, and no-resurrection behavior.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [ ] UT-068, UT-069, UT-070, UT-071, UT-072, UT-073, UT-074, UT-075, UT-076 — profile, observation, history, identity, and deletion projections.
- [ ] IT-071, IT-072, IT-073, IT-074, IT-075, IT-076, IT-077, IT-078, IT-079, IT-080 — profile, alias, exact normalization, edit, and revision-conflict workflows.
- [ ] IT-081, IT-082, IT-083, IT-084, IT-085, IT-086, IT-087, IT-088, IT-089, IT-090 — free-text, card, certainty, context, tendency, tag, and capture-linked observations.
- [ ] IT-111, IT-112, IT-113, IT-114, IT-115, IT-116, IT-117, IT-118, IT-119, IT-120 — offline history, pagination, search, filtering, and tombstone exclusion.
- [ ] IT-121, IT-122, IT-123, IT-124, IT-125, IT-126, IT-127, IT-128, IT-129, IT-130 — profile timelines, provenance, last deck seen, and disclosure-denied history.
- [ ] IT-161, IT-162, IT-163, IT-164, IT-165, IT-166, IT-167, IT-168, IT-169, IT-170 — merge preview, transactional merge, conflict, idempotency, reversal, and alias reassignment.
- [ ] IT-207, IT-208, IT-209, IT-212, IT-213, IT-214 — deletion confirmation, tombstones, undo tokens, and UI recovery.
- [ ] IT-216, IT-217, IT-218, IT-219, IT-225, IT-226, IT-235, IT-247 — purge coordination, FTS removal, no resurrection, notebook detail, and caller authorization.
- [ ] IT-250, IT-251, IT-252, IT-253, IT-259 — keyboard, focus, accessibility, replacement projection, and error-state behavior.
- [ ] E2E-008, E2E-012, E2E-013 — personal notebook/history, identity correction, and deletion/undo journeys.

## Success Criteria

- Every assigned test case implemented and passing
- Users can capture, enrich, find, edit, and understand opponent knowledge entirely offline.
- Identity correction is previewed, conflict-safe, and reversible without losing provenance.
- Deleted content disappears from every read model immediately and cannot return through later background or portability workflows.
