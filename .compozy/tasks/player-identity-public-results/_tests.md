# Test Specification: Player Identity and Public Results

Canonical test contract for Player Identity and Public Results. Companion to
`_techspec.md`. Derived from `_user_stories.md` and the approved Wayfinder handoff.

## Strategy

- Rust unit/property tests cover canonicalization, validation, policy, provider
  mapping, replay, session fencing, and pure restore/export rules with fake clock,
  randomness, HTTP, browser, and filesystem boundaries.
- Rust integration tests use real temporary SQLCipher databases, v1/v2/v3
  migrations, repository/services/runtime, synthetic HTTP fixtures, operation
  coordinator, archive files, and deterministic interruption failpoints.
- React tests use Vitest, Testing Library, Tauri mocks, fake timers, keyboard input,
  and axe against real TypeScript contracts and replacement projections.
- E2E tests enter through the main UI/IPC surface. Synthetic Census and browser
  adapters are the only automated external fakes; no test transmits a real handle.
- `rtk npm run verify` is the local full gate. Packaged Windows and live Census IDs
  remain pending until their named external evidence exists; local green tests do
  not satisfy them.
- IDs are new to this packet and do not claim or reuse completion from the executed
  opponent-notes test catalog.

## Coverage Matrix

| Source | Behavior | Unit | Integration | E2E |
|---|---|---|---|---|
| US-001 | Optional Player workspace | UT-076 | IT-048 | E2E-001 |
| US-001.EC-1 | Blank first run creates no identity | UT-076 | IT-048 | E2E-001 |
| US-001.EC-2 | Keyboard can enter and leave Player tab | UT-080 | IT-055 | E2E-015 |
| US-001.EC-3 | Repeated visits create no state | UT-076 | IT-048 | — |
| US-001.EC-4 | Offline/provider failure does not block notebook | UT-018 | IT-053 | E2E-006 |
| US-001.EC-5 | Deep action without identity is bounded | UT-020 | IT-013 | — |
| US-002 | Create/edit singleton identity | UT-001, UT-051 | IT-004, IT-049 | E2E-002 |
| US-002.EC-1 | Invalid nickname rejected | UT-002 | IT-004 | — |
| US-002.EC-2 | Case-only edit preserves history | UT-003 | IT-034 | E2E-002 |
| US-002.EC-3 | Concurrent edit rejects stale revision | UT-052 | IT-007 | — |
| US-002.EC-4 | Interrupted/replayed save is atomic | UT-013 | IT-006 | — |
| US-002.EC-5 | Edit fences active lookup | UT-039 | IT-034 | — |
| US-003 | Independent source consent | UT-019 | IT-014 | E2E-003 |
| US-003.EC-1 | Version/field mismatch means absent | UT-019 | IT-014 | — |
| US-003.EC-2 | Grant/revoke replay is idempotent | UT-013, UT-014 | IT-006 | — |
| US-003.EC-3 | Revocation fences late response | UT-040 | IT-015 | — |
| US-003.EC-4 | Consent storage failure fails closed | UT-018 | IT-014 | — |
| US-003.EC-5 | Restore leaves consent off | UT-073 | IT-043 | E2E-014 |
| US-004 | Honest route status | UT-018 | IT-050 | E2E-006 |
| US-004.EC-1 | Expiry updates status before request | UT-027 | IT-057 | — |
| US-004.EC-2 | Unknown status fails closed | UT-018 | IT-056 | — |
| US-004.EC-3 | Repeated status is current and bounded | UT-042 | IT-050 | — |
| US-004.EC-4 | Browser unavailable preserves alternatives | UT-045 | IT-023 | E2E-007 |
| US-004.EC-5 | Overlay/capture cannot query Player status | UT-020 | IT-012 | E2E-017 |
| US-005 | Explicit bounded lookup/cancel | UT-041, UT-044 | IT-016–IT-018 | E2E-003, E2E-004 |
| US-005.EC-1 | Missing prerequisite makes zero request | UT-026 | IT-013 | — |
| US-005.EC-2 | Second lookup rejected as busy | UT-041 | IT-016 | — |
| US-005.EC-3 | Cancel/response race has one terminal state | UT-044 | IT-015, IT-018 | E2E-004 |
| US-005.EC-4 | Time/byte/row/preview limits degrade | UT-030, UT-031, UT-025 | IT-019 | — |
| US-005.EC-5 | Early retry shows retry time and makes no request | UT-036, UT-045 | IT-016 | — |
| US-005.EC-6 | Replay does not request twice; mismatch invalid | UT-043, UT-014 | IT-006, IT-016 | — |
| US-006 | Candidate/empty/degraded distinction | UT-024, UT-034, UT-035 | IT-020–IT-022 | E2E-003, E2E-005, E2E-006 |
| US-006.EC-1 | Non-exact match is not candidate | UT-024 | IT-020 | — |
| US-006.EC-2 | More than ten previews is bounded | UT-025 | IT-019 | — |
| US-006.EC-3 | Repeated empty operation creates one row | UT-059 | IT-021 | — |
| US-006.EC-4 | Later positive coexists with empty | UT-011 | IT-030 | E2E-005 |
| US-006.EC-5 | Ephemeral previews disappear on restart | UT-037 | IT-035 | — |
| US-007 | Approved browser handoff | UT-046, UT-047 | IT-023 | E2E-007 |
| US-007.EC-1 | Missing prerequisite opens no browser | UT-020 | IT-013, IT-023 | — |
| US-007.EC-2 | Unsafe route fails closed | UT-048 | IT-023 | — |
| US-007.EC-3 | Browser failure changes no evidence | UT-045 | IT-023 | E2E-007 |
| US-007.EC-4 | Replay opens at most once | UT-050 | IT-006, IT-023 | — |
| US-007.EC-5 | Overlay/capture handoff denied | UT-020 | IT-012 | E2E-017 |
| US-008 | No-fetch manual evidence preview | UT-022, UT-023, UT-049 | IT-024 | E2E-008, E2E-009 |
| US-008.EC-1 | Unsafe/long URL rejected without I/O | UT-022, UT-048 | IT-024 | — |
| US-008.EC-2 | Missing/over-limit fields rejected | UT-023 | IT-024 | — |
| US-008.EC-3 | Invalid/partial card list rejected | UT-008, UT-009 | IT-024 | — |
| US-008.EC-4 | Manual preview replay is ephemeral | UT-043 | IT-024 | — |
| US-008.EC-5 | Identity change/expiry makes preview stale | UT-037–UT-039 | IT-026, IT-034 | — |
| US-009 | Selective exact import | UT-010, UT-038 | IT-026, IT-028 | E2E-003 |
| US-009.EC-1 | Binding/digest mismatch rejects import | UT-038 | IT-026 | — |
| US-009.EC-2 | Concurrent duplicate import yields one record | UT-053 | IT-028 | — |
| US-009.EC-3 | Interrupted batch rolls back | UT-058 | IT-027 | — |
| US-009.EC-4 | Replay creates no duplicate | UT-013 | IT-006, IT-028 | — |
| US-009.EC-5 | Unknown/oversized payload rejected | UT-021 | IT-026 | — |
| US-010 | Provenance view and selection revisions | UT-006, UT-056 | IT-031, IT-046 | E2E-011 |
| US-010.EC-1 | Unknown retained field rejected | UT-010 | IT-031 | — |
| US-010.EC-2 | Concurrent selection rejects stale revision | UT-057 | IT-007, IT-031 | — |
| US-010.EC-3 | Selection replay adds no revision | UT-013, UT-056 | IT-006, IT-031 | — |
| US-010.EC-4 | Revocation/source loss preserves evidence | UT-063 | IT-014 | E2E-011 |
| US-010.EC-5 | Large history is paged with attribution | UT-062 | IT-046 | — |
| US-011 | Explicit non-destructive refresh | UT-053–UT-055 | IT-028–IT-030 | E2E-010 |
| US-011.EC-1 | Identity change stales refresh | UT-039 | IT-034 | — |
| US-011.EC-2 | Repeated refresh dedupes | UT-053 | IT-028 | — |
| US-011.EC-3 | Unselected changed preview expires | UT-037 | IT-035 | — |
| US-011.EC-4 | Disable/revoke preserves evidence | UT-040, UT-063 | IT-015 | E2E-006 |
| US-011.EC-5 | Out-of-order generation ignored | UT-064, UT-065 | IT-047 | — |
| US-012 | Player-owned complete-deck classification | UT-017, UT-060 | IT-032 | E2E-009 |
| US-012.EC-1 | Unsupported/unavailable classifier preserves import | UT-061 | IT-033 | E2E-009 |
| US-012.EC-2 | Reclassification is append-only | UT-060 | IT-032 | — |
| US-012.EC-3 | Stale invalid cards reject import | UT-008, UT-038 | IT-026 | — |
| US-012.EC-4 | 500-row deck remains bounded | UT-008 | IT-032 | — |
| US-012.EC-5 | Identical Player/opponent decks remain separate | UT-060 | IT-008, IT-010 | E2E-017 |
| US-013 | Scoped/cascade Player deletion | UT-066, UT-068 | IT-036, IT-037 | E2E-012, E2E-013 |
| US-013.EC-1 | Stale deletion preview rejected | UT-067 | IT-036 | — |
| US-013.EC-2 | Deletion race leaves no orphans | UT-068 | IT-037 | — |
| US-013.EC-3 | Deletion replay has no extra effect | UT-013 | IT-006, IT-037 | — |
| US-013.EC-4 | Interrupted deletion recovers atomically | UT-068 | IT-038 | — |
| US-013.EC-5 | Missing target is bounded | UT-066 | IT-036 | — |
| US-014 | Encrypted Player portability | UT-070–UT-073 | IT-039–IT-043 | E2E-014 |
| US-014.EC-1 | Invalid archive leaves live state | UT-071 | IT-039 | — |
| US-014.EC-2 | Repeated merge skips duplicate/no resurrection | UT-069 | IT-040, IT-042 | — |
| US-014.EC-3 | Different ID blocks whole merge | UT-070 | IT-041 | — |
| US-014.EC-4 | Interrupted restore reaches complete state | UT-074 | IT-039 | E2E-014 |
| US-014.EC-5 | Replace restores data but not consent | UT-073, UT-075 | IT-043 | E2E-014 |
| US-015 | Precise plaintext export | UT-072 | IT-044, IT-045 | E2E-014 |
| US-015.EC-1 | Missing warning writes no file | UT-072 | IT-044 | — |
| US-015.EC-2 | No Player identity yields valid omission | UT-072 | IT-044 | — |
| US-015.EC-3 | Cancel/failure leaves no published partial | UT-072 | IT-044 | — |
| US-015.EC-4 | Existing destination requires overwrite | UT-072 | IT-044 | — |
| US-015.EC-5 | Large export streams with attribution | UT-072 | IT-044 | — |
| US-016 | Fail-closed Census enablement | UT-026, UT-027 | IT-056–IT-060 | E2E-016 |
| US-016.EC-1 | Placeholder/expired/shared credential disabled | UT-027 | IT-056 | — |
| US-016.EC-2 | Schema drift degrades without content leak | UT-032, UT-035 | IT-022, IT-025 | — |
| US-016.EC-3 | Config expiry fences in-flight work | UT-040 | IT-057 | — |
| US-016.EC-4 | Local/macOS evidence cannot close external gates | — | IT-060 | E2E-016 |
| US-016.EC-5 | Disabled provider preserves imports/no fallback | UT-026, UT-063 | IT-053, IT-056 | E2E-006 |
| `PlayerResultProvider` | Disabled/synthetic/live contract and errors | UT-026–UT-035 | IT-017–IT-022 | E2E-003, E2E-006 |
| `PlayerPublicResults` | Closed service surface and bindings | UT-019–UT-050 | IT-011–IT-035 | E2E-003–E2E-011 |
| `PlayerStore` | Singleton/immutable/transactional persistence | UT-051–UT-062 | IT-001–IT-010, IT-026–IT-033 | E2E-002, E2E-003 |
| `PlayerWorkspaceView` | Monotonic bounded replacement projection | UT-063–UT-065, UT-076–UT-085 | IT-046–IT-055 | E2E-001–E2E-015 |

