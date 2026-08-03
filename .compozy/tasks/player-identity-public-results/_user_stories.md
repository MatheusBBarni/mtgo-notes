# User Stories: Player Identity and Public Results

Canonical behavior catalog for the optional Player workspace and its public-result
workflow. Companion to `_prd.md`; consumed by `_techspec.md` and `_tests.md`.

## Personas

- **MTGO player** — uses the local companion to manage their own nickname and
  review verifiable public tournament context without turning the application into
  an account client.
- **Privacy-conscious player** — wants to understand and control every external
  disclosure, keep saved evidence local, and remove or export it predictably.
- **Release verifier** — validates packaged Windows behavior and may enable the
  conditional Census provider only when current authorization and contract evidence
  satisfy every production gate.

## Story Index

| ID | Feature Area | Persona | Story |
|---|---|---|---|
| US-001 | Workspace | MTGO player | Use the application without Player setup |
| US-002 | Identity | MTGO player | Create and edit one local Player identity |
| US-003 | Consent | Privacy-conscious player | Control each external destination independently |
| US-004 | Source status | MTGO player | Understand which public routes are available |
| US-005 | Lookup | MTGO player | Explicitly start and cancel a Census lookup |
| US-006 | Lookup outcomes | MTGO player | Distinguish candidates, scoped absence, and degradation |
| US-007 | Browser handoff | Privacy-conscious player | Open approved public searches explicitly |
| US-008 | Manual evidence | MTGO player | Preview an official result without application fetching |
| US-009 | Import | MTGO player | Select and import exact-match evidence |
| US-010 | Saved evidence | MTGO player | Inspect provenance and revise retained fields |
| US-011 | Refresh | MTGO player | Refresh without overwriting historical evidence |
| US-012 | Classification | MTGO player | Classify only complete validated decklist evidence |
| US-013 | Deletion | Privacy-conscious player | Delete scoped or all Player-owned data |
| US-014 | Encrypted portability | Privacy-conscious player | Back up and restore Player-owned canonical data safely |
| US-015 | Plaintext export | Privacy-conscious player | Export the correct Player data with explicit warning |
| US-016 | Provider enablement | Release verifier | Keep Census disabled until every live gate passes |

## Workspace and Identity

### US-001: Use the application without Player setup

**As an** MTGO player, **I want** Player setup to remain optional, **so that** I can
keep using notebook, capture, overlay, opponent history, and opponent enrichment
without saving my own nickname.

Acceptance criteria:

- AC-1: Given no Player identity, when the application opens, then existing
  non-Player workflows remain usable and the Player tab is visible.
- AC-2: Given no Player identity, when the Player tab opens, then it explains the
  local-only identity and public-source choices without blocking navigation.
- AC-3: Given the player leaves the tab without setup, when they return later, then
  no identity, consent, lookup, or evidence has been created implicitly.

Edge cases:

- EC-1: Blank first-run notebook → Player setup remains optional and no empty
  identity is created.
- EC-2: Player tab opened from keyboard navigation → focus enters the first useful
  explanatory control and can leave the tab normally.
- EC-3: Repeated tab visits → no duplicate state or consent prompt is created.
- EC-4: Offline startup or provider failure → the local notebook remains usable and
  the Player tab shows an honest unavailable state.
- EC-5: A deep-linked Player action without an identity → the action is unavailable
  with guidance to save an identity, not a global application error.

### US-002: Create and edit one local Player identity

**As an** MTGO player, **I want** to save and edit one local MTGO nickname, **so
that** future explicit public-result actions use the identity I control.

Acceptance criteria:

- AC-1: Given no identity, when a valid nickname is saved, then one local Player
  identity appears and no network request starts.
- AC-2: Given an existing identity, when the nickname is edited and confirmed,
  then future actions use the new nickname while saved evidence and empty outcomes
  retain their original lookup nickname.
- AC-3: Given an identity edit, when the player reviews the warning, then the UI
  states that the change creates no alias and does not relink history.
- AC-4: Given one identity already exists, when another create attempt occurs, then
  the product treats it as an edit or rejects the conflicting operation rather than
  storing a second identity.

Edge cases:

- EC-1: Blank, control-character, or over-limit nickname → save is rejected with a
  field-specific message and prior identity remains unchanged.
- EC-2: Case-only nickname edit → displayed spelling may change, but history is not
  rewritten and no duplicate identity is created.
