# Technical Specification: MTGO Opponent Notes

## Executive Summary

MTGO Opponent Notes is a Windows 10/11 x64 Tauri 2 application with React/TypeScript webviews and a Rust host. The Rust host is the trusted core: it detects visible MTGO context through Windows UI Automation and cropped Windows OCR, enforces the encounter state machine and tournament-conservative disclosure policy, owns the SQLCipher database and DPAPI-sealed key, performs public-deck lookups, classifies complete decks locally, and executes destructive or portable-data operations. React renders capability-scoped `main`, `overlay`, and `capture` windows from typed projections; it never receives encryption keys, OCR captures, direct SQL access, or history that the current phase forbids.

The app is tray-resident, offline-first, account-free, and free of telemetry. Official MTGO decklists are the only V1 automatic public-deck target, contingent on validating documented automated access; otherwise the user confirms a result through the official site. Complete decks are classified by immutable release-bundled signature rules and a local k-nearest-neighbors fallback. `DESIGN.md` supplies the visual tokens and component character, adapted from a marketing-site rhythm to a dense, accessible desktop companion.

## System Architecture

### Component Overview

| Component | Location | Responsibility and boundary | Stories |
|---|---|---|---|
| `DesktopShell` | `src-tauri/src/shell/` | Tauri startup, tray, global shortcut, autostart preference, window creation/focus, single-instance handling, signed updater | US-001, US-006, US-007, US-011, US-019 |
| `ContextDetector` | `src-tauri/src/detection/` | User-authorized MTGO window selection, UIA events, cropped capture, Windows OCR, confidence/evidence; never persists pixels or OCR text | US-001–US-005, US-011 |
| `EncounterEngine` | `src-tauri/src/encounters/` | Ordered opponent/phase reducer, one-active-encounter invariant, completion/incomplete/reopen/undo transitions | US-002–US-005 |
| `DisclosurePolicy` | `src-tauri/src/disclosure/` | Produces allowed projections and denies restricted history/search commands; sole source for overlay data | US-004, US-006, US-012 |
| `NotebookRepository` | `src-tauri/src/notebook/` | SQLCipher connection, DPAPI key custody, migrations, transactions, FTS5, tombstones, drafts, audit records | US-002–US-019 |
| `PublicDeckService` | `src-tauri/src/providers/decks/` | Consent-aware official MTGO lookup, rate limits, provenance, stale-response rejection, confirmation | US-001, US-010, US-011 |
| `ArchetypeClassifier` | `src-tauri/src/classifier/` | Validates signed bundled assets, signature-first classification, k-NN fallback, append-only reclassification jobs | US-010, US-019 |
| `NotebookService` | `src-tauri/src/services/` | Profiles, aliases, observations, tags, deck records, merges, deletion and undo orchestration | US-003, US-007–US-009, US-012, US-013, US-017 |
| `PortabilityService` | `src-tauri/src/portability/` | Streaming encrypted backup, staged merge/replace restore, plaintext text export, atomic files | US-014–US-016 |
| `OperationCoordinator` | `src-tauri/src/operations/` | Serializes migration/restore/purge; snapshots reads for backup/export; cancellation and progress | US-014–US-018 |
| `DiagnosticsService` | `src-tauri/src/diagnostics/` | Structured local logs, redacted preview, explicit support bundle creation; no automatic upload | US-018 |
| `App IPC` | `src-tauri/src/commands/`, `src/lib/ipc/` | Typed request/response envelopes, per-window capability grants, host-to-window events | All |
| `MainApp` | `src/features/` | Onboarding, encounter/history/profile/deck views, search, settings, backup/restore/export, updates | US-001–US-019 |
| `OverlayApp` | `src/overlay/` | Always-on-top policy-filtered context; click-through until explicitly expanded; no search cache | US-002, US-004–US-007, US-010, US-019 |
| `CaptureApp` | `src/capture/` | Single-instance keyboard-first draft editor; Enter saves, Escape dismisses, failed input remains recoverable | US-007 |
| `DesignSystem` | `src/ui/` | Tokens and accessible primitives derived from `DESIGN.md` | All visible stories |