## Unit Tests

### Identity, Canonicalization, and Domain Rules

- **UT-001** (happy): `normalize_player_nickname("  Teichou_Aisu  ")` returns display `Teichou_Aisu` and the canonical case-folded normalized value.
- **UT-002** (boundary): nickname validation accepts 128 scalar values and rejects blank, control-character, and 129-value input with `invalid_request`.
- **UT-003** (state): case-only identity edit updates display spelling and revision without changing evidence lookup/source nicknames.
- **UT-004** (happy): Census source-key encoding of provider/catalog/start/as-of/case-folded nickname is deterministic and length-prefix unambiguous.
- **UT-005** (happy): official source-key encoding uses canonical artifact URL while retaining the exact entered attribution URL separately.
- **UT-006** (ordering): canonical source-digest serialization is stable across object/map insertion order and changes when any scoped payload value changes.
- **UT-007** (state): preview digest changes when lookup or observation metadata changes while source digest remains constant.
- **UT-008** (boundary): complete-deck validation accepts 500 unique card/zone rows at quantity 250 and rejects row 501, quantity 251, and normalized duplicates.
- **UT-009** (error): reference-only evidence accepts zero cards; a nonempty incomplete/partial card set returns `manual_evidence_invalid`.
- **UT-010** (error): selection validation requires source identity/attribution and rejects fields absent from the approved preview.
- **UT-011** (state): only a validated response with zero exact matches constructs `EmptyLookupResult`; every provider error path returns degradation.
- **UT-012** (error): Player error serialization exposes code/recovery/optional retry time and removes provider text, URL, token, payload, and card content.
- **UT-013** (idempotency): equal command kind/identity/request digest replays the stored receipt result without a new effect.
- **UT-014** (error): reusing one operation key with a different canonical digest returns `invalid_request`.
- **UT-015** (state): `PlayerTombstone` serialization contains only entity kind/ID, Player ID, and deletion time.
- **UT-016** (state): a changed source digest may supersede only evidence with the same Player ID and source key.
- **UT-017** (state): classification eligibility is true only for complete, format-valid official published-decklist evidence.
- **UT-018** (state): provider status maps disabled, invalid, expired, cooldown, busy, and unavailable states to distinct fail-closed views.
- **UT-019** (error): consent validation requires exact route, disclosure version, and canonical field-set digest; any mismatch returns `consent_required`.
- **UT-020** (state): trusted-phase policy permits status/cancel/revoke everywhere and denies other public actions outside Idle/PreMatch/BetweenGames/Finished.
- **UT-021** (boundary): command envelope accepts exactly 256 KiB encoded and rejects one byte more with `payload_too_large` before side effects.
- **UT-022** (boundary): official artifact URL accepts the exact allowlisted HTTPS route at 2,048 characters and rejects longer, wrong host/path, credential, fragment, and ambiguous forms.
- **UT-023** (boundary): manual fields accept title 200 and format/placement/record 64 characters and reject blank required/control/over-limit values.
- **UT-024** (happy): exact matcher accepts only case-insensitive full-string equality and rejects substring, fuzzy, whitespace-mutated, and alias candidates.
- **UT-025** (boundary): candidate projection accepts 10 exact previews and maps an 11th to the bounded provider response error without partial import.

