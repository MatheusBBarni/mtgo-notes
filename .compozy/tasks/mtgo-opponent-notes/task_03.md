---
status: pending
title: Deliver automatic match context and rapid in-match capture
type: backend
complexity: critical
---

# Task 03: Deliver automatic match context and rapid in-match capture

## Overview

Deliver the companion's core live-match experience: user-authorized MTGO window selection, conservative automatic opponent and phase detection, encounter lifecycle handling, tray and overlay context, and a keyboard-first quick-capture window. The trusted host must preserve tournament-safe disclosure while making note capture fast and resilient.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- Detection MUST operate only on the visible MTGO top-level window explicitly selected by the user and MUST NOT inspect process memory, MTGO logs, private network traffic, hidden windows, or unrelated applications.
- Windows UI Automation MUST be the primary evidence source; cropped Windows OCR MAY fill inaccessible fields only while the authorized window is visible, and raw pixels and OCR strings MUST remain ephemeral.
- Evidence MUST include provider session, generation, sequence, monotonic time, confidence, and provenance so stale, duplicate, conflicting, and previous-opponent responses cannot mutate current state.
- The host MUST require confirmation before persisting a candidate opponent and MUST treat a newly confirmed different opponent as the usual end of the previous match through one reversible compound transition.
- Gameplay uncertainty MUST immediately restrict historical disclosure, and the overlay MUST render only complete replacement projections authorized by DisclosurePolicy.
- The overlay MUST avoid focus theft, remain click-through while collapsed, support an explicit expanded interaction state, and clear forbidden data before a restricted phase is rendered.
- The quick-capture window MUST be single-instance, keyboard-first, and draft-safe: Enter saves, Escape dismisses, and failed saves preserve recoverable encrypted input.
- Pausing detection, minimizing/occluding MTGO, losing the selected window, or going offline MUST degrade to clear manual workflows without blocking local capture.
</requirements>

## Subtasks

- [ ] 3.1 Deliver onboarding consent, disclosed field selection, MTGO window authorization, re-selection, pause, and revoke controls.
- [ ] 3.2 Implement event-driven UI Automation evidence for opponent, phase, format, game, and visible result signals.
- [ ] 3.3 Implement bounded cropped Windows OCR fallback, confidence handling, backoff, and ephemeral evidence cleanup.
- [ ] 3.4 Connect provider generations and evidence streams to the encounter reducer, confirmation, automatic rollover, incomplete persistence, undo, and resume flows.
- [ ] 3.5 Deliver manual opponent, phase, match-end, and correction controls for unsupported or uncertain contexts.
- [ ] 3.6 Implement tray residency, lifecycle actions, selected-window status, pause state, and safe shutdown behavior.
- [ ] 3.7 Deliver the always-on-top overlay with policy-filtered replacement projections, restricted states, accessibility, and no-focus-steal behavior.
- [ ] 3.8 Deliver the global quick-capture shortcut and single-instance capture editor with encrypted draft recovery.
- [ ] 3.9 Complete least-privilege shell, capture, detection-resource, and caller-capability integration.
- [ ] 3.10 Add deterministic provider fixtures plus unit, integration, performance, accessibility, and end-to-end coverage for the entire live-match flow.

## Implementation Details

Implement the TechSpec's “Detector Design,” “Encounter State Machine,” “Runtime and Data Flow,” and window-specific requirements. The detector produces evidence only; EncounterEngine and DisclosurePolicy from Task 02 remain authoritative for state transitions, persistence, and renderer-visible data.

### Relevant Files

- `src-tauri/src/detection/` — provider lifecycle, UIA events, cropped OCR, normalization, evidence, and confidence.
- `src-tauri/resources/detection/` — signed, versioned semantic locators, OCR regions, language, and supported MTGO UI versions.
- `src-tauri/src/commands/providers.rs` — consent, window selection, pause, resume, and capability status.
- `src-tauri/src/commands/encounters.rs` — candidate confirmation, manual controls, completion, reopen, and undo.
- `src-tauri/src/commands/capture.rs` — capture-window lifecycle, draft, save, and dismissal.
- `src-tauri/src/shell/` — tray, global shortcut, window lifecycle, single instance, and focus behavior.
- `src/features/onboarding/` and `src/features/encounter/` — consent and live encounter controls.
- `src/overlay/` — policy-filtered compact and expanded overlay.
- `src/capture/` — keyboard-first quick-capture experience.
- `src/lib/ipc/provider.ts`, `src/lib/ipc/encounter.ts`, `src/lib/ipc/capture.ts` — typed renderer contracts.