Story ownership is explicit so task decomposition cannot orphan a requirement:

| Story | Primary technical owner | Supporting components |
|---|---|---|
| US-001 | `DesktopShell` | `ContextDetector`, `PublicDeckService`, `MainApp` |
| US-002 | `EncounterEngine` | `ContextDetector`, `NotebookService`, `OverlayApp` |
| US-003 | `NotebookService` | `NotebookRepository`, `MainApp` |
| US-004 | `EncounterEngine` | `ContextDetector`, `DisclosurePolicy` |
| US-005 | `EncounterEngine` | `NotebookRepository`, `OperationCoordinator` |
| US-006 | `DisclosurePolicy` | `DesktopShell`, `OverlayApp` |
| US-007 | `CaptureApp` | `DesktopShell`, `NotebookService` |
| US-008 | `NotebookService` | `NotebookRepository`, `MainApp` |
| US-009 | `NotebookService` | `EncounterEngine`, `NotebookRepository` |
| US-010 | `PublicDeckService` | `NotebookService`, `ArchetypeClassifier` |
| US-011 | `DesktopShell` | all local services and provider fallbacks |
| US-012 | `NotebookRepository` | `DisclosurePolicy`, `MainApp` |
| US-013 | `NotebookService` | `NotebookRepository`, `MainApp` |
| US-014 | `PortabilityService` | `OperationCoordinator`, `NotebookRepository` |
| US-015 | `PortabilityService` | `OperationCoordinator`, staging repository |
| US-016 | `PortabilityService` | `OperationCoordinator`, `MainApp` |
| US-017 | `NotebookService` | `OperationCoordinator`, `NotebookRepository` |
| US-018 | `DiagnosticsService` | `OperationCoordinator`, `MainApp` |
| US-019 | `ArchetypeClassifier` | `NotebookRepository`, signed updater |

### Runtime and Data Flow

1. `DesktopShell` starts hidden-to-tray, unwraps the SQLCipher key with current-user DPAPI, migrates the database, validates bundled classifier assets, and restores only safe resumable state.
2. After disclosed consent, `ContextDetector` subscribes to UIA changes for the selected visible MTGO window. It invokes cropped OCR only when accessible text is missing and the window is visible.
3. `EncounterEngine` reduces monotonic evidence into an internal phase and opponent candidate. Confirming an opponent performs one transaction that closes any different active encounter, starts or reuses the profile, and records undo metadata.
4. Every state transition invokes `DisclosurePolicy`. The host emits a complete replacement `OverlayView`; entering a restricted phase clears historical/public data before the overlay renders the new phase.
5. Confirmed opponent and format trigger `PublicDeckService` only with provider consent. Confirmation stores an immutable source snapshot and complete deck revision, then queues local archetype classification.
6. UI mutations call typed Tauri commands. Commands validate the caller window, disclosure state, entity version, and operation conflicts before delegating to domain services and repository transactions.
7. Backup, restore, export, purge, diagnostics, reclassification, and updates publish progress events. Capture and encounter transitions have higher priority than background work.

### Encounter State Machine

Internal states are `idle`, `candidate`, `pre_match`, `in_game_restricted`, `between_games`, `completion_pending`, `finished`, and `incomplete`. The public model maps `in_game_restricted` and any uncertain possible gameplay to the PRD's in-game disclosure row.

- Enter `in_game_restricted` immediately on one high-confidence visible gameplay signal.
- Leave the restricted state only after a stable trusted UIA signal or corroborated OCR evidence. Unknown, conflicting, stale, or missing evidence stays restricted.
- Opponent candidates require a trusted UIA value or a stable OCR value above the shipped threshold; persistence always requires player confirmation.
- Confirming a different opponent increments the encounter generation, finishes the previous active encounter, starts the new encounter, and records a reversible compound transition.
- A trusted visible result screen moves to `completion_pending`; player confirmation finishes it. Loss of the MTGO window or app shutdown without a confident end persists the encounter as resumable/incomplete.
- Evidence contains `provider_session`, `generation`, `sequence`, and monotonic time. Older-generation and duplicate sequence values are ignored.