### Provider, Runtime, Routes, and Fencing

- **UT-026** (error): `DisabledProvider.lookup` returns `provider_disabled` without constructing or invoking HTTP and without fallback.
- **UT-027** (boundary): live configuration rejects missing, `s:example`, placeholder, expired, wrong-scope, and unreviewed inputs and accepts only a current reviewed fingerprint.
- **UT-028** (happy): Census request builder emits only fixed HTTPS host/path, Service ID, catalog/start/as-of parameters, and no nickname/cookie/referrer/arbitrary query.
- **UT-029** (error): redirect response is rejected as `provider_invalid_response` without following location.
- **UT-030** (boundary): decompressed response at 1 MiB is accepted and one byte more returns `response_too_large`.
- **UT-031** (boundary): 2,000 validated rows are accepted and row 2,001 returns `response_too_large` before matching.
- **UT-032** (error): missing/wrong-type/out-of-range MOCS fields return `provider_invalid_response` and expose no partial candidate.
- **UT-033** (happy): validated rows are matched locally against the frozen nickname with exact case-insensitive comparison.
- **UT-034** (state): zero exact matches from a valid response produces the approved scoped empty outcome fields.
- **UT-035** (error): timeout, throttle, transport, schema, size, and configuration errors map to degraded outcomes, never empty.
- **UT-036** (state): cooldown selects the later of local 60-second minimum and a valid provider retry time.
- **UT-037** (boundary): session/preview is valid immediately before 15 minutes and returns expired at the boundary.
- **UT-038** (error): import rejects token/session/source-key/source-digest/preview-digest mismatch with no verified batch.
- **UT-039** (state): identity ID or revision change invalidates an active session and all attached previews.
- **UT-040** (concurrency): consent/config epoch change before completion fences the late provider result.
- **UT-041** (concurrency): runtime grants one global machine lookup lease and rejects a second with `lookup_in_progress`.
- **UT-042** (boundary): audit ring retains the newest 100 allowlisted summaries and contains no forbidden content fields.
- **UT-043** (idempotency): ephemeral lookup/manual/cancel replay returns one bounded prior result and clears after restart.
- **UT-044** (concurrency): cancel/result race commits exactly one terminal session outcome and rejects the loser.
- **UT-045** (error): retryable errors expose only validated `retryAt`; browser/open/config errors do not invent retry times.
- **UT-046** (happy): official MTGO route builder percent-encodes the host-loaded nickname into the fixed player-search URL.
- **UT-047** (happy): MTGTop8 route builder uses its independent fixed route and never produces an import attribution.
- **UT-048** (error): any unapproved scheme/host/path/query/redirect mode returns `unsafe_source` before browser/network work.
- **UT-049** (state): manual preview pure validator performs zero calls to injected DNS/HTTP/browser spies.
- **UT-050** (idempotency): browser receipt state permits one open for one operation key and returns the prior outcome on exact replay.

