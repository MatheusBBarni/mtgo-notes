# WinUI 3 Rewrite Plan

Branch: `feat/winui-rewrite`
Date: 2026-08-17
Status: in progress — WinUI host is the repository root. The Tauri tree lives in `__oldversion__/`.

This is the execution plan for replacing the Tauri 2 / React / Rust host with an unpackaged WinUI 3 / .NET 10 app that attaches to MTGO through MTGOSDK and keeps the existing opponent-notes product.

Decisions already made:

- [ADR-008](adrs/adr-008.md) — WinUI 3 host, no sidecar, no WebView2 notebook
- [ADR-009](adrs/adr-009.md) — read-only MTGOSDK attach as the primary live source

Unchanged on purpose:

- ADR-001 tournament-conservative disclosure
- ADR-003 local notebook, encrypted backup, one-way text export
- ADR-006 SQLCipher + current-user DPAPI (port the design, change the language)
- ADR-007 signed local classifier
- `CONTEXT.md` language (Player identity vs Opponent profile, exact nickname match, confirm-before-import)
- SQL schema v2 and the `.mtgonotes` backup contract — a rewrite that cannot open today's notebook is a data loss bug

## 1. Product target

Ship a native Windows companion that feels like [Videre Tracker](https://videreproject.com/) for *live match context* and like this repo for *memory*.

In scope for the rewrite cutover:

- Detect the current opponent, format, game number, and phase from a running, already-logged-in MTGO client
- Confirm before creating or updating persistent history
- Compact always-on-top overlay with the current disclosure table
- Keyboard-first capture under five seconds
- Local encrypted notebook: profiles, aliases (now including MTGO `User.Id`), encounters, notes, cards, tags, public snapshots, classifier runs
- Official public-deck lookup (existing consent + confirm rules)
- Search, merge/unmerge, deletion undo, backup/restore, text export
- UIA/OCR + manual fallback when live attach is off, broken, or version-mismatched

Out of scope for this rewrite (say no until notes+live-context ship):

- Collection, trades, product openings
- Full game logs (zones, stack, life history, replays)
- Metagame dashboards, hosted Videre API as a required backend
- `Client.LogOn`, chat send, queue, concede, any write into MTGO
- Casual full-dossier-in-game mode
- Cloud accounts, sync, telemetry
- macOS / ARM64 native
- Deleting the Tauri tree before parity

Videre-like chrome that *is* allowed once Phase 6 works: event name, match record, game 1/2/3, sideboarding label, your seat vs opponent. That is presentation of evidence we already store, not a tracker.

## 2. Kill gates

Stop the rewrite and reassess if any of these fail.

| Gate | When | Pass |
|---|---|---|
| Overlay spike | Before any domain port | Unpackaged WinUI window is always-on-top, click-through, does not activate on show, can expand to take clicks, then collapse, against a dummy MTGO-sized window. No focus steal. |
| Schema open | Before UI | New repository opens a copied SQLCipher v2 database from the Tauri app and reads profiles/encounters/notes. |
| Live attach | Before overlay wiring | Attach to logged-in MTGO, emit opponent + phase + format, survive MTGO restart, refuse `LogOn`. |
| Disclosure | Before first public build | Restricted phase never delivers historical notes or public snapshots to overlay/main search. |

## 3. Target solution

```
winui/
  MTGONotes.sln
  Directory.Build.props              # net10.0-windows10.0.19041.0, x64, nullable, treat warnings as errors
  Directory.Packages.props           # central package versions
  global.json                        # pin .NET 10 SDK
  MTGONotes.Core/                    # no WinRT, no MTGOSDK, no SQL
  MTGONotes.Data/                    # SQLCipher, DPAPI, migrations, FTS
  MTGONotes.Live/                    # MTGOSDK adapter only
  MTGONotes.Detection/               # UIA + cropped OCR fallback
  MTGONotes.Providers/               # official decks, HTTPS allowlist
  MTGONotes.Classifier/              # signed assets, signature + k-NN
  MTGONotes.Portability/             # backup / restore / export
  MTGONotes.App/                     # WinUI 3 unpackaged app
  tests/
    MTGONotes.Core.Tests/
    MTGONotes.Data.Tests/
    MTGONotes.Live.Tests/            # Windows-only, can no-op without MTGO
    MTGONotes.App.Tests/
```

Leave `src/`, `src-tauri/`, `tests/`, `package.json` untouched until cutover. They are the spec-by-example.

### Process layout

```
MTGO (player launched, already logged in)
        ▲ read-only RemoteClient / ClrMD
MTGONotes.App.exe
  ClientSession          attach, reconnect, version check
  IContextSource         Live | UiaOcr | Manual
  EncounterReducer       port of src-tauri/src/encounters + detection reduce
  DisclosurePolicy       port of src-tauri/src/disclosure
  NotebookRepository     SQLCipher + DPAPI, schema v2
  Shell
    MainWindow           notebook
    OverlayWindow        projection only
    CaptureWindow        draft only
    Tray + hotkey        still Win32
```

### Facade rule (replaces Tauri capabilities)

| Window | Allowed types | Forbidden |
|---|---|---|
| Overlay | `IOverlayFacade` (confirm opponent, correct phase, open capture, finish, undo, pause) | search, profile read of history, backup, decks, settings writes except overlay prefs |
| Capture | `ICaptureFacade` (save / discard draft) | everything else |
| Main | `INotebookFacade` | raw SQL, raw MTGOSDK objects, unfiltered history during restricted phase |

`DisclosurePolicy.Authorize` is called inside the facades, not in the views. Overlay and capture projects must not reference `MTGONotes.Data` or `MTGONotes.Live`.

## 4. What ports versus what dies

| Current | Fate |
|---|---|
| `src-tauri/src/domain/` | Port 1:1 to `MTGONotes.Core` |
| `src-tauri/src/disclosure/` | Port 1:1 |
| Encounter reducer + generation/sequence evidence | Port 1:1 |
| `src-tauri/src/notebook/schema.rs` v2 | Keep SQL. Reimplement migrations in C# with the same checksums if possible |
| DPAPI key custody (`key.rs`) | Port: 32-byte key, current-user DPAPI, atomic sealed-key write |
| `.mtgonotes` backup envelope | Port byte-for-byte if the format is already shipped; otherwise keep reader compatibility |
| Classifier assets + rules | Port |
| Official deck provider + consent | Port |
| UIA/OCR detector | Port as fallback after live attach works |
| `src/overlay`, `src/capture`, `src/main`, `src/ui` | Rewrite in XAML + `DESIGN.md` resources |
| Tauri capabilities, Vite, npm | Delete at cutover |
| `src-tauri/src/shell/` | Rewrite with AppWindow + NotifyIcon + `RegisterHotKey` |
| Signed Tauri updater | Replace later with a WinUI/MSIX or custom signed updater. Not a Phase 1 blocker if sideload NSIS is enough |

## 5. Phases

Do these in order. Do not start N+1 if N's exit criteria are red. No "while we're here" tracker features.

### Phase 0 — Overlay spike — waived

Do not create `winui/spikes/`. Click-through, `SW_SHOWNOACTIVATE`, and expand/collapse live in `MTGONotes.App` (`Native/OverlayHwnd.cs`). Validate on the first Windows run of the real app.

### Phase 1 — Solution skeleton

- `winui/MTGONotes.sln` with the projects in §3
- `net10.0-windows10.0.19041.0`, win-x64, nullable, warnings as errors
- Central package versions: Windows App SDK, CommunityToolkit.Mvvm, CommunityToolkit.WinUI, xUnit, SQLCipher provider (pick one and lock it)
- Unpackaged App project. No MSIX identity required
- CI job on `windows-2022` that restores, builds, and runs `MTGONotes.Core.Tests`
- macOS `npm run verify` remains the Tauri gate until cutover; do not break it

Exit: empty MainWindow builds on Windows CI.

### Phase 2 — Core domain (can start on any OS)

Port, do not redesign:

- `InternalPhase`, `EncounterStatus`, `CardCertainty`, IDs (UUIDv7), `UtcMillis`, `RepoError` codes
- `ContextEvidence` / `EvidenceProvenance` (`mtgosdk` | `uia` | `ocr` | `manual`)
- `EncounterReducer` table tests from `src-tauri` (generation, sequence, stale candidate, rollover, incomplete)
- `DisclosurePolicy` including `is_disclosure_restricted` and overlay stripping

Copy test names from `.compozy/tasks/mtgo-opponent-notes/_tests.md` that are reducer/disclosure/identity. Mark each ported case in a checklist under `winui/tests/PORT.md`.

Exit: Core tests cover every reducer/disclosure case that exists in Rust today.

### Phase 3 — Persistence

- Current-user DPAPI protector + 32-byte key + atomic sealed file next to the DB
- SQLCipher open with raw key, WAL, foreign keys, secure delete
- Recreate schema v2 exactly (`schema.rs`). Prefer applying the same SQL strings
- FTS5 triggers
- One-active-encounter partial unique index
- Open-existing-DB test using a fixture exported from the Rust test suite

Add nullable `mtgo_user_id INTEGER` on `opponent_aliases` or a new `opponent_identities` table only if you cannot store it in `provenance` without breaking uniqueness. Prefer a new optional column plus a unique index on `(mtgo_user_id) WHERE mtgo_user_id IS NOT NULL`. Schema bump to v3 must be forward-only and must still open v2 files.

Exit: Data tests create, reopen, and refuse to open a DB when the sealed key is missing.

### Phase 4 — Live attach

`MTGONotes.Live` references MTGOSDK.

- `ClientSession.Attach()` requires `IsLoggedIn`. No password APIs in the solution.
- Record `Client.Version` and fail with `provider_unavailable` if outside the supported list.
- From `EventManager.JoinedEvents`, pick the active `Match`. Opponent is the `User` that is not `Client.CurrentUser`.
- Emit evidence with session/generation/sequence/monotonic timestamps.
- Subscribe to `OnGameStarted`, `OnGameEnded`, match state, disconnect. `ClearEvents()` on dispose.
- Reconnect loop when MTGO dies. New attach increments `provider_session` / `generation` so stale evidence cannot mutate the encounter.
- Unit tests with fakes. One manual Windows checklist: login, join a match (or replay if available), confirm handle + phase, kill MTGO, confirm fallback.

Exit: fake source drives the reducer; manual attach checklist is written. Still no Games/Collection/Trade usage.

### Phase 5 — Shell

- Tray: Open, Show/Hide Overlay, Pause Live Attach, Quit
- Close Main hides to tray; last tray Quit disposes the client and flushes SQL
- Global shortcut opens Capture (p95 ≤ 250 ms after warm start)
- Single-instance
- Autostart opt-in
- Window sizes from `tauri.conf.json`
- Overlay uses the Phase 0 recipe
- Navigation allowlist is gone (no webview). External URLs still open in the system browser through an HTTPS allowlist service

Exit: packaged or `dotnet publish -r win-x64` run on a Windows box shows tray + three windows with correct focus behavior.

### Phase 6 — Native UI

Re-express `DESIGN.md` as `Themes/Tokens.xaml`:

- Canvas `#ffffff`, ink `#181d26`, body `#333840`, muted `#41454d`, hairline `#dddddd`
- 4 px spacing, 6/10/12 px radii, Inter if relicensed for WinUI or Segoe UI
- Blue focus rings, no gradients, scarce shadows

Screens, in this order:

1. Onboarding / live-attach consent (disclose unofficial process attach + disable path)
2. Overlay projection (phase, handle, this-match notes only when restricted)
3. Capture (Enter save, Escape dismiss, draft survive failed save)
4. Encounter + confirm opponent
5. History search (blocked when restricted)
6. Profile / notes / cards / tags
7. Public snapshot confirm
8. Settings (consent, pause, overlay, autostart, diagnostics)
9. Backup / restore / export
10. Player workspace (existing player-identity + public results flow)

No marketing chrome from videreproject.com. Dense desktop tool.

Exit: a player can confirm an opponent, save a note from the shortcut, and see it in history after the match without using the Tauri app.

### Phase 7 — Fallback detector + public decks + classifier

- Port UIA/OCR behind `IContextSource` after live attach is default
- Official deck provider, consent, interactive fallback
- Signed classifier assets, append-only runs, no in-app editor

Exit: same provider/classifier contracts as the TechSpec, with `mtgosdk` as an additional provenance.

### Phase 8 — Portability and operations

- Streaming `.mtgonotes` backup, staged restore, merge/replace, rollback
- One-way `.txt` export with the unencrypted warning
- Deletion undo + purge
- Redacted diagnostics bundle
- Serial operation coordinator (no backup during restore, etc.)

Exit: restore a Tauri-created backup into the WinUI app (or document a one-shot migrator if the envelope must change).

### Phase 9 — Cutover

- Windows CI: build, Core/Data tests, unpackaged publish
- NSIS or equivalent x64 installer bootstraps Windows App SDK runtime if missing
- Update `README.md`, `AGENTS.md`, `CONTEXT.md` flagged ambiguities, and mark ADR-004/002/005 superseded in the original folder
- Freeze Tauri: remove npm/Rust from the default verify path or keep a `verify:legacy` script for one release
- Delete `src/`, `src-tauri/` only after one successful Windows install opens a real notebook

## 6. Evidence contract (do not invent a new one)

Keep the Rust shape so the reducer ports mechanically:

```csharp
public sealed record ContextEvidence(
    string ProviderSession,
    ulong Generation,
    ulong Sequence,
    ulong MonotonicMs,
    ContextField Field,
    string NormalizedValue,
    string DisplayValue,
    float Confidence,
    ConfidenceClass ConfidenceClass,
    EvidenceProvenance Provenance,
    long? MtgoUserId = null);
```

Phase map from MTGOSDK:

| SDK signal | `InternalPhase` |
|---|---|
| No joined match | `idle` or keep current incomplete rules |
| Match exists, not started / pairings | `pre_match` |
| `CurrentGame` non-null | `in_game_restricted` |
| Match active, `CurrentGame` null | `between_games` |
| Match finished | `completion_pending` |
| Attach lost / conflicting | `in_game_restricted` |

## 7. Testing

| Layer | How |
|---|---|
| Core | xUnit table tests ported from Rust reducer/disclosure/identity |
| Data | temp SQLCipher files, key missing, migration v2→v3, FTS |
| Live | fakes by default; `[Trait("RequiresMTGO","true")]` for optional attach |
| App | facade authorization tests (overlay type cannot call search) |
| Windows manual | `tests/release/` checklist rewritten for WinUI: overlay focus, DPI, attach, fallback, backup |

Do not claim packaged-Windows proof from macOS. Same rule as today.

Port priority from `_tests.md`: UT/IT cases for encounter, disclosure, consent, backup, classifier. Drop cases that only exist to lint Tauri capability JSON.

## 8. Toolchain and machines

- Windows 10 22H2 or Windows 11 x64 with Visual Studio 2026 / .NET 10 SDK for App, Live, Detection
- MTGO installed for Phase 4/5/6 manual loops
- This Mac can edit `MTGONotes.Core` and plan docs only
- Pin MTGOSDK; rebuild reference assemblies when MTGO moves (their live-at-head rule)
- SQLCipher redistribution terms must be checked before locking the Data package (same open item as the TechSpec)

## 9. Suggested calendar (elapsed, one developer)

| Phase | Elapsed | Depends on Windows? |
|---|---|---|
| 0 Overlay spike | waived | implemented in App |
| 1 Skeleton | 1 day | Yes to confirm unpackaged |
| 2 Core | 3–5 days | No |
| 3 Data | 4–6 days | Partial (DPAPI) |
| 4 Live | 4–7 days | Yes |
| 5 Shell | 3–5 days | Yes |
| 6 UI | 10–15 days | Yes |
| 7 Fallback + decks + classifier | 5–8 days | Yes |
| 8 Portability | 5–8 days | Partial |
| 9 Cutover | 3–5 days | Yes |

This is a full product port, not a weekend host swap. Work starts at Phase 1/2 in `winui/`.

## 10. First coding session

Phase 0 was waived. Implement Core + App shell on this branch.

## 11. Open items that do not block Phase 0

- Exact SQLCipher .NET package (SQLitePCLRaw e_sqlcipher vs commercial)
- Whether v3 schema gets `mtgo_user_id` or a new identities table
- Replacing Tauri signed updater
- Whether Videre's public HTTP API becomes a second deck provider after official MTGO
- Inter font licensing inside a WinUI packaged resource

## 12. Working agreement

- No sidecar.
- No Games/Collection/Trade subscriptions until a later, explicit ADR.
- No `Client.LogOn`.
- No deleting the Tauri tree before Phase 9.
- No "while we're here" redesign of disclosure, backup, or classifier rules.
- Overlay HWND code ships in the real app, not a spike.
