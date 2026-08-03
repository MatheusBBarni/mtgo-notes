# MTGO Opponent Notes

A private, local-first companion that helps an MTGO player remember opponents and review verifiable public context without becoming an MTGO account client.

## Language

**Player identity**:
The local representation of the person using the companion, identified by their self-entered MTGO nickname.
_Avoid_: User profile, account, opponent profile

**Opponent profile**:
The local record for another MTGO player encountered by the person using the companion.
_Avoid_: Player identity, account

**Public player result**:
A verifiable, source-attributed tournament result or published decklist associated with a player's exact MTGO nickname.
_Avoid_: Account data, match history

**Public result lookup**:
An explicit, consented request that searches an enabled public provider for results associated with the saved **Player identity** and previews them before import.
_Avoid_: Automatic sync, background lookup

**Conditional public provider**:
A machine-readable public provider included in the V1 strategy but unavailable until its external authorization and product safety gates are satisfied.
_Avoid_: Enabled provider, automatic fallback

**Trusted lookup host**:
The local authority that derives disclosure-sensitive lookup inputs, enforces consent, validates evidence, and owns public-source side effects.
_Avoid_: Renderer request builder, browser authority

**Outbound consent**:
A versioned opt-in for one external destination and its exact disclosed field set.
_Avoid_: Public-data consent, network consent

**Lookup session**:
A host-owned generation that binds one lookup intent to a Player identity snapshot, consent, provider configuration, and its expiring previews.
_Avoid_: Browser request, UI session

**Lookup audit**:
A bounded, content-free, in-memory record of trusted-host public-source operations.
_Avoid_: Lookup history, activity log

**Lookup outcome**:
A closed trusted-host result that distinguishes candidates, scoped absence, duplication, cancellation, and browser handoff from fail-closed errors.
_Avoid_: Provider response, generic error

**Approved public source route**:
An exact host, path, parameter, and access-mode contract that the **Trusted lookup host** may use for one public destination.
_Avoid_: URL allowlist, provider endpoint

**Public operation key**:
A UUIDv7 idempotency identity bound to one command kind, Player identity, and canonical request digest.
_Avoid_: Request token, retry counter

**Exact nickname match**:
A public result candidate whose player nickname equals the saved MTGO nickname without regard to letter case.
_Avoid_: Partial match, fuzzy match, inferred identity

**Imported public result**:
A user-selected, read-only snapshot of a **Public player result** stored with its source attribution in the local notebook.
_Avoid_: Synced result, editable result, personal note

**Public result evidence**:
A Player-owned immutable provenance envelope whose typed payload is either an MOCS leaderboard entry or an official published decklist.
_Avoid_: Encounter snapshot, opponent deck record

**Evidence provenance mode**:
The explicit origin of **Public result evidence**, either `provider_observed` or `user_attested_official_source`.
_Avoid_: Verified flag, trust score

**Evidence preview**:
The complete canonical **Public result evidence** statement shown for approval before any durable import.
_Avoid_: Summary card, mutable draft

**Evidence source key**:
The provider-specific compound identity of one logical public source statement.
_Avoid_: Source digest, local evidence ID

**Source digest**:
The canonical digest of source scope, source nickname, and typed payload used for public-source change detection.
_Avoid_: Preview digest, source key

**Preview digest**:
The canonical digest of the complete approved **Evidence preview**, including its lookup and observation metadata.
_Avoid_: Source digest, mutable checksum

**Evidence selection**:
The append-only local revision that records which payload fields from an approved **Evidence preview** the player chose to retain.
_Avoid_: Source revision, editable import

**Official-source manual import**:
A user-entered **Imported public result** attributed to an exact official MTGO event or decklist URL without the companion fetching or parsing that page.
_Avoid_: Parsed import, third-party import

**External source handoff**:
An explicit action that opens a public source in the system browser without fetching, parsing, or persisting its contents in the companion.
_Avoid_: Embedded browser, provider lookup

**Result refresh**:
An explicit **Public result lookup** that compares provider candidates with existing imports using stable source identity and previews only new or changed information.
_Avoid_: Background sync, silent overwrite