## Implementation Design

### Core Interfaces

```rust
pub trait ContextProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> ProviderCapabilities;
    async fn start(&self, consent: ProviderConsent)
        -> Result<ContextStream, ProviderError>;
    async fn stop(&self) -> Result<(), ProviderError>;
}
```

```rust
pub trait EncounterReducer {
    fn reduce(
        &self,
        current: EncounterRuntime,
        evidence: ContextEvidence,
    ) -> Result<Vec<EncounterAction>, TransitionError>;
}
```

```rust
pub trait DisclosurePolicy {
    fn overlay(&self, state: &NotebookState) -> OverlayView;
    fn authorize(&self, query: QueryKind, phase: InternalPhase)
        -> Result<(), DisclosureError>;
}
```

```rust
pub trait NotebookRepository {
    async fn transact<T>(&self, op: NotebookTx<T>) -> Result<T, RepoError>;
    async fn search(&self, query: HistoryQuery)
        -> Result<Page<HistoryHit>, RepoError>;
    async fn snapshot(&self) -> Result<ReadSnapshot, RepoError>;
}
```

```rust
pub trait DeckClassifier {
    fn validate(&self, assets: &[u8]) -> Result<AssetManifest, ClassifierError>;
    fn classify(
        &self,
        deck: &CompleteDeck,
        assets: &ClassifierAssets,
    ) -> Result<ClassificationResult, ClassifierError>;
}
```

```ts
export type CommandResult<T> =
  | { ok: true; data: T; revision: number }
  | { ok: false; error: AppError };

export type AppError = {
  code: ErrorCode;
  message: string;
  retryable: boolean;
  field?: string;
};
```

Errors use stable snake-case codes. Expected user errors never cross IPC as Rust panics or untyped strings. Mutations accept an idempotency key and, where applicable, `expectedRevision`; conflicts return `revision_conflict`.

### Detector Design

- Use `windows-rs` COM bindings for `IUIAutomation`, scoped to the selected MTGO top-level window. Subscribe to name, structure, focus, and window events instead of scanning continuously.
- Store a signed `DetectionProfile` resource containing supported MTGO UI version range, semantic locator alternatives, OCR regions, language, and evidence rules. It contains no user data.
- Use Windows Graphics Capture for a user-authorized visible window and `Windows.Media.Ocr` for named rectangular crops only. Stop capture while minimized, occluded beyond usable bounds, or paused.
- UIA evidence has precedence over OCR at equal recency. OCR runs at most once per second during unresolved states and backs off to once per five seconds after repeated misses.
- Normalize Unicode with NFKC, trim visual separators, preserve display casing, and compare handles with an invariant case-folded key. Never fuzzy-merge profiles automatically.
- Keep evidence in a bounded in-memory ring buffer for troubleshooting counters only; discard raw strings after reduction and never log handles.

### Public Deck Provider

`PublicDeckProvider.lookup(confirmed_handle, format, encounter_generation)` returns zero or more `DeckCandidate` values with provider, event, publication date, source URL, complete card list when available, provider label, and response token.

- The only automatic V1 target is the official MTGO decklist source. An implementation spike must establish a documented endpoint or explicit permission, response stability, rate limits, and redistributable fields before enabling automated lookup.
- If the spike fails, `lookup` returns `interactive_required`; the UI opens official MTGO player search in the system browser and accepts a user-selected official URL for parsing/confirmation. It does not scrape MTGGoldfish or another third party.
- Send only confirmed handle and format after consent. Validate HTTPS, host allowlist, response size, content type, date, format, and card quantities.
- Apply exponential backoff with jitter for transient failures, honor server retry instructions, stop after three automatic attempts, and expose manual retry.
- Bind every response to encounter generation and request token. A late or format-stale response cannot update the active encounter.

### Archetype Classifier

Classifier assets are signed application resources:

```yaml
schemaVersion: 1
classifierVersion: "2026.07.1"
effectiveDate: "2026-07-28"
formats:
  Modern:
    k: 5
    minConfidence: 0.30
    archetypes: []
    corpusDigest: "sha256:..."
```

Each archetype has a stable ID, display name, ordered signature cards, and `strictMode`. A signature constraint contains card oracle ID, display name, optional `minCopies`, and optional `exactCopies`; absence of both defaults to `minCopies: 1`. Every listed constraint must match. `strictMode` labels cannot be produced by k-NN.

For fallback, combine main and sideboard counts into a canonical oracle-ID vector, cap each count at four except basic lands, and compute cosine similarity against the bundled labeled corpus. Select the five highest neighbors ordered by similarity then stable corpus ID. Sum similarity by eligible label; confidence is the winning weight divided by all selected weight. Resolve equal weights by the asset's archetype order. Return `Unclassified` below the bundle threshold.

`classification_runs` are append-only and unique on `(deck_revision_id, classifier_version)`. An update validates signature, schema, corpus digest, supported formats, unique IDs, and deterministic golden vectors before activating assets. Reclassification is resumable in batches of 25, pauses while MTGO is foreground or an interactive operation runs, and keeps the previous successful result visible until each new run commits.

### Data Models

All identifiers are UUIDv7 strings. Timestamps are UTC milliseconds. Mutable aggregates have integer `revision` fields for optimistic concurrency.

| Model | Essential fields |
|---|---|
| `ProviderConsent` | `provider_id`, `version`, `granted_at`, `revoked_at`, disclosed field set |
| `OpponentProfile` | `id`, `primary_handle`, `normalized_handle`, `created_at`, `revision`, `deleted_at` |
| `OpponentAlias` | `id`, `profile_id`, `display_handle`, `normalized_handle`, provenance |
| `Encounter` | `id`, `profile_id`, format, started/ended timestamps, status, phase, source, `revision`, incomplete reason |
| `EncounterTransition` | `id`, encounter, sequence, from/to, trigger, confidence class, created time, undo group |
| `Observation` | `id`, encounter, text, created/edited time, `revision`, deletion deadline/tombstone |
| `CardObservation` | observation, oracle/card name, quantity, certainty (`observed`/`suspected`), context |
| `TendencyTag` | `id`, normalized label, display label; many-to-many observation links |
| `DeckRecord` | `id`, profile, source class, format, completeness, provider label, user label, current revision |
| `DeckRevision` | `id`, deck, revision number, canonical digest, complete flag, created time |
| `DeckCard` | deck revision, oracle ID, display name, main/sideboard zone, quantity |
| `PublicSnapshot` | `id`, encounter, deck revision, provider, event, publication date, source URL, confirmation |
| `ClassificationRun` | deck revision, classifier version/digest, result ID/name, method, confidence, explanation JSON, status |
| `ProfileMerge` | `id`, primary profile, state, created/reversed time, reassignment plan |
| `DeletionTombstone` | entity type/id, requested/effective time, undo token digest, purge state |
| `BackgroundJob` | kind, payload version, cursor, state, priority, progress, last error |
| `OperationRecord` | kind, idempotency key, state, requested/completed time, rollback location |
| `CaptureDraft` | encounter, encrypted text, updated time, claimed window instance |

### Database Schema

The SQLCipher schema uses normalized tables matching the models above plus:

- `schema_migrations(version PRIMARY KEY, checksum, applied_at)`.
- Partial unique index allowing only one encounter with `status = 'active'`.
- Unique normalized primary handles and aliases among non-deleted profiles, checked across both tables inside identity transactions.
- Unique `(encounter_id, source_token)` for imported provider snapshots and `(deck_revision_id, classifier_version)` for classification idempotency.
- `history_fts` FTS5 over permitted non-deleted handles, aliases, note text, deck labels, cards, and tags. Triggers update it in the same transaction.
- `settings` with typed JSON values and schema version; secrets are forbidden.
- WAL, foreign keys, secure delete, bounded busy timeout, periodic checkpoint, and integrity check after unclean shutdown.

