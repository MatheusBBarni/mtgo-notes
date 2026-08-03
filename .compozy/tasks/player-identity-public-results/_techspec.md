# Technical Specification: Player Identity and Public Results

## Executive Summary

Player Identity and Public Results is an additive bounded context inside the
existing Tauri 2 application. A new Rust `player` subsystem owns the singleton
Player identity, immutable public-result evidence, source-specific consent,
lookup/runtime policy, deletion, classification linkage, and projections. It uses
the existing SQLCipher repository, migration runner, operation coordinator, typed
IPC envelopes, system-browser shell, classifier engine, and portability framework,
but never stores Player data in opponent profiles, encounters, deck records,
public snapshots, or opponent classification runs.

Migration v3 adds dedicated Player tables. A host-only
`PlayerPublicResultsRuntime` starts with Census disabled, exposes a synthetic
adapter only through tests, and owns the one active lookup session, preview tokens,
cooldown, cancellation, transient replay, and content-free audit. React receives
monotonic replacement projections through a main-window-only event and renders the
approved optional Player workspace with a feature-local reducer. Local automated,
packaged Windows, and live Census verification remain separate evidence gates.

## System Architecture

### Component Overview

| Component | Location | Responsibility | Stories |
|---|---|---|---|
| `PlayerDomain` | `src-tauri/src/player/models.rs` | Player identity, evidence, scope/payload, selection, empty outcome, classification, consent, typed outcomes/errors | US-002–US-016 |
| `PlayerRepository` | `src-tauri/src/player/repository.rs` | v3 Player-table queries, immutable writes, optimistic revisions, receipts, tombstones, paging | US-002, US-009–US-016 |
| `PlayerIdentityService` | `src-tauri/src/player/identity.rs` | Singleton identity create/edit with no external side effect and session fencing | US-001–US-002 |
| `PlayerPublicResultsService` | `src-tauri/src/player/service.rs` | Consent, lookup, manual preview, import, selection, refresh, browser handoff, workspace projection | US-003–US-012 |
| `PlayerPublicResultsRuntime` | `src-tauri/src/player/runtime.rs` | Host-only provider config, session generation, cancellation, previews, cooldown, replay, 100-entry content-free audit | US-003–US-007, US-009, US-011, US-016 |
| `CensusProvider` | `src-tauri/src/player/census.rs` | Disabled/synthetic/live adapter, fixed route, response bounds, exact local match | US-004–US-006, US-011, US-016 |
| `PlayerSourceRoutes` | `src-tauri/src/player/routes.rs` | Host-built official/MTGTop8 URLs and no-fetch official artifact validation/canonicalization | US-007–US-008 |
| `PlayerClassificationService` | `src-tauri/src/player/classification.rs` | Complete-evidence adapter to existing pure classifier and Player-owned run persistence | US-012 |
| `PlayerDeletionService` | `src-tauri/src/player/deletion.rs` | Bound deletion previews, scoped/cascade delete, runtime fence, consent revoke, tombstones | US-013 |
| `PortabilityService` extensions | `src-tauri/src/portability/` | Canonical Player archive rows, restore identity preflight, no-resurrection, export scopes | US-014–US-015 |
| Player commands/events | `src-tauri/src/commands/player.rs`, `src-tauri/src/ipc/` | Main-only typed command surface and `player://workspace-v1` replacement event | US-001–US-016 |
| TypeScript IPC | `src/lib/ipc/player.ts`, `src/lib/ipc/events.ts` | Bounded view/request types and typed clients | US-001–US-016 |
| Player workspace | `src/features/player/`, `src/main/MainApp.tsx` | Optional tab, inline identity/consent, lookup states, selection/import, evidence, deletion | US-001–US-015 |
| Release evidence | `tests/release/`, `.github/workflows/` | Packaged Windows focus/accessibility/browser/encryption/offline/capability evidence | US-001, US-003–US-016 |

### Runtime and Data Flow

1. Startup migrates the encrypted notebook through v3, constructs the Player
   repository/services, and creates `PlayerPublicResultsRuntime` in `Disabled`
   Census mode unless a host-only reviewed live configuration is present.
