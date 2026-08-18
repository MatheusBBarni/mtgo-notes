# Signed Runtime Archetype Conversion

Label: wayfinder:map
GitHub: https://github.com/MatheusBBarni/mtgo-notes/issues/9

## Destination

Reach an evidence-backed, implementation-ready decision set for converting the reviewed Modern, Pauper, Pioneer, and Standard YAML archetype definitions into a reproducible, versioned, Ed25519-signed runtime classifier bundle that the app validates and uses without weakening its fail-closed security boundary.

## Notes

- Domain: private, local-first MTGO companion with an immutable signature-first classifier and local k-NN fallback.
- Planning only: implementation and delivery begin after this map is resolved.
- Consult `AGENTS.md`, `src-tauri/resources/classifier/archetypes/README.md`, `src-tauri/src/classifier/mod.rs`, the classifier tests, `.compozy/tasks/mtgo-opponent-notes/_techspec.md`, and ADR-007 before resolving tickets.
- Use `wayfinder` for map operations. Use `curate-mtg-archetypes` only when source definitions need evidence-backed remediation.
- Preserve deterministic ordering, stable identifiers, explicit provenance, `Unclassified` behavior, strict-mode exclusion from k-NN, startup signature validation, append-only classifier runs, and fail-closed activation.
- Current facts: the app embeds `manifest.json`, `definitions.json`, `corpus.json`, and `golden_vectors.json`; the YAML files are not loaded directly; Modern has unconstrained signatures rejected by the current YAML validator; Standard records one unresolved archetype.

## Decisions so far

- [Research Canonical Card Identity and Redistributable Corpus Sources](https://github.com/MatheusBBarni/mtgo-notes/issues/10) — Use frozen Scryfall Oracle IDs; no reviewed public competitive corpus is cleared for bundling without an explicit redistribution grant.

## Not yet specified

- The executable implementation breakdown and final acceptance matrix remain fog until source admission, identity mapping, compiler, corpus, signing, and lifecycle decisions are resolved.
- Whether conversion should become a release-only tool or a contributor-facing repeatable workflow depends on the compiler and key-custody decisions.

## Out of scope

- Replacing the local classifier with a remote classification service.
- Adding a YAML editor, user-imported classifiers, or archetype configuration UI.
- Weakening or bypassing Ed25519 verification, corpus-digest checks, golden vectors, or fail-closed activation.
- Refreshing the approved MTGDecks taxonomy unless a readiness decision explicitly sends a definition back for remediation.
- Implementing, signing, committing, or releasing the converted bundle; this map ends at an approved execution handoff.
