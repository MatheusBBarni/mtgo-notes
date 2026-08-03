---
status: completed
title: Integrate the Accessible Player Workspace
type: frontend
complexity: critical
---

# Task 05: Integrate the Accessible Player Workspace

## Overview

Integrate the completed Player backend into an optional, main-window-only, typed, responsive, and accessible workspace that implements the approved prototype states. This final slice owns renderer projections, navigation, interaction/focus behavior, end-to-end regression coverage, and honest packaged-Windows evidence without moving policy authority into React.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- The Player tab MUST always be discoverable, MUST not be the default, and MUST never block or auto-initialize identity/consent/session/evidence for existing workflows.
- React MUST treat complete host `PlayerWorkspaceView` projections as authoritative and MUST NOT derive consent, provider status, routes, provenance, or policy locally.
- The workspace reducer MUST ignore equal/older revisions and request a fresh full snapshot after a revision gap or deserialization failure.
- Saved evidence MUST remain visible through loading, cancellation, empty, degraded, disabled, retry, and browser-failure states.
- Identity, consent/revoke, source actions, manual entry, candidate/field selection, import, refresh, and deletion MUST be keyboard-operable with visible unobscured focus and no status-driven focus theft.
- Every state MUST render text plus programmatic status independent of color, and raw error details/provider content/tokens/digests/secrets MUST never render.
- The UI MUST implement the approved two-column layout and stack in reading order at the prototype's 900px compact breakpoint without clipping/overflow.
- Candidate provenance/attribution MUST be mandatory; import MUST disable at zero selections and summarize selected results/fields exactly.
- Player deletion MUST display the bound target/counts, require explicit confirmation, and send no mutation intent when cancelled; create a bounded Player-specific confirmation surface because no shared destructive-dialog primitive currently exists.
- Player permissions MUST exist only in the main capability manifest and runtime caller checks; overlay/capture manifests MUST remain Player-free.
- Local axe/jsdom/macOS results MUST NOT mark packaged Windows native accessibility, browser, focus, encryption, or installer evidence complete.
</requirements>

## Subtasks

- [x] 5.1 Add the always-present optional Player tab while preserving default and mounted-panel behavior for every existing workspace.
- [x] 5.2 Deliver typed Player IPC clients, safe errors, event allowlist, replacement projection fixtures, and main-only capability integration.
- [x] 5.3 Deliver the revision-safe feature reducer/subscription lifecycle and snapshot recovery after gaps or malformed events.
- [x] 5.4 Deliver first-use, local identity save/edit, historical warning, source status, and inline consent/revocation states.
- [x] 5.5 Deliver lookup/loading/cancel/retry/candidate/empty/degraded/disabled/browser-handoff states while preserving evidence.
- [x] 5.6 Deliver manual official evidence entry, exact candidate/field selection, mandatory provenance, and stable import summary.
- [x] 5.7 Deliver immutable evidence list/detail, version links, selection revisions, classification state, and explicit refresh.
- [x] 5.8 Deliver bounded scoped/whole Player deletion preview and confirmation interactions.
- [x] 5.9 Deliver approved responsive layout, keyboard/focus restoration, live announcements, safe copy, and axe coverage.
- [x] 5.10 Run integrated Player/opponent/privacy regressions and collect or leave explicitly pending the assigned packaged Windows evidence.

## Implementation Details

Implement the TechSpec section “Replacement Event and Frontend State” using the approved prototype as the interaction authority. `MainApp` force-mounts panels, so Player mount/bootstrap MUST remain read-only; simply loading or visiting the tab cannot create identity, consent, session, evidence, or external access.

Task 05 consumes Tasks 01–04 host commands/views. It may finish native projection/event wiring needed by UT-063/UT-064 and capability/E2E integration, but it MUST NOT reimplement provider, consent, evidence, deletion, or portability rules in TypeScript.

### Relevant Files