2. `get_player_workspace` loads canonical identity, source status, newest outcome,
   and a bounded evidence page. No identity is a normal projection and never blocks
   existing application initialization.
3. Every Player command validates encoded payload size, caller identity, operation
   binding, and required phase before loading authoritative identity, consent, and
   provider configuration from the host/repository.
4. Lookup creates one runtime session bound to identity ID/revision, nickname
   snapshot, consent epoch/version, provider-configuration fingerprint, UUIDv7
   operation key, host generation, and 15-minute expiry. The provider receives only
   the approved source scope.
5. The Census adapter validates the fixed response envelope and bounds before exact
   case-insensitive matching in Rust. It returns typed candidates or scoped empty;
   provider/configuration failures return degradation and never empty.
6. Candidate/manual previews live in the runtime and bind session, source key,
   source digest, preview digest, and expiry. Renderer payloads contain only opaque
   preview tokens and bounded view data.
7. Import revalidates every binding and writes the immutable evidence batch,
   selected payload/card rows, first selection revisions, durable receipts, and
   replacement projection in one repository transaction. Classification queues
   only after commit and cannot roll back evidence.
8. Identity edits, consent revocation, session supersession, configuration change,
   and deletion increment/fence generations and cancel matching work. Late results
   cannot mutate runtime or durable state.
9. Portability snapshots Player canonical tables explicitly. Restore preflights
   Player identity compatibility before any live merge mutation and resets all
   external source access to disabled.

### Story-to-Component Mapping

| Story | Primary components |
|---|---|
| US-001 | Player workspace, `get_player_workspace`, `MainApp` navigation |
| US-002 | `PlayerIdentityService`, repository, runtime fencing |
| US-003 | `PlayerPublicResultsService`, consent table/runtime epochs |
| US-004 | runtime/provider status projection, Player workspace |
| US-005 | runtime, `CensusProvider`, cancellation/operation coordinator |
| US-006 | Census validator, outcome mapper, empty-outcome repository |
| US-007 | `PlayerSourceRoutes`, browser shell, operation receipts |
| US-008 | manual validator/canonicalizer, preview runtime |
| US-009 | preview binding, repository transaction, selection summary UI |
| US-010 | evidence repository/paging, selection revisions, evidence UI |
| US-011 | lookup runtime, dedupe/version linking, refresh UI |
| US-012 | Player classification adapter/table, existing classifier engine |
| US-013 | `PlayerDeletionService`, tombstones, runtime/consent fencing |
| US-014 | archive registry, restore preflight/merge/replace |
| US-015 | plaintext exporter and scope-specific renderers |
| US-016 | host-only provider factory, config validator, release evidence |

## Implementation Design

### Core Interfaces

The provider boundary accepts only a host-created scope and never a nickname:

```rust
#[async_trait]
pub trait PlayerResultProvider: Send + Sync {
    fn status(&self, now: UtcMillis) -> ProviderStatus;
    async fn lookup(
        &self,
        scope: ApprovedCensusScope,
        cancellation: CancellationToken,
    ) -> Result<ValidatedLeaderboard, PlayerLookupError>;
}
```

The service accepts bounded intent and loads authoritative state internally:

```rust
pub trait PlayerPublicResults {
    fn workspace(&self, page: PageRequest) -> Result<PlayerWorkspaceView, RepoError>;
    async fn start_lookup(&self, input: LookupIntent) -> PlayerResult<LookupOutcome>;
    fn manual_preview(&self, input: ManualEvidenceInput) -> PlayerResult<PreviewView>;
    fn import(&self, input: ImportIntent) -> PlayerResult<ImportOutcome>;
    fn set_selection(&self, input: SelectionIntent) -> PlayerResult<EvidenceView>;
}
```

Durable repository operations preserve transaction and revision boundaries:

```rust
pub trait PlayerStore {
    fn identity(&self) -> Result<Option<PlayerIdentity>, RepoError>;
    fn save_identity(&self, input: SaveIdentity) -> Result<PlayerIdentity, RepoError>;
    fn import_batch(&self, batch: VerifiedImportBatch) -> Result<ImportOutcome, RepoError>;
    fn append_selection(&self, input: VerifiedSelection) -> Result<EvidenceView, RepoError>;
    fn delete(&self, input: VerifiedDeletion) -> Result<DeletionOutcome, RepoError>;
}
```

The UI consumes a complete replacement projection rather than reconstructing host
policy from partial events:

```ts
export type PlayerWorkspaceView = Readonly<{
  revision: number;
  identity: PlayerIdentityView | null;
  sources: readonly PlayerSourceStatus[];
  lookup: PlayerLookupView;
  evidence: Page<PlayerEvidenceView>;
  deletion: PlayerDeletionPreview | null;
}>;
```

### Domain Types

`PlayerId`, `PlayerEvidenceId`, `PlayerSelectionId`, `PlayerEmptyOutcomeId`, and
`PlayerOperationKey` wrap UUIDv7 `EntityId`. Every entity uses `UtcMillis` and a
monotonic integer revision where mutable local state exists.

`EvidenceKind` is closed:

- `MocsLeaderboardEntry` with reviewed label, catalog ID, start date, as-of date,
  total points, Top 8 finishes, and best score.
- `OfficialPublishedDecklist` with event title/date, format, optional
  placement/record, and `ReferenceOnly` or `CompleteDeck` contents.

`EvidenceProvenance` is `ProviderObserved` or `UserAttestedOfficialSource`.
`PlayerSourceRoute` is `CensusMocs`, `OfficialMtgoBrowser`, or `MtgTop8Browser`.
`LookupOutcome` is `Candidates`, `Empty`, `AlreadyImported`, `Cancelled`, or
`BrowserHandoffOpened`. Failures use the closed PRD error taxonomy.

Canonical JSON serialization uses explicit versioned structs, sorted object keys,
NFC-normalized strings where the existing identity convention allows, stable card
ordering by zone/oracle-or-normalized-name, and no floating-point fields. SHA-256
produces source/request/preview digests. Source-key encoders use length-prefixed
segments to prevent delimiter ambiguity.

### Database Schema and Migration

Migration v3 creates tables in dependency order and records its checksum through
the existing migration manager. All foreign keys use `ON DELETE CASCADE` only
inside the Player bounded context; no Player foreign key targets an opponent table.

#### `player_identities`

- `singleton INTEGER PRIMARY KEY CHECK (singleton = 1)`
- `id TEXT NOT NULL UNIQUE` (UUIDv7)
- `display_nickname TEXT NOT NULL`
- `normalized_nickname TEXT NOT NULL`
- `created_at INTEGER NOT NULL`, `updated_at INTEGER NOT NULL`
- `revision INTEGER NOT NULL CHECK (revision > 0)`

The service enforces trimmed, nonblank, control-free input at no more than 128
Unicode scalar values, matching the existing handle bound without sharing records.

#### `player_source_consents`

- `player_identity_id TEXT NOT NULL`
- `route TEXT NOT NULL`
- `disclosure_version TEXT NOT NULL`
- `outbound_fields_json TEXT NOT NULL`
- `fields_digest TEXT NOT NULL`
- `granted_at INTEGER NOT NULL`, `revision INTEGER NOT NULL`
- primary key `(player_identity_id, route)`

This table is local encrypted state but is excluded from backup/export. Revocation
deletes the row transactionally and increments the runtime route epoch.

#### `player_evidence`

- immutable IDs/ownership: `id`, `player_identity_id`, `evidence_schema_version`
- identity/provenance: `kind`, `provenance_mode`, `provider_id`,
  `attribution_url`, nullable `canonical_source_url`
- frozen lookup: `lookup_nickname`, `source_nickname`, `exact_match_rule`,
  `scope_json`, `observed_at`, `imported_at`
- canonical identity: `source_key`, `source_digest`, `preview_digest`
- retained data: `payload_json`, `selected_fields_json`
- version link: nullable `supersedes_evidence_id`
- uniqueness `(player_identity_id, source_key, source_digest)`