- EC-3: Two edits submitted concurrently → one revision wins and the stale edit is
  rejected with current data preserved.
- EC-4: Save interrupted or repeated with the same operation identity → the result
  is atomic and replay does not create another identity.
- EC-5: Edit during an active lookup → the active session becomes stale and cannot
  import evidence under the changed identity revision.

## Consent and Source Status

### US-003: Control each external destination independently

**As a** privacy-conscious player, **I want** separate consent for Census, official
MTGO, and MTGTop8, **so that** I understand and control each disclosure.

Acceptance criteria:

- AC-1: Given absent consent, when a source disclosure expands, then it names the
  destination, purpose, access mode, and exact outbound fields before grant.
- AC-2: Given consent for one destination, when another destination is used, then
  the second route still requires its own valid consent.
- AC-3: Given valid consent, when it is revoked, then matching in-flight access is
  cancelled, late responses are rejected, and future access is blocked.
- AC-4: Given consent is revoked, then existing imported evidence remains available
  and a previously displayed local-only preview remains importable until expiry.

Edge cases:

- EC-1: Consent version or disclosed-field set changes → prior consent is treated as
  absent and no request or browser handoff occurs.
- EC-2: Repeated grant or revoke with the same operation identity → the original
  outcome returns without duplicate side effects.
- EC-3: Revocation races with response completion → the late response cannot update
  candidates or durable state.
- EC-4: Consent storage cannot be read or written → the source fails closed while
  local evidence remains accessible.
- EC-5: Restore from backup → every outbound consent remains off on the destination.

### US-004: Understand which public routes are available

**As an** MTGO player, **I want** visible source-specific status, **so that** I know
whether I can look up, open, retry, or import manually without guessing.

Acceptance criteria:

- AC-1: The Player workspace shows Census as enabled, disabled, expired,
  misconfigured, cooling down, busy, or unavailable using text and accessible state.
- AC-2: Official MTGO and MTGTop8 each show their independent consent and browser
  availability; manual official import is identified as local/no-fetch.
- AC-3: A disabled or degraded source offers only actions that are actually valid,
  and never opens or contacts a fallback automatically.

Edge cases:

- EC-1: Provider configuration expires while the tab is open → status updates to
  disabled before another request can start.
- EC-2: Unknown provider status → the UI fails closed and does not render an enabled
  action.
- EC-3: Status changes repeatedly or at scale → one current source status is shown
  without stale announcements or duplicated controls.
- EC-4: Browser opening is unsupported → source status preserves manual alternatives
  and explains the handoff limitation.
- EC-5: Overlay or capture attempts to query status → no Player-source capability is
  exposed to those surfaces.

## Lookup and Handoffs

### US-005: Explicitly start and cancel a Census lookup

**As an** MTGO player, **I want** to start or cancel a bounded public-result lookup,
**so that** no background process searches for me without an immediate choice.

Acceptance criteria:

- AC-1: Given a saved identity, valid Census consent, trusted outside-gameplay
  phase, and enabled provider, when lookup starts, then progress and cancel are
  visible and existing evidence remains visible.
- AC-2: The lookup sends only approved source-scope parameters; nickname matching
  occurs locally and the nickname is not transmitted to Census.
- AC-3: Given cancellation, timeout, throttling, or provider failure, then the flow
  ends in a typed state with valid recovery guidance and no partial import.
- AC-4: Only one machine lookup may be active globally and no retry starts
  automatically.

Edge cases:

- EC-1: Missing identity, consent, configuration, or trusted phase → zero network
  requests and a specific recovery message.
- EC-2: A second start arrives while lookup is active → it is rejected as busy and
  does not supersede the active lookup silently.
- EC-3: Cancel and response race → one terminal outcome wins and a late response is
  fenced.
- EC-4: Response exceeds time, byte, row, or preview limits → lookup degrades with
  no raw provider content exposed.
- EC-5: Retry before local/provider cooldown ends → zero request and the next valid
  retry time is shown.
- EC-6: Exact replay of the same lookup operation → the prior bounded result returns
  without another request; changed inputs with the same key are rejected.

### US-006: Distinguish candidates, scoped absence, and degradation

**As an** MTGO player, **I want** truthful lookup outcomes, **so that** I do not
mistake provider failure or a narrow result for a claim about my complete history.

Acceptance criteria:

- AC-1: A valid response exposes only exact case-insensitive nickname matches as
  candidates, with at most ten previews.