**Empty lookup result**:
An evidence-limited outcome stating that a named provider returned no verified public result at the recorded lookup time.
_Avoid_: Player not found, no public history

**Degraded public lookup**:
A provider-scoped unavailable outcome that preserves local evidence and waits for the player to choose whether to retry, open an official source, or import manually.
_Avoid_: Empty lookup result, automatic fallback

**Player workspace**:
The top-level Player tab where the person manages their **Player identity**, runs public lookups, reviews candidates, and inspects imported results.
_Avoid_: Settings, opponent profile

**Implementation handoff**:
An approved decision packet that authorizes Compozy requirements, design, test-contract, and task planning without asserting feature or release completion.
_Avoid_: Release approval, feature complete

## Relationships

- A **Player identity** is distinct from every **Opponent profile**.
- A local notebook stores at most one **Player identity** in V1.
- A **Player identity** may reference zero or more **Public player results**.
- Changing the saved **Player identity** nickname never rewrites or rekeys existing **Public result evidence** or **Empty lookup results**.
- A **Player identity** owns zero or more **Public result evidence** records independently of opponent encounters.
- Encrypted backup and restore treat the **Player identity**, **Public result evidence**, evidence-selection revisions, and **Empty lookup results** as canonical notebook data.
- Encrypted backup and restore exclude outbound consent grants, provider configuration and Service IDs, active lookup sessions, preview tokens, audit trails, cooldowns, replay caches, and machine-bound secrets; restore never enables outbound access.
- Merge restore imports an archived **Player identity** when none exists, and merges Player-owned records only when the archived and destination identities share the same stable identity ID.
- Different stable Player identity IDs are a hard whole-merge conflict: the user must replace or cancel, and evidence is never reassigned across identities.
- A plaintext `complete_notebook` export includes the **Player identity**, **Public result evidence**, evidence-selection revision history, and **Empty lookup results** in human-readable form with timestamps and source attribution.
- A plaintext `selected_opponent` export includes no Player identity or Player-owned result data.
- A user may delete an individual **Public result evidence** record or **Empty lookup result** after a scoped preview and explicit confirmation; deletion does not make immutable evidence content editable.
- Deleting the **Player identity** atomically deletes all Player-owned evidence, selection revisions, and empty outcomes, cancels active Player lookups, and revokes Player-specific provider consent without changing opponent profiles, encounters, or opponent-provider consent.
- Player-data deletion retains only non-content tombstones needed to prevent merge restore resurrection; an explicit replace restore may restore previously backed-up Player data.
- The Player-owned public-result workflow and encounter-bound opponent public-deck enrichment are distinct domain workflows; neither creates, triggers, or rewrites the other's records.
- The Player-owned public-result workflow receives a new Compozy **Implementation handoff** packet rather than rewriting the executed opponent-notes packet; integration references do not transfer ownership of requirements, tests, or task status.
- The Player-owned workflow may share generic trusted-host, persistence, encryption, and renderer infrastructure with opponent enrichment, but it never reuses opponent profiles, encounter triggers, public-deck snapshots, evidence schemas, or command authority.
- Saving a **Player identity** never starts a **Public result lookup**.
- Daybreak Census is the only V1 **Conditional public provider** and remains disabled unless every production gate passes.
- A **Conditional public provider** may be specified and implemented against synthetic fixtures while disabled; external authorization and live-contract evidence gate production enablement rather than Player-workspace planning or the independent local/manual workflow.
- Machine lookup and refresh give the **Trusted lookup host** only Player identity, provider, and operation identity; the host loads the nickname, consent, and approved source scope.
- Import gives the **Trusted lookup host** an opaque preview token, matching preview digest, selected fields, and idempotency key.
- Manual preview gives the **Trusted lookup host** typed player-entered facts and an official URL but performs no network request.
- Public-source commands are closed and intent-specific; V1 exposes no generic provider request, arbitrary evidence import, or arbitrary URL-opening path.
- Only the main Player workspace may invoke public-source commands; overlay and capture surfaces have no public-source capability.
- Provider status, cancellation, and **Outbound consent** revocation remain available at every gameplay phase.
- Consent grant, lookup, refresh, manual preview, import, selection change, and browser handoff require a trusted outside-gameplay phase.
- Unknown or stale gameplay phase is disclosure-restricted and fails closed.
- Every public-source command accepts only bounded, schema-known fields; over-limit values, unknown fields, duplicate normalized card rows, control characters, and unbounded collections are rejected before network access or persistence.
- A **Lookup session** binds Player identity ID and revision, exact nickname snapshot, provider and consent version, provider-configuration version and scope, operation key, and host generation.
- V1 permits only one active machine **Lookup session** and performs no automatic provider retry.
- Retry requires another explicit action after the host cooldown and any later provider-directed backoff.
- A **Lookup session** and all of its previews expire after fifteen minutes.
- **Lookup audit** retains at most one hundred operation summaries and is cleared when the application restarts.
- **Lookup audit** records command and caller identity, provider and policy versions, session generation, timing, outcome codes, bounded counts, and cancellation reason without retaining lookup or evidence content.
- Only imported evidence and **Empty lookup results** persist their required audit metadata; diagnostics receive aggregate lookup counters only.
- Candidates, **Empty lookup results**, already-imported evidence, cancellation, and completed browser handoff are successful **Lookup outcomes** rather than errors.
- Policy, configuration, admission, provider, source-validation, fencing, browser, and persistence failures use a closed error taxonomy with typed recovery guidance and optional retry time.
- Raw provider bodies and messages never enter a **Lookup outcome**, renderer payload, log, or diagnostic bundle.
- Census has one **Approved public source route** for the fixed MTGO leaderboard request, with approved scope parameters only, no redirects, and no transmitted nickname.
- Official MTGO and MTGTop8 handoffs use separate host-built **Approved public source routes** opened only in the system browser.
- Manual evidence validates an official MTGO artifact route without fetching, resolving, embedding, or parsing it.
- Any host, path, parameter, redirect, or access-mode deviation produces no external request or **Evidence preview**.
- Every side-effecting public-source command requires a **Public operation key**; exact replay returns the original result without repeating side effects.
- Reusing a **Public operation key** with different canonical inputs is invalid.
- Durable mutations store replay results transactionally, while lookup, cancellation, manual preview, and browser handoff retain only bounded session replay records.
- Each preview token additionally binds its **Lookup session**, source key, source digest, and preview digest.
- Identity change, session supersession, token mismatch, or digest mismatch makes the preview stale and prevents import.
- A **Public result lookup** requires an explicit action and consent for each contacted provider.
- Census lookup, official MTGO browser handoff, and MTGTop8 browser handoff each require independent **Outbound consent**.
- Census **Outbound consent** permits only approved source-scope parameters; the lookup nickname remains local and is never transmitted.
- Official MTGO and MTGTop8 **Outbound consent** each permits the exact nickname only in a host-built browser URL.
- Manual official-source preview and import require no **Outbound consent** because they perform no network access.
- A provider, disclosure-version, or outbound-field mismatch is treated as absent **Outbound consent**.
- Revoking **Outbound consent** immediately cancels matching in-flight access, rejects late responses, and blocks future lookup, refresh, or browser handoff for that destination.
- Consent revocation preserves existing imports and leaves an already displayed **Evidence preview** locally importable until its normal expiry.
- A **Public result lookup** previews candidate **Public player results** before any are imported.
- Import confirmation freezes the exact **Evidence preview** and its **Preview digest**.
- Every **Evidence preview** identifies its schema and result kind, provenance and provider, public attribution URL, immutable lookup and source nicknames, exact match rule, typed source scope, observation time, typed payload, **Source digest**, and **Preview digest**.
- An MOCS **Evidence source key** combines provider, catalog ID, start date, as-of date, and case-folded source nickname.
- An official published-decklist **Evidence source key** combines provider, canonical official artifact URL, and case-folded source nickname.
- Official published-decklist evidence retains the exact player-entered attribution URL separately from the trusted-host-derived canonical artifact URL used in its **Evidence source key**.
- A **Source digest** identifies public-source content and never substitutes for the **Evidence source key**.
- The same **Evidence source key** and **Source digest** is a duplicate and creates no additional evidence snapshot.
- The same **Evidence source key** with a different **Source digest** is changed evidence that returns to preview as a new immutable version linked to the previous import.
- Equal **Source digests** under different **Evidence source keys** remain distinct source statements and are never merged automatically.
- A later lookup under a changed **Player identity** nickname uses the new nickname without alias-linking or merging prior evidence.
- Finding an existing **Evidence source key** and **Source digest** shows the result as already imported and preserves its original lookup provenance.
- Only an **Exact nickname match** may be presented as a candidate **Public player result**.
- A candidate is never linked automatically when its source does not clearly identify the nickname.
- The player may turn a selected preview candidate into an **Imported public result**.
- V1 permits an **Official-source manual import** but does not parse official MTGO pages.
- Third-party sources are limited to **External source handoffs** for corroboration and never become V1 import sources.
- V1 **External source handoffs** are limited to official MTGO Decklists and optional MTGTop8 corroboration.
- An **Imported public result** is durable and read-only, while personal notes remain separate and editable.
- Shared source and lookup provenance lives in **Public result evidence**, while leaderboard and published-decklist facts remain distinct typed payloads.
- Import adds only local identity, import time, and the selected-field manifest to the frozen **Evidence preview**.
- Durable evidence retains mandatory provenance and the current **Evidence selection**, while unselected preview values are discarded after confirmation.
- A later change in retained fields appends an **Evidence selection** revision to the same source evidence rather than creating a new source snapshot.
- `provider_observed` evidence comes from a validated Census response seen by the trusted host.
- An MOCS leaderboard evidence scope includes its reviewed label, catalog ID, start date, and as-of date; its payload includes total points, Top 8 finishes, and best score.
- Missing or malformed required MOCS scope or payload fields produce a **Degraded public lookup**, never a partial **Evidence preview**.
- `user_attested_official_source` evidence records player-entered facts attributed to an exact official MTGO URL that the companion did not fetch or parse.
- Official published-decklist evidence requires event title, event date, and format; placement and record are optional player-entered facts.
- Official published-decklist contents are either `reference_only` with no stored cards or `complete` after format-aware validation; partial card lists never become an **Evidence preview**.
- Only `complete` official published-decklist evidence may enter deck classification.
- A **Result refresh** deduplicates by a provider's stable identifier or canonical source URL.
- A **Result refresh** never silently overwrites an **Imported public result**.
- An **Empty lookup result** records the provider and lookup time and remains distinct from provider failure or unavailability.
- An **Empty lookup result** never claims that the **Player identity** has no public history.
- An **Empty lookup result** immutably records its local and Player identity IDs, provider, exact lookup nickname, exact-match rule, typed Census scope, provider-configuration version, completion time, and lookup operation key.
- Repeating the same empty-result command with its lookup operation key is idempotent.
- A later positive result coexists with the earlier time-scoped **Empty lookup result**; the newest outcome is shown by default.
- A **Degraded public lookup** never contacts or opens another provider automatically.
- A **Degraded public lookup** leaves existing imports unchanged and keeps the local Player workflow usable.
- V1 keeps no general lookup-history log.
- Imported **Public result evidence** carries its own frozen lookup metadata, while a valid scoped no-match persists only as an **Empty lookup result**.
- Successful unimported previews and degraded attempts are discarded when their lookup session ends.
- The **Player workspace** owns the identity and public-result workflow; Settings contains only configuration.
- The **Player workspace** uses a compact responsive two-column layout: Player identity and public-source controls occupy the narrow column, while lookup, candidate review, and saved evidence occupy the primary column; the columns stack at constrained widths.
- **Outbound consent** status stays visible beside each public source, while disclosure and grant or revocation controls expand inline only when consent is absent or the player explicitly reviews it; consent never requires a modal or Settings detour.
- An **Evidence preview** candidate appears in one exact-match review list with a result-level selection control and an inline **Evidence selection** disclosure; mandatory source identity and attribution cannot be deselected.
- The Player workspace exposes one import primary action in a stable selection-summary bar and disables it when no candidate is selected.
- An **Empty lookup result** and a **Degraded public lookup** render as distinct inline outcomes above saved evidence, which remains visible and unchanged in both states.
- The **Player identity** nickname is edited inline with an explicit historical-provenance warning; saving is local-only, starts no lookup, and creates no alias or evidence relinking.
- **Result refresh** is an explicit secondary action beside saved evidence; it reuses the current nickname and consent, previews only new or changed evidence, identifies already-imported evidence, and shares lookup loading, cancellation, empty, and failure states.
- A **Public player result** must retain its public source and must not imply access to private MTGO account data.