### Repository, Evidence, Classification, and Projection

- **UT-051** (state): singleton identity insert rejects a second stable ID while allowing revision-bound edit of the existing row.
- **UT-052** (concurrency): identity update with stale expected revision returns `identity_revision_conflict` and preserves current data.
- **UT-053** (idempotency): same Player/source key/source digest resolves to the existing evidence ID without insert.
- **UT-054** (state): same source key/new digest creates an immutable row linked through `supersedes_evidence_id`.
- **UT-055** (state): equal digests under different source keys create distinct evidence rows.
- **UT-056** (ordering): selection update appends revision N+1 and leaves revisions 1..N unchanged.
- **UT-057** (concurrency): selection update with stale expected revision returns revision conflict and appends nothing.
- **UT-058** (error): one invalid preview in a verified import batch rejects the batch before repository mutation.
- **UT-059** (idempotency): repeating an empty-result operation key returns the existing outcome and inserts no second row.
- **UT-060** (state): Player classification result maps to `player_classification_runs` and cannot construct an opponent classification insert.
- **UT-061** (error): unavailable/unsupported classifier maps evidence to unclassified status without an evidence delete/update.
- **UT-062** (boundary): evidence paging returns stable order/cursor at zero, typical, and large histories with attribution intact.
- **UT-063** (state): loading/empty/degraded/cancelled workspace projections retain the current evidence page.
- **UT-064** (ordering): native workspace projection revisions increase monotonically across committed state changes.
- **UT-065** (ordering): frontend reducer ignores equal/older replacement revisions and accepts the next newer complete view.

