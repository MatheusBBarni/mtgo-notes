# Product Requirements Document: MTGO Opponent Notes

## Overview

MTGO Opponent Notes is a private Windows companion for individual Magic: The Gathering Online players. It automatically identifies the current opponent and match phase through an approved or user-authorized source, presents relevant public and personal context at permitted times, and lets the player capture subjective observations without sacrificing meaningful match-clock time.

The product differs from match trackers and deck analytics tools by preserving what the player personally learns about an opponent: observed or suspected cards, deck identity, play tendencies, and contextual notes across repeat encounters. It supplements those observations with a confirmed, dated public deck snapshot for the current format while preserving source and uncertainty.

The canonical notebook remains on the player's device. Local capture, history, search, correction, backup, restore, and text export continue to work without external services. No cloud account, streaming workflow, or network telemetry is required.

## Goals

- Automatically surface a candidate opponent and match phase without requiring routine manual entry.
- Require player confirmation before detected identity or public deck data becomes persistent history.
- Show historical opponent context before a match, between games, and after completion while preventing historical disclosure during active gameplay.
- Let the player save a useful free-text observation through a keyboard-first flow without completing structured fields.
- Preserve dated, attributable opponent history that distinguishes public data, user-authored observations, observed cards, suspected cards, edits, and incomplete encounters.
- Classify complete decklists locally into explainable archetypes using versioned definitions shipped with the application.
- Keep the complete private notebook usable offline and during external-provider failures.
- Let the player correct identity mistakes through reversible merges and searchable aliases.
- Give the player durable local ownership through encrypted backup, safe restore, permanent deletion controls, and one-way human-readable text export.
- Make automatic integration replaceable and disableable rather than coupling the product to MTGBot, Twitch, or an undocumented MTGO access method.

## User Stories

- **US-001** — External-provider consent and controls.
- **US-002–US-003** — Automatic opponent confirmation and manual fallback.
- **US-004–US-005** — Match phases, completion, reopening, and incomplete encounters.
- **US-006–US-007** — Phase-scoped overlay and rapid capture.
- **US-008–US-009** — Structured observations and note lifecycle.
- **US-010–US-011** — Public deck snapshots and offline degradation.
- **US-012–US-013** — Historical recall, search, aliases, and reversible profile merges.
- **US-014–US-016** — Encrypted backup, safe restore, and text export.
- **US-017–US-018** — Retention, deletion, and private support diagnostics.
- **US-019** — Explainable local archetype classification.

[Full user stories](_user_stories.md)

## Core Features

### Automatic Opponent and Match Context

The companion obtains opponent identity and match-state candidates through at least one approved or user-authorized source. Automatic detection is required for V1 completion; manual entry is the runtime fallback for outages, uncertainty, unsupported contexts, and player correction.

The player confirms a detected opponent before the system creates or updates persistent history. Existing primary handles and aliases produce profile suggestions. Incorrect candidates can be corrected without persisting the wrong identity.

The same context boundary supports the phases pre-match, in-game, between-games, and finished. Automatic phases always have a visible manual correction path. Uncertainty applies the in-game disclosure boundary.

### Phase-Scoped Compact Overlay

The companion provides an optional compact always-on-top summary. A confirmed new opponent causes the summary to appear without taking input focus. The summary can expand for capture and can be hidden or disabled immediately.

Before the match and between games, the overlay can show the confirmed handle, the latest confirmed format-matched public deck snapshot, and private historical context. During active gameplay, it shows only opponent identity and observations from the current encounter. After completion, it exposes full history and editing.

### Rapid Observation Capture

A keyboard-first capture action opens with focus in the free-text field. Enter saves non-empty text; Escape dismisses capture without saving. Free text is the only required input.

Deck identity, card observations, contextual details, and tendency tags remain optional during capture and can be added afterward. A failed save preserves the player's text for retry or copying. Saved current-match notes appear immediately in the permitted in-game view.

### Structured and Attributable Observations

