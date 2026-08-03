# Product Requirements Document: Player Identity and Public Results

## Overview

MTGO Opponent Notes currently represents other players through opponent profiles
and encounter history, but it has no distinct representation of the person using
the application. The Player Identity and Public Results feature adds an optional,
top-level Player workspace where one local MTGO nickname can explicitly discover,
preview, and import verifiable public tournament results or published decklists.

The feature serves MTGO players who want trustworthy context about their own
published results without logging into MTGO, exposing private account data, or
turning the local companion into a background synchronization client. It preserves
the application's privacy-first value by making every external action explicit,
destination-specific, independently consented, source-attributed, bounded, and
fail-closed.

## Goals

- Let a player optionally save and edit one local Player identity without blocking
  any existing notebook, capture, overlay, opponent-history, or opponent-enrichment
  workflow.
- Let a player understand which approved public routes are available and choose
  every lookup, browser handoff, retry, import, refresh, or revocation explicitly.
- Preview only exact case-insensitive nickname matches and preserve the narrow,
  source-scoped meaning of every candidate, empty result, and degraded outcome.
- Let a player import selected public facts as immutable, typed, source-attributed
  Player-owned evidence while retaining only the optional fields they choose.
- Preserve historical lookup nicknames, provenance, source versions, and selection
  revisions across identity edits, refreshes, consent changes, and source changes.
- Keep the complete local/manual Player workflow usable offline and when Census is
  unavailable, unauthorized, expired, or disabled.
- Extend deletion, encrypted backup/restore, and plaintext export with precise
  Player-owned data boundaries that never mutate opponent records or restore
  external authorization.
- Make every Player-workspace state keyboard-complete, visibly focused,
  screen-reader-exposed, responsive, and understandable without color alone.
- Prevent an implemented Census adapter from making production requests until its
  current authorization, configuration, policy, and live-contract gates all pass.

## User Stories

- `US-001`–`US-002`: optional workspace access and one local Player identity.
- `US-003`–`US-004`: independent consent and honest source availability.
- `US-005`–`US-007`: bounded lookup, truthful outcomes, and explicit browser handoffs.
- `US-008`–`US-012`: manual preview, selective import, provenance, refresh, and
  complete-deck classification.
- `US-013`–`US-015`: scoped deletion, encrypted portability, and plaintext export.
- `US-016`: conditional Census production enablement.

[Full user stories](_user_stories.md)

## Core Features

### Optional Player workspace

The application exposes a top-level Player tab alongside the existing workspaces.
The tab has a useful first-use state with no identity, explains local identity and
public-source behavior, and allows normal navigation away. Player setup never gates
the rest of the application.

The workspace uses the approved compact responsive two-column model: local identity
and source controls occupy the narrow column; lookup, candidate review, saved
evidence, empty outcomes, and degraded outcomes occupy the primary column. Columns
stack at constrained widths. Important saved evidence remains visible during
loading, cancellation, empty, and failure states.

### Local Player identity

One local notebook stores at most one Player identity. The identity contains a
self-entered MTGO nickname and stable local identity/revision metadata. It is not an
MTGO login, account identifier, opponent profile, or claim about a natural person.

Saving or editing the nickname is local-only and never starts a lookup. A nickname
edit affects future explicit actions only. Historical evidence and empty outcomes
retain the exact nickname used when they were created and are never rekeyed,
alias-linked, or merged because of the edit.

### Source status and independent outbound consent

The workspace keeps source-specific status visible beside each route. Consent
controls expand inline when consent is absent or the player chooses to review them;
they never require a modal or Settings detour.

V1 has three independent outbound consent boundaries:

- Census machine lookup permits only approved source-scope parameters. The Player
  nickname stays local and is never sent to Census.
- Official MTGO browser handoff permits the exact nickname in one host-built URL.
- MTGTop8 browser handoff separately permits the exact nickname in one host-built
  URL.

