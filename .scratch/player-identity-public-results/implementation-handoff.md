# Player Identity and Public Results — V1 Implementation Handoff

Status: approved
Map: https://github.com/MatheusBBarni/mtgo-notes/issues/1
Decision ticket: https://github.com/MatheusBBarni/mtgo-notes/issues/8

## Authority and boundary

This handoff authorizes creation of a new Compozy planning packet at
`.compozy/tasks/player-identity-public-results/`. It does not authorize a release
claim and does not rewrite the executed `.compozy/tasks/mtgo-opponent-notes/`
packet.

The Player-owned public-result workflow is additive. It may reuse generic
trusted-host, persistence, encryption, portability, classification, and renderer
infrastructure, but it does not reuse opponent profiles, encounter triggers,
opponent public-deck snapshots, evidence schemas, or command authority.

## Accepted V1 product contract

- One editable local Player identity exists per notebook and remains distinct
  from every opponent profile. Saving or editing it is local-only and never
  starts a lookup or relinks historical evidence.
- Every external action is explicit, source-specific, independently consented,
  host-built, and fail-closed. Only the main Player workspace has the capability.
- Daybreak Census MOCS leaderboard data is the only conditional machine-readable
  provider. The adapter may be planned and implemented against synthetic fixtures
  while disabled, but production use requires all external enablement gates.
- Official MTGO is a browser handoff and a user-attested manual evidence source.
  MTGTop8 is an optional browser-only corroboration handoff. Neither browser route
  is fetched or parsed by the application.
- Exact case-insensitive nickname matching is mandatory. There is no partial,
  fuzzy, inferred, or alias-based match.
- Imported public-result evidence is Player-owned, typed, source-attributed,
  immutable, selectively retained, version-linked, and separate from editable
  notes and encounter-bound opponent deck records.
- Lookup and refresh are explicit. Candidates are previewed before import;
  duplicate, changed, empty, degraded, cancelled, and browser-opened states remain
  distinct. No provider fallback is automatic.
- The approved Player workspace uses the compact responsive two-column prototype,
  inline identity and consent controls, one exact-match review list, a stable
  selection-summary action, non-destructive empty and failure states, and explicit
  refresh.
- Encrypted portability includes canonical Player identity, evidence, selection
  revisions, and scoped empty outcomes while excluding authorization,
  operational state, provider secrets/configuration, and machine-bound secrets.
  Restore never re-enables access and never merges two different Player identity
  IDs.
- Plaintext complete-notebook export includes Player-owned canonical data with
  human-readable attribution; selected-opponent export includes none of it.
- Individual evidence/empty outcomes and the whole Player identity have scoped,
  explicitly confirmed deletion. Whole-identity deletion cascades only through
  Player-owned data, cancels Player lookups, revokes Player-specific consent, and
  leaves opponent data untouched.

## Compozy planning deliverables

The new packet must contain:

1. A PRD and user-story catalog that preserve the accepted vocabulary, V1 scope,
   non-goals, degradation behavior, and independent consent boundaries.
2. A TechSpec and canonical `_tests.md` that define schemas, invariants, typed IPC,
   trusted-host policy, approved routes, session fencing, idempotency, limits,
   persistence, portability, deletion, UI projections, and evidence ownership.
3. Focused ADRs where implementation choices affect durable schema identity,
   trusted-host authority, provider enablement, or integration with existing
   portability/classification infrastructure.
4. Independently executable task files with explicit dependencies and assigned
   canonical test IDs.
5. Workflow memory that records the external Census gate and all Windows-native
   evidence still outstanding after local implementation.

## Recommended implementation breakdown

1. **Domain and migration foundation** — Player identity aggregate; typed evidence
   envelopes and payloads; source keys and digests; selection revisions; scoped
   empty outcomes; tombstones; migrations; repository invariants.
2. **Trusted-host policy and IPC** — closed commands; caller and phase capability;
   independent consent; approved routes; configuration expiry; operation keys;
   replay behavior; session/token fencing; bounded audit; typed outcomes/errors.
3. **Source paths** — disabled-by-default Census adapter with synthetic fixtures;
   official MTGO manual preview; official MTGO and MTGTop8 system-browser handoffs;
   URL canonicalization and no-fetch enforcement.
