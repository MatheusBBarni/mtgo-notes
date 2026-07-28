---
status: pending
title: Deliver official deck enrichment and local archetype classification
type: backend
complexity: high
---

# Task 05: Deliver official deck enrichment and local archetype classification

## Overview

Add consent-aware official MTGO deck enrichment and deterministic local archetype classification for complete confirmed decklists. The result enriches an opponent's last-deck context without third-party scraping, editable classifier rules, or loss of source and classifier provenance.

<critical>
- ALWAYS READ the PRD, the TechSpec, and their catalogs (`_user_stories.md`, `_tests.md`) before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — implement every test case assigned in ## Tests
</critical>

<requirements>
- Automated lookup MUST remain disabled until an implementation spike confirms documented or explicitly permitted official MTGO access, stable response semantics, limits, and redistributable fields.
- If automated official access is not validated, the provider MUST return `interactive_required`, open only an allowlisted official MTGO page, and accept a user-confirmed official result without scraping third-party sites.
- Provider access MUST be explicit opt-in and MUST send only the confirmed opponent handle and format; requests and responses MUST be bound to encounter generation and request token.
- Confirmed public results MUST persist immutable source metadata and complete deck revisions transactionally and MUST reject stale, partial, malformed, oversized, non-HTTPS, wrong-host, or wrong-format inputs.
- Classification MUST operate only on complete canonical deck revisions, evaluate all signature constraints first, and use deterministic local k-nearest-neighbors fallback only for labels not marked strict.
- Shipped classifier definitions and corpus MUST be immutable, signed, schema-validated application assets; V1 MUST NOT include an in-app archetype editor.
- Classification runs MUST be append-only and unique per deck revision and classifier version, retain explanations and provenance, and preserve the previous successful result until a replacement commits.
- Classification and provider behavior MUST satisfy the TechSpec performance, offline, pause, retry, and privacy constraints.
</requirements>

## Subtasks

- [ ] 5.1 Complete and document the official MTGO access validation spike, including the fail-closed decision that controls automatic-provider enablement.
- [ ] 5.2 Deliver the consent-aware official provider, allowlists, validation, retries, response binding, interactive fallback, and user confirmation.
- [ ] 5.3 Persist immutable official source snapshots and complete canonical deck revisions with stable provenance and idempotency.
- [ ] 5.4 Define and bundle signed, immutable classifier manifests, signature definitions, labeled corpus data, golden vectors, and release provenance.
- [ ] 5.5 Implement exact signature-card matching, copy constraints, strict labels, stable tie behavior, and explainable results.
- [ ] 5.6 Implement deterministic canonical vectors, cosine neighbors, weighted label confidence, thresholds, and `Unclassified` fallback.
- [ ] 5.7 Persist append-only classification runs and expose current and historical classifier provenance through typed projections.
- [ ] 5.8 Deliver resumable batch reclassification that yields to MTGO foreground activity and interactive operations.
- [ ] 5.9 Add the official-deck confirmation and read-only archetype presentation surfaces without exposing an asset editor.
- [ ] 5.10 Add provider fixtures, classifier golden vectors, deterministic benchmarks, and complete unit, integration, and end-to-end coverage.

## Implementation Details

Implement the TechSpec's “Public Deck Provider” and “Archetype Classifier” contracts. The automatic adapter is a gated outcome of the access spike, not an assumption; the interactive official-site path is a first-class V1 result when documented automation cannot be established.

### Relevant Files

- `src-tauri/src/providers/decks/` — official provider contract, access mode, validation, retries, confirmation, and immutable snapshots.
- `src-tauri/src/classifier/` — asset validation, signature matching, k-nearest-neighbors fallback, explanations, and reclassification.
- `src-tauri/resources/classifier/` — signed built-in archetype definitions, labeled corpus, manifest, and golden vectors.
- `src-tauri/src/commands/decks.rs` and `classifier.rs` — caller-aware lookup, confirmation, detail, provenance, and reclassification commands.
- `src/features/decks/` — consent, lookup status, interactive confirmation, deck detail, and provenance UI.
- `src/features/classifier/` — read-only archetype result, explanation, version, and reclassification status UI.
- `src/lib/ipc/decks.ts` and `src/lib/ipc/classifier.ts` — typed provider and classifier contracts.
- `tests/fixtures/providers/` and `tests/fixtures/classifier/` — deterministic response and classification evidence.

### Dependent Files

- `src-tauri/src/services/history.rs` — last-deck-seen reads include official and classifier provenance.
- `src-tauri/src/notebook/` — immutable deck revisions, snapshots, and append-only classification runs.
- `src-tauri/src/operations/` — reclassification progress, priority, pause, resume, and cancellation.
- `src-tauri/src/shell/` — allowlisted system-browser handoff and later signed asset updates.
- `src/overlay/` — receives only disclosure-approved current archetype context.

### Related ADRs

- [ADR-002](adrs/adr-002.md) — disclosure-safe deck context.
- [ADR-004](adrs/adr-004.md) — typed provider/classifier commands and safe projections.
- [ADR-005](adrs/adr-005.md) — constrained official network integration and interactive fallback.
- [ADR-006](adrs/adr-006.md) — resumable reclassification operation behavior.
- [ADR-007](adrs/adr-007.md) — immutable signed archetype assets and deterministic classification.

## Deliverables

- A documented official-access decision with gated automatic and supported interactive official-site paths.
- Immutable, provenance-rich official deck snapshots and complete deck revisions.
- Signed bundled archetype assets plus deterministic signature and local k-nearest-neighbors classification.
- Read-only deck/archetype UI and resumable append-only reclassification.
- Every test case assigned in `## Tests` implemented and passing **(REQUIRED)**.

## Tests

Cases assigned from `_tests.md`, the test contract — read each ID's full definition there before writing tests.

- [ ] UT-046, UT-047, UT-048, UT-049, UT-050, UT-051, UT-052, UT-053, UT-054, UT-055, UT-056 — official provider validation, consent, response binding, retry, and interactive fallback behavior.
- [ ] UT-057, UT-058, UT-059, UT-060, UT-061, UT-062, UT-063, UT-064, UT-065, UT-066, UT-067, UT-111 — signature constraints, canonical vectors, k-nearest-neighbors determinism, confidence, strict labels, and asset validation.
- [ ] IT-091, IT-092, IT-093, IT-094, IT-095, IT-096, IT-097, IT-098, IT-099, IT-100 — provider consent, access modes, official lookup, retries, and interactive confirmation.
- [ ] IT-181, IT-182, IT-183, IT-184, IT-185, IT-186, IT-187, IT-188, IT-189, IT-190 — signed asset loading, signature classification, fallback classification, explanations, and deterministic results.
- [ ] IT-210, IT-211, IT-215, IT-229, IT-230 — immutable snapshot persistence, deck detail, provenance, and stale-result protection.
- [ ] IT-248, IT-249, IT-261, IT-262, IT-270, IT-280 — read-only classifier UI, performance, reclassification yielding, and disabled-editor boundaries.
- [ ] E2E-010 — official deck confirmation through local archetype classification and provenance display.

## Success Criteria

- Every assigned test case implemented and passing
- V1 never performs undocumented automatic scraping or sends data beyond the disclosed confirmed handle and format.
- Every shown archetype is reproducible from an immutable deck revision and signed classifier version with a human-readable explanation.
- Classifier rules are shipped and updateable only through trusted release mechanisms, with no in-app editing surface.

