# Workflow Memory

Keep only durable, cross-task context here. Do not duplicate facts that are obvious from the repository, PRD documents, or git history.

## Current State

## Shared Decisions

- Renderer replacement-event revisions are a host-wide monotonic ordering stream, independent from entity revisions used for persistence concurrency.
- Provider interruption or selected-window generation changes must persist and publish restricted disclosure before any status that could otherwise leave stale historical data visible.
- Notebook schema version 1 is checksum-stable. Retired tendency tags arrive through the additive version-2 migration; future schema changes must add migrations rather than edit `INITIAL_SCHEMA` in place.

## Shared Learnings

- Local deterministic provider fixtures and host tests do not satisfy packaged-Windows UIA, OCR, focus, accessibility, performance, or end-to-end evidence gates.

## Open Risks

- The current Task 03 detector has static Windows window/UIA validation but no live UIA event subscription or Windows Graphics Capture/OCR implementation; Task 03 remains pending until those paths and their packaged evidence exist.

## Handoffs