Every observation belongs to a dated encounter. User-authored deck identity remains distinct from public deck snapshots. Card entries distinguish **observed** from **suspected** and may contain contextual notes. Play tendencies use free-form observations and optional user-created tags rather than a product-defined behavioral taxonomy.

Completing an encounter adds its current-match observations to historical context automatically. Post-match review is optional. Edits retain the encounter timestamp and show an edited marker. Deletion provides a short undo opportunity before permanent removal.

### Public Deck Enrichment

At each confirmed encounter start, the companion requests the opponent's most recent public deck matching the current format. Before persistence, it shows the deck or archetype, event, publication date, provider, and source link for confirmation.

Each confirmed result becomes a separate dated snapshot. Refreshing never overwrites prior snapshots. Public data and personal deck observations remain separately labeled when they conflict; neither is declared the opponent's current deck.

Formats without provider coverage retain the complete manual note workflow. Missing, rejected, stale, rate-limited, or offline public data never blocks the encounter.

### Local Archetype Classification

The companion classifies complete confirmed public decklists and complete user-entered decklists locally. It first applies versioned, format-specific signature-card rules shipped with the application. If no signature rule matches, it uses a bundled local k-nearest-neighbors corpus. Results below the shipped confidence threshold remain **Unclassified**.

Every result records the classifier version, method, confidence, and matched signature cards or nearest-neighbor explanation. Partial card observations never trigger automatic classification. Archetype definitions and training data are app-owned, have no in-app editor, and change only through signed application releases.

When a release updates definitions or training data, the companion reclassifies stored complete decklists in the background. It appends a new classification run, retains prior runs for audit, and displays the newest completed result by default.

### Searchable Historical Recall

Outside active gameplay, the player can search primary handles and aliases and filter history by deck, observed card, suspected card, tendency tag, date, and note text.

Profiles show chronological encounters, source labels, edit markers, certainty, and incomplete status. "Last deck seen" identifies the source type, date, and format. Unconfirmed deck data from incomplete encounters is excluded from confirmed last-deck summaries.

### Identity Correction

The player can preview and merge duplicate or renamed opponent profiles. A merge requires selection of a primary handle, retains prior handles as aliases, and preserves every encounter's original timestamps and provenance.

Merges are reversible. If a merged profile receives new data before undo, the undo flow previews how that data will be reassigned rather than discarding it.

### Local Ownership, Recovery, and Export

The canonical notebook remains local and account-free. The user selected SQLite as the V1 persistence constraint for the TechSpec.

Encrypted backup includes the complete notebook. Backup creation requires a destination, passphrase, and explicit acknowledgement that the passphrase cannot be recovered. Restore validates and previews the backup before offering merge or replace. Both paths preserve the current notebook for rollback.

Merge restore skips exact duplicates, uses the reversible identity rules, and retains both versions of genuine conflicts for player resolution. Failed or interrupted restore never leaves a partially applied notebook.

Text export supports the complete notebook or one selected opponent. The `.txt` file is organized by opponent and encounter and includes timestamps, provenance, certainty, edit state, incomplete state, and public source attribution. The product warns that the file is unencrypted. Text export is not an import or restore format.

### Offline and Private Operation

Profiles, capture, history, search, editing, backup, restore, export, deletion, and support diagnostics remain available without connectivity. Automatic detection falls back to manual entry; public enrichment is omitted.

The product sends no network telemetry. A player may explicitly create a diagnostic bundle for support only after previewing it. The bundle excludes handles, aliases, note content, public lookup results, and source URLs.

## Business Rules

### Identity Rules

1. A persistent opponent profile must have one player-confirmed primary handle.
2. A profile may have multiple searchable aliases.
3. Automatic detection may suggest an identity but cannot persist it without confirmation.
4. Repeated confirmed detection of the same primary handle or alias must reuse the existing profile.
5. A merge must preserve all encounter timestamps, observations, public snapshots, source labels, and aliases.
6. A merge must be reversible and must preview how post-merge data will be reassigned.
7. Permanently deleted profiles and encounters cannot be silently recreated by late provider responses.

