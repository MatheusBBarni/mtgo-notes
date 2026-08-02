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

**Exact nickname match**:
A public result candidate whose player nickname equals the saved MTGO nickname without regard to letter case.
_Avoid_: Partial match, fuzzy match, inferred identity

**Imported public result**:
A user-selected, read-only snapshot of a **Public player result** stored with its source attribution in the local notebook.
_Avoid_: Synced result, editable result, personal note

**Result refresh**:
An explicit **Public result lookup** that compares provider candidates with existing imports using stable source identity and previews only new or changed information.
_Avoid_: Background sync, silent overwrite

**Empty lookup result**:
An evidence-limited outcome stating that a named provider returned no verified public result at the recorded lookup time.
_Avoid_: Player not found, no public history

**Player workspace**:
The top-level Player tab where the person manages their **Player identity**, runs public lookups, reviews candidates, and inspects imported results.
_Avoid_: Settings, opponent profile

## Relationships

- A **Player identity** is distinct from every **Opponent profile**.
- A local notebook stores at most one **Player identity** in V1.
- A **Player identity** may reference zero or more **Public player results**.
- Saving a **Player identity** never starts a **Public result lookup**.
- A **Public result lookup** requires an explicit action and consent for each contacted provider.
- A **Public result lookup** previews candidate **Public player results** before any are imported.
- Only an **Exact nickname match** may be presented as a candidate **Public player result**.
- A candidate is never linked automatically when its source does not clearly identify the nickname.
- The player may turn a selected preview candidate into an **Imported public result**.
- An **Imported public result** is durable and read-only, while personal notes remain separate and editable.
- A **Result refresh** deduplicates by a provider's stable identifier or canonical source URL.
- A **Result refresh** never silently overwrites an **Imported public result**.
- An **Empty lookup result** records the provider and lookup time and remains distinct from provider failure or unavailability.
- An **Empty lookup result** never claims that the **Player identity** has no public history.
- The **Player workspace** owns the identity and public-result workflow; Settings contains only configuration.
- A **Public player result** must retain its public source and must not imply access to private MTGO account data.

## Example dialogue

> **Dev:** "Does entering a nickname create an **Opponent profile**?"
> **Domain expert:** "No. It creates the local **Player identity**; any **Public player results** are separately discovered and source-attributed."

## Flagged ambiguities

- "Profile" previously referred only to an opponent record; the app owner is now the distinct **Player identity**.
- Multiple saved MTGO identities are outside V1; each local notebook represents at most one player.
- "Information from a nickname" means verifiable public results, not private account, collection, rating, or assumed complete match-history data.
- Saving a nickname is local-only; it does not imply permission to contact a public provider.
- Nickname matching ignores letter case but never uses partial, fuzzy, or inferred identity matching.
- Importing public information preserves a read-only snapshot; it does not make the source data editable or merge it into personal notes.
- Public data is refreshed only by explicit action; new or changed candidates return to preview before import.
- A provider returning no verified candidates is not evidence that no public results exist elsewhere.
- Identity entry, consented lookup, result preview, import, and refresh belong in the top-level **Player workspace**.