- AC-2: A valid scoped response with no exact match creates an Empty lookup result
  naming provider, scope, nickname, and completion time without claiming that no
  public history exists.
- AC-3: Disabled, unavailable, throttled, malformed, expired, misconfigured, or
  over-limit access produces a Degraded public lookup and no empty result.
- AC-4: Candidate, empty, already imported, cancelled, and browser-opened outcomes
  are distinct from typed failures, and saved evidence remains visible throughout.

Edge cases:

- EC-1: Partial, fuzzy, or differently punctuated match → it is not presented as a
  candidate and cannot be imported through inference.
- EC-2: More than ten exact matches → the bounded result rejects or deterministically
  limits according to the trusted contract without silently importing any.
- EC-3: Repeated identical empty operation → one scoped durable outcome exists.
- EC-4: A later positive result follows an empty outcome → both remain, with the
  newest outcome shown by default.
- EC-5: Successful unimported previews expire or the application restarts → they
  disappear; no general lookup history is reconstructed.

### US-007: Open approved public searches explicitly

**As a** privacy-conscious player, **I want** to open approved official or
corroboration searches in my system browser, **so that** I retain control over the
external visit.

Acceptance criteria:

- AC-1: Given valid destination-specific consent and a saved identity, an explicit
  action opens a host-built official MTGO or MTGTop8 player-search URL.
- AC-2: The companion never fetches, parses, embeds, redirects through, or infers an
  empty result from the opened page.
- AC-3: The disclosure states that the exact nickname is placed in the host-built
  URL and ordinary browser metadata may reach the destination.

Edge cases:

- EC-1: Missing consent, identity, trusted phase, or approved route → no browser is
  opened and a specific recovery action is shown.
- EC-2: Route host/path/parameter validation fails → the handoff fails closed rather
  than opening a near-match URL.
- EC-3: Browser open fails → the app reports failure without marking handoff success
  or changing saved evidence.
- EC-4: Repeated action with the same operation key → at most one browser side effect.
- EC-5: Overlay or capture invokes the handoff → capability denial occurs before an
  external action.

## Evidence Preview, Import, and Refresh

### US-008: Preview an official result without application fetching

**As an** MTGO player, **I want** to enter facts from an exact official MTGO
artifact, **so that** I can preserve source-attributed evidence when machine lookup
is unavailable.

Acceptance criteria:

- AC-1: The player supplies an exact official MTGO event or decklist URL plus typed
  result facts; creating the preview performs no network, DNS, preflight, or parse.
- AC-2: A preview shows user-attested provenance, entered and canonical attribution,
  exact source nickname, event title/date/format, optional placement/record, and
  either reference-only or complete deck contents.
- AC-3: Partial decklists never become a preview; complete lists pass format-aware
  validation, while reference-only evidence stores no cards.
- AC-4: Unsafe URLs or malformed required facts produce no preview and preserve the
  entered form for correction where safe.

Edge cases:

- EC-1: URL is non-HTTPS, wrong host/path, over 2,048 characters, or ambiguous to
  canonicalize → rejected without any external request.
- EC-2: Required event title/date/format/nickname is blank or over its bound → the
  relevant field is rejected and no preview token is issued.
- EC-3: Card rows are duplicated after normalization, exceed 500 rows, exceed 250
  quantity, contain control characters, or are partial → no evidence preview.
- EC-4: Preview creation is repeated → the bounded replay result returns without a
  durable evidence record.
- EC-5: Identity changes or the 15-minute preview expires → import is unavailable and
  a fresh preview is required.

### US-009: Select and import exact-match evidence

**As an** MTGO player, **I want** to select results and retained fields before
import, **so that** I save only the public evidence I intend to keep.

Acceptance criteria:

- AC-1: Exact-match candidates appear in one review list with result-level selection
  and inline optional-field choices; source identity and attribution are mandatory.
- AC-2: The stable selection-summary action is disabled when nothing is selected and
  imports only the confirmed previews and fields.
- AC-3: Import freezes each preview and digest, records Player identity and import
  time, discards unselected values, and starts no additional source request.
- AC-4: A duplicate source key and source digest creates no new evidence; the UI
  identifies the existing import.

Edge cases:

- EC-1: Token, session, source, or preview digest mismatch → the import is rejected
  as stale/mismatched and nothing is partially saved.
- EC-2: Concurrent import of the same evidence → one immutable record exists and
  both callers receive a consistent outcome.