### Encounter Rules

1. The notebook has at most one active encounter at a time.
2. An encounter belongs to exactly one confirmed opponent profile.
3. The user-visible phases are **pre-match**, **in-game**, **between-games**, **finished**, and **incomplete**.
4. A confirmed new opponent automatically finishes the prior active encounter, starts the new encounter, and exposes undo or reopen.
5. When no confident end signal arrives, the companion prompts for confirmation. An ignored prompt produces an incomplete encounter.
6. Incomplete encounters remain resumable, finishable, and deletable.
7. Unconfirmed deck information from incomplete encounters cannot become confirmed "last deck seen" data.
8. Repeated lifecycle events must not create duplicate encounters or phase transitions.

### Disclosure Rules

| Phase | Opponent identity | Current-match observations | Private historical observations | Confirmed public deck history |
|---|---|---|---|---|
| Pre-match | Visible | Visible when present | Visible | Visible |
| In-game | Visible | Visible | Hidden | Hidden |
| Between-games | Visible | Visible | Visible | Visible |
| Finished | Visible | Visible and editable | Visible and editable | Visible |
| Incomplete or uncertain possible gameplay | Visible when confirmed | Visible | Hidden | Hidden |

1. Any uncertain phase that might represent active gameplay uses the in-game disclosure row.
2. Hiding or disabling the overlay must not end or delete the active encounter.
3. Historical content must not leak during gameplay through search, notifications, previews, shortcuts, or stale overlay rendering.
4. The product may describe this behavior as **Tournament-conservative** but cannot claim official safety, compliance, or approval.

### Capture and Observation Rules

1. Free text is the only required observation field.
2. Blank or whitespace-only observations cannot be saved.
3. The primary capture flow must let the player reach save through one shortcut, focused text entry, and Enter.
4. The player must be able to dismiss capture with Escape without creating data.
5. The user-facing capture interaction must complete in under five seconds under normal local operation.
6. Every saved observation belongs to the current encounter and records its encounter timestamp.
7. Card certainty is exactly **observed** or **suspected**.
8. Tendency tags are optional and user-created.
9. User-authored deck identity and external public deck data are different source classes.
10. Editing preserves the encounter timestamp and adds an edited marker.
11. Completing an encounter promotes its current observations automatically; review remains optional.

### Public Context Rules

1. V1 is not complete until automatic opponent and phase detection works through at least one approved or user-authorized source.
2. No integration may inspect MTGO process state, memory, files, logs, or network traffic without explicit Daybreak approval.
3. External lookup requires onboarding disclosure and consent before sending a confirmed opponent handle and format.
4. The player can disable each external provider at any time.
5. Public deck lookup runs at the beginning of every confirmed encounter when consent, connectivity, format coverage, and provider availability allow.
6. The primary result is the most recent public deck matching the current format.
7. A public result becomes persistent only after player confirmation.
8. A stored public snapshot includes provider, event, format, publication date, source link, and confirmation state.
9. Each confirmed refresh creates a new snapshot rather than overwriting prior history.
10. Public and personal deck records remain separately labeled when they conflict.
11. Provider failure or unsupported format falls back to manual context and cannot block local capture.

### Archetype Classification Rules

1. Automatic classification accepts only complete confirmed public decklists or complete user-entered decklists.
2. Partial observed or suspected cards cannot trigger automatic archetype classification.
3. Classifier assets are versioned, bundled with signed application releases, and immutable at runtime.
4. Format-specific signature rules run before the local k-nearest-neighbors fallback.
5. Signature rules support minimum-copy, exact-copy, and strict-match constraints.
6. A result below the shipped confidence threshold is **Unclassified**.
7. Each run records deck revision, classifier version, method, confidence, and explanation.
8. Provider-supplied labels and local classifier results retain separate provenance when they conflict.
9. Updating classifier assets appends reclassification runs for stored complete decklists and never deletes prior runs.
10. The newest successful classification run is the default display; failed or interrupted reclassification leaves the prior successful result active.

