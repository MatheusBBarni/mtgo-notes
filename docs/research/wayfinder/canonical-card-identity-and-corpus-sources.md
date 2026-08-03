# Canonical Card Identity and Redistributable Corpus Sources

**Decision record for [issue #10](https://github.com/MatheusBBarni/mtgo-notes/issues/10)**  
**Researched:** 2026-08-03  
**Scope:** source admission for the signed, offline classifier. This is an engineering and provenance recommendation, not legal advice.

## Resolution

Use **Scryfall's `oracle_id`** as the classifier's semantic card key. It is the only reviewed identifier whose documentation explicitly says it is stable across reprinted editions and disambiguates same-name cards; Scryfall's `id` identifies a particular database card/printing instead. For `reversible_card`, resolve the oracle ID from the relevant face rather than assuming a top-level value. [Scryfall Card Objects](https://scryfall.com/docs/api/cards)

Resolve and validate those identifiers only in **release/build tooling**, record a frozen source snapshot, and emit the smallest possible generated classifier vectors. Do not treat card names, prints, MTGO IDs, images, Oracle text, prices, or a complete card database as classifier data. The current placeholder values such as `oracle-murktide-regent` must therefore be replaced by real UUID-shaped `oracle_id` values before an asset can be admitted.

There is **no currently reviewed public source that is cleared to bundle a reproducible, labeled competitive corpus for Modern, Pauper, Pioneer, and Standard**. The release bundle may contain only project-authored/deck-owner-authorized records or records covered by an explicit written data license that permits redistribution and derivative classifier use. MTGDecks, MTGGoldfish, Melee, MTGO, and MTGTop8 remain discovery, reference, or permission-request sources—not corpus inputs.

This resolves the source question with a fail-closed policy: a format with no qualifying corpus stays unsupported/`Unclassified`; it does not inherit scraped decks, a public taxonomy label, or a lower-quality source.

## Card-identity source decision

| Source | What it establishes | Freshness and access | License / attribution constraint | Admission decision |
| --- | --- | --- | --- | --- |
| [Scryfall Card Objects](https://scryfall.com/docs/api/cards) | `oracle_id` is the cross-print semantic key; `id` is an individual Scryfall card object. `reversible_card` IDs live on faces. | [Bulk metadata](https://api.scryfall.com/bulk-data/oracle-cards) exposes an `updated_at` and timestamped `jsonl_download_uri`. [Bulk files](https://scryfall.com/docs/api/bulk-data) are daily gzipped JSONL, collected every 12–24 hours; weekly/post-release acquisition is normally sufficient for gameplay data. | [Scryfall's API rules](https://scryfall.com/docs/api/) permit Magic software/research/community use under the Wizards policy, but forbid paywalling and simply repackaging, republishing, or proxying Scryfall data. Do not imply endorsement or use card imagery in the bundle. | **Release tooling only.** Fetch `oracle_cards` to resolve IDs; record the metadata object and input digest. A signed classifier may contain only the opaque IDs necessary in project-authored vectors, never Scryfall bulk files or a standalone card database. |
| [MTGJSON identifiers](https://mtgjson.com/data-models/identifiers/) | Provides `scryfallOracleId`, plus print-oriented `mtgoId`, `scryfallId`, and `multiverseId`. Its docs confirm the Oracle field has the intended cross-reprint semantics. | [AllIdentifiers](https://mtgjson.com/downloads/all-files/) is downloadable; builds are daily, and [Meta.json](https://mtgjson.com/api/v5/Meta.json) plus adjacent `.sha256` files provide a version/date and integrity input. | [MTGJSON licenses its website/content under MIT](https://mtgjson.com/license/), requiring the notice to accompany substantial copies. It also documents that it aggregates multiple upstreams, including Gatherer and Scryfall, so preserve upstream provenance and do not infer broader rights than the published source terms. | **Preferred reproducible resolver input, conditionally bundleable only as a pruned mapping.** Carry the MIT notice and source lock; do not ship its raw downloads, images, text, prices, or unrelated identifiers. |
| [MTGJSON All Decks](https://mtgjson.com/downloads/all-decks/) | Describes formally released product decks, with deck name/type/release date and sealed-product linkage. | Downloadable, but not a tournament corpus. | Same MTGJSON notice applies. | **Not a Modern/Pauper/Pioneer/Standard competitive corpus.** It lacks the reviewed archetype labels and coverage the classifier needs. |

### Acquisition rules

1. A compiler reads a pinned Scryfall bulk metadata object (or MTGJSON version/date/checksum), streams the download, and writes a source lock containing URL, retrieval time, upstream `updated_at`/version, and SHA-256 of the downloaded input and generated resolver.
2. It validates every source card name against exactly one `oracle_id`; rejects ambiguous, absent, and malformed values. Store the canonical source namespace (for example, `scryfall.oracle_id`) alongside the compiler version so an identifier-system migration cannot silently change vectors.
3. Scryfall requires accurate `User-Agent` and `Accept` headers. Its documented limits are 2 requests/second for search/named/random/collection, 10/minute for `/cards/manifest`, and 10/second for other API methods. Cache data for at least 24 hours, respect every `429` (the stated initial restriction is 30 seconds), and use bulk files rather than N card requests. [Scryfall rate limits](https://scryfall.com/docs/api/rate-limits)
4. Never perform this resolution from the desktop app. It belongs to an authenticated, reproducible release build; retain the source lock and generated resolver so the signed result remains traceable and repeatable.

## Competitive-deck corpus source decision

| Candidate | Coverage / utility | Why it cannot enter the bundle now | Permitted use |
| --- | --- | --- | --- |
| [MTGDecks](https://mtgdecks.net/) | Current format pages/archetype taxonomy and individual decklists cover the target formats. | Its [terms](https://mtgdecks.net/pages/terms) limit content to personal, non-commercial use and prohibit reproduction, publication, distribution, modification, and derivatives. A contributor's grant to MTGDecks is not a grant to this project. | Reference URLs and manually reviewed taxonomy evidence only; no scraping, caching raw lists, or copied labels/decks. |
| [MTGGoldfish](https://www.mtggoldfish.com/) | Broad multi-format deck archive and historical discovery. | Its [terms](https://www.mtggoldfish.com/policies/terms-of-use) impose the same personal-use restriction and prohibit reproduction/distribution/derivatives. No public bulk/API or usable published request budget was found. | Reference only; request a separate written data license before using any list or label. |
| [Melee](https://melee.gg/) | Potentially current organizer tournament data; labels and public exposure are organizer-controlled. | [API access](https://help.melee.gg/docs/api-use/) is limited to organizations and designated/cleared staff because it can expose PII. It is not a public-harvesting API or a redistribution grant. | Only under an organization-specific written agreement that scopes fields, retention, license, and privacy; never as an anonymous public corpus. |
| [Official MTGO decklists](https://www.mtgo.com/decklists) | First-party event decklist reference for the app's domain, but it does not provide a durable archetype label. | No documented machine endpoint, rate limit, or redistributable field set has been established. Wizards' February 2026 [decklist policy reversal](https://www.mtgo.com/news/reversing-decklist-changes-02202026) confirms that the publication path is mutable. | Retain the existing `interactive_required`/official-browser flow. Do not scrape, bundle, or enable automatic access without written permission and a new access spike. |
| MTGTop8 or a public dataset mirrored from any of the above | Useful for discovery only. | Visibility/downloadability is not a license. A mirror's license cannot grant upstream decklist rights, labels, player data, or derivative-use authority. | Admit only after tracing every record to an explicit redistributable grant. |

The Wizards [Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy) reinforces the conservative result: public fan content must be free, clearly unofficial, respect third-party IP, and must not verbatim repost Wizards material. It is not a blanket grant to redistribute third-party deck aggregators' content. Any public artifact that relies on Wizards IP needs the policy's required unofficial/ownership notice and a fresh legal check at release time.

## What may ship

**Conditionally bundleable after its source lock is reviewed:**

- Project-authored stable archetype IDs, display names, constraints, and thresholds.
- A minimal oracle-ID resolver snapshot derived through the documented identity pipeline, with the applicable MTGJSON/Scryfall/Wizards notices and no standalone card-data export.
- Complete deck vectors that are either created by the project or submitted under a recorded contributor grant, plus the project-reviewed archetype label. A grant must explicitly permit perpetual redistribution, modification, signing, and use for local classifier training; it must cover every submitted deck and exclude player-identifying fields unless separately required.

**Never bundle from the reviewed public sources without an additional written grant:** raw decklists, source-created archetype labels, names/handles, event records, HTML/JSON responses, card images/art, full Oracle text, prices, or a full Scryfall/MTGJSON export.

**Release tooling/reference only:** Scryfall and MTGJSON bulk inputs; MTGDecks/MTGGoldfish/MTGO/MTGTop8 discovery pages; Melee only with authorized credentials. Treat all provider labels as evidence for a human review, not as canonical runtime IDs.

## Required provenance record before corpus admission

Each input deck must have a signed/locked record containing at least:

```text
corpus_id, format, reviewed_archetype_id, source_label, source_kind,
source_url_or_contributor_record, source_published_at, retrieved_at,
license_or_grant_id, license_url_or_document_digest, raw_deck_digest,
identity_source, identity_snapshot_version_or_updated_at, resolver_digest,
normalizer_version, reviewer, review_date
```

The compiler must reject any missing license/grant, format mismatch, duplicate raw digest, unresolved card, or label outside the reviewed definitions. Preserve `source_label` separately from `reviewed_archetype_id`; neither an aggregation site's taxonomy nor an organizer's editable label is an identity contract. This fills the source-manifest requirement in [ADR-007](../../../.compozy/tasks/mtgo-opponent-notes/adrs/adr-007.md) without weakening its signed, immutable asset boundary.

## Downstream implications

- Ticket #13 should define a UUID syntax/source check and the reversible-card resolution rule; the current loader accepts any nonblank `oracle_id` string.
- Ticket #15 should define corpus coverage/balance and contributor/license acceptance thresholds. Until it does, no synthetic or scraped record may fill an incomplete format.
- The current official provider posture remains correct: `OFFICIAL_ACCESS_SPIKE.md` says automated lookup is disabled until permission, stable semantics, limits, and redistributable fields are demonstrated.

## Primary evidence

1. [Scryfall Card Objects](https://scryfall.com/docs/api/cards), [bulk-data documentation](https://scryfall.com/docs/api/bulk-data), [bulk metadata endpoint](https://api.scryfall.com/bulk-data/oracle-cards), [API rules](https://scryfall.com/docs/api/), and [rate limits](https://scryfall.com/docs/api/rate-limits).
2. [MTGJSON identifiers](https://mtgjson.com/data-models/identifiers/), [downloads](https://mtgjson.com/downloads/all-files/), [FAQ and integrity files](https://mtgjson.com/faq/), and [license](https://mtgjson.com/license/).
3. [MTGDecks terms](https://mtgdecks.net/pages/terms), [MTGGoldfish terms](https://www.mtggoldfish.com/policies/terms-of-use), [Melee API policy](https://help.melee.gg/docs/api-use/), [MTGO decklist publication update](https://www.mtgo.com/news/reversing-decklist-changes-02202026), and the [Wizards Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy).