### Dependent Files

- `src-tauri/src/encounters/` — consumes ordered evidence and owns encounter transitions.
- `src-tauri/src/disclosure/` — authorizes every visible overlay projection and restricted query.
- `src-tauri/src/notebook/` — persists confirmed encounters, transitions, drafts, and observations.
- `src-tauri/capabilities/` and `src-tauri/tauri.conf.json` — scoped window and shell grants.
- `tests/fixtures/detection/` — deterministic supported, degraded, stale, and conflicting evidence scenarios.

### Related ADRs

- [ADR-001](adrs/adr-001.md) — trusted local detection and persistence boundary.
- [ADR-002](adrs/adr-002.md) — conservative encounter detection and disclosure state.
- [ADR-004](adrs/adr-004.md) — caller-scoped commands and replacement events.
- [ADR-005](adrs/adr-005.md) — constrained external and Windows-platform integrations.

## Deliverables

- User-authorized UIA detection with bounded cropped OCR fallback and deterministic fixtures.
- Automatic and manual encounter lifecycle controls with reversible opponent rollover and incomplete recovery.
- Tray, overlay, and quick-capture experiences that remain useful offline and fail closed during uncertainty.
- Complete capability, performance, accessibility, and packaged live-flow coverage.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [ ] UT-001, UT-002, UT-003, UT-004, UT-005, UT-006, UT-007, UT-008 — context evidence normalization, confidence, precedence, and generation semantics.
- [ ] UT-095, UT-096, UT-097, UT-098, UT-099, UT-100, UT-101, UT-102, UT-103, UT-104 — live-window, overlay, capture, and shell projection behavior.
- [ ] UT-109, UT-110, UT-112 — manual fallback, draft recovery, and restricted replacement-event behavior.
- [ ] IT-001, IT-002, IT-003, IT-004, IT-005, IT-006, IT-007, IT-008, IT-009, IT-010 — consent, window selection, provider lifecycle, and safe detection startup.
- [ ] IT-011, IT-012, IT-013, IT-014, IT-015, IT-016, IT-017, IT-018, IT-019, IT-020 — UIA opponent and phase evidence plus candidate confirmation flows.
- [ ] IT-021, IT-022, IT-023, IT-024, IT-025, IT-026, IT-027, IT-028, IT-029, IT-030 — OCR fallback, crop constraints, visibility, confidence, and evidence cleanup.
- [ ] IT-031, IT-032, IT-033, IT-034, IT-035, IT-036, IT-037, IT-038, IT-039, IT-040 — stale, duplicate, conflicting, and reordered evidence handling.
- [ ] IT-041, IT-042, IT-043, IT-044, IT-045, IT-046, IT-047, IT-048, IT-049, IT-050 — encounter start, rollover, completion, incomplete, reopen, and undo transitions.
- [ ] IT-051, IT-052, IT-053, IT-054, IT-055, IT-056, IT-057, IT-058, IT-059, IT-060 — conservative disclosure changes and overlay replacement projections.
- [ ] IT-061, IT-062, IT-063, IT-064, IT-065, IT-066, IT-067, IT-068, IT-069, IT-070 — tray, shortcut, capture single-instance, save, dismissal, and draft recovery.
- [ ] IT-194, IT-195, IT-196, IT-197, IT-198, IT-199, IT-200, IT-201, IT-202, IT-203, IT-204, IT-205, IT-206 — manual fallback, provider interruption, restart, and unsupported-client behavior.
- [ ] IT-236, IT-237, IT-238, IT-239, IT-240, IT-241, IT-242, IT-243, IT-244, IT-245, IT-246 — overlay accessibility, focus, click-through, layout, and rendering boundaries.
- [ ] IT-265, IT-266, IT-267, IT-269 — detection/capture performance and event-priority constraints.
- [ ] IT-273, IT-274, IT-275, IT-276, IT-277, IT-282 — restricted-data clearing, caller authorization, and raw-capture privacy boundaries.
- [ ] E2E-001, E2E-002, E2E-003, E2E-004, E2E-005, E2E-006, E2E-007 — onboarding through automatic/manual encounter detection, disclosure, rollover, overlay, and rapid capture journeys.

## Success Criteria

- Every assigned test case implemented and passing
- A user can move from first-run consent to a confirmed live opponent and save a note without opening the main window.
- Restricted or uncertain gameplay never exposes forbidden history, even during reordered events or window transitions.
- Detection and capture meet the TechSpec latency, resource, accessibility, and no-focus-steal budgets on supported packaged Windows builds.