Migrations are forward-only Rust modules with checksums. Before a migration, copy the encrypted database through SQLite's online backup API to a rollback path. Commit the new schema version only after invariant checks. On failure, close handles and restore the encrypted rollback copy atomically.

### Backup, Restore, Export, and Deletion

- Backup takes a consistent repository snapshot and streams a canonical logical archive into passphrase-based authenticated encryption. The manifest contains format version, creation time, record counts, schema range, checksums, and classifier provenance. Write to a sibling `.partial` file, flush file and parent directory, then atomically rename.
- Restore decrypts into a newly keyed temporary SQLCipher database, verifies manifest/checksums/references, migrates the staging schema, and computes a read-only preview. Merge uses stable IDs and content digests to skip exact duplicates and preserve divergent records. Replace builds a complete database. Neither mutates live state until explicit confirmation.
- Before merge or replace, make an encrypted rollback snapshot. Commit merge in one transaction; replace uses closed-handle atomic swap. Recovery resolves `OperationRecord` to old or new complete state.
- Export streams the selected opponent or complete notebook in UTF-8 text grouped by opponent and encounter, with timestamps, incomplete/edit/certainty markers and source attribution. It contains no import metadata and is never accepted by restore.
- Observation deletion remains reversible until its deadline. Profile/encounter/notebook deletion requires scope confirmation. Purge removes dependent rows and FTS entries in one transaction, then checkpoints and records only a non-identifying operation audit.

### Tauri Command Surface

There is no network-facing application API. These capability-scoped Tauri commands are the public application surface:

| Group | Commands | Success data | Documented errors |
|---|---|---|---|
| Bootstrap | `bootstrap`, `get_settings`, `update_settings` | safe app state/settings | `key_unavailable`, `migration_failed`, `revision_conflict` |
| Consent/detection | `list_providers`, `set_provider_consent`, `select_mtgo_window`, `pause_detection` | provider status | `consent_required`, `window_not_found`, `provider_unavailable` |
| Encounter | `confirm_opponent`, `enter_opponent`, `correct_phase`, `finish_encounter`, `reopen_encounter`, `undo_transition` | encounter summary | `candidate_stale`, `identity_conflict`, `invalid_transition`, `undo_expired` |
| Capture | `open_capture`, `save_observation`, `discard_draft` | observation/current view | `no_active_encounter`, `blank_observation`, `save_failed`, `already_open` |
| Notes/decks | `update_observation`, `set_card_observations`, `set_tendency_tags`, `confirm_public_snapshot`, `save_complete_deck` | revised aggregate | `revision_conflict`, `invalid_card`, `stale_provider_result`, `deck_incomplete` |
| History | `search_history`, `get_profile`, `get_encounter`, `get_deck_details` | paged policy-safe data | `disclosure_restricted`, `not_found`, `invalid_cursor` |
| Identity | `preview_merge`, `apply_merge`, `preview_unmerge`, `apply_unmerge` | plan/result | `merge_conflict`, `revision_conflict`, `operation_busy` |
| Portability | `start_backup`, `preview_restore`, `apply_restore`, `start_export`, `cancel_operation` | operation ID/preview | `wrong_passphrase`, `invalid_backup`, `destination_unwritable`, `operation_busy`, `cancel_unsafe` |
| Privacy/diagnostics | `request_deletion`, `undo_deletion`, `preview_diagnostics`, `create_diagnostics` | deadline/preview/path | `scope_mismatch`, `undo_expired`, `redaction_failed` |
| Classifier/update | `get_classification`, `start_reclassification`, `check_update`, `install_update` | run/progress/update | `assets_invalid`, `job_busy`, `update_unavailable`, `signature_invalid` |

Host events are versioned replacement messages: `encounter://state-v1`, `overlay://view-v1`, `capture://draft-v1`, `operation://progress-v1`, `provider://status-v1`, `classifier://progress-v1`, and `update://status-v1`. Consumers reject unknown major versions and request `bootstrap` instead of merging partial unknown state.