### Deletion, Portability, Export, and UI Presentation

- **UT-066** (happy): deletion preview digest binds exact target, current revision, dependent counts, and expiry.
- **UT-067** (error): expired token, digest mismatch, or changed target revision returns `deletion_preview_stale`.
- **UT-068** (state): whole-identity deletion plan contains only Player tables/consents/runtime fence and no opponent table or generic consent action.
- **UT-069** (state): active Player tombstones suppress matching archive records and dependent rows before merge insertion.
- **UT-070** (state): restore compatibility allows absent/same Player ID and reports hard whole-merge conflict for a different ID.
- **UT-071** (state): archive registry includes the seven canonical Player tables in FK order and excludes consent/receipts/runtime/config/secrets.
- **UT-072** (state): export scope mapper includes Player section only for acknowledged `CompleteNotebook` and preserves opponent-only selection.
- **UT-073** (state): restored runtime/provider/consent projection is disabled regardless of archived Player records.
- **UT-074** (state): restore diff counts Player imports/duplicates/conflicts/tombstone skips deterministically.
- **UT-075** (state): different Player ID still permits explicit `Replace` while removing `Merge` from allowed modes.
- **UT-076** (state): first-use view has null identity, visible Player tab/source explanation, no implicit consent/session/evidence, and enabled navigation.
- **UT-077** (happy): consent disclosure renders destination, purpose, mode, exact outbound fields, grant/revoke, and local retention text.
- **UT-078** (state): selection bar is disabled at zero selected results and summarizes exactly the selected result/field counts otherwise.
- **UT-079** (state): every source/lookup state has text plus accessible status independent of color.
- **UT-080** (ordering): command completion/status replacement restores intended trigger focus and never focuses a status message automatically.
- **UT-081** (boundary): Player layout uses two columns above and one reading-order column below the approved breakpoint without overflow classes.
- **UT-082** (error): every documented Player error code maps to specific safe copy/recovery and never renders raw `details` content.
- **UT-083** (happy): saved evidence details render provenance, provider, attribution, lookup/source nicknames, scope/time, retained fields, and version link.
- **UT-084** (state): deletion confirmation renders exact target/counts, requires explicit confirm, and cancels without mutation intent.
- **UT-085** (happy): axe finds zero violations for first-use, consent, ready, loading, candidates, empty, degraded, imported, nickname-edit, and deletion-preview fixtures.