Manual official-source preview/import performs no network access and requires no
outbound consent. Each grant names the destination, purpose, access mode, disclosure
version, and exact outbound fields. A mismatch means consent is absent. Revocation
immediately cancels matching work, fences late results, and blocks future access
without deleting imports.

### Conditional Census lookup

Daybreak Census MOCS leaderboard data is the only machine-readable V1 candidate.
It remains unavailable in production until every enablement rule in this PRD passes.
The locally implemented adapter may use synthetic fixtures while disabled.

Lookup and refresh are explicit actions. They require a saved Player identity,
current Census consent, enabled and unexpired provider configuration, and a trusted
outside-gameplay phase. The trusted application boundary derives the nickname and
approved scope; the nickname is matched locally against the validated response.

Only one machine lookup may be active globally. Progress and cancellation remain
visible, no provider retry is automatic, and a retry requires a new explicit action
after the later of the local cooldown and a valid provider retry time.

### Truthful lookup outcomes

Only an exact case-insensitive nickname match may become a candidate. Partial,
fuzzy, inferred, alias-based, or unclear matches are never shown as the player's
result. At most ten exact-match previews may be presented for one lookup session.

A valid configured Census response with no exact match may create an Empty lookup
result. It records provider, exact nickname, exact-match rule, approved source scope,
configuration version, completion time, local/Player identity, and lookup operation.
It never claims that the player has no public history.

Disabled, unavailable, throttled, malformed, expired, misconfigured, timed-out, or
over-limit provider access produces a Degraded public lookup, never an empty result.
Candidates, scoped empty, already imported, cancelled, and browser-opened are
successful typed outcomes; policy, admission, provider, source, fencing, browser,
and persistence failures are typed errors with bounded recovery guidance.

### Explicit browser handoffs

Official MTGO Decklists is an explicit system-browser handoff. MTGTop8 is an
optional, separately consented system-browser corroboration handoff and is never an
import source. Each handoff opens only a trusted route built by the application from
the current nickname after source-specific consent.

The companion never embeds, fetches, resolves, parses, or infers results from the
opened page. Browser pages never create a durable empty outcome. A browser failure
changes no evidence and is reported as a recoverable typed error.

### Official-source manual evidence preview

A player may enter typed result facts attributed to an exact official MTGO event or
decklist URL. The companion validates and canonicalizes the approved official route
without making any network, DNS, preflight, embedded-browser, or parse request.

Manual official published-decklist evidence requires event title, event date,
format, exact source nickname, and the exact player-entered official URL. Placement
and record are optional. Deck contents are either `reference_only`, storing no
cards, or `complete` after format-aware validation. Partial card lists never become
a preview or import.

### Typed evidence preview and selective import

Every candidate or manual preview is a complete immutable public-result statement
that shows:

- evidence schema and typed result kind;
- provenance mode (`provider_observed` or `user_attested_official_source`);
- provider identity and public attribution containing no credential or Service ID;
- immutable lookup and source nicknames plus the exact-match rule;
- typed source scope and observation time;
- typed result payload;
- provider-specific source key;
- source digest for content change detection; and
- preview digest for the exact approved envelope.

Candidates appear in one exact-match review list with result-level selection and
inline retained-field selection. Source identity and attribution are mandatory and
cannot be deselected. One stable selection-summary bar owns the primary import
action and disables it when no candidate is selected.

Import freezes the exact preview, provenance, and digests; adds only local evidence
identity, Player identity, import time, and selected-field manifest; and discards
unselected payload values. Import does not refetch, parse, or silently normalize the
source. A later retained-field change appends a local selection revision without
editing the immutable source statement.

### Source identity, deduplication, and refresh

Census evidence source identity combines provider, catalog ID, start date, as-of
date, and case-folded source nickname. Official published-decklist identity combines
provider, trusted canonical official artifact URL, and case-folded source nickname;
the exact entered attribution URL remains separately visible and auditable.

The source digest identifies source content and never substitutes for the source
key:

- same source key and digest is already imported and creates nothing new;
- same source key with a different digest is changed evidence that returns to
  preview as a linked immutable version; and