### Window Capabilities

- `main`: all user-initiated notebook commands except raw detection internals; history calls remain policy-denied during active gameplay.
- `overlay`: `confirm_opponent`, `correct_phase`, `open_capture`, `finish_encounter`, `undo_transition`, and safe bootstrap only.
- `capture`: `save_observation`, `discard_draft`, and safe capture bootstrap only.
- No webview receives filesystem, SQL, shell, process, global shortcut registration, updater installation, or arbitrary HTTP capability. External URLs pass through a Rust HTTPS allowlist and open in the system browser.

### User Interface and `DESIGN.md`

Use `DESIGN.md` as the authoritative visual token source, adapted for a dense desktop tool:

- Bundle Inter Variable; fallback to `Segoe UI`, `system-ui`, sans-serif. Use 14 px body, 13 px metadata, 16 px controls, 18/20/24 px titles, and restrained 400/500/600 weights.
- Use `#ffffff` canvas, `#f8fafc` soft surfaces, `#181d26` ink/primary actions, `#333840` body, `#41454d` muted text, `#dddddd` hairlines, `#9297a0` strong borders, and the documented blue focus/info colors. Reserve pastel signature colors for small provenance/status badges.
- Use the 4 px spacing scale, 6 px control radius, 10 px cards, 12 px major panels, hairline borders, and minimal shadows. No gradients.
- Main window minimum is 960×680; a 240 px navigation rail collapses below 1080 px. Forms use 44 px text controls. Long lists virtualize.
- Overlay defaults to 360×220 logical pixels and remembers monitor-relative position. It is always-on-top, does not activate on automatic show, and is click-through until the user deliberately expands or invokes capture.
- Capture is a single 420×160 logical-pixel tool window placed near the overlay, activates only because the player invoked the shortcut, focuses the text field, and never opens duplicates.
- Every control has keyboard operation, visible blue focus, programmatic label, high-contrast status text/icon, error and disabled states, and a minimum 32×32 desktop target. Do not rely on color alone.
- The Archetype Cleaner image informs result explanation only. V1 has no archetype configuration screen, YAML editor, New/Save/Update/Delete controls, or user-imported classifier assets.

### Performance and Resource Budgets

- Global shortcut to focused capture editor: p95 ≤ 250 ms on the minimum target after warm startup.
- Local observation commit and overlay replacement: p95 ≤ 100 ms; the human capture journey remains under five seconds.
- Entering a restricted phase to cleared overlay render: p95 ≤ 100 ms.
- History search over 10,000 encounters and 100,000 observations: p95 ≤ 200 ms for the first 50 results.
- Complete 100-card classification: p95 ≤ 250 ms. Reclassification yields between batches and never delays capture.
- Idle detector: ≤ 1% average CPU; unresolved OCR: ≤ one crop/second and ≤ 5% average CPU on the minimum target.
- Background operations use bounded 64 MiB buffers. Backup/export stream without loading the notebook into memory.
- Startup to tray-ready: p95 ≤ 2 seconds excluding first-time WebView2 installation and migrations.

## Integration Points

| Integration | Data sent/received | Authorization and failure policy |
|---|---|---|
| Windows UI Automation | Reads visible accessibility tree for selected MTGO window | Explicit onboarding consent; local only; unavailable falls back to cropped OCR/manual |
| Windows Graphics Capture/OCR | Captures named visible crops; returns transient text | Explicit window selection and consent; no persistence; uncertainty fails closed |
| Official MTGO decklists | Sends confirmed handle and format; receives public results | Separate consent; enable automatic adapter only after documented access validation; otherwise interactive official-site flow |
| Windows DPAPI | Sends random DB key to current-user protection APIs | Local OS boundary; failure blocks notebook open without replacing data |
| WebView2 | Renders signed local frontend resources | Installer ensures minimum runtime; no arbitrary navigation |
| Signed release endpoint | Sends target, architecture, and current version when opt-in checking is enabled | No device/user identifier; confirmation before download/install; offline is non-blocking |

