# Prototype the Player Workspace

Type: prototype
Status: closed
Blocked by: 03, 04, 05
GitHub: https://github.com/MatheusBBarni/mtgo-notes/issues/7

## Question

What is the smallest accessible Player-tab interaction model that makes local save, provider consent, explicit lookup, candidate review, selective import, refresh, absence, failure, and nickname-change behavior understandable without crowding the existing tabbed workspace?

## Comments

Resolution: Approved a compact responsive two-column Player tab. Player identity and source controls stay in the narrow column; lookup, candidate review, and saved evidence use the primary column. Consent and nickname editing expand inline, exact-match candidates support result and field selection with mandatory provenance, refresh is explicit and non-destructive, and scoped absence remains distinct from provider failure while saved evidence stays visible.

Prototype asset: https://github.com/MatheusBBarni/mtgo-notes/blob/6e39dcc8affe913c8dd1c15d43e0d89b8f433428/.scratch/player-identity-public-results/prototypes/player-workspace/index.html