- different source keys remain distinct even when their digests are equal.

Refresh is explicit and reuses the current identity and valid consent. It previews
only new or changed statements, identifies already-imported statements, and never
overwrites, deletes, or relinks prior evidence. Empty and degraded refresh outcomes
leave saved evidence unchanged.

### Local classification bridge

Only complete, format-valid official published-decklist evidence may enter the
existing local classifier. Reference-only and partial data never classify. The
classification retains its own immutable classifier provenance and remains linked
to Player evidence without creating or mutating opponent profiles, encounters,
deck records, revisions, snapshots, or classification ownership.

An unsupported format or unavailable classifier does not block evidence import; it
produces an honest local unclassified state.

### Player-owned deletion

An imported evidence record or Empty lookup result may be deleted individually
after a scoped destructive preview and explicit confirmation. Deletion does not
make immutable evidence editable.

Whole-identity deletion previews the exact identity and dependent counts. On
confirmation it atomically removes the Player identity, all Player-owned evidence,
selection revisions, and empty outcomes; cancels active Player sessions; and revokes
Player-specific consent. It never deletes or edits opponent profiles, encounters,
observations, decks, snapshots, or opponent-provider consent.

Deletion retains only non-content tombstones required to prevent merge restore from
resurrecting removed Player records. Explicit replace restore may restore previously
backed-up Player data.

### Encrypted backup and restore

Encrypted complete-notebook backup treats Player identity, imported evidence,
selection revisions, scoped empty outcomes, and required tombstones as canonical
notebook data. It excludes outbound consent, provider configuration and Service IDs,
active sessions, previews/tokens, bounded audit, cooldowns, transient replay caches,
and machine-bound secrets.

Restore never enables outbound access. Merge imports an archived Player identity
when none exists. When the archived and destination Player identities have the same
stable ID, Player records merge by immutable identity and digest rules. Different
Player identity IDs are a hard whole-merge conflict; the user may replace or cancel,
but cannot silently choose, combine, or partially reassign Player evidence.

### Plaintext export

The existing explicit unencrypted, one-way warning remains mandatory. A
complete-notebook export includes the Player identity, evidence, selection history,
empty outcomes, timestamps, and source attribution in human-readable form. A
selected-opponent export includes no Player identity or Player-owned result data.

Plaintext export excludes consent, provider configuration, operational state,
secrets, unimported previews, and restorable import metadata.

### Production provider enablement

Census remains disabled and makes zero production requests unless all of the
following are simultaneously current and reviewed:

- an appropriate Daybreak-issued Service ID and distributed desktop-client use
  model;
- current provider policy, quota, caching, privacy, and attribution requirements;
- an expiry-bound approved configuration for the exact MTGO leaderboard scope with
  a safe disabled state;
- synthetic fixtures proving validation, local exact matching, bounded execution,
  typed failure, scoped empty results, cancellation, and non-destructive behavior;
- live response-contract evidence against the approved scope; and
- packaged Windows evidence for configuration custody, privacy boundaries, and the
  user-visible workflow.

Placeholder, `s:example`, shared, expired, malformed, missing, or unreviewed
configuration keeps the provider unavailable. Service IDs never appear in public
attribution, renderer state, logs, diagnostics, backup, or plaintext export.

## Business Rules

### Identity and ownership

- A notebook has zero or one Player identity in V1.
- A Player identity is never an opponent profile and never aliases one.
- Player setup is optional and cannot block any existing non-Player capability.
- A nickname is a local screen-name string, not proof of account ownership, person
  identity, current-name continuity, or complete result history.
- Saving or editing identity performs no external action.
- Historical evidence and empty outcomes keep their original lookup and source
  nicknames after identity changes.

### Source and consent

- Every external destination has independent versioned consent for its exact
  disclosed field set and access mode.
- Missing, stale, mismatched, or unreadable consent means no external action.
- Revocation is immediately effective for matching future and in-flight work.
- Census sends no nickname; official MTGO and MTGTop8 browser routes may include the
  exact current nickname only after their respective consent.