## Impact Analysis

| Component | Impact Type | Description and Risk | Required Action |
|---|---|---|---|
| Repository root | new | Greenfield Tauri/React/Rust workspace | Scaffold pinned toolchain, lint, format, test, build, and Windows CI |
| Native detector | new/high | MTGO UI variation and focus safety | Build fixture-driven adapter plus packaged manual validation harness |
| Encounter/disclosure core | new/high | Incorrect transitions could leak history | Pure reducer, host-only projections, exhaustive state/property tests |
| Encrypted repository | new/high | Key or migration failure can lock data | DPAPI/SQLCipher integration, rollback migrations, recovery tests |
| Public deck provider | new/high | External permission and schema remain unvalidated | Complete access spike before enabling automatic adapter |
| Classifier | new/medium | Asset drift and deterministic results | Signed manifests, golden vectors, append-only runs |
| React surfaces | new/medium | Focus, stale state, accessibility | Capability-separated apps and packaged keyboard/focus tests |
| Portability | new/high | Partial restore/export can damage trust | Streaming archive, staging DB, atomic swap, interruption tests |
| Diagnostics/updater | new/medium | Accidental disclosure or supply-chain risk | Allowlisted fields, preview, signed artifacts, opt-in network |

## Testing Approach

- Rust uses `cargo test`/`cargo nextest`, table-driven reducers, property tests for state-machine and merge invariants, golden classifier vectors, temporary real SQLCipher databases, and Windows-only integration tests for DPAPI/UIA/OCR/window APIs.
- React uses Vitest, Testing Library, `@tauri-apps/api/mocks`, axe checks, fake timers, and typed IPC fixtures. Fakes exist only at IPC and external-I/O boundaries; disclosure behavior is tested against real Rust projections.
- Provider integration uses an allowlisted local HTTP fixture server with recorded synthetic official response shapes. No CI request hits MTGO or sends a real handle.
- Windows E2E uses packaged debug/release fixtures with Tauri's WebDriver-compatible harness where supported plus a native focus/window probe. Synthetic MTGO windows expose controlled UIA trees and raster regions.
- Recovery suites terminate child processes at deterministic failpoints during migration, save, backup, restore, export, purge, and reclassification, then assert one complete recoverable state.
- Manual release evidence covers actual MTGO window selection, UIA/OCR detection profiles, DPI 100/125/150/200%, multiple monitors, always-on-top behavior, non-activation, Windows 10 22H2, Windows 11, and installer/updater signatures.
- Every concrete case and traceability row is defined in `_tests.md`.

## Development Sequencing

### Build Order

1. Scaffold Tauri/React/Rust workspace, pinned toolchains, Windows CI, design tokens, typed IPC envelope, and capability manifests.
2. Implement domain IDs, errors, encounter reducer, disclosure policy, and pure projections.
3. Implement DPAPI key custody, SQLCipher repository, schema, migrations, FTS, drafts, and recovery journal.
4. Implement main, overlay, and capture shells with tray/global shortcut and accessibility/focus harness.
5. Implement profile, encounter, observation, deck, search, merge, deletion, and undo services.
6. Implement UIA provider, cropped OCR fallback, evidence reducer, calibration, and provider consent.
7. Validate official MTGO decklist access; implement either the approved automatic adapter or interactive official-site fallback.
8. Implement immutable classifier assets, signature engine, k-NN, explanations, and resumable reclassification.
9. Implement streaming backup, staged restore, text export, operation coordination, and failure recovery.
10. Implement diagnostics preview/bundle, opt-in autostart, signed updater, onboarding, settings, and release validation.

### Technical Dependencies

- A Windows signing identity and release endpoint are required before production installer/updater validation.
- SQLCipher native build and redistribution terms must be verified before persistence implementation is locked.
- The official MTGO decklist access spike is a release dependency for automatic public enrichment, not for local notes or on-screen opponent/phase detection.
- Detection profiles require redacted synthetic fixtures and manual validation against currently supported MTGO builds.
- Classifier corpora require documented redistribution rights, stable labels, and signed build provenance.
- Inter Variable font files and license text must be bundled.

