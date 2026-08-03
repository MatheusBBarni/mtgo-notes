## Resolution

Approved a compact responsive two-column Player tab:

- The narrow column owns the local Player identity and visible source-specific consent status.
- The primary column owns explicit lookup, candidate review, selective import, refresh, and saved evidence.
- Identity and consent controls expand inline; neither requires a modal or a Settings detour.
- Exact-match candidates use result-level selection plus inline retained-field selection. Source identity and attribution are mandatory.
- One stable selection bar owns the `Import selected results` primary action and disables it when nothing is selected.
- Scoped absence and provider failure remain distinct inline outcomes above saved evidence. Neither changes imports or triggers another provider.
- Nickname changes are local-only, start no lookup, preserve historical lookup nicknames, and create no alias or evidence relinking.
- Refresh is explicit and non-destructive. It previews only new or changed source statements and identifies already-imported evidence.

### Prototype asset

- [Interactive Player workspace prototype](https://github.com/MatheusBBarni/mtgo-notes/blob/6e39dcc8affe913c8dd1c15d43e0d89b8f433428/.scratch/player-identity-public-results/prototypes/player-workspace/index.html)
- [Design anchor, state matrix, and approval log](https://github.com/MatheusBBarni/mtgo-notes/blob/6e39dcc8affe913c8dd1c15d43e0d89b8f433428/.scratch/player-identity-public-results/prototypes/player-workspace/README.md)

### Prototype verification

- Nine domain states exercised: first use, consent, ready, loading, candidates, empty, failure, nickname change, and imported.
- Automated axe/jsdom pass: zero violations and zero runtime errors across all nine states; color contrast remains a required rendered-pixel verification during implementation.
- Prototype formatting and repository `git diff --check` passed.

This resolves the Player-workspace behavior decision. Application implementation remains outside this planning map.
