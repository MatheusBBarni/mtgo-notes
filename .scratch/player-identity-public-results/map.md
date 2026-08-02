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

## Not yet specified

- The implementation breakdown and final verification matrix remain fog until the provider, evidence model, security contract, and Player workspace behavior are resolved.

## Out of scope

- MTGO login, session handling, account impersonation, or access to private account data.
- Collection access, rating claims, or an assumed complete personal match history.
- Authenticated scraping, bypassing access controls, or relying on an undocumented endpoint without permission.
- Background lookup or synchronization.
- Multiple active Player identities in one V1 notebook.
- Application implementation and delivery; this map ends at an approved planning handoff.