## Monitoring and Observability

V1 has no telemetry or automatic crash upload. Operational visibility is local and privacy-preserving:

- Structured rotating logs contain timestamp, level, component, event code, app/schema/classifier versions, duration bucket, and non-identifying error code.
- Never log handles, aliases, note/card text, source URLs, OCR output, screenshots, encryption material, passphrases, filesystem destinations, or full decklists.
- Local counters track detector availability, confidence class counts, phase transitions, ignored stale evidence, provider status, operation progress, migration result, classifier job cursor, and window focus assertions.
- Retain seven days or 20 MiB, whichever is smaller. Purge follows notebook privacy controls.
- Diagnostics preview lists every file, field class, and redaction count. Bundle creation fails closed if the redaction validator finds prohibited content.
- User-visible health states cover detector paused/unavailable, OCR language missing, notebook locked, migration recovery, provider retry paused, background reclassification, backup/restore progress, and update signature failure.

## Technical Considerations

### Key Decisions

- **Tauri 2 + React/TypeScript + Rust**: maximizes native control and least privilege; costs a two-language stack.
- **UIA then Windows OCR**: reads only visible user-authorized data; costs profile maintenance and probabilistic evidence.
- **Host-owned disclosure projections**: prevents restricted data from reaching overlay memory; costs more explicit IPC models.
- **SQLCipher + DPAPI**: provides no-prompt at-rest protection; costs native packaging and Windows-user binding.
- **Official MTGO public source only**: respects the selected policy boundary; may degrade to interactive confirmation.
- **Immutable signature-first + local k-NN classifier**: explains strong matches and covers more complete decks; requires curated signed assets.
- **Append-only reclassification**: improves history without erasing prior interpretations; costs storage and background jobs.
- **Signed opt-in update checks**: delivers classifier revisions without undisclosed network traffic; requires release infrastructure.

### Known Risks

- The current MTGO accessibility tree and OCR regions are not yet validated. Mitigate with a release-blocking detection spike and versioned profiles.
- Official MTGO decklist automated access may be unavailable or unstable. Ship the interactive official-site path and do not substitute undocumented scraping.
- SQLCipher, DPAPI, WebView2, and OCR behavior differ across supported Windows builds. Maintain a physical/VM compatibility matrix.
- Tournament rules or Daybreak policy can change. Keep providers independently disableable, disclose exact data use, and retain manual mode.
- A classifier corpus can encode stale or disputed labels. Preserve version/explanation, allow `Unclassified`, and never present results as the opponent's current deck.
- A renderer bug could retain stale history. Use replacement projections, clear on restricted transitions, and deny host queries regardless of renderer state.

## Architecture Decision Records

- [ADR-001: Stage the Opponent-Memory V1 Around a Tournament-Conservative Core](adrs/adr-001.md) — Defines the encounter ledger and conservative disclosure boundary.
- [ADR-002: Require Policy-Bounded Automatic Match Context](adrs/adr-002.md) — Requires replaceable automatic context with manual fallback.
- [ADR-003: Keep the Notebook Local While Supporting Recovery and Text Export](adrs/adr-003.md) — Keeps canonical data local and defines recovery/export intent.
- [ADR-004: Build the Windows Companion on Tauri 2](adrs/adr-004.md) — Selects runtime, window topology, tray lifecycle, baseline, and updater boundary.
- [ADR-005: Detect Visible MTGO Context Through UI Automation and OCR](adrs/adr-005.md) — Selects the user-authorized visible context provider and official public source boundary.
- [ADR-006: Encrypt the Live Notebook with SQLCipher and DPAPI](adrs/adr-006.md) — Defines live encryption, key custody, logical backup, and atomic restore.
- [ADR-007: Ship an Immutable Versioned Local Archetype Classifier](adrs/adr-007.md) — Defines immutable signature/k-NN assets and append-only reclassification.