Repository writes are insert-only. Any correction creates a new evidence row with a
new source digest and valid `supersedes_evidence_id` in the same source-key chain.

#### `player_evidence_cards`

- `evidence_id`, `oracle_id`, `display_name`, `zone`, `quantity`, `basic_land`
- primary key `(evidence_id, oracle_id, zone)`

Rows exist only for `CompleteDeck`; validation requires no duplicate normalized
card/zone, no partial list, no more than 500 rows, and quantity 1–250.

#### `player_selection_revisions`

- `id`, `evidence_id`, `revision_number`, `selected_fields_json`, `created_at`
- unique `(evidence_id, revision_number)`

The first revision is written with import. Later revisions append after comparing
the expected current revision and never modify prior revisions.

#### `player_empty_outcomes`

- `id`, `player_identity_id`, `provider_id`, `lookup_nickname`,
  `exact_match_rule`, `scope_json`, `provider_configuration_version`,
  `completed_at`, `operation_key`
- unique `(player_identity_id, operation_key)`

Only a fully validated Census response with zero exact matches may insert a row.

#### `player_classification_runs`

- `id`, `evidence_id`, `classifier_version`, `classifier_digest`, `result_id`,
  `result_name`, `method`, `confidence`, `created_at`
- unique `(evidence_id, classifier_version, classifier_digest)`

The table is append-only and Player-owned. It reuses shared classifier result types
and signed assets but has no opponent deck foreign key.

#### `player_tombstones`

- `entity_kind`, `entity_id`, `player_identity_id`, `deleted_at`
- primary key `(entity_kind, entity_id)`

Tombstones contain no nickname, URL, payload, digest, or cards. Archive restore
loads tombstones before records and suppresses any referenced deleted subtree.

#### `player_operation_receipts`

- `operation_key`, `command_kind`, `player_identity_id`, `request_digest`,
  `result_code`, nullable `result_locator`, `created_at`
- primary key `(operation_key, command_kind)`

Receipts support idempotent durable mutations and at-most-once browser opening.
They contain no raw nickname, URL, source key/digest, preview token, payload, or
cards and are excluded from portability/export. Lookup/manual-preview/cancel replay
is bounded in-memory and disappears on restart.

### Trusted Host Runtime

`PlayerPublicResultsRuntime` is application state protected by narrow locks:

- `provider_mode`: `Disabled`, test-only `Synthetic`, or host-created `Live`;
- validated configuration fingerprint/version/expiry and approved Census scope;
- one optional active `LookupSession` with generation and cancellation token;
- route consent epochs used to fence revocation;
- preview store keyed by 256-bit random opaque tokens;
- per-provider cooldown and valid `retry_at`;
- bounded in-memory replay entries for ephemeral commands; and
- `VecDeque<LookupAuditSummary>` capped at 100 and cleared at restart.

No lock is held across await, repository transaction, browser open, or event emit.
Session state is copied into an immutable work lease; completion reacquires the
runtime, verifies session/generation/identity/consent/configuration bindings, and
then may publish an outcome.

The audit allowlist is command, caller, provider/config/consent versions, session
generation, timestamps/duration, outcome/error code, bounded byte/row/preview
counts, and cancellation reason. Diagnostics receive aggregate counts only.

### Census Provider Boundary

`DisabledProvider` returns `provider_disabled` without creating an HTTP client or
request. `SyntheticProvider` is compiled/injected only for tests and reads controlled
fixtures. `LiveCensusProvider` can be constructed only from a host-owned
`ReviewedCensusConfiguration` that proves non-placeholder Service ID, fixed HTTPS
host/path, approved fields, version, expiry, and configuration fingerprint.

The live request:

- uses only `https://census.daybreakgames.com` and the fixed MTGO leaderboard path;
- includes Service ID and approved `digitalobjectcatalogid`, `startdate`, and `date`;
- sends no nickname, cookies, browser credentials, referrer, or arbitrary query;
- refuses redirects;
- uses 5-second connect and 15-second total timeouts;
- streams/decompresses to at most 1 MiB and parses at most 2,000 rows;
- validates required schema/types/ranges before matching; and
- produces at most 10 exact case-insensitive matches.

