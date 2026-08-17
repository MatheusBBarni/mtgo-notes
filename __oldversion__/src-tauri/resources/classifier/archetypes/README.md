# Curated Archetype Cleaner definitions

`modern.yaml`, `pauper.yaml`, `pioneer.yaml`, and `standard.yaml` follow the
original Archetype Cleaner contract:

```yaml
format: Modern
date: "2026-08-03"
archetypes:
  - name: Example
    signatureCards:
      - name: Signature Card
        minCopies: 2
    strictMode: true
```

These are reviewed source definitions, not runtime classifier assets. The
application only loads the immutable signed JSON bundle one directory above
this one.

The curated set captures the current MTGDecks.net taxonomy for Modern, Pauper,
Pioneer, and Standard:

- top 20 archetypes from the site's `Last 60 days` metagame view;
- the generic `Rogue` bucket excluded because it is not a stable archetype label;
- source URLs, displayed metagame shares, and source update timestamps retained
  as YAML comments rather than non-contract fields;
- every Pauper, Pioneer, and resolved Standard definition compared two distinct
  rendered main decks, with both reviewed deck URLs retained beside the rule;
- Pauper, Pioneer, and resolved Standard signature cards appear in both
  reviewed decks, and each `minCopies` threshold is no greater than the smaller
  observed quantity;
- unresolved Standard labels remain explicit in `standard.yaml` instead of
  receiving inferred signature cards.

Runtime integration is a separate release step. It requires canonical oracle
IDs, corpus decks with redistribution provenance, golden vectors, a recomputed
corpus digest, and a valid publisher signature.