## Example dialogue

> **Dev:** "Does entering a nickname create an **Opponent profile**?"
> **Domain expert:** "No. It creates the local **Player identity**; any **Public player results** are separately discovered and source-attributed."

## Flagged ambiguities

- "Profile" previously referred only to an opponent record; the app owner is now the distinct **Player identity**.
- Multiple saved MTGO identities are outside V1; each local notebook represents at most one player.
- "Information from a nickname" means verifiable public results, not private account, collection, rating, or assumed complete match-history data.
- Saving a nickname is local-only; it does not imply permission to contact a public provider.
- Including Daybreak Census in the V1 strategy does not mean it is enabled; unresolved authorization or safety gates keep the **Conditional public provider** unavailable.
- "Implementing Census" does not mean enabling Census: an adapter may exist behind a fail-closed disabled state, but missing, expired, or unreviewed enablement evidence must cause zero Census requests.
- Renderer-supplied nicknames, Census scopes, canonical URLs, consent state, and evidence provenance are never authoritative.
- Nickname matching ignores letter case but never uses partial, fuzzy, or inferred identity matching.
- Historical evidence and empty outcomes retain the exact lookup nickname used when they were created.
- Importing public information preserves a read-only snapshot; it does not make the source data editable or merge it into personal notes.
- The existing encounter-bound public deck snapshot is not the **Public result evidence** model for the Player workspace.
- Existing PRD and TechSpec statements about encounter-triggered opponent public-deck enrichment do not authorize automatic Player lookups; the Player Identity implementation handoff is authoritative within the Player-owned public-result workflow, while opponent behavior remains unchanged unless separately redesigned.
- A generic "verified" label must not collapse the distinct **Evidence provenance modes**.
- Import never refetches or silently renormalizes an approved **Evidence preview**.
- The full **Preview digest** remains auditable even though unselected payload values are not retained.
- Public attribution never persists credentials or a Daybreak Service ID.
- An official URL that cannot be safely validated and canonicalized without guessing produces no **Evidence preview**, and its displayed attribution is never silently rewritten.
- A third-party page may help locate an official MTGO artifact, but only the exact official URL and user-entered facts may form an **Official-source manual import**.
- Researching Decklist Data, MTGDecks, MTGGoldfish, UrzaTools, and AetherHub did not place them in the V1 source catalog.
- Public data is refreshed only by explicit action; new or changed candidates return to preview before import.
- A provider returning no verified candidates is not evidence that no public results exist elsewhere.
- Only a valid provider response with no **Exact nickname match** may produce an **Empty lookup result**; disabled, unavailable, throttled, malformed, or misconfigured providers produce a **Degraded public lookup**.
- Lookup-session state is not durable evidence unless the player imports it or it becomes a scoped **Empty lookup result**.
- Portability of Player-owned records does not imply portability of provider authorization or operational state; restored provider access always remains fail-closed until separately authorized on the destination installation.
- A merge restore cannot silently choose between, combine, or partially import two different Player identities.
- Plaintext export remains an explicitly acknowledged, unencrypted, one-way format and excludes consent, provider configuration, operational state, secrets, and unimported previews.
- Individual and whole-identity Player deletion require a scoped destructive preview and explicit confirmation; neither silently expands into opponent-notebook deletion.
- Identity entry, consented lookup, result preview, import, and refresh belong in the top-level **Player workspace**.
- Provider consent, lookup progress, candidate selection, scoped absence, provider failure, identity changes, and import completion must each have a visible keyboard-accessible state in the **Player workspace** without hiding saved evidence behind a modal.
- "Implementation-ready" means the **Implementation handoff** is coherent enough to plan; local automation, packaged Windows evidence, and any live-provider enablement remain separate completion gates.
