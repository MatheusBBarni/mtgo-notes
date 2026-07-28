# User Stories: MTGO Opponent Notes

Canonical behavior catalog for MTGO Opponent Notes. Companion to `_prd.md`; consumed by `_techspec.md` for component mapping and `_tests.md` for the coverage matrix.

## Personas

- **Individual MTGO player** — A player using a private Windows companion during and between MTGO matches who needs automatic context, fast note capture, trustworthy historical recall, and control over local data without a streaming workflow or cloud account.

## Story Index

| ID | Feature Area | Persona | Story |
|---|---|---|---|
| US-001 | Consent and controls | Individual MTGO player | Understand and control external context providers |
| US-002 | Encounter context | Individual MTGO player | Confirm an automatically detected opponent |
| US-003 | Encounter context | Individual MTGO player | Continue through manual opponent entry |
| US-004 | Encounter lifecycle | Individual MTGO player | Follow and correct automatic match phases |
| US-005 | Encounter lifecycle | Individual MTGO player | Complete, reopen, or recover encounters |
| US-006 | Overlay | Individual MTGO player | Use a phase-scoped compact overlay |
| US-007 | Capture | Individual MTGO player | Save an observation in a few keystrokes |
| US-008 | Observations | Individual MTGO player | Add optional deck, card, and tendency structure |
| US-009 | Observations | Individual MTGO player | Promote, edit, and delete encounter observations |
| US-010 | Public deck context | Individual MTGO player | Confirm a format-relevant public deck snapshot |
| US-011 | Resilience | Individual MTGO player | Keep working offline or without enrichment |
| US-012 | Historical recall | Individual MTGO player | Search and review trustworthy opponent history |
| US-013 | Identity correction | Individual MTGO player | Merge duplicate or renamed opponent profiles |
| US-014 | Backup | Individual MTGO player | Create an encrypted local backup |
| US-015 | Restore | Individual MTGO player | Preview and safely restore a backup |
| US-016 | Export | Individual MTGO player | Export readable opponent history |
| US-017 | Privacy controls | Individual MTGO player | Retain and erase local notebook data |
| US-018 | Support diagnostics | Individual MTGO player | Create a private diagnostic bundle |
| US-019 | Archetype classification | Individual MTGO player | Understand a locally classified deck archetype |

## Consent and Controls

### US-001: Understand and control external context providers

**As an** individual MTGO player, **I want** to understand and control external opponent and deck lookups, **so that** I know when confirmed handles and formats leave my device.

Acceptance criteria:

- AC-1: Given first launch, when automatic context is introduced, then the player sees what data each provider receives and why before any lookup occurs.
- AC-2: Given the disclosure, when the player grants consent, then automatic context becomes available without requiring consent before every encounter.
- AC-3: Given prior consent, when the player disables an external provider, then new requests stop and the manual workflow remains available.
- AC-4: Given provider access is revoked or expires, when the next request would occur, then the player sees the unavailable state and can continue manually.
- AC-5: Given the overlay is enabled, when the player uses the global disable control, then automatic overlay display stops until explicitly enabled again.

Edge cases:

- EC-1: Consent text or provider identity is missing → no external request is sent and consent cannot be completed.
- EC-2: No provider is configured → onboarding completes in manual mode with a clear automatic-context unavailable state.
- EC-3: A provider imposes a quota or rate limit → the player sees the temporary limit and retains manual access.
- EC-4: Provider authorization is invalid or revoked → access is treated as unavailable without retrying unauthorized requests indefinitely.
- EC-5: Enable and disable actions overlap → the final explicit player choice wins and is shown accurately.
- EC-6: The app closes during consent → no partial consent is recorded and the disclosure reappears next launch.
- EC-7: The player grants consent twice → the second action is idempotent and creates no duplicate authorization.
- EC-8: A lookup is attempted before consent → it is blocked and the disclosure is shown first.
- EC-9: A provider becomes disallowed after consent → it is disabled without deleting existing attributed snapshots.
- EC-10: Many providers become available → each is independently disclosed and controlled without hiding the manual fallback.

## Encounter Context

### US-002: Confirm an automatically detected opponent

**As an** individual MTGO player, **I want** the companion to detect and confirm my opponent automatically, **so that** the correct history appears without consuming match time.

Acceptance criteria:

- AC-1: Given an approved or user-authorized source reports an opponent, when confidence is sufficient, then the compact overlay shows the candidate handle for confirmation.
- AC-2: Given a candidate matches an existing handle or alias, when the player confirms it, then the encounter attaches to that profile.
- AC-3: Given no existing profile matches, when the player confirms the candidate, then a new profile is created.
- AC-4: Given the candidate is wrong, when the player corrects it, then only the corrected opponent is persisted.
- AC-5: Given private history exists, when the opponent is confirmed, then the compact summary indicates its presence without exposing phase-restricted content.

Edge cases:

- EC-1: A detected handle is malformed or blank → it is not persisted and manual correction is requested.
- EC-2: Detection returns no opponent → no empty profile is created and manual entry remains available.
- EC-3: Multiple candidates have equal confidence → none is selected automatically and the player chooses or enters one.
- EC-4: Provider authorization disappears during confirmation → the candidate remains correctable and the encounter can continue manually.
- EC-5: Two detection updates arrive concurrently → the latest confirmed player choice wins and stale candidates cannot overwrite it.
- EC-6: The app closes before confirmation → the unconfirmed candidate is discarded or shown as unconfirmed on recovery, never attached silently.
- EC-7: The same confirmed detection repeats → it does not create a duplicate profile or encounter.
- EC-8: A phase update arrives before opponent confirmation → it remains provisional until an encounter exists.
- EC-9: The candidate references a deleted profile → the player is offered a new profile or another active match, not silent restoration.
- EC-10: The notebook contains 100 times the typical profile count → matching and confirmation remain usable and do not expose unrelated profiles.

### US-003: Continue through manual opponent entry

**As an** individual MTGO player, **I want** to enter or select an opponent manually, **so that** outages, unsupported formats, and uncertain detection never block note-taking.

Acceptance criteria:

- AC-1: Given automatic detection is unavailable or incorrect, when the player enters a handle, then matching local profiles and aliases are suggested.
- AC-2: Given a suggested profile, when the player selects it, then the encounter attaches to that profile.
- AC-3: Given no suggestion is correct, when the player confirms the new handle, then a new profile is created.
- AC-4: Given the player cancels manual entry, then no profile or encounter is created.

Edge cases:

- EC-1: The handle contains unsupported or hostile input → it is rejected with a clear validation message and no data is created.
- EC-2: The handle is blank → confirmation remains unavailable.
- EC-3: The suggestion list is large → results are narrowed and remain navigable without loading the entire profile set at once.
- EC-4: The local notebook is unavailable to the current Windows user → entry fails without exposing or replacing another user's data.
- EC-5: Two manual submissions occur together → one encounter is created and duplicate submission is ignored.
- EC-6: The app closes after typing but before confirmation → no unconfirmed profile is persisted.
- EC-7: The player submits the same new handle repeatedly → existing matching identity is reused.
- EC-8: Manual entry starts after an encounter was already confirmed → the player must explicitly correct the active opponent rather than creating a second active encounter.
- EC-9: The selected suggestion is deleted before confirmation → the player is returned to valid choices without data loss.
- EC-10: The notebook has no profiles or very many profiles → the same entry flow works with empty suggestions or scaled search.

## Encounter Lifecycle

### US-004: Follow and correct automatic match phases

**As an** individual MTGO player, **I want** match phases to update automatically with manual correction, **so that** disclosure rules stay conservative without demanding constant input.

Acceptance criteria:

- AC-1: Given a confirmed encounter, when the approved source reports pre-match, in-game, between-games, or finished state, then the visible phase updates.
- AC-2: Given a reported phase is wrong, when the player selects the correct phase, then disclosure changes immediately to match the correction.
- AC-3: Given phase confidence is insufficient, then the product fails closed by treating possible gameplay as in-game.
- AC-4: Given the player moves from in-game to between-games, then historical context becomes available only after that transition is confirmed.
- AC-5: Given the player returns to gameplay, then historical content disappears before current-game capture continues.

Edge cases:

- EC-1: The source returns an unknown phase → the encounter uses the fail-closed in-game disclosure state.
- EC-2: No phase signal arrives → manual phase controls remain available.
- EC-3: Phase changes arrive faster than the interface can render → only the newest valid state is shown.
- EC-4: The provider loses authorization mid-match → the current phase remains conservative and manual correction is enabled.
- EC-5: Provider and manual changes occur concurrently → the most recent explicit manual correction wins until a newer confirmed lifecycle event.
- EC-6: The app restarts during a game → the recovered encounter starts fail-closed until phase is re-established.
- EC-7: A duplicate phase event repeats → no duplicate lifecycle entry or disclosure flicker occurs.
- EC-8: A finished signal arrives before opponent confirmation → it cannot complete an unattached encounter.
- EC-9: A phase change targets a deleted or completed encounter → it is ignored and cannot reopen history silently.
- EC-10: A long multi-game match produces many phase changes → the latest state and complete phase history remain accurate.

