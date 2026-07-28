---
status: pending
title: Scaffold the secure multi-window desktop foundation
type: infra
complexity: high
---

# Task 01: Scaffold the secure multi-window desktop foundation

## Overview

Create the greenfield Tauri 2, React, TypeScript, and Rust workspace that every later slice builds on. This task establishes the three-window security boundary, typed IPC foundation, Windows packaging baseline, and the accessible visual system derived from `DESIGN.md`.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- The workspace MUST use pinned, mutually compatible Tauri 2, React, TypeScript, Rust, test, lint, and build dependencies with reproducible lockfiles.
- The application MUST define separate `main`, `overlay`, and `capture` entrypoints whose labels, initial dimensions, visibility, focus behavior, and capability grants match the TechSpec.
- Rust MUST remain the trusted boundary; webviews MUST NOT receive direct SQL, filesystem, process, unrestricted shell, arbitrary HTTP, encryption-key, raw OCR, or updater-install access.
- IPC contracts MUST expose versioned, serializable request/result/error/event shapes, stable snake-case error codes, and complete replacement events for renderer projections.
- The design foundation MUST encode the colors, typography, spacing, borders, focus states, density, and accessible primitives from `DESIGN.md`, including bundled Inter assets and licensing.
- Windows builds MUST target Windows 10 22H2 and Windows 11 x64 and use the supplied application icon assets.
</requirements>

## Subtasks

- [ ] 1.1 Establish the pinned frontend and Rust workspace, lockfiles, developer scripts, formatting, linting, type checking, and test runners.
- [ ] 1.2 Create independent HTML and React bootstrap entrypoints for the main, overlay, and quick-capture windows.
- [ ] 1.3 Define the initial Tauri window configuration, labels, lifecycle behavior, dimensions, and secure navigation policy.
- [ ] 1.4 Build the shared design tokens and accessible desktop primitives specified by `DESIGN.md`.
- [ ] 1.5 Define typed command results, application errors, caller identities, and versioned host-to-window replacement events.
- [ ] 1.6 Add a safe Rust application bootstrap and command-registration seam for later domain services.
- [ ] 1.7 Configure least-privilege per-window capabilities and automated checks that reject capability drift.
- [ ] 1.8 Integrate the supplied Windows icons, bundled Inter font files, and required attribution/license material.
- [ ] 1.9 Add scaffold-level unit and integration coverage for serialization, window identity, design tokens, and security configuration.
- [ ] 1.10 Add a Windows x64 continuous-integration baseline that builds and tests the greenfield desktop shell.

## Implementation Details

Create the workspace and security seams described by the TechSpec's “Component Overview,” “App IPC,” and “Security Architecture” sections. No feature service belongs in this task; later tasks should plug into stable host composition, typed command, and renderer projection boundaries without widening webview authority.

### Relevant Files

- `package.json` and the selected lockfile — reproducible frontend workspace and validation commands.
- `rust-toolchain.toml` — pinned Rust toolchain for local and CI parity.
- `src/main/`, `src/overlay/`, `src/capture/` — isolated React application entrypoints.
- `src/lib/ipc/` — shared renderer-side IPC contracts and validation helpers.
- `src/ui/` — design tokens, global styles, and accessible shared primitives.
- `src-tauri/Cargo.toml` — trusted-host dependencies and build metadata.
- `src-tauri/src/main.rs` and `src-tauri/src/lib.rs` — Tauri composition and command-registration boundary.
- `src-tauri/src/commands/` and `src-tauri/src/ipc/` — caller-aware command/result contracts.
- `src-tauri/capabilities/` and `src-tauri/tauri.conf.json` — per-window authority and packaging configuration.
- `assets/icons/` — supplied source artwork for generated Windows application assets.

### Dependent Files

- `DESIGN.md` — authoritative visual tokens and component character.
- `.github/workflows/` — Windows build and test automation.
- `src-tauri/src/shell/` — later window, tray, shortcut, and updater orchestration.
- `src/features/` — later main-window feature slices consume the shared primitives.

### Related ADRs

- [ADR-001](adrs/adr-001.md) — establishes the trusted Rust host and local-first desktop boundary.
- [ADR-004](adrs/adr-004.md) — constrains IPC and least-privilege window capabilities.

## Deliverables

- A buildable three-window Tauri workspace with pinned dependencies and reproducible validation commands.
- Typed IPC foundations and least-privilege capabilities for every initial window.
- An accessible shared design system faithful to `DESIGN.md`.
- Windows icon, font, license, packaging, and CI baselines.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [ ] UT-105, UT-106, UT-107, UT-108 — typed IPC envelope and stable application-error serialization.
- [ ] UT-113, UT-114, UT-115, UT-116, UT-117, UT-118, UT-119, UT-120 — design tokens, accessible primitives, window identities, and configuration invariants.
- [ ] IT-191, IT-192, IT-193, IT-272 — per-window capability isolation, secure bootstrap, and packaged configuration boundaries.

## Success Criteria

- Every assigned test case implemented and passing
- All three window entrypoints build and start through the shared trusted-host composition boundary.
- Automated checks demonstrate that renderer windows cannot acquire forbidden host capabilities.
- The shared UI foundation matches the required visual tokens and accessibility floors from `DESIGN.md`.