## Integration Tests

### Migration, Repository, and Isolation

- **IT-001**: v2 encrypted notebook → run migration manager → v3 Player tables/indexes/FKs/checksum exist and prior opponent rows remain unchanged.
- **IT-002**: inject v3 migration failure before commit → reopen database → schema remains v2 with no partial Player table and migration error is reported.
- **IT-003**: save Player identity/evidence in real SQLCipher DB → close/reopen with DPAPI-test key → exact rows/revisions/provenance remain readable.
- **IT-004**: two singleton create calls with different IDs → repository stores one and returns `player_identity_conflict` for the other.
- **IT-005**: import candidate with cards/selection/receipt → one transaction commits all Player rows and FK check passes.
- **IT-006**: commit durable operation receipt → restart service → exact replay returns stored locator and changed digest returns `invalid_request`.
- **IT-007**: run two identity/selection writes from one starting revision → one commits; the other returns revision conflict with no lost update.
- **IT-008**: classify complete Player evidence → `player_classification_runs` gains one row and opponent `classification_runs` count is unchanged.
- **IT-009**: delete Player evidence → its cards/selections/classifications cascade, tombstone remains, and unrelated Player evidence remains.
- **IT-010**: execute identity/import/selection against DB containing opponent history → byte-level logical opponent digest and opponent consent rows remain unchanged.

### Trusted Host, Provider, and Routes

- **IT-011**: invoke every Player command as `Main` with correct capability → typed result; command registration and main manifest match.
- **IT-012**: invoke every Player command as overlay/capture and inspect capability manifests → denial occurs before repository/provider/browser calls.
- **IT-013**: invoke each phase-restricted command in in-game/unknown/stale phase → `disclosure_restricted` and zero external/durable effects; status/cancel/revoke remain callable.
- **IT-014**: grant three route consents independently → revoke one → only its row/epoch changes; unreadable/mismatched consent makes that route unavailable.
- **IT-015**: synthetic provider blocks at completion while consent is revoked/cancelled → late response is fenced, no preview/empty row/event is published.
- **IT-016**: hold one synthetic lookup active → second start and early retry return busy/cooldown with exactly one provider request.
- **IT-017**: synthetic Census lookup captures request → exact host/path/scope present, nickname/cookies/referrer absent, and exact local matches produce bounded previews.
- **IT-018**: timeout/cancellation failpoints during connect/body/parse → one terminal typed outcome, no partial preview/import, and lease is released.
- **IT-019**: synthetic 1MiB/2,000-row boundaries succeed; over-byte/row/10-preview fixtures return bounded degradation and no durable empty.
- **IT-020**: response contains exact-case, fuzzy, substring, and alias rows → only exact-case rows become candidate previews.
- **IT-021**: valid response has no exact row → one scoped empty outcome commits transactionally and exact replay returns it.
- **IT-022**: disabled/expired/throttled/malformed/schema-drift fixtures → each produces its exact degraded error and zero empty/evidence rows.
- **IT-023**: official and MTGTop8 handoff commands with valid consent → host-built URLs reach browser fake; unsafe/missing consent/open failure/replay follow typed at-most-once behavior.
- **IT-024**: manual official preview with reference/complete/invalid inputs → pure canonicalizer issues valid tokens only for valid inputs and all I/O spies remain zero.
- **IT-025**: seed provider body with nickname/URL/token/card/secret markers → IPC, audit, logs, diagnostics, and error objects contain none of those markers.