### US-005: Complete, reopen, or recover encounters

**As an** individual MTGO player, **I want** encounters to close reliably without losing uncertain information, **so that** my history remains accurate when signals or sessions fail.

Acceptance criteria:

- AC-1: Given a confident match-end signal, when it arrives, then the active encounter completes.
- AC-2: Given a different opponent is confirmed while an encounter is active, then the prior encounter completes automatically, the new one starts, and undo or reopen is offered.
- AC-3: Given no confident end signal arrives, when the companion detects uncertainty, then it prompts the player to confirm completion.
- AC-4: Given the prompt is ignored or the app closes, then the encounter remains incomplete and unconfirmed deck information is excluded from historical summaries.
- AC-5: Given an incomplete encounter, when the player returns, then it can be resumed, completed, or deleted.

Edge cases:

- EC-1: A completion signal lacks an active encounter → no synthetic encounter is created.
- EC-2: An encounter contains no notes or deck information → it may still complete with its identity and timestamps.
- EC-3: Many incomplete encounters exist → they are listed distinctly and can be resolved individually.
- EC-4: The current Windows user cannot persist completion → the encounter remains active or incomplete and the error is visible.
- EC-5: A completion and new-opponent event arrive together → one ordered transition closes the old encounter before starting the new one.
- EC-6: The app closes during automatic completion → recovery produces one completed or one incomplete encounter, never both.
- EC-7: The same end signal repeats → completion is idempotent.
- EC-8: Reopen is requested after a newer encounter starts → the older encounter can be edited but does not replace the current active encounter.
- EC-9: The encounter was deleted before a late signal arrives → it stays deleted.
- EC-10: Years of incomplete and completed encounters exist → incomplete items remain discoverable without degrading normal history browsing.

## Overlay

### US-006: Use a phase-scoped compact overlay

**As an** individual MTGO player, **I want** a compact optional overlay that obeys match-phase disclosure, **so that** I receive useful context without losing focus or exposing restricted history.

Acceptance criteria:

- AC-1: Given a confirmed opponent before a match or between games, then the overlay may show the handle, dated format-matched public deck, and permitted private history.
- AC-2: Given active gameplay, then the overlay shows opponent identity and current-match observations only.
- AC-3: Given the encounter is finished, then the overlay allows full history review and editing.
- AC-4: Given the overlay is compact, when the capture shortcut is used, then it expands without requiring the player to find the main window.
- AC-5: Given any phase, when the player hides or disables the overlay, then it stops displaying immediately without ending the encounter.
- AC-6: Given external context is uncertain, then the overlay shows the uncertainty and uses fail-closed content.

Edge cases:

- EC-1: Overlay content contains malformed public data → unsafe markup is not rendered and the source is marked unavailable.
- EC-2: No opponent or history exists → the overlay presents a neutral ready state rather than stale prior content.
- EC-3: Content exceeds compact space → it is summarized without obscuring hide and capture controls.
- EC-4: The overlay lacks permission to appear above another window → the main companion remains usable and explains the limitation.
- EC-5: Hide, expand, and phase-change actions overlap → hidden state and the newest disclosure state win without flashing historical content.
- EC-6: The overlay process restarts mid-match → it returns hidden or fail-closed and preserves saved current notes.
- EC-7: The capture shortcut repeats rapidly → only one editor opens.
- EC-8: The overlay opens before consent or opponent confirmation → no external or historical data is shown.
- EC-9: The active profile is deleted → the overlay clears it and requests current-opponent resolution.
- EC-10: A profile contains extensive history → compact rendering remains bounded and full details stay in the expanded view.

## Capture

### US-007: Save an observation in a few keystrokes

**As an** individual MTGO player, **I want** to save a free-text observation rapidly, **so that** note-taking does not consume meaningful match-clock time.

Acceptance criteria:

- AC-1: Given a confirmed active encounter, when the player invokes capture, then focus starts in the free-text field.
- AC-2: Given non-empty text, when the player presses Enter, then the observation saves to the current encounter and success is visible.
- AC-3: Given capture is open, when the player presses Escape, then it closes without saving.
- AC-4: Given the player enters only free text, then no structured field is required before saving.
- AC-5: Given a successful save during gameplay, then the note is immediately visible among current-match observations.
- AC-6: Given a save fails, then the text remains available for retry or copying.

Edge cases:

- EC-1: Text contains invalid control characters or hostile markup → it is stored or rejected safely without executing content.
- EC-2: Text is blank or whitespace-only → save is unavailable and no empty observation is created.
- EC-3: Text exceeds the supported input limit → the player sees the limit and the unsaved text is preserved.
- EC-4: The current Windows user cannot write local data → save fails visibly and the text remains recoverable.
- EC-5: Two save actions occur concurrently → one observation is created.
- EC-6: The app closes during save → recovery shows either one saved observation or the preserved unsaved text, never silent loss or duplication.
- EC-7: Enter is pressed repeatedly after success → duplicate observations are not created.
- EC-8: Capture opens without an active encounter → the player must select or create the encounter before persistence.
- EC-9: The encounter completes while capture is open → save targets that encounter only after explicit confirmation, otherwise remains unsaved.
- EC-10: The encounter already contains many observations → capture time and save feedback remain suitable for active play.

## Observations

### US-008: Add optional deck, card, and tendency structure

**As an** individual MTGO player, **I want** optional structured context around my observations, **so that** I can search and interpret them later without slowing capture.

Acceptance criteria:

- AC-1: Given an observation, when the player adds deck identity, then it is labeled as user-authored and dated with the encounter.
- AC-2: Given a card entry, when the player classifies it, then it is explicitly marked observed or suspected.
- AC-3: Given a card entry, then the player may add contextual free text.
- AC-4: Given a tendency observation, then the player may attach user-created tags without choosing a predefined taxonomy.
- AC-5: Given rapid capture, then all structured fields can be added during capture or edited afterward.

Edge cases:

- EC-1: A card or tag contains invalid or hostile text → it is rejected or rendered safely without affecting the note.
- EC-2: No structured data is provided → the free-text observation remains valid.
- EC-3: A note accumulates many cards or tags → the interface summarizes them and keeps all entries accessible.
- EC-4: Structured reference data is unavailable offline → free-text entry and user-created labels remain available.
- EC-5: The same card or tag is added concurrently → duplicates are consolidated or shown clearly without losing context.
- EC-6: The app closes while structured edits are unsaved → the saved free-text note remains intact and unsaved changes are not misrepresented.
- EC-7: The same observed card is submitted twice → repetition does not create ambiguous duplicate facts.
- EC-8: A suspected card is changed to observed after the encounter → the updated certainty retains the original encounter provenance and an edited marker.
- EC-9: A tag is removed from later use → historical notes retain readable text and are not deleted.
- EC-10: Thousands of user-created tags exist → search and selection remain bounded and do not require loading every tag at once.

### US-009: Promote, edit, and delete encounter observations

**As an** individual MTGO player, **I want** current observations to become correctable history, **so that** the notebook remains useful without requiring mandatory post-match administration.

Acceptance criteria:

- AC-1: Given an encounter completes, then its current-match observations join that dated historical encounter automatically.
- AC-2: Given completion, then the player receives an optional review action rather than a blocking approval step.
- AC-3: Given an existing observation, when the player edits it, then the encounter timestamp remains and an edited marker appears.
- AC-4: Given an observation is deleted, then a short undo action is available before permanent removal.
- AC-5: Given the undo period has ended, then the deleted observation no longer appears in history, search, backup, or export.

Edge cases:

- EC-1: An edit contains invalid input → the saved prior version remains visible and unchanged.
- EC-2: An encounter has no observations → completion does not create placeholder notes.
- EC-3: A bulk review contains many observations → changes can be applied without losing unmodified items.
- EC-4: The player cannot write local changes → edit or deletion fails visibly and prior data remains.
- EC-5: Edit and delete occur concurrently → one explicit final state is preserved and shown.
- EC-6: The app closes during the undo period → deletion state recovers consistently and does not resurrect after permanent removal.
- EC-7: Delete is repeated → it remains idempotent.
- EC-8: An observation is edited after its opponent profiles are merged → it stays attached to its original encounter provenance.
- EC-9: The containing encounter is permanently deleted → its observations cannot be edited or restored through ordinary history.
- EC-10: A profile has extensive observation history → editing one item does not rewrite or reorder unrelated encounters.