HTTP/schema/provider text never crosses the provider adapter. It maps to typed
errors and optional validated retry time. No automatic retry exists.

### Manual Evidence and Browser Routes

`PlayerSourceRoutes` has two independent functions:

- build a browser URL from a closed route enum and host-loaded current nickname;
- validate/canonicalize an exact official MTGO artifact URL without I/O.

The browser command accepts only `route` and operation key. It never accepts a URL
or nickname from the renderer. It records an at-most-once receipt before/with the
shell side effect according to the existing operation coordinator's commit boundary.

Manual preview accepts a closed `ManualEvidenceInput`. It rejects unknown fields,
control characters, over-limit values, unsafe host/path/query/fragment patterns,
invalid dates, incomplete deck data, and payloads over 256 KiB before issuing a
token. URL parsing/canonicalization is pure and performs no DNS/network operation.

### Session, Preview, and Idempotency Binding

Every side-effecting command receives UUIDv7 `operationKey`. The canonical request
digest includes command kind, Player identity ID, revision where applicable, and
bounded semantic inputs. Exact receipt replay returns the stored result locator;
reuse with a different digest returns `invalid_request`.

Lookup sessions bind identity ID/revision, nickname snapshot, provider, consent
version/epoch, configuration version/fingerprint/scope, operation key, generation,
and expiry. Preview entries additionally bind source key, source digest, preview
digest, and full canonical preview. Import accepts only token, preview digest,
selected fields, and operation key; all bindings are rechecked host-side.

### Phase and Capability Enforcement

Player commands use `CallerIdentity::Main`; `src-tauri/capabilities/main.json`
receives the command permissions, while overlay/capture manifests receive none.
Commands still verify caller at runtime so manifest mistakes fail closed.

The trusted phase predicate permits consent grant, lookup, refresh, manual preview,
import, selection change, and browser handoff only in `Idle`, `PreMatch`,
`BetweenGames`, or `Finished`. Candidate/in-game/completion-pending/incomplete,
unknown, or stale phase denies before network/browser/persistence. Provider status,
cancellation, and consent revocation remain permitted in all phases.

### Tauri Command Surface

All commands return the existing typed `CommandResult<T>` envelope and stable error
codes. Commands validate payload/caller/phase before side effects.

| Group | Commands | Success data |
|---|---|---|
| Workspace/identity | `get_player_workspace`, `save_player_identity` | replacement view, identity view |
| Provider status/consent | `get_public_provider_status`, `set_public_provider_consent` | route statuses, consent view |
| Lookup | `start_public_result_lookup`, `cancel_public_result_lookup`, `refresh_public_results` | typed lookup outcome/status |
| Manual/browser | `create_manual_evidence_preview`, `open_public_source` | preview, handoff outcome |
| Evidence | `import_public_result`, `update_evidence_selection` | import/evidence view |
| Deletion | `preview_player_deletion`, `confirm_player_deletion` | bound preview, deletion outcome |

`get_player_workspace` is read-only and needs no operation key. Provider status may
be returned within it and remains separately callable without a key. Every other
mutation/external action uses a bound operation key.

### Error Contract

The existing IPC error mapper adds these stable Player codes:

- policy/configuration: `consent_required`, `provider_disabled`,
  `provider_configuration_invalid`, `provider_configuration_expired`,
  `disclosure_restricted`, `capability_denied`;
- admission/input: `player_identity_required`, `player_identity_conflict`,
  `identity_revision_conflict`, `lookup_in_progress`, `lookup_cooldown`,
  `invalid_request`, `payload_too_large`;
- provider: `lookup_timeout`, `provider_rate_limited`, `provider_unavailable`,
  `provider_invalid_response`, `response_too_large`;
- source/fencing: `unsafe_source`, `manual_evidence_invalid`,
  `lookup_session_stale`, `preview_expired`, `preview_mismatch`;
- local side effect: `browser_open_failed`, `save_failed`,
  `player_restore_identity_conflict`, `deletion_preview_stale`.