- Manual official-source preview/import requires no consent because it performs no
  network action.
- No source is contacted or opened automatically as a fallback.
- V1 has no arbitrary provider request, arbitrary evidence import, or arbitrary URL
  opening capability.

### Phase and surface authority

- Only the main Player workspace may invoke Player public-source commands.
- Overlay and capture surfaces have no Player public-source capability.
- Provider status, cancellation, and consent revocation remain available during any
  gameplay phase.
- Consent grant, lookup, refresh, manual preview, import, selection change, and
  browser handoff require a trusted outside-gameplay phase.
- Unknown or stale phase fails closed.

### Sessions, replay, and limits

- One machine lookup may be active globally.
- Lookup sessions and preview tokens expire after 15 minutes.
- Census uses a 5-second connection timeout, 15-second total timeout, 1 MiB maximum
  decompressed response, and 2,000 maximum response rows.
- No provider retry is automatic. Explicit retry waits at least 60 seconds or until
  a later valid provider retry time.
- Public command payloads are at most 256 KiB encoded.
- Official URLs are at most 2,048 characters; event titles at most 200 characters;
  format, placement, and record fields at most 64 characters.
- Complete deck evidence contains at most 500 unique card-and-zone rows and quantity
  at most 250 per row.
- One lookup exposes at most 10 exact-match previews.
- Unknown fields, unbounded maps/arrays, duplicate normalized card rows, control
  characters, malformed required data, and over-limit values are rejected before
  network access or persistence.
- Every side-effecting Player public-source action uses a UUIDv7 operation identity
  bound to action kind, Player identity, and canonical inputs. Exact replay returns
  the original result; reuse with different inputs is invalid.

### Evidence and outcomes

- Imported evidence is immutable, typed, source-attributed, and Player-owned.
- There is no generic verified flag; provenance mode remains explicit.
- Only exact case-insensitive nickname matches may become provider candidates.
- Import must bind the active identity/session, opaque preview token, source key,
  source digest, and preview digest.
- A duplicate source statement creates no new evidence; changed content creates a
  linked immutable version.
- Selection changes append revisions; they never edit public-source content.
- Only complete validated official decklists may classify.
- Empty outcomes require a valid scoped provider response with no exact match.
- Provider failure, invalid configuration, timeout, throttling, malformed data, or
  over-limit response can never create an empty outcome.
- V1 keeps no general durable lookup history. Unimported previews and degraded
  attempts disappear when the session ends; imported evidence and scoped empty
  outcomes retain only their required metadata.
- A bounded content-free runtime audit may retain at most 100 summaries and clears
  on application restart. It contains no nickname, URL, source key/digest, preview
  token, payload field, or card.

### Portability and deletion

- Player-owned canonical data participates in encrypted backup, restore, and
  complete-notebook plaintext export according to the exact scopes above.
- Consent and authorization never become portable.
- Different stable Player identity IDs block whole merge restore.
- Selected-opponent plaintext export is guaranteed Player-free.
- Individual and whole-identity deletion require a bound preview and explicit
  confirmation.
- Whole-identity deletion cannot expand into opponent data and must leave enough
  non-content tombstone state to prevent merge resurrection.

## User Experience

### Primary journey

1. The player opens the optional Player tab and sees first-use guidance plus source
   availability without being forced to save an identity.
2. The player saves one local nickname. The UI confirms that saving is local-only
   and no lookup begins.
3. The player reviews a source disclosure and independently grants only the route
   they intend to use.
4. The player explicitly starts Census lookup, opens an approved browser route, or
   enters an official manual result. Progress, cancel, and current evidence remain
   visible.
5. The player sees exact candidates, scoped empty, or degraded status with language
   that preserves the narrow source meaning.
6. The player selects candidate results and optional retained fields, reviews the
   stable selection summary, and imports.
7. The player inspects provenance and versions, revises retained fields, refreshes
   explicitly, exports/backs up, or deletes scoped data.