- `src/main/MainApp.tsx` — always-present non-default Player tab/panel.
- `src/features/player/usePlayerWorkspace.ts` — replacement-view reducer, local drafts/selections, pending actions, focus targets.
- `src/features/player/PlayerWorkspace.tsx` — responsive container and status region.
- `src/features/player/PlayerIdentityPanel.tsx` — first use, save/edit, historical warning.
- `src/features/player/PlayerSourceControls.tsx` — inline consent, status, revoke, and source actions.
- `src/features/player/PlayerLookupPanel.tsx` — progress/cancel/outcomes/retry.
- `src/features/player/PlayerCandidateList.tsx` and `PlayerSelectionBar.tsx` — result/field selection and stable import action.
- `src/features/player/PlayerEvidenceList.tsx` and `PlayerEvidenceDetails.tsx` — immutable history, provenance, versions, classification.
- `src/features/player/ManualEvidenceForm.tsx` — bounded typed manual input.
- `src/features/player/PlayerDeletionDialog.tsx` — Player-specific scoped confirmation surface.
- `src/lib/ipc/player.ts`, `contracts.ts`, and `events.ts` — typed commands/errors/replacement event.
- `src/ui/global.css` and `src/ui/primitives/` — layout, focus, status, and existing accessible controls.
- `src-tauri/capabilities/main.json`, `overlay.json`, and `capture.json` — positive/negative capability proof.
- `tests/unit/main-navigation.test.tsx` and new Player unit/integration suites — navigation/state/accessibility coverage.
- `tests/release/README.md` and `validate-windows.ps1` — packaged Player evidence contract.

### Dependent Files

- `src-tauri/src/player/` and `src-tauri/src/commands/player.rs` — completed authoritative services/projections.
- `src-tauri/src/ipc/event.rs` and `src-tauri/src/lib.rs` — event/command registration.
- `src/features/notebook/NotebookWorkspace.tsx` and `decks/DeckEnrichmentPanel.tsx` — existing main flows that must remain unchanged.
- `src/capture/CaptureApp.tsx` and `src/overlay/OverlayApp.tsx` — must remain incapable of Player commands/data.
- `.scratch/player-identity-public-results/prototypes/player-workspace/` — approved state/layout/interaction reference.

### Related ADRs

- [ADR-001: Keep the Player Workspace Optional and Additive](adrs/adr-001.md) — owns optional navigation and no setup gate.
- [ADR-002: Use Explicit Conditional and Manual Public Source Routes](adrs/adr-002.md) — owns visible source distinctions and explicit actions.
- [ADR-003: Preserve Immutable Player-Owned Public Result Evidence](adrs/adr-003.md) — owns provenance/version presentation.
- [ADR-004: Use Dedicated Player Persistence and Trusted-Host Runtime](adrs/adr-004.md) — host projection remains authoritative.
- [ADR-005: Persist Player Classification Runs Independently](adrs/adr-005.md) — Player classification presentation.
- [ADR-006: Keep Census Configuration Host-Only and Disabled by Default](adrs/adr-006.md) — UI exposes status but no configuration authority.

## Deliverables

- Typed main-only Player IPC/event/projection integration and safe error copy.
- Optional top-level Player workspace matching the approved responsive interaction model.
- Complete identity, consent, lookup, manual preview, import, evidence, refresh, and deletion states.
- Keyboard/focus/live-region/axe/responsive behavior plus existing-workflow regressions.
- Packaged Windows evidence hooks/checklist with native-only cases left pending until actually collected.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [x] UT-063–UT-065 — evidence-preserving native projections and monotonic frontend replacement reducer.
- [x] UT-076–UT-085 — first use, disclosures, selection, accessible states/focus/layout/errors/evidence/deletion, and axe fixtures.
- [x] IT-046–IT-055 — typed IPC/events, navigation, identity, consent/status, lookup/candidate/empty/degraded/manual, keyboard, and axe integration.
- [x] IT-059 — full existing opponent/local verification regression with populated Player data.
- [x] E2E-001–E2E-013 — optional setup through identity, source actions, evidence, refresh, classification, and deletion journeys.
- [ ] E2E-015 — packaged Windows 10/11 keyboard/screen-reader/focus/contrast/scaling/clipping/browser evidence; pending because native packaged artifacts were not collected in this environment.
- [x] E2E-017 — packaged mixed-workflow opponent isolation and overlay/capture capability denial.

## Success Criteria

- Every assigned locally runnable test case implemented and passing; native-only assigned cases remain explicitly pending until their required evidence exists.
- A user can complete every approved Player journey by keyboard with truthful focus/status behavior while saved evidence remains visible across outcomes.
- No Player tab mount, renderer input, overlay, or capture surface can create or authorize an external/durable Player action independently of the host.
- Existing notebook, capture, overlay, opponent enrichment, portability, and release verification remain green with zero or populated Player data.