### Data Ownership and Privacy Rules

1. The canonical notebook is local to the player's device and requires no product account.
2. Notebook data remains indefinitely until the player deletes it.
3. Observation deletion provides a short undo opportunity.
4. Permanent deletion of an encounter, opponent, or the entire notebook requires explicit scope confirmation.
5. Permanently deleted content must not appear in active history, search, new backups, new exports, or diagnostic bundles.
6. The product sends no network telemetry.
7. Diagnostic bundles require explicit creation and preview and exclude all opponent identity, note, and public-result content.
8. External-provider consent does not authorize cloud synchronization, public sharing, or crowdsourcing.

### Backup, Restore, and Export Rules

1. Encrypted backup contains the complete current notebook.
2. Backup creation requires acknowledgement that a forgotten passphrase cannot be recovered.
3. An incomplete or interrupted backup cannot be presented as valid.
4. Restore validates and previews contents before any mutation.
5. Restore offers exactly **merge** or **replace**.
6. Restore preserves the current notebook for rollback before mutation.
7. Merge skips exact duplicates and preserves both versions of genuine conflicts.
8. Interrupted restore resolves to the complete prior notebook or the complete restored notebook, never a partial state.
9. Text export offers exactly **complete notebook** or **selected opponent** scope.
10. Text export is unencrypted and requires a warning before creation.
11. Text export is one-way and cannot be imported or used for restoration.

## User Experience

### Primary Journey

1. **First launch**
   - The player learns that the notebook is local and account-free.
   - The player sees what automatic context providers receive.
   - The player grants or declines consent and can change the choice later.
   - The player learns the Tournament-conservative disclosure behavior and the immediate overlay-disable control.

2. **Opponent detection**
   - A provider reports an opponent candidate.
   - The compact overlay appears without taking focus.
   - The player confirms an existing profile, corrects the candidate, or creates a new profile.
   - If automatic context is unavailable, manual entry suggests local profiles and aliases.

3. **Pre-match context**
   - The overlay shows the confirmed opponent, whether private history exists, and the latest confirmed format-matched public deck.
   - A complete decklist shows its newest local archetype result, classifier version, confidence, and explanation.
   - The player can expand the full permitted history.
   - A new public result is attributed and awaits confirmation before persistence.

4. **Active gameplay**
   - The overlay hides historical and public context.
   - The player sees only opponent identity and current-match observations.
   - A shortcut focuses free-text capture; Enter saves and Escape dismisses.
   - Optional deck, card-certainty, context, and tendency tags never block save.

5. **Between games**
   - Automatic phase change restores permitted historical and public context.
   - The player can review prior encounters and current-match notes.
   - Returning to gameplay hides historical context before capture continues.

6. **Completion**
   - A confident end signal finishes the encounter.
   - A confirmed new opponent also finishes the prior encounter and offers undo or reopen.
   - Current observations become dated history automatically.
   - An optional review action supports correction without blocking the next match.
   - Uncertain endings prompt for confirmation and otherwise remain incomplete.

7. **Later recall and maintenance**
   - The player searches by handle, alias, deck, card certainty, tendency, date, or note text.
   - The player edits observations, deletes data, resolves public/personal conflicts, and reversibly merges duplicate identities.
   - The player can create an encrypted backup, safely restore, or export readable text.

### UX Requirements

- Keyboard operation must cover opponent confirmation, rapid capture, save, dismiss, overlay hide, phase correction, and history navigation.
- Visible focus must always identify the active control.
- The overlay must not take gameplay focus automatically.
- Phase, certainty, source, incomplete state, and save failure cannot rely on color alone.
- Historical and current-match content must remain visually and semantically distinct.
- Public results must always show recency and provenance; "last deck seen" must not imply current deck certainty.
- Long backup, restore, export, deletion, and diagnostic operations must show progress and allow safe cancellation where cancellation cannot corrupt data.
- Errors must preserve recoverable player input and state the available fallback.
- Destructive actions must name the affected scope before confirmation.