Renderer errors contain code, recovery enum, and optional `retryAt`; never raw
provider messages, response fragments, paths, secrets, content, or tokens.

### Replacement Event and Frontend State

The native host emits `player://workspace-v1` with a complete
`PlayerWorkspaceView`. `revision` is monotonic for the workspace projection. React
ignores older/equal revisions and requests a fresh snapshot after event gaps or
deserialization failure.

`src/features/player/usePlayerWorkspace.ts` uses `useReducer` for view replacement,
local form drafts, selected preview tokens/fields, pending command identity, and
focus-restoration target. Host state is never recomputed from local source status.
Saved evidence remains in the replacement view during `Loading`, `Empty`,
`Degraded`, and `Cancelled` lookup states.

The feature module includes:

- `PlayerWorkspace.tsx` — responsive container and live-status region;
- `PlayerIdentityPanel.tsx` — first use, save/edit, historical warning;
- `PlayerSourceControls.tsx` — inline disclosure, consent/revoke, status/actions;
- `PlayerLookupPanel.tsx` — loading/cancel/outcome/retry;
- `PlayerCandidateList.tsx` and `PlayerSelectionBar.tsx` — exact candidate and field
  selection with mandatory provenance;
- `PlayerEvidenceList.tsx`/`PlayerEvidenceDetails.tsx` — paged immutable history;
- `ManualEvidenceForm.tsx` — bounded typed official-source input; and
- `PlayerDeletionDialog.tsx` — bounded Player-specific scoped confirmation surface
  composed from existing accessible controls; no shared destructive-dialog
  primitive currently exists.

The Player tab is always present but never auto-selected during onboarding. UI uses
existing 44px controls, visible focus, reading-order tab flow, textual status, and
`aria-live`/status semantics. Status replacement never steals focus. The layout
matches the approved prototype and stacks without clipping at the existing compact
breakpoint.

### Deletion Design

`preview_player_deletion` accepts a closed target (`Evidence`, `EmptyOutcome`, or
`WholeIdentity`), loads current revisions/counts, and returns a random preview token,
digest, exact scope, counts, and 15-minute expiry. Confirmation accepts only token,
digest, and operation key.

The transaction verifies unchanged scope, inserts non-content tombstones, deletes
the target subtree, writes the durable receipt, and commits. Whole-identity deletion
also deletes `player_source_consents`; after commit the runtime increments every
route epoch, cancels active work, clears previews, and emits an empty first-use view.
Opponent tables and generic/opponent consent tables are not referenced by the
service SQL.

### Classification Design

After complete official evidence commits, a background job maps its immutable card
rows to the existing `CompleteDeck` classifier input. The pure classifier and signed
assets are reused. The result is persisted only to `player_classification_runs`.
Classifier unavailable/unsupported/failure updates the Player projection to an
honest unclassified state without changing or rolling back evidence.

Existing classifier-update reclassification discovers both opponent deck revisions
and eligible Player evidence through separate repository adapters and preserves
independent append-only runs.

### Portability and Restore

`portability/records.rs::TABLE_SPECS` adds portable Player tables in this order:

1. `player_identities`
2. `player_evidence`
3. `player_evidence_cards`
4. `player_selection_revisions`
5. `player_classification_runs`
6. `player_empty_outcomes`
7. `player_tombstones`

Consent and operation receipts are intentionally absent. Runtime state/configuration
is not stored in SQL archive records at all. Archive manifests/diffs gain Player
counts while retaining format/schema compatibility checks.

Restore verification extracts the optional archived Player ID before staging/live
merge. If live has a different Player ID, preview marks a hard conflict and removes
`Merge` from allowed modes; apply defensively rechecks before mutation. No live
identity accepts the archived ID. Same ID merges immutable rows and skips exact
duplicates. Tombstones are applied before records and suppress referenced subtrees.
Replace retains existing atomic swap/rollback behavior. Startup after either mode
has no Player consent and constructs a disabled runtime.

`export.rs` writes a Player section only for `CompleteNotebook`. It renders identity,
evidence/version/selection, typed retained values, classification attribution, empty
outcomes, and source/time attribution. `SelectedOpponent` stays on its existing
opponent-only query path and contains no Player joins. Export remains one-way text.