- EC-3: Import is interrupted or storage fails → the entire selected batch rolls
  back and existing evidence remains unchanged.
- EC-4: Exact replay after success → the original result returns without duplicate
  evidence or selection revision.
- EC-5: Unknown fields or oversized command payload → rejected before persistence.

### US-010: Inspect provenance and revise retained fields

**As an** MTGO player, **I want** to inspect saved evidence and change which optional
fields I retain, **so that** provenance stays trustworthy while local retention
remains under my control.

Acceptance criteria:

- AC-1: Saved evidence shows provenance mode, provider, attribution, original lookup
  and source nicknames, scope, observation/import time, typed retained payload, and
  version relationship.
- AC-2: Changing retained fields appends a selection revision to the same source
  evidence and never edits the immutable source statement.
- AC-3: Mandatory source identity and attribution cannot be removed.
- AC-4: Personal notes remain separate and editable; the evidence view never implies
  access to private account data or complete history.

Edge cases:

- EC-1: Selection update names a field absent from the approved preview → rejected
  without changing current selection.
- EC-2: Two selection updates race → stale revision is rejected and the current
  selection is shown.
- EC-3: Repeated selection operation → no duplicate revision is appended.
- EC-4: Evidence source later disappears or consent is revoked → saved attribution
  and immutable evidence remain readable.
- EC-5: Large evidence history → views remain bounded/paged without dropping source
  attribution or version links.

### US-011: Refresh without overwriting historical evidence

**As an** MTGO player, **I want** explicit refresh to show only new or changed
source statements, **so that** history is preserved and updates remain reviewable.

Acceptance criteria:

- AC-1: Refresh is an explicit secondary action using the current identity and valid
  consent; it shares lookup progress, cancellation, cooldown, and failure behavior.
- AC-2: Same source key and digest is marked already imported; changed digest is a
  new preview linked to the prior immutable version; different source keys remain
  distinct even with equal digests.
- AC-3: Refresh never overwrites or deletes imports and never relinks evidence after
  a nickname change.
- AC-4: Empty and degraded refresh outcomes leave saved evidence visible and intact.

Edge cases:

- EC-1: Identity changes during refresh → session is stale and no candidate imports.
- EC-2: Refresh repeats after success → source-key/digest rules prevent duplicates.
- EC-3: Changed source statement is not selected → prior version remains current and
  the unimported preview expires normally.
- EC-4: Provider becomes disabled or consent is revoked mid-refresh → request is
  cancelled/fenced and existing evidence remains unchanged.
- EC-5: Results arrive out of order → only the active generation can update the UI.

### US-012: Classify only complete validated decklist evidence

**As an** MTGO player, **I want** eligible official deck evidence classified
locally, **so that** I can understand it without weakening source provenance.

Acceptance criteria:

- AC-1: Only complete, format-valid official published-decklist evidence may enter
  the existing local classifier.
- AC-2: Reference-only evidence is saved without classification; partial decklists
  cannot be previewed or imported.
- AC-3: Classification output retains its classifier provenance and remains linked
  to Player evidence without creating or mutating opponent deck records.

Edge cases:

- EC-1: Unsupported format or unavailable classifier → evidence import succeeds and
  displays a truthful unclassified state.
- EC-2: Classifier update/reclassification repeats → source evidence remains
  immutable and classification history follows existing append-only rules.
- EC-3: Card validation fails after a stale preview → import is rejected atomically.
- EC-4: Large but valid 500-row deck → bounded local classification completes or
  reports a recoverable local failure without source mutation.
- EC-5: Opponent and Player evidence contain identical cards → they remain separate
  records with independent ownership and provenance.

## Lifecycle and Portability

### US-013: Delete scoped or all Player-owned data

**As a** privacy-conscious player, **I want** explicit deletion controls, **so that**
I can remove imported results or my entire Player identity without harming opponent
notes.

Acceptance criteria:

- AC-1: Individual evidence and empty outcomes each have a scoped destructive
  preview and explicit confirmation; immutable content is deleted, not edited.
- AC-2: Whole-identity preview names the Player identity and counts all evidence,
  selection revisions, and empty outcomes that will be removed.
- AC-3: Confirming whole-identity deletion atomically removes Player-owned content,
  cancels Player lookups, revokes Player-specific consent, and leaves opponent data
  and opponent-provider consent untouched.
- AC-4: Non-content tombstones prevent later merge restore from resurrecting deleted
  Player records; explicit replace restore remains possible.

Edge cases:

- EC-1: Preview expires or data changes before confirmation → confirmation is
  rejected and a fresh preview is required.
- EC-2: Delete races with lookup/import/selection → conflicting work is cancelled or
  rejected and no orphaned Player records remain.
- EC-3: Repeated confirmation → no additional deletion or opponent mutation occurs.
- EC-4: Deletion is interrupted or storage fails → prior complete state remains or
  recovery reaches one complete state.
- EC-5: No Player identity or already-deleted item → a bounded not-found outcome is
  shown without expanding deletion scope.

### US-014: Back up and restore Player-owned canonical data safely

**As a** privacy-conscious player, **I want** encrypted portability to include my
Player evidence without copying authorization, **so that** restored data remains
useful and external access remains off.

Acceptance criteria:

- AC-1: Encrypted backup includes Player identity, evidence, selection revisions,
  empty outcomes, and required non-content tombstones as canonical notebook data.
- AC-2: Backup excludes consent, provider configuration/Service IDs, sessions,
  previews, audit, cooldowns, replay caches, and machine-bound secrets.
- AC-3: Restore into a notebook with no Player identity imports it; same stable
  identity ID may merge by immutable IDs/digests; a different ID blocks the entire
  merge and offers replace or cancel.
- AC-4: Every restored external source remains disabled until separately authorized
  on the destination installation.

Edge cases:

- EC-1: Wrong passphrase, invalid archive, checksum/reference failure, or unsupported
  schema → live notebook remains unchanged.
- EC-2: Merge repeats → duplicates remain skipped and deleted Player records are not
  resurrected.
- EC-3: Different identity conflict occurs alongside mergeable opponent data → whole
  merge remains blocked; no partial identity choice occurs.
- EC-4: Restore is interrupted → staging/rollback behavior reaches an old or new
  complete state with no enabled consent.
- EC-5: Replace restore contains prior Player data → explicit replace may restore it,
  but provider access still remains off.

### US-015: Export the correct Player data with explicit warning

**As a** privacy-conscious player, **I want** plaintext export scopes to be precise,
**so that** I understand what sensitive data leaves encryption.

Acceptance criteria:

- AC-1: Complete-notebook export requires the existing unencrypted one-way warning
  and includes Player identity, evidence, selection history, empty outcomes,
  timestamps, and source attribution in human-readable form.
- AC-2: Selected-opponent export contains no Player identity or Player-owned result
  data.
- AC-3: Plaintext export excludes consent, provider configuration, operational state,
  secrets, unimported previews, and import metadata that could make it restorable.

Edge cases:

- EC-1: Warning is not acknowledged → no file is written.
- EC-2: Complete notebook has no Player identity → export remains valid and clearly
  omits the Player section rather than inventing empty data.
- EC-3: Export is cancelled, interrupted, or destination is unwritable → no partial
  published file remains.
- EC-4: Existing destination without explicit overwrite → export is rejected.
- EC-5: Large evidence history → export streams completely within existing bounded
  resource behavior and preserves attribution for every record.

## Production Enablement

### US-016: Keep Census disabled until every live gate passes

**As a** release verifier, **I want** production Census access to fail closed, **so
that** an implemented adapter cannot make unauthorized or stale requests.

Acceptance criteria:

- AC-1: The adapter may run only against synthetic fixtures until a reviewed
  desktop-client Service ID/use model, current policy/quota/caching/attribution
  review, expiry-bound approved scope, and live contract evidence all exist.
- AC-2: Missing, expired, malformed, unreviewed, or mismatched evidence causes zero
  Census requests and an honest disabled status.
- AC-3: Production configuration never exposes or persists the Service ID in public
  attribution, renderer state, logs, diagnostics, backup, or plaintext export.
- AC-4: Revoked authorization or changed live contract can disable Census without
  removing the local/manual Player workflow or prior imports.

Edge cases:

- EC-1: `s:example`, placeholder, expired, or shared credential is configured →
  provider stays disabled.
- EC-2: Live schema differs from the reviewed fixture/contract → response is rejected
  as unavailable and raw content does not escape.
- EC-3: Configuration expires during a request → work is cancelled/fenced and no
  candidate or empty result is produced.
- EC-4: Release evidence exists only on macOS or local fixtures → Census production
  enablement and packaged Windows completion remain unproven.
- EC-5: Provider is disabled after successful prior use → imports remain readable,
  refresh is unavailable, and no fallback source is contacted.