## High-Level Technical Constraints

- The product is a Windows desktop companion.
- The user selected local SQLite persistence for V1; the TechSpec owns its schema and integration design.
- The TechSpec selects Tauri 2 with React/TypeScript webviews and a Rust host for Windows overlay behavior, focus control, packaging, accessibility, and maintainability.
- The compact overlay must support always-on-top behavior without requiring MTGO client modification.
- The primary capture path must complete in under five seconds under normal local operation.
- At least one automatic context provider must satisfy the approved or user-authorized boundary before V1 is complete.
- The product must remain provider-independent and must not assume MTGBot exposes a reusable API.
- External integration must not inspect MTGO process state, memory, files, logs, or network traffic without explicit Daybreak approval.
- Local notebook functions must work without network access.
- Archetype classification must run locally from immutable, release-bundled assets and must not require a network service.
- External deck lookup sends only the confirmed opponent handle and current format after consent.
- Stored public data must retain source attribution and a user confirmation state.
- Backup must be encrypted and restorable without partial application.
- Text export must be human-readable, unencrypted, one-way, and explicitly user initiated.
- Network telemetry is prohibited in V1.

## Non-Goals (Out of Scope)

- **Unofficial direct MTGO inspection** — Process hooks, memory inspection, file or log parsing, and traffic interception are excluded without explicit Daybreak approval.
- **Manual-only product completion** — Manual entry is a fallback; it does not satisfy the automatic-context V1 requirement.
- **Casual full-dossier gameplay mode** — Active gameplay always hides historical and public context.
- **Cloud accounts or synchronization** — The canonical notebook remains local and account-free.
- **Shared or crowdsourced opponent profiles** — V1 does not publish, exchange, or aggregate private observations.
- **Streaming integrations** — Twitch chat, OBS overlays, and viewer-facing output are excluded; MTGBot is an interaction reference only.
- **Runtime archetype configuration** — Players cannot edit, import, activate, or delete classifier definitions or training corpora inside V1.
- **General match analytics** — Win-rate dashboards, league records, opening-hand analysis, and broad deck tracking are excluded.
- **Predictive strategic advice** — The product does not infer hidden cards, predict an opponent's current deck, or recommend plays.
- **Fixed behavioral profiling** — The product does not assign predefined personality or behavior labels to opponents.
- **Text import** — Human-readable `.txt` exports cannot reconstruct or merge a notebook.
- **Network telemetry** — Usage analytics and automatic crash reporting are excluded.
- **Non-Windows clients** — Cross-platform desktop, mobile, and web applications are excluded from this PRD.

## Architecture Decision Records

- [ADR-001: Stage the Opponent-Memory V1 Around a Tournament-Conservative Core](adrs/adr-001.md) — Establishes the encounter ledger, conservative disclosure, and differentiated subjective-memory boundary.
- [ADR-002: Require Policy-Bounded Automatic Match Context](adrs/adr-002.md) — Requires automatic opponent, phase, and public-deck context through an approved or user-authorized source with manual fallback.
- [ADR-003: Keep the Notebook Local While Supporting Recovery and Text Export](adrs/adr-003.md) — Keeps canonical data local while adding encrypted recovery and one-way human-readable export.
- [ADR-004: Build the Windows Companion on Tauri 2](adrs/adr-004.md) — Selects Tauri, React, Rust-owned trust boundaries, system-tray lifecycle, and the Windows release baseline.
- [ADR-005: Detect Visible MTGO Context Through UI Automation and OCR](adrs/adr-005.md) — Selects user-authorized visible UI detection and the official MTGO public-deck boundary.
- [ADR-006: Encrypt the Live Notebook with SQLCipher and DPAPI](adrs/adr-006.md) — Defines at-rest encryption, key custody, portable backups, and staged atomic restore.
- [ADR-007: Ship an Immutable Versioned Local Archetype Classifier](adrs/adr-007.md) — Defines signature-first local classification, k-nearest-neighbors fallback, and append-only reclassification.
