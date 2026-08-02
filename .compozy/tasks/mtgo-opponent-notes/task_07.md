---
status: pending
title: Ship private diagnostics, opt-in updates, offline resilience, and Windows release validation
type: infra
complexity: critical
---

# Task 07: Ship private diagnostics, opt-in updates, offline resilience, and Windows release validation

## Overview

Finish the V1 operational boundary with privacy-preserving local diagnostics, explicit signed update flows, classifier-asset updates, offline resilience, and release-grade Windows evidence. This task turns the completed feature set into a trustworthy packaged application without telemetry or silent network behavior.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- Provider access, application updates, classifier updates, autostart, and diagnostics bundle creation MUST each remain explicit, independently stored user choices with privacy-preserving defaults.
- The application MUST contain no telemetry, analytics, automatic diagnostic upload, or undisclosed network path.
- Local logs MUST use an allowlist schema, exclude notebook content and raw detection data, rotate within seven days and 20 MiB, and expose a redacted preview before explicit bundle creation.
- Redaction MUST fail closed: uncertain fields MUST be removed rather than included, and diagnostics bundles MUST contain only the previewed allowlisted artifacts.
- Update checks MUST send only the documented minimal metadata after opt-in, verify signed metadata and artifacts, require explicit install confirmation, and recover safely from interruption or signature failure.
- Classifier asset updates MUST validate signature, schema, digest, supported formats, unique IDs, and golden vectors before activation, preserving the last known-good assets on any failure.
- Loss of network access MUST immediately preserve all local detection, encounter, note, history, search, backup, restore, export, and classification workflows, with clear provider/update degradation.
- Autostart and tray settings MUST reflect the user's explicit choice and remain reversible without changing notebook data.
- Release validation MUST exercise packaged Windows 10 22H2 and Windows 11 x64 builds, including real UIA/OCR, multi-window, shortcut, encryption, offline, updater, accessibility, and resource evidence.
- No completion claim may bypass the full automated test contract, packaging checks, privacy review, performance budgets, and documented manual evidence or blockers.
</requirements>

## Subtasks

- [ ] 7.1 Deliver versioned local settings and consent lifecycle for provider access, app updates, classifier updates, autostart, and diagnostics.
- [ ] 7.2 Deliver allowlisted structured logging, retention limits, rotation, startup cleanup, and privacy regression checks.
- [ ] 7.3 Deliver diagnostics preview, fail-closed redaction, explicit bundle creation, cancellation, and no-upload guarantees.
- [ ] 7.4 Deliver minimal-metadata signed application update checks, explicit install confirmation, interruption recovery, and last-known-good behavior.
- [ ] 7.5 Integrate signed classifier asset update validation, activation, rollback, and resumable background reclassification.
- [ ] 7.6 Complete offline status, retry, and graceful-degradation behavior across providers, updates, local features, and restart.
- [ ] 7.7 Complete caller-aware settings, diagnostics, update, progress, and error IPC plus accessible user controls.
- [ ] 7.8 Add packaged Windows automation for x64 build, install, launch, encryption, multi-window, offline, update, and security boundaries.
- [ ] 7.9 Capture and document required manual Windows UIA/OCR, tray, shortcut, overlay, accessibility, updater, and performance evidence.
- [ ] 7.10 Run the full unit, integration, end-to-end, packaging, privacy, performance, and release gate and record any unresolved external blocker precisely.

## Implementation Details

Implement the TechSpec's `DiagnosticsService`, signed updater, classifier-update, settings, offline, and release-validation requirements. Diagnostics and updates remain independent opt-ins, and every external path must be both capability-restricted and observable to the user.

### Relevant Files

- `src-tauri/src/diagnostics/` — allowlisted logging, retention, redaction, preview, and bundle creation.
- `src-tauri/src/shell/autostart.rs` and `src-tauri/src/shell/updater.rs` — explicit OS integration and signed update lifecycle.
- `src-tauri/src/commands/diagnostics.rs` and `updates.rs` — caller-aware preview, bundle, check, and confirm commands.
- `src-tauri/src/notebook/settings.rs` — versioned non-secret consent and preference persistence.
- `src-tauri/src/operations/` — update, diagnostics, and reclassification progress and cancellation integration.
- `src/features/settings/`, `src/features/diagnostics/`, and `src/features/updates/` — accessible controls, disclosures, status, and recovery.
- `src/features/onboarding/` — privacy defaults and independent consent choices.
- `src/lib/ipc/diagnostics.ts`, `updates.ts`, and `settings.ts` — typed renderer projections and stable errors.
- `src-tauri/tauri.conf.json` and `src-tauri/capabilities/` — signed updater endpoints, keys, and least-privilege grants.
- `.github/workflows/` and `tests/release/` — packaged Windows validation and evidence artifacts.

### Dependent Files

- `src-tauri/src/classifier/` and `src-tauri/resources/classifier/` — signed asset validation, activation, rollback, and reclassification.
- `src-tauri/src/providers/decks/` — offline and consent-aware provider degradation.
- `src-tauri/src/detection/` — privacy-safe counters without raw handles or OCR content.
- `src-tauri/src/portability/` — diagnostics and release tests verify long-operation privacy and offline behavior.
- `DESIGN.md` — final accessibility, density, focus, color, and desktop interaction evidence.

### Related ADRs

- [ADR-002](adrs/adr-002.md) — conservative disclosure remains enforced in packaged runtime conditions.
- [ADR-003](adrs/adr-003.md) — encrypted persistence and private recovery evidence.
- [ADR-004](adrs/adr-004.md) — least-privilege settings, diagnostics, and updater IPC.
- [ADR-005](adrs/adr-005.md) — constrained opt-in network paths and signed external content.
- [ADR-006](adrs/adr-006.md) — durable progress, cancellation, and restart behavior.
- [ADR-007](adrs/adr-007.md) — signed classifier asset validation and last-known-good activation.

## Deliverables

- Independent privacy-preserving settings and opt-ins for network, startup, update, and diagnostics behavior.
- Allowlisted retained local logs plus previewed, redacted, user-created support bundles with no upload path.
- Signed explicit application and classifier update flows with rollback-safe last-known-good behavior.
- Offline-degradation handling and complete packaged Windows x64 release evidence.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [ ] UT-089, UT-090, UT-091, UT-092, UT-093, UT-094 — log allowlists, retention, redaction, update metadata, signature, and offline state behavior.
- [ ] IT-101, IT-102, IT-103, IT-104, IT-105, IT-106, IT-107, IT-108, IT-109, IT-110 — independent settings, consent persistence, opt-in checks, autostart, and offline degradation.
- [ ] IT-171, IT-172, IT-173, IT-174, IT-175, IT-176, IT-177, IT-178, IT-179, IT-180 — diagnostics logging, retention, preview, redaction, bundle creation, and no-upload boundaries.
- [ ] IT-227, IT-228, IT-231, IT-232, IT-260, IT-263, IT-264, IT-271, IT-281 — signed app/classifier updates, interruption recovery, packaged security, offline resilience, and privacy regression.
- [ ] E2E-011, E2E-018, E2E-019 — offline-first use, private diagnostics/opt-in application update, and signed classifier update/reclassification journeys.

## Success Criteria

- Every assigned test case implemented and passing
- Default installation performs no telemetry, diagnostic upload, provider request, update check, classifier update check, or autostart action without the corresponding user choice.
- Corrupt, unsigned, interrupted, or offline update paths preserve the last known-good application and classifier assets.
- Packaged Windows evidence covers all supported operating systems, security boundaries, accessibility floors, and performance budgets required for V1 release.