## Public Deck Context

### US-010: Confirm a format-relevant public deck snapshot

**As an** individual MTGO player, **I want** a recent public deck for the current format with clear provenance, **so that** I can recall relevant public information without mistaking it for current truth.

Acceptance criteria:

- AC-1: Given a confirmed opponent and format, when an external provider is enabled, then the companion requests the most recent matching public deck at encounter start.
- AC-2: Given a result exists, then the player sees its archetype or deck, event, publication date, provider, and source link before persistence.
- AC-3: Given the player confirms the result, then a dated snapshot is stored.
- AC-4: Given the player rejects or corrects the result, then the rejected result is not stored as confirmed.
- AC-5: Given another encounter begins later, then the lookup refreshes and each newly confirmed result becomes a separate historical snapshot.
- AC-6: Given public and user-authored deck identities conflict, then both remain visible with separate labels, dates, and sources.

Edge cases:

- EC-1: Provider data is malformed or links to an unsafe destination → the result is not confirmed and the source error is shown.
- EC-2: No matching public deck exists → the encounter continues with no public snapshot and manual deck identification remains available.
- EC-3: The provider returns many results → only the format-relevant most recent candidate is primary, with provenance retained.
- EC-4: Provider consent or authorization is absent → no request occurs and manual context remains available.
- EC-5: Multiple refresh responses arrive out of order → the response tied to the current confirmed encounter is shown.
- EC-6: Connectivity fails after a result appears but before confirmation → the visible candidate remains confirmable only with its already received provenance.
- EC-7: The same public result is confirmed repeatedly → one snapshot is stored for that encounter.
- EC-8: The format changes before confirmation → the stale-format candidate is invalidated and a relevant lookup may replace it.
- EC-9: A source later removes the deck → the confirmed historical snapshot remains with its original attribution.
- EC-10: A profile has many years of public snapshots → the latest summary remains fast while older snapshots stay accessible.

## Archetype Classification

### US-019: Understand a locally classified deck archetype

**As an** individual MTGO player, **I want** a complete decklist classified into an explainable archetype locally, **so that** I can recognize prior public deck context without depending on a classification service or mistaking a weak guess for fact.

Acceptance criteria:

- AC-1: Given a complete confirmed public or user-entered decklist, when classification runs, then the companion applies the bundled format-specific signature rules before any fallback algorithm.
- AC-2: Given no signature rule matches, then the companion applies its bundled local k-nearest-neighbors corpus and returns **Unclassified** below the shipped confidence threshold.
- AC-3: Given classification completes, then the result records the archetype or **Unclassified**, classifier version, deck revision, method, confidence, and matched-signature or nearest-neighbor explanation.
- AC-4: Given a provider supplies an archetype label that differs from the local result, then both remain visible with separate provenance.
- AC-5: Given a signed application release contains newer classifier assets, then stored complete decklists are reclassified in the background, prior runs remain available, and the newest successful run becomes the default.
- AC-6: Given the player reviews a classified deck, then no control permits editing, importing, activating, or deleting classifier definitions or training data.

Edge cases:

- EC-1: A decklist is partial, malformed, or has an unknown format → automatic classification does not run and the reason is shown.
- EC-2: Multiple signature rules match → the shipped deterministic tie-break order selects one result and the explanation identifies the decisive rule.
- EC-3: A signature rule requires an exact absence or copy count → the classifier distinguishes `exactCopies: 0` from a missing minimum constraint.
- EC-4: The k-nearest-neighbors vote ties → the deterministic shipped tie-break order applies; if confidence remains below threshold, the result is **Unclassified**.
- EC-5: Classifier assets fail signature, schema, or compatibility validation → the release assets are rejected and the last valid classifier remains active.
- EC-6: Reclassification is interrupted by shutdown → completed prior runs remain active and unfinished work resumes idempotently later.
- EC-7: The same classifier version processes the same deck revision repeatedly → only one successful run exists for that pair.
- EC-8: A decklist changes after classification → the old run remains attached to the old revision and a new run is created for the new revision.
- EC-9: A format has no bundled classifier coverage → the result is **Unclassified** without blocking deck confirmation or encounter capture.
- EC-10: Thousands of stored decklists require reclassification → work is bounded, resumable, lower priority than capture, and reports progress without blocking the overlay.