## Integration Points

| Integration | Data sent/received | Authorization and failure policy |
|---|---|---|
| Daybreak Census | Sends Service ID plus approved catalog/start/as-of scope; receives bounded MOCS rows; sends no nickname | Disabled until reviewed live configuration; independent consent; no redirects/retry; typed failure; zero request on any gate failure |
| Official MTGO browser | Host-built player-search URL containing exact current nickname | Independent disclosure/consent; system browser only; no fetch/parse/empty inference |
| MTGTop8 browser | Host-built player-search URL containing exact current nickname | Separate consent; corroboration only; never import source |
| Official MTGO artifact URL | Player-entered URL validated/canonicalized locally | No network/DNS/preflight; exact allowlisted HTTPS artifact route only |
| Local classifier | Complete validated deck and format | Local-only; Player-owned result; import survives classifier failure |
| SQLCipher/DPAPI | Player canonical/consent/receipt rows in encrypted notebook | Existing host-only key custody; packaged Windows evidence required |
| System browser shell | Validated host-built URL | Main-only capability and caller check; failure is typed and idempotently bounded |

## Impact Analysis

| Component | Impact Type | Description and Risk | Required Action |
|---|---|---|---|
| Notebook migrations/schema | modified/high | v3 adds independent encrypted Player graph | Checksummed forward migration, rollback/failure tests, FK/integrity checks |
| Rust domain/services | new/high | trusted authority, sessions, evidence, deletion | Add `player/` subsystem with closed types and transaction boundaries |
| Provider/network | new/critical | external policy, bounds, secret/config custody | Disabled/synthetic/live adapters; gate live construction; fixture tests |
| IPC/capabilities | modified/high | new main-only surface and error/event types | Register commands only in main manifest plus runtime caller checks |
| React main workspace | modified/medium | optional top-level tab and nine-plus states | Add feature reducer/components using approved prototype and primitives |
| Classifier | modified/medium | second owner uses engine, separate persistence | Add Player adapter/discovery without changing opponent rows |
| Portability | modified/critical | archive ordering, merge identity conflict, exports | Explicit table specs/preflight/tombstones/scope tests |
| Privacy/deletion | new/high | cascade and no-resurrection boundary | Dedicated service, bound preview, Player-only SQL regression tests |
| Diagnostics/settings | modified/medium | must exclude new content/secrets | Extend allowlist/privacy regressions; no plaintext provider config |
| Windows release evidence | modified/high | focus/browser/DPAPI/capabilities/installer | Extend packaged evidence script/checklist and artifact retention |

## Testing Approach

- Rust unit tests use table-driven canonicalization/validation/error mapping,
  property tests for source-key/digest and replay invariants, fake clocks/randomness,
  and synthetic provider inputs. I/O is faked only at HTTP, browser, clock, and
  filesystem boundaries.
- Rust integration tests use real temporary SQLCipher databases, the migration
  runner, real repository/services/runtime, local synthetic HTTP fixtures, operation
  coordinator, archive files, and deterministic interruption failpoints.
- React unit/integration tests use Vitest, Testing Library, Tauri mocks, fake timers,
  keyboard input, and axe. Tests consume real TypeScript contracts and replacement
  projections rather than duplicating policy in UI fixtures.
- End-to-end tests exercise the main-window public command/UI surface with synthetic
  Census and browser adapters. Existing opponent workflows run as regressions with
  no Player identity and with populated Player data.
- Packaged Windows 10/11 evidence covers native focus/accessibility tree, screen
  reader/keyboard, browser handoff, capability denial, SQLCipher/DPAPI restart,
  backup/restore/export, offline behavior, scaling/contrast/clipping, installer, and
  release recovery. macOS cannot satisfy these cases.
- Live Census enablement is a separate evidence checklist and is never represented
  as passing by synthetic fixtures or an implemented disabled adapter.

All concrete IDs and expected results live in `_tests.md`.

## Development Sequencing

### Build Order

1. Domain types, v3 migration, repository, receipts, and canonicalization — no new
   feature dependency.