4. **Durable lifecycle** — candidate import; duplicate/change/version behavior;
   selection updates; explicit refresh; individual and cascade deletion; no
   resurrection; cancellation and failure atomicity.
5. **Player workspace** — implement the approved responsive prototype states with
   real typed projections, keyboard/focus behavior, accessible status, and saved
   evidence preserved across empty and degraded outcomes.
6. **Existing-service integration** — generic repository/encryption/operation
   reuse; complete-deck classification bridge only for eligible evidence;
   backup/restore/export extensions; strict separation from opponent enrichment.
7. **Verification and release evidence** — complete the canonical local matrix,
   then collect packaged Windows evidence and, separately, live Census enablement
   evidence before making the corresponding completion claims.

## Verification matrix

| Boundary | Required evidence | Gate |
|---|---|---|
| Domain model | Unit/property tests for one identity, immutable evidence, exact source-key/digest rules, selection revisions, nickname-history preservation, scoped emptiness, deletion, and no resurrection | Local |
| Persistence and migrations | Real encrypted-repository integration tests for atomic import, replay, conflict, rollback, migration, restart, and foreign-key/tombstone invariants | Local, plus packaged Windows encryption evidence |
| Trusted-host authority | Integration tests proving renderer fields are non-authoritative, callers/phases are fenced, commands are closed, consent is independent, revocation cancels/fences, and raw provider content never escapes | Local |
| Network and route safety | Synthetic HTTP tests for exact Census host/path/parameters, no nickname transmission, no redirects, time/size/row limits, expiry, cooldown, cancellation, and typed degradation | Local |
| Census production enablement | Reviewed Service ID/use model, current policy/quota/caching/attribution review, expiry-bound configuration, and live response-contract evidence | Live-provider gate; zero requests until complete |
| Manual and browser paths | Tests for official artifact validation without network access, bounded typed input, host-built browser URLs, independent consent, and browser-open failure | Local, plus packaged Windows browser-handoff evidence |
| Evidence lifecycle | Integration tests for preview binding, exact replay, duplicate and changed evidence, selective persistence, classification eligibility, refresh, deletion, and transaction rollback | Local |
| Portability | Backup/restore/export tests for inclusion/exclusion, same-identity merge, different-identity hard conflict, consent-off restore, plaintext scope, and tombstone no-resurrection | Local, plus packaged Windows DPAPI/SQLCipher/filesystem evidence |
| Workflow isolation | Regression tests proving Player actions never create or mutate opponent profiles, encounters, snapshots, consent, or encounter-triggered lookup behavior | Local |
| Player workspace | Component/integration coverage for all approved states, keyboard-only operation, focus order/restoration, announcements, responsive stacking, loading/cancel/retry, and axe with zero violations | Local |
| Rendered accessibility | Pixel contrast, scaling, clipping, native focus, screen-reader/accessibility-tree, system-browser handoff, and no-focus-steal evidence on packaged Windows 10/11 builds | Release gate |
| Offline and privacy | Tests showing the full local/manual workflow remains usable offline; no telemetry, background lookup, fallback provider, leaked content, or consent bypass | Local, plus packaged Windows confirmation |

Partial test output, macOS-only native evidence, prototype axe results, or an
implemented-but-disabled Census adapter cannot satisfy a stronger gate.

## Explicit non-goals

- MTGO login, private account access, collection access, ratings, or complete
  personal match history.
- Background lookup, synchronization, polling, or automatic fallback.
- Authenticated scraping, access-control bypass, arbitrary provider requests,
  arbitrary URL opening, or undocumented endpoint use without permission.
- Multiple active Player identities in one V1 notebook.
- Third-party import, webpage parsing, partial decklist import, or automatic
  linkage between Player evidence and opponent records.
- Treating this planning approval as implementation, Windows release, or Census
  production approval.

## Approval test

The handoff is planning-ready when the accepted decisions above may be translated
into the new Compozy PRD, TechSpec, canonical test catalog, ADRs, and task breakdown
without reopening a product decision. Implementation, packaged Windows behavior,
and Census production enablement remain independently evidenced gates.

Approved by the product owner on 2026-08-03.