## Resilience

### US-011: Keep working offline or without enrichment

**As an** individual MTGO player, **I want** all private notebook functions to work without external services, **so that** outages and unsupported formats never block my match workflow.

Acceptance criteria:

- AC-1: Given no network connection, then local profiles, history, search, capture, editing, backup, restore, and export remain available.
- AC-2: Given automatic opponent detection is unavailable, then manual opponent entry is offered.
- AC-3: Given public deck lookup is unavailable, then the player can continue without it and enter deck identity manually.
- AC-4: Given the current format lacks provider coverage, then the complete encounter workflow remains available without presenting an error as a blocker.
- AC-5: Given connectivity returns, then future encounters may use automatic context without rewriting prior manual history.

Edge cases:

- EC-1: Network state is reported incorrectly → failed requests degrade to manual behavior rather than blocking the notebook.
- EC-2: No cached external data exists → the app shows absence rather than stale data from another opponent.
- EC-3: Repeated provider failures reach a retry limit → automatic retries pause and the player can retry explicitly later.
- EC-4: Provider permission expires offline → no unauthorized request is queued for silent execution.
- EC-5: Connectivity changes during manual entry → automatic results cannot overwrite confirmed manual identity.
- EC-6: The app restarts offline → all saved local workflows remain available immediately.
- EC-7: The same failed lookup is retried → no duplicate encounter or error record is created.
- EC-8: Connectivity returns after encounter completion → the product does not enrich closed history without player confirmation.
- EC-9: A provider is permanently removed → existing attributed snapshots remain readable.
- EC-10: Many offline encounters accumulate → later online use does not require bulk external lookup or block normal operation.

## Historical Recall

### US-012: Search and review trustworthy opponent history

**As an** individual MTGO player, **I want** to search and review historical opponent information, **so that** accumulated knowledge remains useful outside and between active games.

Acceptance criteria:

- AC-1: Given stored profiles, when the player searches, then matches may be found by primary handle or alias.
- AC-2: Given historical data, then the player can filter by deck, observed card, suspected card, tendency tag, date, and note text.
- AC-3: Given an opponent profile, then encounters appear chronologically with timestamps and source labels.
- AC-4: Given confirmed deck history, then "last deck seen" identifies whether the record is public or user-authored and shows its date and format.
- AC-5: Given an incomplete encounter or unconfirmed deck, then it is labeled and excluded from confirmed last-deck summaries.
- AC-6: Given active gameplay, then search cannot reveal phase-restricted historical content through the overlay or another in-game surface.

Edge cases:

- EC-1: Search contains malformed or hostile syntax → it is treated safely and cannot execute content.
- EC-2: No result matches → the player sees an empty result state without stale profile data.
- EC-3: Filters produce more results than fit at once → results are paged or incrementally revealed without loss.
- EC-4: Historical access is attempted during a restricted phase → only permitted current-match content is returned.
- EC-5: Search results change while a profile is edited → the open profile indicates the newer state rather than silently replacing edits.
- EC-6: The app closes during a filtered review → no data is changed and the next launch begins from a valid view.
- EC-7: The same filter is applied repeatedly → results remain stable and no duplicate rows appear.
- EC-8: A deep link or shortcut targets deleted history → the player sees that it no longer exists.
- EC-9: A profile is merged while open → the view redirects to the canonical merged profile with provenance intact.
- EC-10: The notebook reaches 100 times typical history volume → search remains bounded and does not expose data from another local user.

## Identity Correction

### US-013: Merge duplicate or renamed opponent profiles

**As an** individual MTGO player, **I want** to merge identities reversibly, **so that** typos and handle changes do not fragment history or destroy provenance.

Acceptance criteria:

- AC-1: Given two profiles, when the player starts a merge, then a preview shows their encounters, handles, and conflicts.
- AC-2: Given confirmation, then the player chooses the primary handle and prior handles become searchable aliases.
- AC-3: Given merged profiles, then all encounters retain their original timestamps, observations, and source labels.
- AC-4: Given a recent merge, when the player selects undo, then the original profiles and associations are restored.
- AC-5: Given automatic detection uses an alias, then the canonical profile is suggested for confirmation.

Edge cases:

- EC-1: A merge target is invalid, identical, or malformed → merge is rejected without change.
- EC-2: One profile has no encounters → its handle can still become an alias after confirmation.
- EC-3: Profiles contain extensive history → preview summarizes scale and preserves access to detail.
- EC-4: The player cannot modify local data → merge is unavailable and both profiles remain intact.
- EC-5: Two merges involving the same profile occur concurrently → only one consistent result is committed.
- EC-6: The app closes during merge → recovery shows either the original profiles or the complete merged profile.
- EC-7: The same merge is submitted twice → no duplicate aliases or encounters are created.
- EC-8: Undo is requested after one merged profile receives new encounters → undo previews how new data will be assigned before applying.
- EC-9: A profile is deleted before merge confirmation → the preview is invalidated and no merge occurs.
- EC-10: Many aliases exist → matching remains usable and clearly identifies the canonical handle.

## Backup

### US-014: Create an encrypted local backup

**As an** individual MTGO player, **I want** to create an encrypted backup, **so that** device loss or corruption does not erase my notebook.

Acceptance criteria:

- AC-1: Given a valid notebook, when the player creates a backup, then the complete current notebook is included.
- AC-2: Given backup creation, then the player chooses a destination and passphrase.
- AC-3: Given the passphrase step, then the player must acknowledge that forgotten passphrases cannot be recovered.
- AC-4: Given successful creation, then the player sees the backup destination and completion state.
- AC-5: Given backup failure, then no incomplete file is presented as a valid backup.

Edge cases:

- EC-1: The destination or passphrase input is invalid → creation is blocked with a clear correction message.
- EC-2: The notebook is empty → the player is told the backup contains no profiles before proceeding.
- EC-3: The destination lacks space or file-size capacity → creation fails safely without corrupting an older backup.
- EC-4: The destination is not writable → no backup is created and the notebook remains unchanged.
- EC-5: Two backups target the same path concurrently → overwrite requires explicit resolution and produces one valid file.
- EC-6: The app closes or destination disconnects during creation → the partial artifact is removed or marked invalid.
- EC-7: Backup is repeated after success → a new explicit version is created or overwrite is confirmed.
- EC-8: Backup starts while destructive deletion is pending → the product requires a consistent resolved notebook state first.
- EC-9: Source data becomes unavailable during creation → backup fails rather than claiming completeness.
- EC-10: The notebook contains 100 times typical data → progress remains visible and cancellation does not damage the notebook.

## Restore

### US-015: Preview and safely restore a backup

**As an** individual MTGO player, **I want** to preview and safely restore an encrypted backup, **so that** recovery never silently overwrites or corrupts my current notebook.

Acceptance criteria:

- AC-1: Given a backup file and passphrase, when validation succeeds, then a preview shows its contents, version, and effect.
- AC-2: Given the preview, then the player chooses merge or replace before applying.
- AC-3: Given either choice, then the current notebook is preserved for rollback before restore.
- AC-4: Given merge, then exact duplicates are skipped, matching profiles use reversible alias rules, and conflicting records are preserved and flagged.
- AC-5: Given replace, then the backup becomes the notebook only after successful validation and application.
- AC-6: Given successful restore, then the player sees a summary and can access rollback.

Edge cases:

- EC-1: The passphrase is wrong or the file is malformed → validation fails without changing current data.
- EC-2: The backup is empty → preview states that clearly before merge or replace.
- EC-3: The backup is too large for available storage → restore stops before modifying data.
- EC-4: The player cannot read the file or write local data → restore is blocked and current data remains available.
- EC-5: Two restore actions start concurrently → only one may proceed.
- EC-6: The app closes during restore → recovery returns to the pre-restore notebook or the fully restored notebook, never a partial merge.
- EC-7: The same backup is merged repeatedly → exact duplicates remain single records.
- EC-8: Replace is requested before preview and validation → the operation is blocked.
- EC-9: A backup references deleted or obsolete provider data → historical attributed snapshots remain readable without reactivating providers.
- EC-10: Restore contains 100 times typical history → preview, progress, and rollback remain bounded and understandable.

## Export

### US-016: Export readable opponent history

**As an** individual MTGO player, **I want** a human-readable text export, **so that** I can inspect and retain my data outside the application.

Acceptance criteria:

- AC-1: Given the export action, then the player chooses the complete notebook or one selected opponent.
- AC-2: Given export content, then it is organized by opponent and dated encounter with source, certainty, edit, and incomplete-state labels.
- AC-3: Given public deck snapshots, then provider, event, format, date, and source link appear.
- AC-4: Given export begins, then the player sees that the resulting `.txt` file is unencrypted.
- AC-5: Given successful export, then the player sees the destination and can open the containing location.
- AC-6: Given a text export, then the product does not present it as an import or restore format.