### Accessibility and interaction requirements

- The approved responsive two-column prototype is the behavioral reference; the
  narrow and primary columns stack without clipping at constrained widths.
- Identity and consent controls expand inline; no required workflow hides in a
  modal or Settings detour.
- Consent, revoke, save, lookup, cancel, retry, browser handoff, manual entry,
  candidate/field selection, import, refresh, export, and deletion are operable by
  keyboard alone.
- Focus order follows reading order, remains visibly unobscured, restores after
  transient actions, and never moves merely because a status changes.
- Loading, candidate, imported, empty, degraded, cancelled, stale, and failure
  messages are programmatically exposed without stealing focus.
- Color is never the only state cue. Controls retain accessible names, text status,
  visible focus, and at least the project's WCAG 2.2 AA contrast/target baseline.
- Destructive previews name their scope and counts before confirmation.
- Saved evidence remains visible during lookup, empty, degraded, and retry states.

## High-Level Technical Constraints

- The feature is additive and receives a new Compozy packet. The executed opponent
  packet remains authoritative for opponent behavior and is not rewritten.
- Player identity, evidence, selections, empty outcomes, and tombstones must have
  independent ownership from opponent profiles, encounters, deck snapshots, and
  commands.
- The trusted native host owns consent, identity loading, provider configuration,
  approved routes, source validation, canonicalization, operation replay, session
  fencing, persistence, deletion, portability, and external side effects. Renderer
  inputs express bounded intent only.
- Existing generic encrypted notebook, migration, operation, portability,
  classifier, browser-shell, IPC, and UI infrastructure may be reused when doing so
  does not transfer opponent ownership or authority.
- The application remains local-first, account-free, offline-capable, and free of
  telemetry or background public lookup.
- Raw provider bodies/messages, nicknames, URLs, source keys/digests, preview tokens,
  evidence payloads, and cards must not enter logs, diagnostics, or content-free
  operation audit.
- Local automated evidence, packaged Windows evidence, and live Census enablement
  evidence are separate gates. macOS results, prototype axe checks, or synthetic
  provider fixtures cannot prove packaged Windows or production-provider behavior.
- Packaged Windows 10/11 verification must cover native focus/accessibility,
  browser handoffs, encrypted persistence and portability, offline behavior,
  capability isolation, and installer/release integration before release claims.

## Non-Goals (Out of Scope)

- MTGO login, session handling, account impersonation, account ownership proof, or
  private account access.
- Collection access, ratings, complete match history, real-time results, or claims
  that a nickname represents the same person over time.
- Background lookup, polling, synchronization, automatic refresh, automatic retry,
  or automatic provider fallback.
- Authenticated scraping, browser-page parsing, client-memory/traffic/log extraction,
  access-control bypass, undocumented endpoint use, or arbitrary provider requests.
- Multiple active Player identities in one V1 notebook.
- Partial decklist import or classification.
- Third-party result import; MTGTop8 remains browser-only corroboration.
- Automatic linkage, aliasing, merging, or mutation between Player evidence and
  opponent profiles, encounters, notes, decks, snapshots, or consent.
- Treating implementation-ready planning, local verification, packaged Windows
  verification, or Census production authorization as interchangeable completion
  claims.

## Architecture Decision Records

- [ADR-001: Keep the Player Workspace Optional and Additive](adrs/adr-001.md) —
  Player setup remains optional and separate from every existing opponent workflow.
- [ADR-002: Use Explicit Conditional and Manual Public Source Routes](adrs/adr-002.md)
  — Census is gated; official MTGO is browser/manual; MTGTop8 is browser-only.
- [ADR-003: Preserve Immutable Player-Owned Public Result Evidence](adrs/adr-003.md)
  — exact-match source statements use independent immutable evidence and history.

## Open Questions

No product-scope question remains. The TechSpec must resolve implementation choices
for independent persistence, classification linkage, trusted-host state, and test
evidence without changing the approved product behavior.
