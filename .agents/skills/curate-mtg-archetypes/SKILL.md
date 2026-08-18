---
name: curate-mtg-archetypes
description: "Research a user-supplied Magic: The Gathering format on MTGDecks, inspect representative main decks, and create or refresh an evidence-backed Archetype Cleaner YAML draft. Use when the user invokes `curate-mtg-archetypes` with a format parameter, or asks to fetch, curate, refresh, or draft archetype signature cards for a format such as Standard, Pioneer, Modern, or Pauper."
---

# Curate MTG Archetypes

Create a review-only YAML archetype draft for one MTG format. Preserve source
evidence and refuse to infer signature cards when deck evidence is incomplete.

## Required parameter

Read the required `format` parameter from the invocation:

```text
$curate-mtg-archetypes format=Standard
```

Also accept a single positional format for convenience:

```text
$curate-mtg-archetypes Pauper
```

If the format is missing, ask only for it. Preserve the display spelling from
MTGDecks and derive the output filename as lowercase kebab-case, for example
`src-tauri/resources/classifier/drafts/pioneer.yaml`.

## Workflow

1. Inspect the repository guidance and existing files under
   `src-tauri/resources/classifier/drafts/`. Preserve unrelated dirty state.
2. Read and use the available Computer Use skill. Open
   `https://mtgdecks.net/<Format>` and capture:
   - the selected `Last 60 days` window;
   - the displayed site update timestamp;
   - the top 20 archetype names, slugs, and metagame shares.
3. Exclude the generic `Rogue` bucket. Do not exclude a named archetype merely
   because its share is small.
4. Re-open every retained archetype page. Inspect at least two distinct,
   rendered main decks. Prefer inspecting up to four recent candidates and
   select a representative pair with the strongest shared nonland core.
5. Read only main-deck sections such as Creature, Instant, Sorcery,
   Enchantment, Artifact, Land, Planeswalker, and Battle. Exclude Sideboard.
6. Choose two to four distinctive signature cards when the evidence supports
   them. Every selected card must appear in both reviewed main decks.
   `minCopies` must not exceed the lower observed quantity.
7. Avoid basic lands and generic format staples unless no cleaner signature is
   available. Distinguish neighboring archetypes with AND combinations or
   evidence-backed copy thresholds.
8. If two clean deck pages cannot be inspected, omit that archetype and record
   it as unresolved in the YAML header. Never substitute Card Stats, aggregate
   popularity, the archetype name, or a remembered list for two deck pages.
9. Create or update the YAML draft and the drafts README. Leave both
   uncommitted for review.

Use the dedicated browser skill only when Computer Use cannot reliably inspect
the rendered deck tables. Before browser work, follow that skill's connector
check, setup, evidence, and tab-finalization rules.

## YAML contract

Keep provenance in comments so the runtime schema remains exact:

```yaml
# Draft only. Not loaded by the runtime classifier.
# Source: https://mtgdecks.net/Standard
# Window: Last 60 days
# Site last updated: 2026-08-03 18:00:29
# Captured with Computer Use.
# The generic Rogue bucket (5.49%) is intentionally excluded.

format: Standard
date: "2026-08-03"
archetypes:
  # MTGDecks share: 16.49%
  # Source: https://mtgdecks.net/Standard/izzet-prowess
  # Reviewed deck 1: https://mtgdecks.net/Standard/example-deck-1
  # Reviewed deck 2: https://mtgdecks.net/Standard/example-deck-2
  - name: Izzet Prowess
    signatureCards:
      - name: Slickshot Show-Off
        minCopies: 4
```

Allowed contract fields are:

- top level: `format`, `date`, `archetypes`;
- archetype: `name`, `signatureCards`, optional `strictMode`;
- signature card: `name`, `minCopies`, optional `exactCopies`.

Do not add provenance objects, source URLs, shares, notes, or unresolved items
as YAML fields. Keep them in comments.

## Safety boundary

- Treat the result as a provisional research artifact until the user approves
  it.
- Do not stage or commit unless explicitly requested after review.
- Do not modify or promote into signed runtime assets, including
  `manifest.json`, `definitions.json`, `corpus.json`, or
  `golden_vectors.json`.
- Do not claim all labels are resolved when any archetype lacks two clean deck
  pages.

## Validation

Resolve the script path relative to this `SKILL.md`, then run:

```text
ruby scripts/validate_draft.rb <draft.yaml> <Format> [expected-count]
```

Require a successful exit before presenting the draft. Report:

- resolved archetype count;
- signature constraint count;
- reviewed URL count and uniqueness;
- explicitly unresolved archetypes;
- confirmation that the work remains uncommitted and signed assets untouched.
