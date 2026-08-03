---
status: completed
title: Build the Fail-Closed Public Source Runtime
type: backend
complexity: critical
---

# Task 02: Build the Fail-Closed Public Source Runtime

## Overview

Deliver the trusted native boundary for source status, independent consent, conditional Census lookup, bounded sessions, typed outcomes, and approved browser handoffs. The runtime must remain disabled by default and make every external side effect impossible from renderer-authored nicknames, URLs, scopes, consent, configuration, or provenance.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- Census runtime MUST start disabled and issue zero HTTP requests for absent, placeholder, expired, wrong-scope, malformed, or unreviewed configuration.
- Census MUST use only the fixed HTTPS host/path plus Service ID and approved catalog/start/as-of scope; nickname, cookies, referrer, arbitrary query, redirects, and fallback providers are forbidden.
- Every Player public-source command MUST be `Main`-only at both capability and runtime caller boundaries, with denial before repository, provider, or browser effects.
- Consent MUST be route-specific and bound to disclosure version plus canonical outbound-field digest; revocation MUST cancel/fence matching work immediately.
- The runtime MUST allow one global machine lookup, enforce 5-second connect/15-second total timeouts, 1 MiB/2,000-row/10-preview limits, and perform no automatic retry.
- Only a fully validated response with zero local exact matches MAY create a scoped empty outcome; configuration/provider/schema/limit failures MUST degrade and never become empty.
- Sessions MUST bind identity/revision/nickname snapshot, consent epoch, configuration fingerprint/scope, generation, operation key, and 15-minute expiry; late completions MUST publish nothing.
- Browser handoffs MUST accept only a closed route plus operation key, derive the nickname host-side, and provide at-most-once opening through content-free durable receipts.
- Audit, diagnostics, errors, IPC, and events MUST NOT reveal provider text, nicknames, URLs, tokens, source digests, payloads, cards, or Service IDs.
- Local/synthetic/macOS evidence MUST NOT mark live Census or packaged Windows gates complete.
</requirements>

## Subtasks

- [x] 2.1 Establish the Player runtime lifecycle, disabled provider default, host-only reviewed configuration, and synthetic test injection.
- [x] 2.2 Deliver independent route status and versioned consent admission for Census, official MTGO, and MTGTop8.
- [x] 2.3 Enforce caller, payload, operation-key, identity, and trusted-phase admission before any effect.
- [x] 2.4 Deliver fixed-route Census request construction and strict redirect/timeout/byte/row/schema validation.
- [x] 2.5 Deliver exact local nickname matching plus distinct candidate, scoped-empty, cancelled, and degraded outcomes.
- [x] 2.6 Deliver the one-lookup lease, immutable work leases, cancellation, expiry, cooldown, retry-time, and generation fencing.
- [x] 2.7 Deliver bounded content-free runtime audit and ephemeral replay semantics.
- [x] 2.8 Deliver host-built, separately consented official/MTGTop8 browser handoffs with at-most-once receipts.
- [x] 2.9 Establish the main-only Player command/event/capability admission seam consumed by later command handlers.
- [x] 2.10 Deliver privacy and gate-honesty verification without claiming live or Windows completion.

## Implementation Details

Implement the TechSpec sections “Trusted Host Runtime,” “Census Provider Boundary,” “Phase and Capability Enforcement,” the browser half of “Manual Evidence and Browser Routes,” and the source-policy portions of the command/error surface. Task 02 owns universal command admission and registration; Task 03 extends the same façade for manual preview/import/selection rather than creating a parallel surface.

The current opponent `open_official_deck_page(url)` command is a reference only and MUST NOT be reused because it accepts a renderer-provided URL. If the existing blocking HTTP client is retained, execute it behind a controlled worker boundary and never hold runtime locks across I/O.

### Relevant Files