### Evidence Lifecycle and Frontend Contract

- **IT-026**: create preview → mutate token/digest/session/identity/card selection independently → each import is rejected and repository remains unchanged.
- **IT-027**: import three selected previews and fail after second insert → transaction rolls back evidence/cards/selections/receipts for all three.
- **IT-028**: concurrent imports/refreshes of same source key/digest → one evidence row and one first selection revision exist.
- **IT-029**: import same source key with changed digest → new immutable row links to prior row; prior payload/selection remains byte-identical.
- **IT-030**: persist empty outcome then refresh with positive exact match → both records coexist; newest outcome projection shows candidate/import state.
- **IT-031**: append retained-field selection through command → revision and view update; stale/unknown/replay paths append nothing extra.
- **IT-032**: import valid complete 500-row official deck → evidence commits, shared classifier runs, Player classification row persists, opponent tables unchanged.
- **IT-033**: import complete deck with unsupported format or failing classifier → evidence remains committed and projection is unclassified with typed reason.
- **IT-034**: active lookup/manual preview then identity edit → session cancels, previews stale, future request uses new nickname, historical evidence remains unchanged.
- **IT-035**: create unimported previews/audit/ephemeral replay → restart runtime → all disappear while imported evidence/empty outcomes/durable receipts remain.

### Deletion, Portability, and Export

- **IT-036**: preview/confirm individual evidence and empty-outcome deletion → exact target subtree is removed, tombstone written, stale/missing/replay cases are bounded.
- **IT-037**: preview/confirm whole identity with active lookup and opponent history → Player graph/consents removed and runtime fenced; opponent logical digest/consent unchanged.
- **IT-038**: fail whole deletion before/at/after tombstone failpoints → restart yields one complete pre-delete or post-delete state with no orphan rows.
- **IT-039**: backup populated mixed notebook → restore through staging with correct passphrase and interruption failpoints → manifest/FKs/checksums pass and one complete state remains.
- **IT-040**: merge archive into no-Player and same-ID notebooks twice → first imports/skips correctly, second is idempotent, and consent remains absent.
- **IT-041**: archive Player ID differs from live while opponent rows are mergeable → preview disallows merge and apply recheck performs zero Player/opponent live mutation.
- **IT-042**: merge archive containing records deleted by live Player tombstones → deleted subtree is skipped and cannot resurrect on repeated merge.
- **IT-043**: replace with archived Player data → data restores and rollback exists; all Player routes/runtime start disabled with no consent/receipt restored.
- **IT-044**: complete-notebook export with/without Player data and large evidence history → acknowledged atomic text includes exact attribution/history and excludes forbidden state/partial files.
- **IT-045**: selected-opponent export from mixed notebook → output contains selected opponent content and no Player nickname/evidence/empty/classification markers.

### IPC, UI, Privacy, and Release Gates

