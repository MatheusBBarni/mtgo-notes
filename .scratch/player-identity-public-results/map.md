# Player Identity and Public Results

Label: wayfinder:map
GitHub: https://github.com/MatheusBBarni/mtgo-notes/issues/1

## Destination

Reach an evidence-backed, implementation-ready decision set for a Player workspace where one locally saved MTGO nickname can explicitly discover, preview, and import verifiable public tournament results or published decklists without becoming an MTGO account client.

## Notes

- Domain: private, local-first MTGO companion.
- Planning only: implementation and delivery begin after this map is resolved.
- Consult [`CONTEXT.md`](../../../CONTEXT.md), the existing Compozy PRD and TechSpec, and the trusted-host provider boundaries before resolving a ticket.
- Use `wayfinder` for map operations and `grill-with-docs` for human decisions.
- Use `ui-craft` and `ui-ux-pro-max` when prototyping the Player workspace.
- Standing decisions: one editable Player identity per notebook; saving is local-only; public lookup requires an explicit action and provider consent; matching is exact and case-insensitive; imports are selected, durable, read-only, and source-attributed; refresh is explicit and non-destructive; empty results are provider- and time-scoped; the workflow has a top-level Player tab; old imports retain the nickname used to find them.

## Decisions so far

- [Research Official MTGO Player Data](https://github.com/MatheusBBarni/mtgo-notes/issues/2) — Daybreak Census MOCS leaderboard data is the only gated machine-readable candidate; official decklists remain browser/manual.
- [Research Reputable Third-Party MTGO Player Data](https://github.com/MatheusBBarni/mtgo-notes/issues/3) — Surveyed third-party sources support only explicit browser corroboration, not permissioned automated V1 ingestion.
- [Choose the V1 Public Source Strategy](https://github.com/MatheusBBarni/mtgo-notes/issues/4) — Census is the sole gated adapter; official MTGO and MTGTop8 remain explicit handoffs, with official-URL manual imports and fail-closed degradation.
- [Define the Public Result Evidence Model](https://github.com/MatheusBBarni/mtgo-notes/issues/5) — Player-owned typed evidence separates provenance, source identity, content change, selective persistence, and scoped empty outcomes.
- [Define the Trusted-Host Lookup Contract](https://github.com/MatheusBBarni/mtgo-notes/issues/6) — Closed host-owned commands enforce independent consent, approved routes, session fencing, bounded execution, content-free audit, and typed failure.
- [Prototype the Player Workspace](https://github.com/MatheusBBarni/mtgo-notes/issues/7) — Approved a compact responsive two-column Player tab with inline consent and identity editing, selective preview import, explicit refresh, and distinct non-destructive empty and failure states.
- [Approve the Player Identity Implementation Handoff](https://github.com/MatheusBBarni/mtgo-notes/issues/8) — Approved the additive Compozy planning handoff, implementation breakdown, explicit acceptance boundaries, and separate local, packaged-Windows, and live-provider verification gates.

## Not yet specified

- None. The V1 planning handoff is approved; implementation and delivery remain outside this map.

## Out of scope

- MTGO login, session handling, account impersonation, or access to private account data.
- Collection access, rating claims, or an assumed complete personal match history.
- Authenticated scraping, bypassing access controls, or relying on an undocumented endpoint without permission.
- Background lookup or synchronization.
- Multiple active Player identities in one V1 notebook.
- Application implementation and delivery; this map ends at an approved planning handoff.