- `src-tauri/src/player/runtime.rs` — lookup lease, sessions, generations, previews, cooldown, replay, and audit.
- `src-tauri/src/player/census.rs` — disabled/synthetic/live provider modes and bounded validation.
- `src-tauri/src/player/routes.rs` — closed host-built browser routes and manual canonicalization interface.
- `src-tauri/src/player/service.rs` — source status, consent, lookup, cancellation, empty outcomes, and handoffs.
- `src-tauri/src/commands/player.rs` — main-only wrappers, universal admission, payload/replay binding, events.
- `src-tauri/src/ipc/error.rs` and `event.rs` — safe Player errors and `player://workspace-v1`.
- `src-tauri/src/ipc/caller.rs` — runtime caller identity guard.
- `src-tauri/src/disclosure/mod.rs` — authoritative gameplay phase input.
- `src-tauri/src/operations/mod.rs` — cancellation and operation-coordination patterns.
- `src-tauri/src/shell/mod.rs` — validated system-browser side effect boundary.
- `src-tauri/src/diagnostics/mod.rs` — aggregate-only allowlist.
- `src-tauri/capabilities/main.json` and `src-tauri/permissions/app.toml` — main-only Player permissions.

### Dependent Files

- `src-tauri/src/player/repository.rs` — Task 01 identity, consent, empty-outcome, and receipt transaction contracts.
- `src-tauri/src/commands/providers.rs` — consent/status pattern reference; its weaker generic contract is not reused.
- `src-tauri/src/commands/decks.rs` and `providers/decks/mod.rs` — browser/provider plumbing reference only.
- `src-tauri/src/lib.rs` and `commands/mod.rs` — runtime management and command registration.
- `src/lib/ipc/player.ts` and `src/features/player/` — later consume bounded view/error/event contracts.
- Overlay/capture capability manifests — MUST remain Player-command-free.

### Related ADRs

- [ADR-001: Keep the Player Workspace Optional and Additive](adrs/adr-001.md) — public access cannot gate existing workflows.
- [ADR-002: Use Explicit Conditional and Manual Public Source Routes](adrs/adr-002.md) — fixes the V1 source catalog and no-fallback rule.
- [ADR-003: Preserve Immutable Player-Owned Public Result Evidence](adrs/adr-003.md) — constrains preview binding and exact-match semantics.
- [ADR-004: Use Dedicated Player Persistence and Trusted-Host Runtime](adrs/adr-004.md) — establishes native authority and Player isolation.
- [ADR-006: Keep Census Configuration Host-Only and Disabled by Default](adrs/adr-006.md) — owns live-construction and zero-request gates.

## Deliverables

- Host-owned Player runtime with sessions, cancellation, fencing, cooldown, replay, and content-free audit.
- Disabled/synthetic/live Census adapter boundary with strict route and response validation.
- Independent source consent/status and exact-match/scoped-empty/degraded behavior.
- Host-built official and MTGTop8 browser handoffs with main-only at-most-once authority.
- Safe Player error/event/capability/diagnostic contracts and honest live/Windows gate status.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [ ] UT-011–UT-012 — scoped-empty construction and redacted Player error contract.
- [ ] UT-018–UT-021 — status, consent, phase, and pre-effect payload admission.
- [ ] UT-024–UT-037 — exact matching, preview cap, provider modes/config/request/response bounds, empty/degraded, cooldown, and expiry.
- [ ] UT-039–UT-048, UT-050 — fencing, lease/race/audit/replay/retry, trusted routes, and browser receipt.
- [ ] IT-011–IT-023, IT-025 — main capability, consent, provider races/bounds/outcomes, browser behavior, and privacy leakage.
- [ ] IT-056–IT-058, IT-060 — disabled live configuration, config fencing, privacy scan, and evidence-gate honesty.
- [ ] E2E-016 — live Census remains disabled until current external authorization/configuration/contract evidence exists.

## Success Criteria

- Every assigned test case implemented and passing.
- No renderer, overlay, capture surface, invalid phase, absent consent, or invalid configuration can cause a Player network/browser effect.
- Valid synthetic Census responses yield only bounded exact candidates or truthful scoped empty outcomes; every failure path degrades without content leakage.
- Live Census and packaged Windows status remain explicitly pending unless their external evidence is actually present.