- **IT-046**: Rust workspace/evidence projections serialize through generated TypeScript fixtures → all bounded fields round-trip and forbidden runtime/provider fields are absent.
- **IT-047**: deliver replacement events out of order/gapped → frontend ignores stale revisions and refreshes snapshot after gap without losing current evidence display.
- **IT-048**: render MainApp with no identity and navigate all existing tabs/actions → Player first-use is visible and every preexisting workflow remains enabled.
- **IT-049**: create/edit identity through UI mock → local view updates, no provider call occurs, warning/focus behavior is correct, and stale edit renders conflict recovery.
- **IT-050**: render all source consent/status variants → independent inline controls, exact disclosures, valid actions, and accessible text match the projection.
- **IT-051**: run lookup UI with controllable promise → loading keeps evidence visible, cancel is keyboard reachable, terminal focus/status is correct.
- **IT-052**: render candidates and select result/fields → mandatory provenance cannot deselect, summary counts update, import calls exact tokens/digests/fields once.
- **IT-053**: render empty/degraded/cancelled/provider-disabled transitions → each has distinct announcement/recovery and saved evidence remains visible.
- **IT-054**: use manual form at valid/boundary/invalid values → field errors and preview requests match bounds, no arbitrary/unknown field is sent.
- **IT-055**: keyboard/axe sweep all approved states at desktop/stacked widths → no trap, focus loss/obscuring, color-only state, clipping, or axe violation.
- **IT-056**: start production runtime with absent/placeholder/expired/unreviewed live configuration → factory constructs Disabled mode and HTTP request count stays zero.
- **IT-057**: expire/replace reviewed configuration while synthetic live-shaped request is in flight → completion is fenced and provider status becomes disabled.
- **IT-058**: run diagnostics/privacy scanners over Player operations → only aggregate/allowlisted fields appear and all seeded forbidden markers are absent.
- **IT-059**: run full existing opponent test/verify suite with populated Player graph → all existing behavior passes and no task status is promoted by this test alone.
- **IT-060**: release-gate validator receives only local/macOS/synthetic evidence → packaged Windows and live Census gates remain explicitly pending; complete evidence set is required to mark each gate.

## End-to-End Tests

### Optional Workspace and Identity

- **E2E-001**: fresh app with no Player identity → open/leave Player tab → create opponent note/use capture/navigation → all existing workflows succeed and no Player state/consent/request exists.
- **E2E-002**: Player tab → save `Teichou_Aisu` → edit display to `teichou_aisu` after importing historical fixture → no lookup starts and historical lookup/source spellings remain unchanged.

### Lookup, Outcomes, and Handoffs

- **E2E-003**: save identity → grant Census disclosure → start synthetic exact lookup → select result/optional fields → import → saved immutable provenance appears and no nickname was sent.
- **E2E-004**: start delayed lookup → cancel by keyboard → cancelled status announced, evidence remains visible, and released late response changes nothing.
- **E2E-005**: run valid no-match lookup → scoped empty appears → later exact refresh/import → empty and positive evidence coexist with newest outcome shown.
- **E2E-006**: exercise disabled, expired, throttled, malformed, offline, and classifier-unavailable fixtures → distinct degraded/unclassified states appear, existing evidence/local workflows remain usable, and no fallback opens.
- **E2E-007**: review/grant official then MTGTop8 disclosures separately → open each host-built browser route → revoke → future open is blocked; browser failure creates no evidence/empty result.

### Manual Evidence, Refresh, and Classification

- **E2E-008**: enter exact official URL and reference-only result → preview no-fetch attribution → select/import → saved evidence has no cards/classification.
- **E2E-009**: enter complete valid official deck → preview/import → Player-owned classification appears; unsupported-format variant imports as unclassified and opponent deck history remains unchanged.
- **E2E-010**: import source version A → explicit refresh returns duplicate plus changed B → duplicate identified, B previewed/imported as linked version, A remains unchanged.
- **E2E-011**: open saved evidence → revise retained optional fields → append-only selection history and provenance render; revoke source consent → evidence remains readable.

### Deletion and Portability

- **E2E-012**: preview/cancel then confirm individual evidence deletion → only that evidence subtree disappears, tombstone blocks merge resurrection, opponent history remains.
- **E2E-013**: populate Player/opponent data and active lookup → preview/confirm whole Player deletion → Player first-use returns, Player consent/work disappear, opponent data/consent remain.
- **E2E-014**: backup mixed notebook → same-ID merge/different-ID conflict/replace → export complete and selected-opponent scopes → data, consent-off, no-resurrection, and plaintext scope rules all hold.

### Accessibility and External Evidence Gates

- **E2E-015**: packaged Windows 10/11 Player workflow through first-use, consent, lookup, candidate, empty, degraded, imported, edit, and deletion → keyboard/screen-reader/focus/contrast/scaling/clipping/browser behavior meets the release checklist.
- **E2E-016**: release candidate with disabled adapter and synthetic green suite → live Census remains disabled until current Service-ID/use-model/policy/config/live-contract evidence is attached; approved live fixture then proves only the exact gated route.
- **E2E-017**: packaged mixed notebook with Player data → run opponent lookup/capture/overlay/deletion/export and attempt Player commands from overlay/capture → opponent behavior is unchanged and Player capabilities remain denied.