Edge cases:

- EC-1: The destination or filename is invalid → export is blocked without changing notebook data.
- EC-2: The complete notebook or selected profile is empty → the player is warned before creating an empty export.
- EC-3: Export exceeds destination capacity → it fails with no valid-looking partial output.
- EC-4: The destination is not writable → export fails visibly and local data remains unchanged.
- EC-5: Two exports target the same file → overwrite requires explicit confirmation.
- EC-6: The app closes or destination disconnects mid-export → partial output is removed or clearly marked incomplete.
- EC-7: Export is repeated → each run reflects the current confirmed notebook state without duplicating source data internally.
- EC-8: A deleted profile is selected through a stale view → export is blocked and the player returns to active profiles.
- EC-9: Export runs while edits are unsaved → the player chooses to save, discard, or cancel before export.
- EC-10: A very large notebook is exported → progress and cancellation remain available without freezing capture or corrupting data.

## Privacy Controls

### US-017: Retain and erase local notebook data

**As an** individual MTGO player, **I want** direct control over retention and deletion, **so that** private opponent data remains mine.

Acceptance criteria:

- AC-1: Given saved data, then it remains until the player deletes it.
- AC-2: Given an observation or encounter, when deletion is requested, then the player sees the affected scope and receives a short undo opportunity.
- AC-3: Given an opponent profile, when permanent deletion is confirmed, then its encounters, observations, tags, and snapshots disappear from active history, search, backup, and export.
- AC-4: Given erase-notebook, then explicit confirmation identifies that all local profiles and history will be permanently removed.
- AC-5: Given deleted data, then later external detection can create or confirm a new profile without silently restoring deleted history.

Edge cases:

- EC-1: A deletion target is invalid → no other data is affected.
- EC-2: The notebook is already empty → erase reports that no data exists and performs no destructive action.
- EC-3: A profile contains extensive history → the confirmation summarizes the full affected scope.
- EC-4: The player cannot modify local data → deletion fails and nothing disappears from the visible notebook.
- EC-5: Edit, export, backup, and deletion overlap → destructive deletion waits for a consistent state or cancels the conflicting action.
- EC-6: The app closes during the undo period → deletion state recovers consistently and honors permanent completion.
- EC-7: Deletion is submitted twice → it remains idempotent.
- EC-8: Profile deletion is attempted before pending merge resolution → the dependency is shown and must be resolved or explicitly included.
- EC-9: A late provider result arrives for deleted history → it cannot recreate deleted encounters.
- EC-10: Bulk erase covers 100 times typical history → progress is visible and completion leaves no active searchable records.

## Support Diagnostics

### US-018: Create a private diagnostic bundle

**As an** individual MTGO player, **I want** to create a diagnostic bundle explicitly, **so that** I can request support without enabling telemetry or exposing notebook content.

Acceptance criteria:

- AC-1: Given normal use, then the product sends no network telemetry.
- AC-2: Given a support need, when the player creates a diagnostic bundle, then the contents are previewed before saving.
- AC-3: Given the preview, then opponent handles, aliases, note content, public lookup results, and source URLs are absent.
- AC-4: Given successful creation, then the player chooses where to save the bundle and decides whether to share it.
- AC-5: Given a bundle cannot be safely redacted, then creation fails rather than including private content.

Edge cases:

- EC-1: Diagnostic data contains malformed or hostile content → it is rendered safely and redacted before preview.
- EC-2: No diagnostic events exist → the preview states that the bundle contains only environment metadata or is empty.
- EC-3: Diagnostic volume is large → the preview summarizes it and creation remains cancelable.
- EC-4: The destination is not writable → bundle creation fails without sending data elsewhere.
- EC-5: Two bundle creations occur concurrently → each uses a consistent snapshot and cannot mix destinations.
- EC-6: The app closes during creation → no partial bundle is presented as ready to share.
- EC-7: Bundle creation is repeated → no network transmission occurs and each output reflects the chosen snapshot.
- EC-8: Creation is requested before privacy preview → saving is blocked until preview is shown.
- EC-9: A diagnostic source becomes unavailable → the bundle identifies the omission without including notebook data.
- EC-10: Diagnostics cover a long-running installation → redaction applies across the full retained diagnostic set before output.