2. Host runtime, consent, phase/caller policy, provider adapter, and source routes —
   depends on identity/repository contracts.
3. Manual preview, lookup outcomes, import, selection, refresh, and classification
   adapter — depends on steps 1–2.
4. Deletion, tombstones, portability, restore preflight, and export — depends on the
   complete durable model.
5. Typed IPC/event surface and optional Player workspace — depends on stable service
   projections; may begin against generated fixtures after interface freeze.
6. Integrated local verification, opponent regressions, packaged Windows evidence,
   and separately gated live Census validation.

### Technical Dependencies

- Existing SQLCipher/DPAPI notebook, operation coordinator, IPC envelopes,
  classifier engine, browser shell, portability framework, and UI primitives.
- The approved Wayfinder prototype and `CONTEXT.md` remain behavioral authorities.
- Daybreak Service ID/use-model approval and current live contract evidence are not
  implementation blockers; they block only `Live` adapter construction and
  production enablement.
- Packaged Windows infrastructure is required to close release-gated tests but does
  not block local implementation or synthetic verification.

## Monitoring and Observability

The application sends no telemetry. Operational visibility is local and allowlisted:

- runtime audit ring: maximum 100 content-free summaries, cleared on restart;
- diagnostics: aggregate counts by provider/outcome/error, active lookup boolean,
  configuration status/version/expiry class, and bounded timing buckets;
- logs: command kind, status/error code, duration bucket, byte/row/preview counts,
  cancellation class, and non-sensitive operation ID only;
- excluded everywhere: nicknames, URLs, source keys/digests, preview tokens, payloads,
  cards, Service ID, provider bodies/messages, and evidence content.

There is no remote alerting. User-visible provider status, retry time, and recovery
guidance are the operational interface. Release verification fails if privacy tests
find a forbidden field or if a live gate is missing/expired.

## Technical Considerations

### Key Decisions

- **Dedicated bounded context**: Player tables/services preserve ownership and avoid
  opponent regressions; cost is explicit schema/service growth.
- **Host-only runtime**: authoritative identity, consent, configuration, sessions,
  routes, and side effects remain outside renderer control; cost is more native
  orchestration.
- **Disabled/synthetic/live provider split**: local implementation stays testable
  without implying authorization; cost is separate enablement evidence.
- **Replacement projections**: UI cannot assemble stale policy-sensitive fragments;
  cost is larger bounded event payloads and paging.
- **Separate classification persistence**: engine/assets are reused without
  polymorphic opponent migrations; cost is a second run table/adapter.
- **Whole-merge Player ID preflight**: prevents cross-person evidence reassignment;
  cost is blocking otherwise mergeable opponent data until replace/cancel.

### Known Risks

- **Distributed Service ID custody**: a desktop credential may be extractable. Keep
  Live disabled until Daybreak's use model explicitly addresses distribution.
- **Source schema drift**: fixed validation may reject changed data. Treat rejection
  as degradation and require refreshed configuration/fixtures/live evidence.
- **Restore complexity**: Player tombstones and identity preflight interact with
  existing opponent merge. Run failure-injection and mixed-notebook property tests.
- **Evidence content leakage**: new data crosses persistence/UI/export boundaries.
  Use allowlisted projections/logging and privacy regression fixtures.
- **Focus/native drift**: webview tests do not prove packaged behavior. Keep Windows
  evidence assigned and pending until artifacts exist.

## Architecture Decision Records

- [ADR-001: Keep the Player Workspace Optional and Additive](adrs/adr-001.md)
- [ADR-002: Use Explicit Conditional and Manual Public Source Routes](adrs/adr-002.md)
- [ADR-003: Preserve Immutable Player-Owned Public Result Evidence](adrs/adr-003.md)
- [ADR-004: Use Dedicated Player Persistence and Trusted-Host Runtime](adrs/adr-004.md)
- [ADR-005: Persist Player Classification Runs Independently](adrs/adr-005.md)
- [ADR-006: Keep Census Configuration Host-Only and Disabled by Default](adrs/adr-006.md)
