# Test Specification: MTGO Opponent Notes

Canonical test contract for MTGO Opponent Notes. Companion to `_techspec.md`.
Derived from `_user_stories.md` (behavior) and `_techspec.md` (components).

## Strategy

- **Frameworks and harnesses**: Rust `cargo nextest`, table-driven and property tests, temporary real SQLCipher databases, golden classifier assets, Vitest + Testing Library + axe, `@tauri-apps/api/mocks`, a local provider fixture server, synthetic UIA/OCR windows, and packaged Windows E2E/focus probes.
- **Execution**: `test:unit` runs platform-neutral Rust and React suites; `test:integration` runs real repository/domain wiring and local external-boundary fixtures; `test:windows` runs DPAPI/UIA/OCR/window integration on Windows 10 22H2 and Windows 11; `test:e2e` drives the packaged application; `test:recovery` injects process termination at declared failpoints.
- **Fixtures**: Use synthetic handles, notes, cards, URLs, decklists, UIA trees, and raster crops. Never contact MTGO or transmit a real handle in CI. Fakes sit only at OS, filesystem, clock, release endpoint, and public-provider boundaries.
- **Conventions**: Every mutation carries an idempotency key. Tests assert stable error codes, committed database state, emitted replacement views, and absence of prohibited data. Property cases persist their seed on failure. Packaged focus and DPI evidence is a release gate, not a substitute for automated tests.

## Coverage Matrix

### Stories and Edge Cases

| Source | Behavior | Unit | Integration | E2E |
|---|---|---|---|---|
| US-001 | Consent and provider controls | UT-089–UT-091 | IT-191–IT-194 | E2E-001 |
| US-001.EC-1 | Missing disclosure/provider identity | — | IT-001 | — |
| US-001.EC-2 | No configured provider | — | IT-002 | — |
| US-001.EC-3 | Quota/rate limit | — | IT-003 | — |
| US-001.EC-4 | Invalid/revoked authorization | — | IT-004 | — |
| US-001.EC-5 | Overlapping enable/disable | — | IT-005 | — |
| US-001.EC-6 | Close during consent | — | IT-006 | — |
| US-001.EC-7 | Duplicate consent | — | IT-007 | — |
| US-001.EC-8 | Lookup before consent | — | IT-008 | — |
| US-001.EC-9 | Provider later disallowed | — | IT-009 | — |
| US-001.EC-10 | Many providers | — | IT-010 | — |
| US-002 | Confirm detected opponent | UT-001–UT-012 | IT-195 | E2E-002 |
| US-002.EC-1 | Malformed/blank handle | — | IT-011 | — |
| US-002.EC-2 | No opponent result | — | IT-012 | — |
| US-002.EC-3 | Equal-confidence candidates | — | IT-013 | — |
| US-002.EC-4 | Authorization loss during confirmation | — | IT-014 | — |
| US-002.EC-5 | Concurrent detection updates | — | IT-015 | — |
| US-002.EC-6 | Close before confirmation | — | IT-016 | — |
| US-002.EC-7 | Repeated confirmed detection | — | IT-017 | — |
| US-002.EC-8 | Phase before opponent | — | IT-018 | — |
| US-002.EC-9 | Candidate references deleted profile | — | IT-019 | — |
| US-002.EC-10 | 100× profiles | — | IT-020 | — |
| US-003 | Manual opponent entry | UT-068–UT-070 | IT-196 | E2E-003 |
| US-003.EC-1 | Hostile handle | — | IT-021 | — |
| US-003.EC-2 | Blank handle | — | IT-022 | — |
| US-003.EC-3 | Large suggestion list | — | IT-023 | — |
| US-003.EC-4 | Notebook unavailable | — | IT-024 | — |
| US-003.EC-5 | Concurrent submissions | — | IT-025 | — |
| US-003.EC-6 | Close before confirmation | — | IT-026 | — |
| US-003.EC-7 | Repeated same handle | — | IT-027 | — |
| US-003.EC-8 | Entry after active confirmation | — | IT-028 | — |
| US-003.EC-9 | Suggestion deleted before confirmation | — | IT-029 | — |
| US-003.EC-10 | Empty/large notebook | — | IT-030 | — |
| US-004 | Automatic/manual match phases | UT-009–UT-030 | IT-197 | E2E-004 |
| US-004.EC-1 | Unknown phase | — | IT-031 | — |
| US-004.EC-2 | No phase signal | — | IT-032 | — |
| US-004.EC-3 | Render-lagged phase burst | — | IT-033 | — |
| US-004.EC-4 | Authorization loss mid-match | — | IT-034 | — |
| US-004.EC-5 | Provider/manual phase race | — | IT-035 | — |
| US-004.EC-6 | Restart during game | — | IT-036 | — |
| US-004.EC-7 | Duplicate phase | — | IT-037 | — |
| US-004.EC-8 | Finish before opponent confirmation | — | IT-038 | — |
| US-004.EC-9 | Phase targets deleted/completed encounter | — | IT-039 | — |
| US-004.EC-10 | Long phase history | — | IT-040 | — |
| US-005 | Complete/reopen/recover encounter | UT-013–UT-020 | IT-198–IT-201 | E2E-005 |
| US-005.EC-1 | Completion without active encounter | — | IT-041 | — |
| US-005.EC-2 | Empty encounter completion | — | IT-042 | — |
| US-005.EC-3 | Many incomplete encounters | — | IT-043 | — |
| US-005.EC-4 | Completion persistence denied | — | IT-044 | — |
| US-005.EC-5 | Completion/new-opponent ordering | — | IT-045 | — |
| US-005.EC-6 | Close during completion | — | IT-046 | — |
| US-005.EC-7 | Repeated end signal | — | IT-047 | — |
| US-005.EC-8 | Reopen after newer encounter | — | IT-048 | — |
| US-005.EC-9 | Late signal for deleted encounter | — | IT-049 | — |
| US-005.EC-10 | Years of encounter history | — | IT-050 | — |
| US-006 | Phase-scoped overlay | UT-021–UT-030, UT-095–UT-104 | IT-265–IT-272 | E2E-006 |
| US-006.EC-1 | Malformed public markup | — | IT-051 | — |
| US-006.EC-2 | Neutral no-context state | — | IT-052 | — |
| US-006.EC-3 | Overflowing compact content | — | IT-053 | — |
| US-006.EC-4 | Always-on-top denied | — | IT-054 | — |
| US-006.EC-5 | Hide/expand/phase race | — | IT-055 | — |
| US-006.EC-6 | Overlay restart mid-match | — | IT-056 | — |
| US-006.EC-7 | Repeated capture shortcut | — | IT-057 | — |
| US-006.EC-8 | Overlay before consent/confirmation | — | IT-058 | — |
| US-006.EC-9 | Active profile deleted | — | IT-059 | — |
| US-006.EC-10 | Extensive history in compact view | — | IT-060 | — |
| US-007 | Rapid observation capture | UT-071–UT-076, UT-099–UT-102 | IT-202–IT-204 | E2E-007 |
| US-007.EC-1 | Control characters/hostile markup | — | IT-061 | — |
| US-007.EC-2 | Blank observation | — | IT-062 | — |
| US-007.EC-3 | Input over limit | — | IT-063 | — |
| US-007.EC-4 | Save permission denied | — | IT-064 | — |
| US-007.EC-5 | Concurrent saves | — | IT-065 | — |
| US-007.EC-6 | Close during save | — | IT-066 | — |
| US-007.EC-7 | Repeated Enter | — | IT-067 | — |
| US-007.EC-8 | Capture without encounter | — | IT-068 | — |
| US-007.EC-9 | Encounter completes while capture open | — | IT-069 | — |
| US-007.EC-10 | Many current observations | — | IT-070 | — |
| US-008 | Optional deck/card/tag structure | UT-071–UT-076 | IT-205–IT-207 | E2E-008 |
| US-008.EC-1 | Invalid card/tag text | — | IT-071 | — |
| US-008.EC-2 | Free text without structure | — | IT-072 | — |
| US-008.EC-3 | Many cards/tags | — | IT-073 | — |
| US-008.EC-4 | Offline reference data | — | IT-074 | — |
| US-008.EC-5 | Concurrent duplicate card/tag | — | IT-075 | — |
| US-008.EC-6 | Close with unsaved structured edits | — | IT-076 | — |
| US-008.EC-7 | Repeated observed card | — | IT-077 | — |
| US-008.EC-8 | Suspected changed to observed | — | IT-078 | — |
| US-008.EC-9 | Retired tag | — | IT-079 | — |
| US-008.EC-10 | Thousands of tags | — | IT-080 | — |
| US-009 | Promote/edit/delete observations | UT-071–UT-076 | IT-208–IT-209 | E2E-009 |
| US-009.EC-1 | Invalid edit | — | IT-081 | — |
| US-009.EC-2 | Completion without notes | — | IT-082 | — |
| US-009.EC-3 | Large bulk review | — | IT-083 | — |
| US-009.EC-4 | Mutation permission denied | — | IT-084 | — |
| US-009.EC-5 | Edit/delete race | — | IT-085 | — |
| US-009.EC-6 | Close during undo period | — | IT-086 | — |
| US-009.EC-7 | Repeated delete | — | IT-087 | — |
| US-009.EC-8 | Edit after profile merge | — | IT-088 | — |
| US-009.EC-9 | Edit under deleted encounter | — | IT-089 | — |
| US-009.EC-10 | Edit in extensive history | — | IT-090 | — |
| US-010 | Confirm public deck snapshot | UT-046–UT-053 | IT-210–IT-211 | E2E-010 |
| US-010.EC-1 | Malformed/unsafe provider result | — | IT-091 | — |
| US-010.EC-2 | No matching deck | — | IT-092 | — |
| US-010.EC-3 | Many provider results | — | IT-093 | — |
| US-010.EC-4 | No provider consent | — | IT-094 | — |
| US-010.EC-5 | Out-of-order refreshes | — | IT-095 | — |
| US-010.EC-6 | Disconnect before confirmation | — | IT-096 | — |
| US-010.EC-7 | Repeated snapshot confirmation | — | IT-097 | — |
| US-010.EC-8 | Format changes before confirmation | — | IT-098 | — |
| US-010.EC-9 | Source later removed | — | IT-099 | — |
| US-010.EC-10 | Years of snapshots | — | IT-100 | — |
| US-011 | Offline/local resilience | UT-031–UT-053 | IT-273–IT-282 | E2E-011 |
| US-011.EC-1 | Incorrect network state | — | IT-101 | — |
| US-011.EC-2 | No cached external data | — | IT-102 | — |
| US-011.EC-3 | Retry limit reached | — | IT-103 | — |
| US-011.EC-4 | Permission expires offline | — | IT-104 | — |
| US-011.EC-5 | Connectivity during manual entry | — | IT-105 | — |
| US-011.EC-6 | Offline restart | — | IT-106 | — |
| US-011.EC-7 | Repeated failed lookup | — | IT-107 | — |
| US-011.EC-8 | Connectivity after completion | — | IT-108 | — |
| US-011.EC-9 | Provider removed | — | IT-109 | — |
| US-011.EC-10 | Many offline encounters | — | IT-110 | — |
| US-012 | Search and review history | UT-037–UT-045, UT-068–UT-070 | IT-212–IT-215 | E2E-012 |
| US-012.EC-1 | Hostile search syntax | — | IT-111 | — |
| US-012.EC-2 | Empty search result | — | IT-112 | — |
| US-012.EC-3 | Paged result overflow | — | IT-113 | — |
| US-012.EC-4 | Search during restricted phase | — | IT-114 | — |
| US-012.EC-5 | Results change during edit | — | IT-115 | — |
| US-012.EC-6 | Close during filtered review | — | IT-116 | — |
| US-012.EC-7 | Repeated filter | — | IT-117 | — |
| US-012.EC-8 | Deep link to deleted history | — | IT-118 | — |
| US-012.EC-9 | Open profile merged | — | IT-119 | — |
| US-012.EC-10 | 100× history | — | IT-120 | — |
| US-013 | Reversible profile merge | UT-068–UT-076 | IT-216–IT-219 | E2E-013 |
| US-013.EC-1 | Invalid/identical merge target | — | IT-121 | — |
| US-013.EC-2 | Profile without encounters | — | IT-122 | — |
| US-013.EC-3 | Extensive merge preview | — | IT-123 | — |
| US-013.EC-4 | Merge write denied | — | IT-124 | — |
| US-013.EC-5 | Concurrent overlapping merges | — | IT-125 | — |
| US-013.EC-6 | Close during merge | — | IT-126 | — |
| US-013.EC-7 | Repeated merge | — | IT-127 | — |
| US-013.EC-8 | Undo with post-merge data | — | IT-128 | — |
| US-013.EC-9 | Target deleted before confirmation | — | IT-129 | — |
| US-013.EC-10 | Many aliases | — | IT-130 | — |
| US-014 | Encrypted backup | UT-077–UT-088 | IT-220 | E2E-014 |
| US-014.EC-1 | Invalid destination/passphrase | — | IT-131 | — |
| US-014.EC-2 | Empty notebook backup | — | IT-132 | — |
| US-014.EC-3 | Destination out of space | — | IT-133 | — |
| US-014.EC-4 | Destination not writable | — | IT-134 | — |
| US-014.EC-5 | Concurrent same-path backups | — | IT-135 | — |
| US-014.EC-6 | Interrupted/disconnected backup | — | IT-136 | — |
| US-014.EC-7 | Repeat/overwrite backup | — | IT-137 | — |
| US-014.EC-8 | Backup during pending deletion | — | IT-138 | — |
| US-014.EC-9 | Source unavailable mid-backup | — | IT-139 | — |
| US-014.EC-10 | 100× backup/cancel | — | IT-140 | — |
| US-015 | Staged merge/replace restore | UT-080–UT-088 | IT-221–IT-222 | E2E-015 |
| US-015.EC-1 | Wrong passphrase/malformed file | — | IT-141 | — |
| US-015.EC-2 | Empty backup preview | — | IT-142 | — |
| US-015.EC-3 | Insufficient restore storage | — | IT-143 | — |
| US-015.EC-4 | Read/write denied | — | IT-144 | — |
| US-015.EC-5 | Concurrent restores | — | IT-145 | — |
| US-015.EC-6 | Interrupted restore | — | IT-146 | — |
| US-015.EC-7 | Repeated merge restore | — | IT-147 | — |
| US-015.EC-8 | Replace before preview | — | IT-148 | — |
| US-015.EC-9 | Obsolete provider data | — | IT-149 | — |
| US-015.EC-10 | 100× restore | — | IT-150 | — |
| US-016 | Plaintext scoped export | UT-083–UT-088 | IT-223 | E2E-016 |
| US-016.EC-1 | Invalid export destination | — | IT-151 | — |
| US-016.EC-2 | Empty export scope | — | IT-152 | — |
| US-016.EC-3 | Destination capacity exceeded | — | IT-153 | — |
| US-016.EC-4 | Destination not writable | — | IT-154 | — |
| US-016.EC-5 | Concurrent same-file exports | — | IT-155 | — |
| US-016.EC-6 | Interrupted/disconnected export | — | IT-156 | — |
| US-016.EC-7 | Repeated export | — | IT-157 | — |
| US-016.EC-8 | Deleted selected profile | — | IT-158 | — |
| US-016.EC-9 | Unsaved edits before export | — | IT-159 | — |
| US-016.EC-10 | Very large export | — | IT-160 | — |
| US-017 | Retention/deletion/erase | UT-073–UT-076, UT-086–UT-088 | IT-224–IT-225 | E2E-017 |
| US-017.EC-1 | Invalid deletion target | — | IT-161 | — |
| US-017.EC-2 | Already-empty notebook | — | IT-162 | — |
| US-017.EC-3 | Large deletion scope | — | IT-163 | — |
| US-017.EC-4 | Delete write denied | — | IT-164 | — |
| US-017.EC-5 | Delete vs edit/export/backup | — | IT-165 | — |
| US-017.EC-6 | Close during undo | — | IT-166 | — |
| US-017.EC-7 | Repeated deletion | — | IT-167 | — |
| US-017.EC-8 | Delete with pending merge | — | IT-168 | — |
| US-017.EC-9 | Late provider result after delete | — | IT-169 | — |
| US-017.EC-10 | 100× erase | — | IT-170 | — |
| US-018 | Private diagnostics | UT-089–UT-094 | IT-226–IT-227 | E2E-018 |
| US-018.EC-1 | Hostile diagnostic content | — | IT-171 | — |
| US-018.EC-2 | No diagnostic events | — | IT-172 | — |
| US-018.EC-3 | Large diagnostic volume | — | IT-173 | — |
| US-018.EC-4 | Diagnostic destination denied | — | IT-174 | — |
| US-018.EC-5 | Concurrent bundles | — | IT-175 | — |
| US-018.EC-6 | Interrupted bundle | — | IT-176 | — |
| US-018.EC-7 | Repeated bundle creation | — | IT-177 | — |
| US-018.EC-8 | Save before preview | — | IT-178 | — |
| US-018.EC-9 | Diagnostic source unavailable | — | IT-179 | — |
| US-018.EC-10 | Long-retention redaction | — | IT-180 | — |
| US-019 | Local archetype classification | UT-054–UT-067 | IT-229–IT-230 | E2E-019 |
| US-019.EC-1 | Partial/malformed/unknown format | — | IT-181 | — |
| US-019.EC-2 | Multiple signature matches | — | IT-182 | — |
| US-019.EC-3 | Exact zero/copy constraint | — | IT-183 | — |
| US-019.EC-4 | k-NN tie/low confidence | — | IT-184 | — |
| US-019.EC-5 | Invalid classifier assets | — | IT-185 | — |
| US-019.EC-6 | Interrupted reclassification | — | IT-186 | — |
| US-019.EC-7 | Repeated same version/revision | — | IT-187 | — |
| US-019.EC-8 | Deck revision changes | — | IT-188 | — |
| US-019.EC-9 | Unsupported format | — | IT-189 | — |
| US-019.EC-10 | Thousands reclassified | — | IT-190 | — |

### Components, Interfaces, Commands, and Events

| Source | Responsibility | Unit | Integration | E2E |
|---|---|---|---|---|
| `ContextProvider` / `ContextDetector` | Scoped UIA/OCR evidence and errors | UT-001–UT-008 | IT-273–IT-277 | E2E-002, E2E-004 |
| `EncounterReducer` / `EncounterEngine` | Ordered invariant-preserving transitions | UT-009–UT-020 | IT-195–IT-201 | E2E-002–E2E-005 |
| `DisclosurePolicy` | Policy-safe overlay/history projections | UT-021–UT-030 | IT-265–IT-272 | E2E-004, E2E-006, E2E-012 |
| `NotebookRepository` | SQLCipher, DPAPI, migrations, FTS, transactions | UT-031–UT-045 | IT-278–IT-282 | E2E-003, E2E-012, E2E-017 |
| `PublicDeckService` | Consent, official-source validation, provenance, retries | UT-046–UT-053 | IT-091–IT-100, IT-210 | E2E-010 |
| `DeckClassifier` | Asset validation, signature rules, k-NN, runs | UT-054–UT-067 | IT-181–IT-190, IT-228–IT-229 | E2E-019 |
| `NotebookService` | Profiles, notes, decks, tags, merges, deletion | UT-068–UT-076 | IT-195–IT-219, IT-225–IT-226 | E2E-003, E2E-007–E2E-009, E2E-013, E2E-017 |
| `PortabilityService` / `OperationCoordinator` | Backup, restore, export, cancellation/recovery | UT-077–UT-088 | IT-131–IT-160, IT-220–IT-223 | E2E-014–E2E-016 |
| `DiagnosticsService` / updater | Redaction and signed opt-in releases | UT-089–UT-094 | IT-227–IT-228, IT-280–IT-282 | E2E-001, E2E-018, E2E-019 |
| `DesktopShell` | Tray, windows, focus, shortcut, autostart | UT-095–UT-104 | IT-265–IT-277 | E2E-001, E2E-006–E2E-007 |
| `DesignSystem` / React apps | DESIGN.md tokens, keyboard, accessibility, rendering | UT-105–UT-112 | IT-265–IT-272 | E2E-001–E2E-019 |
| IPC `CommandResult` / capabilities | Typed envelopes and least privilege | UT-113–UT-120 | IT-191–IT-264 | E2E-001–E2E-019 |
| Host event contracts | Versioned replacement projections | UT-117–UT-120 | IT-265–IT-272 | E2E-004, E2E-006 |

Command success and failure coverage appears in the integration catalog: IT-191–IT-264. Host event contracts are IT-265–IT-272. OS and external integration boundaries are IT-273–IT-282.

## Unit Tests

### ContextDetector and ContextProvider

- **UT-001** (happy): `normalize_handle("  ＧＰＴ_42  ")` returns display `ＧＰＴ_42` and its NFKC case-folded lookup key.
- **UT-002** (error): `normalize_handle("\u0000")` returns `invalid_handle` and no candidate.
- **UT-003** (state): trusted UIA opponent evidence creates one confirmable candidate without invoking OCR.
- **UT-004** (ordering): newer UIA evidence supersedes older OCR evidence for the same provider generation.
- **UT-005** (boundary): OCR confidence exactly at the bundled threshold is eligible; one representable value below is not.
- **UT-006** (state): minimized or unselected MTGO windows stop capture and emit `provider_unavailable`.
- **UT-007** (idempotency): repeated UIA event sequence numbers emit no duplicate evidence.
- **UT-008** (error): missing OCR language returns `ocr_language_missing` and preserves UIA/manual operation.

### EncounterReducer and EncounterEngine

- **UT-009** (happy): confirmed candidate from `idle` produces profile resolution plus `pre_match` encounter actions.
- **UT-010** (state): any unknown possible-game evidence maps to `in_game_restricted`.
- **UT-011** (state): one strong gameplay signal enters restricted state immediately.
- **UT-012** (boundary): uncorroborated OCR evidence cannot leave restricted state before the stable-duration threshold.
- **UT-013** (ordering): confirming opponent B while A is active emits finish-A before start-B in one undo group.
- **UT-014** (idempotency): repeated end evidence for a finished encounter emits no action.
- **UT-015** (ordering): evidence from an older encounter generation is ignored.
- **UT-016** (state): app recovery of an active encounter starts restricted until fresh phase evidence arrives.
- **UT-017** (error): finish evidence without an attached encounter returns `invalid_transition`.
- **UT-018** (state): ignored completion prompt yields `incomplete` and excludes unconfirmed deck state.
- **UT-019** (state): reopening an older encounter while a newer encounter is active permits editing without replacing the active encounter.
- **UT-020** (concurrency): property-generated event interleavings never produce more than one active encounter.

### DisclosurePolicy

- **UT-021** (happy): pre-match projection includes confirmed handle, permitted history, and confirmed format-matched public snapshot.
- **UT-022** (state): in-game projection includes only identity and current-encounter observations.
- **UT-023** (state): uncertain/incomplete possible gameplay uses the in-game projection.
- **UT-024** (state): finished projection permits full history and editing actions.
- **UT-025** (error): `authorize(SearchHistory, in_game_restricted)` returns `disclosure_restricted`.
- **UT-026** (error): unconfirmed opponent projection contains no external or historical data.
- **UT-027** (state): deleting the active profile produces a neutral resolution projection, not stale content.
- **UT-028** (ordering): restricted replacement view is computed before any post-transition notification.
- **UT-029** (idempotency): equivalent notebook states serialize to byte-equivalent overlay payloads.
- **UT-030** (error): malformed public markup is represented as unavailable text and never as renderable HTML.

### NotebookRepository

- **UT-031** (happy): first launch generates a 256-bit key, DPAPI-seals it, and opens SQLCipher with `cipher_status = 1`.
- **UT-032** (error): DPAPI unseal failure returns `key_unavailable` without creating a replacement database or key.
- **UT-033** (error): plaintext or wrong-key database returns `notebook_invalid`, never an empty notebook.
- **UT-034** (happy): forward migration commits schema version and checksum after invariant checks.
- **UT-035** (error): injected migration failure restores the encrypted rollback copy and prior schema version.
- **UT-036** (concurrency): partial unique index rejects a second active encounter in a separate transaction.
- **UT-037** (happy): FTS transaction indexes handle, alias, note, deck label, cards, and tags.
- **UT-038** (state): restricted/deleted rows are absent from FTS results after the committing transaction.
- **UT-039** (boundary): pagination cursor returns the next stable page without duplicates at an equal timestamp boundary.
- **UT-040** (error): unknown or tampered cursor returns `invalid_cursor`.
- **UT-041** (concurrency): stale aggregate revision returns `revision_conflict` and preserves the winning row.
- **UT-042** (idempotency): repeated source token inserts one public snapshot.
- **UT-043** (idempotency): repeated operation idempotency key returns the recorded result without rerunning mutation.
- **UT-044** (state): unclean-shutdown integrity check selects valid recovery or returns `notebook_invalid`.
- **UT-045** (boundary): 10,000 encounters/100,000 observations satisfy the specified first-page search budget in the benchmark fixture.

### PublicDeckService

- **UT-046** (happy): confirmed handle and format produce an allowlisted official request bound to encounter generation and token.
- **UT-047** (error): missing consent returns `consent_required` before network I/O.
- **UT-048** (error): non-HTTPS or non-allowlisted source URL returns `unsafe_source`.
- **UT-049** (ordering): late response with stale generation returns `stale_provider_result`.
- **UT-050** (boundary): retry policy stops after three transient attempts and exposes manual retry.
- **UT-051** (happy): candidates sort by exact format then newest publication timestamp with provenance retained.
- **UT-052** (error): oversized, malformed, or wrong-content-type response returns `provider_invalid_response`.
- **UT-053** (state): unavailable automated access produces `interactive_required` with official-site URL only.

### DeckClassifier

- **UT-054** (happy): complete Modern deck matching every signature constraint returns `method=signature`.
- **UT-055** (boundary): omitted copy constraint defaults to `minCopies: 1`.
- **UT-056** (boundary): `exactCopies: 0` matches absence and differs from an omitted constraint.
- **UT-057** (ordering): multiple signature matches resolve by specificity then declared stable archetype order.
- **UT-058** (state): `strictMode` archetype is excluded from k-NN voting.
- **UT-059** (happy): no signature match selects the top five cosine-similarity neighbors in deterministic order.
- **UT-060** (boundary): k-NN confidence exactly `0.30` is accepted; the next lower representable score is `Unclassified`.
- **UT-061** (ordering): equal weighted votes resolve by declared archetype order.
- **UT-062** (error): partial deck returns `deck_incomplete` before vectorization.
- **UT-063** (error): unknown format returns `format_unsupported` without loading another format's corpus.
- **UT-064** (error): invalid signature, schema, duplicate ID, or corpus digest rejects the asset bundle.
- **UT-065** (idempotency): the same deck revision/classifier version yields byte-equivalent result and explanation.
- **UT-066** (state): provider label conflict remains separate from local classification in the result view model.
- **UT-067** (boundary): 100-card classification satisfies the 250 ms benchmark budget.

### NotebookService

- **UT-068** (happy): exact normalized primary handle or alias resolves the canonical active profile.
- **UT-069** (error): hostile/blank manual handle returns `invalid_handle` and writes nothing.
- **UT-070** (boundary): profile suggestion query returns a bounded page at empty and 100× profile counts.
- **UT-071** (happy): free-text observation records encounter provenance and requires no structured data.
- **UT-072** (error): whitespace-only note returns `blank_observation` and keeps the draft.
- **UT-073** (state): suspected-to-observed change preserves encounter time and adds edited time.
- **UT-074** (idempotency): duplicate card/tag additions consolidate by normalized identity without losing context.
- **UT-075** (concurrency): merge/edit/delete conflict produces one explicit revision winner.
- **UT-076** (state): merge/unmerge plan preserves timestamps, sources, aliases, and post-merge reassignment.

### PortabilityService and OperationCoordinator

- **UT-077** (happy): backup manifest contains format/schema versions, counts, hashes, and classifier provenance.
- **UT-078** (error): backup without passphrase acknowledgement returns `acknowledgement_required`.
- **UT-079** (error): encryption/write failure leaves no valid-looking final backup and preserves an older destination.
- **UT-080** (happy): correct passphrase decrypts and validates a canonical archive into staging SQLCipher.
- **UT-081** (error): wrong passphrase or checksum mismatch returns `invalid_backup` before live mutation.
- **UT-082** (idempotency): repeated merge skips exact record digests and preserves divergent records.
- **UT-083** (happy): export formatter emits UTF-8 opponent/encounter sections with required provenance markers.
- **UT-084** (error): export formatter excludes tombstoned/purged records and rejects restore/import use.
- **UT-085** (ordering): replace restore makes rollback snapshot before closing and swapping live database.
- **UT-086** (concurrency): coordinator permits snapshot backup/export together but excludes restore, purge, and migration.
- **UT-087** (state): cancellation before commit cleans `.partial`; cancellation after unsafe point returns `cancel_unsafe`.
- **UT-088** (boundary): stream buffers remain under 64 MiB for the 100× notebook fixture.

### Diagnostics and Updater

- **UT-089** (happy): redactor permits event code/version/duration bucket and removes every prohibited field class.
- **UT-090** (error): canary handle/note/URL surviving any source causes `redaction_failed`.
- **UT-091** (boundary): log retention removes oldest files above seven days or 20 MiB.
- **UT-092** (happy): opt-in update check sends only target, architecture, and current version.
- **UT-093** (error): invalid update signature returns `signature_invalid` and never invokes installation.
- **UT-094** (state): disabled launch check performs no release-endpoint request.

### DesktopShell and Windows

- **UT-095** (state): closing `main` hides it while tray, detector, shortcut, and overlay service remain active.
- **UT-096** (state): tray Exit requests operation-safe shutdown and then terminates all windows.
- **UT-097** (idempotency): repeated launch routes to the existing process and opens `main`.
- **UT-098** (state): launch-with-Windows setting is off by default and uses the scoped autostart adapter.
- **UT-099** (concurrency): repeated global shortcut claims one capture window instance.
- **UT-100** (state): automatic overlay show uses non-activating window flags.
- **UT-101** (state): user-invoked capture activates and focuses the text input.
- **UT-102** (error): always-on-top failure leaves `main` usable and returns `overlay_unavailable`.
- **UT-103** (boundary): monitor-relative overlay position clamps to every current work area after DPI/monitor change.
- **UT-104** (state): unknown window label receives no capability set.

### DesignSystem and React Apps

- **UT-105** (happy): token snapshot matches `DESIGN.md` color, 4 px spacing, 6/10/12 px radius, and font-family values.
- **UT-106** (state): primary, secondary, destructive, disabled, hover, active, focus, and error states remain distinguishable without gradients.
- **UT-107** (accessibility): every interactive primitive exposes name, role, state, and visible focus.
- **UT-108** (accessibility): phase, certainty, source, error, and incomplete state remain understandable with color removed.
- **UT-109** (boundary): compact overlay truncates/summarizes overflow while hide and capture controls remain reachable.
- **UT-110** (state): replacement restricted event removes historical DOM nodes and cached view state.
- **UT-111** (state): classifier result shows method/version/confidence/explanation and exposes no editor/import controls.
- **UT-112** (error): IPC failure renders exact fallback action and preserves recoverable draft input.

### IPC Envelopes, Capabilities, and Events

- **UT-113** (happy): successful command serializes `{ok:true,data,revision}` with no Rust-only types.
- **UT-114** (error): expected failure serializes stable code/message/retryable/optional field.
- **UT-115** (error): panic boundary returns `internal_error` correlation code without sensitive debug text.
- **UT-116** (error): missing/invalid idempotency key on mutation returns `invalid_request`.
- **UT-117** (state): each event payload includes name and supported major version.
- **UT-118** (error): unknown event major is rejected and triggers safe bootstrap.
- **UT-119** (accessibility): overlay/capture capability manifests contain only their documented commands.
- **UT-120** (error): wildcard filesystem, SQL, shell, process, updater-install, or arbitrary HTTP capability fails manifest-policy lint.

## Integration Tests

### Story Edge Cases: IT-001–IT-100

- **IT-001**: Disclosure fixture without provider ID cannot persist consent; fixture server receives zero requests.
- **IT-002**: Onboarding with no providers reaches manual-ready state with `automatic_context_unavailable`.
- **IT-003**: HTTP 429 plus retry metadata pauses retries, shows the retry time, and leaves manual entry enabled.
- **IT-004**: HTTP 401 revokes provider availability and produces no retry loop.
- **IT-005**: Concurrent enable then later disable commits disabled state at the highest settings revision.
- **IT-006**: Process termination before consent commit restarts with consent absent and disclosure visible.
- **IT-007**: Reusing consent idempotency key creates one consent version.
- **IT-008**: Provider lookup before consent returns `consent_required`; fixture server records zero calls.
- **IT-009**: Disallowing a provider disables new requests while an existing attributed snapshot remains queryable outside gameplay.
- **IT-010**: Twenty provider descriptors render independent controls and a persistent manual option without unbounded query.
- **IT-011**: Blank/control-only detected handle produces no profile and opens correction with `invalid_handle`.
- **IT-012**: Empty detector stream creates no profile/encounter and keeps manual action available.
- **IT-013**: Two equal-confidence candidates produce a chooser and no automatic selection.
- **IT-014**: Consent revoked after candidate display leaves candidate editable but blocks further provider calls.
- **IT-015**: Two concurrent candidates followed by confirmation of the first cannot be overwritten by the later unconfirmed event.
- **IT-016**: Termination before confirmation recovers no attached profile; any draft candidate remains explicitly unconfirmed.
- **IT-017**: Replayed confirmed evidence creates one profile and one encounter by source token.
- **IT-018**: Pre-confirmation phase evidence is retained only in runtime and applies after encounter creation if still current.
- **IT-019**: Candidate matching a tombstoned profile offers creation/active alternatives and does not clear the tombstone.
- **IT-020**: Matching among the 100× profile fixture meets the budget and returns only bounded relevant suggestions.
- **IT-021**: `<script>` and unsupported control characters in manual handle return `invalid_handle` with zero writes.
- **IT-022**: Whitespace manual handle leaves confirm disabled and produces no command.
- **IT-023**: Ten-thousand-profile suggestions return the first bounded page and keyboard cursor navigation.
- **IT-024**: Foreign Windows-user/failed DPAPI fixture returns `key_unavailable` without exposing or replacing the database.
- **IT-025**: Two `enter_opponent` calls with one idempotency key produce one active encounter.
- **IT-026**: Termination with typed but unconfirmed handle persists no profile.
- **IT-027**: Repeated normalized handle submission reuses the canonical profile.
- **IT-028**: Manual entry during an active encounter returns `explicit_correction_required` and leaves one active encounter.
- **IT-029**: Deleting a suggested profile before confirm returns `candidate_stale` and refreshes suggestions.
- **IT-030**: Empty and 100× notebooks use the same entry command and return valid bounded result shapes.
- **IT-031**: Unknown provider phase immediately emits restricted replacement view.
- **IT-032**: No phase evidence leaves manual phase controls enabled and history restricted when gameplay is possible.
- **IT-033**: A 100-event phase burst commits ordered transitions but renders only the newest replacement payload.
- **IT-034**: Mid-match consent revocation preserves restricted phase and enables correction.
- **IT-035**: Manual correction at sequence 20 wins over provider sequence 19; later trusted sequence 21 may transition.
- **IT-036**: Restart with active encounter boots restricted and emits no historical/public fields.
- **IT-037**: Repeated identical phase evidence creates one transition and no overlay flicker event.
- **IT-038**: Finished evidence without confirmed opponent returns `invalid_transition` and creates no encounter.
- **IT-039**: Phase evidence for deleted/finished encounter generation is ignored.
- **IT-040**: Ten-thousand transitions preserve latest state and page historical transitions in order.
- **IT-041**: End evidence in idle state returns `invalid_transition` and writes nothing.
- **IT-042**: Note-free encounter finishes with identity/start/end timestamps and no placeholder observation.
- **IT-043**: Ten-thousand incomplete encounters page distinctly and resolve by stable ID.
- **IT-044**: Injected commit denial leaves encounter active/incomplete and surfaces `save_failed`.
- **IT-045**: Simultaneous finish-A/confirm-B commits finish before start and one compound undo group.
- **IT-046**: Termination at every completion failpoint recovers exactly one finished or incomplete encounter.
- **IT-047**: Replayed end token returns the existing completion result.
- **IT-048**: Reopen of A after active B permits A edit mode but leaves B the only active encounter.
- **IT-049**: Late end evidence for purged encounter creates no row and no overlay change.
- **IT-050**: 100× mixed encounter fixture meets history/incomplete query budgets.
- **IT-051**: Provider fields containing HTML/script render escaped text and an unavailable source status.
- **IT-052**: Bootstrap without opponent/history clears prior overlay and shows neutral ready state.
- **IT-053**: Oversized history projection summarizes counts; fixed hide/capture controls remain keyboard reachable.
- **IT-054**: Simulated topmost denial returns `overlay_unavailable` while main window retains all allowed actions.
- **IT-055**: Concurrent hide, expand, and restricted transition ends hidden with no historical-content frame.
- **IT-056**: Killing/recreating overlay during gameplay bootstraps hidden restricted state and preserved saved current notes.
- **IT-057**: Twenty shortcut events within 100 ms open one capture instance.
- **IT-058**: Overlay bootstrap before consent/confirmation contains only neutral local controls.
- **IT-059**: Deleting active profile emits a cleared resolution view and blocks stale saves.
- **IT-060**: 100× history never enters compact payload; expanded main query remains paged.
- **IT-061**: Note containing control characters/HTML is safely normalized or rejected and never executes in any view.
- **IT-062**: `save_observation(" \n ")` returns `blank_observation` and creates no row.
- **IT-063**: Input at limit saves; input one code point over returns `input_too_long` with full draft retained.
- **IT-064**: Injected SQLCipher write denial returns `save_failed` and recovers the draft.
- **IT-065**: Concurrent save calls with one draft/idempotency key create one observation.
- **IT-066**: Termination at save failpoints recovers one saved row or one draft, never neither/both.
- **IT-067**: Repeated Enter after success returns the recorded observation without duplicates.
- **IT-068**: Capture save without active encounter returns `no_active_encounter` and retains draft.
- **IT-069**: Completion while capture is open requires explicit target confirmation; cancel keeps draft unsaved.
- **IT-070**: 10,000 current notes do not breach shortcut/open/save budgets and payload stays bounded.
- **IT-071**: Invalid card/tag text returns field-specific validation and preserves original observation.
- **IT-072**: Free-text-only observation completes end-to-end with empty structured collections.
- **IT-073**: Hundreds of cards/tags render summaries and paged details without data loss.
- **IT-074**: Offline missing card reference fixture still saves free card text and user tag.
- **IT-075**: Concurrent identical card/tag inserts consolidate by normalized key and retain contexts.
- **IT-076**: Termination with unsaved structured edits leaves committed free text unchanged and no false edited marker.
- **IT-077**: Repeated observed-card submission produces one normalized fact.
- **IT-078**: Changing suspected to observed keeps encounter provenance and increments revision/edited time.
- **IT-079**: Retiring a tag from suggestions preserves its historical display text.
- **IT-080**: 10,000 tags return bounded search pages and never load the full set.
- **IT-081**: Invalid edit returns validation error and prior observation revision remains visible.
- **IT-082**: Completing encounter without observations creates no placeholder.
- **IT-083**: Bulk review updates selected revisions only and preserves unmodified rows.
- **IT-084**: Injected mutation denial leaves edit/deletion unapplied and visible prior state intact.
- **IT-085**: Concurrent edit/delete yields one revision winner and explicit stale-action error.
- **IT-086**: Restart during undo period restores deadline; restart after purge does not resurrect data.
- **IT-087**: Repeated delete idempotency key returns the same tombstone.
- **IT-088**: Editing after profile merge updates original encounter while canonical profile view reflects it.
- **IT-089**: Editing observation under purged encounter returns `not_found`.
- **IT-090**: Editing one item in 100× history changes no unrelated ordering/revisions.
- **IT-091**: Malformed payload or non-allowlisted URL cannot become a confirmed snapshot.
- **IT-092**: Zero provider results shows no-snapshot state and manual deck action.
- **IT-093**: Multiple results choose newest exact-format candidate and preserve all displayed provenance.
- **IT-094**: Missing provider consent causes zero external requests and offers manual flow.
- **IT-095**: Out-of-order responses display only the current encounter generation/token.
- **IT-096**: Received candidate remains locally confirmable after disconnect with its original provenance only.
- **IT-097**: Repeated confirmation stores one snapshot for encounter/source token.
- **IT-098**: Format change invalidates old candidate before a new lookup.
- **IT-099**: Removing fixture source later does not remove confirmed historical snapshot.
- **IT-100**: Years of snapshots return latest in bounded query and page older results.

### Story Edge Cases: IT-101–IT-190

- **IT-101**: False-online fixture fails provider request once, degrades to manual, and leaves local commands usable.
- **IT-102**: Offline bootstrap with empty cache renders absence and no previous-opponent data.
- **IT-103**: Three transient failures pause automatic retry and expose one explicit Retry action.
- **IT-104**: Expired consent while offline queues no request for later connectivity.
- **IT-105**: Connectivity returning during manual confirmation cannot replace the confirmed manual identity.
- **IT-106**: Offline restart opens encrypted notebook and all local workflows without waiting on providers.
- **IT-107**: Retrying a failed source token creates no duplicate encounter/error record.
- **IT-108**: Connectivity after completion performs no closed-history enrichment without new confirmation.
- **IT-109**: Removing provider code leaves existing attributed snapshots readable and provider disabled.
- **IT-110**: 10,000 offline encounters do not schedule bulk lookups or block a new encounter.
- **IT-111**: SQL/FTS/script-like query text is bound as data and executes no syntax/content.
- **IT-112**: Zero-result query clears prior list and shows empty state.
- **IT-113**: Result set above page size returns stable cursor pages with no loss/duplication.
- **IT-114**: Search during restricted phase returns `disclosure_restricted` from host and no result payload.
- **IT-115**: Concurrent profile edit causes open search hit to report newer revision rather than overwrite form state.
- **IT-116**: Closing filtered review performs zero mutations and next bootstrap uses valid default view.
- **IT-117**: Repeating filter returns stable ordered IDs without duplicate rows.
- **IT-118**: Deep link to purged entity returns `not_found` and neutral history view.
- **IT-119**: Open profile merged concurrently redirects to canonical ID and preserves provenance.
- **IT-120**: 100× history meets query budget and DPAPI fixture proves another local user's database cannot be opened.
- **IT-121**: Missing, identical, malformed, or tombstoned merge IDs return `merge_conflict` with zero writes.
- **IT-122**: Empty profile merges as alias while other profile encounters remain unchanged.
- **IT-123**: 100× histories produce bounded preview counts plus paged conflict detail.
- **IT-124**: Injected write denial returns `save_failed`; both original profiles remain.
- **IT-125**: Concurrent overlapping merges commit one; loser receives `revision_conflict`.
- **IT-126**: Termination at merge failpoints recovers either two originals or one complete canonical profile.
- **IT-127**: Repeated merge idempotency key creates no duplicate aliases/encounters.
- **IT-128**: Unmerge after new encounters requires preview and assigns new records per confirmed plan.
- **IT-129**: Deleting target after preview invalidates apply with `revision_conflict`.
- **IT-130**: 10,000 aliases return bounded canonical suggestions with primary handle identified.
- **IT-131**: Invalid destination or empty passphrase/acknowledgement returns field error before snapshot/encryption.
- **IT-132**: Empty notebook backup requires explicit confirmation and manifest records zero profiles.
- **IT-133**: Simulated out-of-space preserves older backup and removes/invalidates partial.
- **IT-134**: Unwritable destination returns `destination_unwritable` with no notebook mutation.
- **IT-135**: Concurrent same-path backups require overwrite resolution and yield one decryptable final file.
- **IT-136**: Process kill/destination disconnect at each write failpoint leaves no valid-looking partial.
- **IT-137**: Repeated backup creates distinct version or requires confirmed overwrite.
- **IT-138**: Pending destructive deletion blocks backup with `operation_busy` until resolved.
- **IT-139**: Snapshot source failure aborts backup rather than emitting a complete manifest.
- **IT-140**: 100× backup streams progress, cancels safely, and leaves capture/save responsive.
- **IT-141**: Wrong passphrase, malformed header, or checksum failure leaves live database byte-equivalent.
- **IT-142**: Empty valid backup preview states zero records before merge/replace controls.
- **IT-143**: Insufficient staging/rollback space stops before live mutation.
- **IT-144**: Read denial or local write denial returns exact error and leaves notebook available.
- **IT-145**: Second concurrent restore receives `operation_busy`.
- **IT-146**: Termination at restore failpoints recovers old complete or new complete database, never partial.
- **IT-147**: Re-merging same backup leaves exact record counts stable.
- **IT-148**: `apply_restore` without validated preview token returns `invalid_request`.
- **IT-149**: Obsolete provider snapshots restore as historical data without enabling provider consent.
- **IT-150**: 100× restore keeps preview/progress bounded and rollback operable.
- **IT-151**: Invalid filename/path returns field error without reading notebook snapshot.
- **IT-152**: Empty selected scope requires warning confirmation before an empty file.
- **IT-153**: Simulated capacity exhaustion removes/marks partial and preserves any old destination.
- **IT-154**: Unwritable export path returns `destination_unwritable`; notebook unchanged.
- **IT-155**: Concurrent same-file exports require overwrite resolution and produce one complete UTF-8 file.
- **IT-156**: Kill/disconnect mid-export leaves no valid-looking final file.
- **IT-157**: Repeated export reflects current snapshot and creates no internal rows.
- **IT-158**: Stale selection of deleted profile returns `not_found` and refreshes active profiles.
- **IT-159**: Unsaved-edit guard offers save/discard/cancel; export begins only after resolution.
- **IT-160**: 100× export reports progress/cancel and does not exceed capture latency budget.
- **IT-161**: Invalid deletion ID returns `not_found` and changes no other rows.
- **IT-162**: Erase on empty notebook reports no-op and creates no destructive operation.
- **IT-163**: Large profile deletion preview reports exact dependent counts before confirmation.
- **IT-164**: Injected deletion write denial leaves all visible/searchable data intact.
- **IT-165**: Deletion conflicts with edit/export/backup according to coordinator and never snapshots half-purged state.
- **IT-166**: Restart before undo deadline restores tombstone; after effective purge it stays absent.
- **IT-167**: Repeated deletion key returns existing tombstone/deadline.
- **IT-168**: Profile deletion with pending merge returns dependency plan and requires resolution.
- **IT-169**: Late provider response for tombstoned generation cannot recreate profile/encounter.
- **IT-170**: 100× erase reports progress then leaves zero active/search/backup/export records.
- **IT-171**: Hostile diagnostic strings render escaped and redactor removes canary private values.
- **IT-172**: Empty log set previews environment-only manifest or explicit empty bundle.
- **IT-173**: 20 MiB diagnostic fixture previews summarized counts and remains cancelable.
- **IT-174**: Unwritable bundle destination returns error and performs no network action.
- **IT-175**: Concurrent bundles use separate snapshots/destinations and never mix entries.
- **IT-176**: Termination during bundle removes/invalidates partial artifact.
- **IT-177**: Repeated bundle creation performs zero network requests and reflects selected snapshot.
- **IT-178**: Create without valid preview token returns `invalid_request`.
- **IT-179**: Missing diagnostic source appears as omission code without notebook fallback data.
- **IT-180**: Full seven-day/20 MiB corpus passes canary redaction scan before output.
- **IT-181**: Partial, malformed, and unknown-format decks return explicit non-run reason and remain confirmable.
- **IT-182**: Multi-match signature fixture selects declared deterministic winner and explanation names decisive rule.
- **IT-183**: Exact-zero, exact-count, and default-min fixtures produce distinct expected matches.
- **IT-184**: Equal k-NN vote follows stable order; low-confidence tie returns `Unclassified`.
- **IT-185**: Tampered signature/schema/digest assets remain inactive and last valid classifier continues serving.
- **IT-186**: Kill after any reclassification batch leaves prior/new completed runs valid and resumes from cursor.
- **IT-187**: Repeating version/revision job creates one successful run by unique constraint.
- **IT-188**: Editing complete deck creates new revision/run while prior run stays attached to prior revision.
- **IT-189**: Unsupported format returns `Unclassified` and does not block snapshot confirmation/capture.
- **IT-190**: Thousands of decks reclassify in 25-record batches, pause for capture, resume, and publish bounded progress.

### Tauri Command Success Contracts: IT-191–IT-232

- **IT-191**: `bootstrap` from `main` returns safe app/settings/encounter state and current revision.
- **IT-192**: `get_settings` returns typed non-secret settings.
- **IT-193**: `update_settings` with current revision persists overlay/autostart/update preferences.
- **IT-194**: `list_providers` returns disclosed capabilities and independent consent/status.
- **IT-195**: `set_provider_consent` commits one versioned grant/revocation and stops revoked work.
- **IT-196**: `select_mtgo_window` stores only the authorized window selector/profile and starts provider.
- **IT-197**: `pause_detection` stops UIA/OCR activity without ending active encounter.
- **IT-198**: `confirm_opponent` resolves alias/profile and returns the active encounter summary.
- **IT-199**: `enter_opponent` creates/reuses profile and starts one encounter.
- **IT-200**: `correct_phase` commits transition and returns the newly filtered view revision.
- **IT-201**: `finish_encounter` commits completion and promoted observations.
- **IT-202**: `reopen_encounter` returns editable historical encounter without displacing a newer active one.
- **IT-203**: `undo_transition` reverses the compound new-opponent transition inside its window.
- **IT-204**: `open_capture` creates/activates one capture instance bound to active encounter.
- **IT-205**: `save_observation` commits one note and returns current-match projection.
- **IT-206**: `discard_draft` removes only the specified unsaved draft.
- **IT-207**: `update_observation` with current revision preserves encounter time and marks edited.
- **IT-208**: `set_card_observations` atomically replaces normalized observed/suspected entries.
- **IT-209**: `set_tendency_tags` atomically replaces normalized tag links.
- **IT-210**: `confirm_public_snapshot` stores one attributed snapshot and queues classification for a complete deck.
- **IT-211**: `save_complete_deck` creates immutable deck revision and classification job.
- **IT-212**: `search_history` outside restricted phase returns a stable cursor page.
- **IT-213**: `get_profile` returns chronological policy-safe profile detail.
- **IT-214**: `get_encounter` returns source/certainty/edit/incomplete provenance.
- **IT-215**: `get_deck_details` returns deck revision, provider label, and newest local classification separately.
- **IT-216**: `preview_merge` returns exact counts/conflicts/reassignment plan without mutation.
- **IT-217**: `apply_merge` commits chosen primary/aliases and preserved associations.
- **IT-218**: `preview_unmerge` includes post-merge records and proposed assignments.
- **IT-219**: `apply_unmerge` restores profiles according to the confirmed preview token.
- **IT-220**: `start_backup` returns operation ID and produces a decryptable final archive.
- **IT-221**: `preview_restore` validates/decrypts into staging and returns merge/replace effects.
- **IT-222**: `apply_restore` with preview token commits selected mode and rollback record.
- **IT-223**: `start_export` writes the selected UTF-8 plaintext scope after warning acknowledgement.
- **IT-224**: `cancel_operation` before commit cancels and cleans partial state.
- **IT-225**: `request_deletion` returns exact scope, undo deadline, and tombstone state.
- **IT-226**: `undo_deletion` within deadline restores active row and FTS entry.
- **IT-227**: `preview_diagnostics` returns files/field classes/redaction counts without private values.
- **IT-228**: `create_diagnostics` with preview token writes a locally saved, canary-clean bundle.
- **IT-229**: `get_classification` returns newest run plus prior version metadata and no editor controls.
- **IT-230**: `start_reclassification` schedules/resumes one low-priority job.
- **IT-231**: `check_update` when opted in returns signed release notes/classifier change summary.
- **IT-232**: `install_update` after confirmation verifies signature and invokes passive Windows installation.

### Tauri Command Failure Contracts: IT-233–IT-264

- **IT-233**: `bootstrap` with unsealable DPAPI blob returns `key_unavailable`, `retryable=false`, and no data.
- **IT-234**: `bootstrap` after unrecoverable migration returns `migration_failed` and rollback status.
- **IT-235**: Any stale mutable aggregate command returns `revision_conflict` with no partial write.
- **IT-236**: Consent-gated provider command returns `consent_required` before I/O.
- **IT-237**: `select_mtgo_window` for absent/stale window returns `window_not_found`.
- **IT-238**: Detection command with missing UIA/OCR capability returns `provider_unavailable` and manual fallback.
- **IT-239**: `confirm_opponent` for superseded generation returns `candidate_stale`.
- **IT-240**: Conflicting active normalized identity returns `identity_conflict` with resolution choices.
- **IT-241**: Impossible phase/completion/reopen action returns `invalid_transition`.
- **IT-242**: Transition/deletion undo after deadline returns `undo_expired`.
- **IT-243**: Capture/note command without active encounter returns `no_active_encounter`.
- **IT-244**: Blank save returns `blank_observation` with `field=text`.
- **IT-245**: Injected repository failure returns `save_failed`, `retryable=true`, and preserved draft.
- **IT-246**: Second capture claim returns `already_open` plus existing window identity.
- **IT-247**: Unknown/invalid card quantity or certainty returns `invalid_card` with field path.
- **IT-248**: Confirmation of old provider token returns `stale_provider_result`.
- **IT-249**: Classification request for partial deck returns `deck_incomplete`.
- **IT-250**: History command during gameplay returns `disclosure_restricted` and no payload.
- **IT-251**: Read/mutation of purged entity returns `not_found`.
- **IT-252**: Tampered search cursor returns `invalid_cursor`.
- **IT-253**: Invalid merge graph/preview returns `merge_conflict`.
- **IT-254**: Conflicting destructive/background operation returns `operation_busy`.
- **IT-255**: Restore with wrong passphrase returns `wrong_passphrase` without revealing archive validity details.
- **IT-256**: Malformed/checksum-invalid archive returns `invalid_backup` before live mutation.
- **IT-257**: Unwritable backup/export/diagnostic destination returns `destination_unwritable`.
- **IT-258**: Cancellation after commit point returns `cancel_unsafe` while operation finishes/recoveries safely.
- **IT-259**: Deletion confirmation for changed entity graph returns `scope_mismatch`.
- **IT-260**: Diagnostic canary leak returns `redaction_failed` and writes no final bundle.
- **IT-261**: Invalid classifier signature/schema/digest returns `assets_invalid` and last valid assets remain active.
- **IT-262**: Duplicate/manual classifier job while one runs returns `job_busy`.
- **IT-263**: No newer signed release returns `update_unavailable`.
- **IT-264**: Tampered release artifact returns `signature_invalid` and cannot install.

### Host Event Contracts: IT-265–IT-272

- **IT-265**: `encounter://state-v1` replaces the complete encounter summary at a strictly increasing revision.
- **IT-266**: `overlay://view-v1` contains only policy-allowed fields and replaces prior overlay state.
- **IT-267**: `capture://draft-v1` targets one encounter/window claim and preserves recoverable text.
- **IT-268**: `operation://progress-v1` reports monotonic completed/total/state values through terminal state.
- **IT-269**: `provider://status-v1` reports consent/availability/retry state without handles.
- **IT-270**: `classifier://progress-v1` reports version/cursor/count and no full deck data.
- **IT-271**: `update://status-v1` reports check/download/verify/install stages without device identifiers.
- **IT-272**: Unknown event major is discarded, local sensitive view is cleared, and `bootstrap` is requested.

### OS and External Boundaries: IT-273–IT-282

- **IT-273**: Synthetic UIA window yields expected opponent/phase evidence without process-memory/file/network access.
- **IT-274**: UIA-missing synthetic window invokes only configured visible OCR crops and yields the expected text.
- **IT-275**: DPI 100/125/150/200% transforms each OCR region to the same logical target.
- **IT-276**: Minimized/closed MTGO window stops capture within one scheduler tick and falls back conservatively.
- **IT-277**: OCR rate/backoff and detector CPU meet budgets on minimum Windows fixture.
- **IT-278**: Current-user DPAPI seals/unseals key; a different-user fixture cannot.
- **IT-279**: Packaged SQLCipher opens/migrates/backs up with encryption active on both supported Windows versions.
- **IT-280**: Local official-provider fixture verifies HTTPS allowlist, request minimization, retry, and response size/content validation.
- **IT-281**: Signed updater fixture accepts trusted artifact and rejects altered manifest/binary.
- **IT-282**: Packaged WebView2 bootstrap, tray, shortcut, autostart, multi-monitor position, and non-activating overlay pass Windows compatibility matrix.

## End-to-End Tests

- **E2E-001** (US-001): First launch → review local/privacy/provider/update disclosures → grant selected consent → disable provider/overlay → verify no further requests or automatic overlay while manual entry remains.
- **E2E-002** (US-002): Synthetic MTGO candidate → non-activating overlay → confirm existing alias and then a new handle → verify correct canonical profile and no restricted-history leak.
- **E2E-003** (US-003): Pause detector → enter handle → navigate bounded suggestions → select existing/create new/cancel → verify exactly the chosen persistent outcome.
- **E2E-004** (US-004): Drive pre-match → in-game → between-games → in-game → finished evidence plus manual correction → verify each visible phase and immediate fail-closed disclosure replacement.
- **E2E-005** (US-005): Start encounter A → confirm opponent B → verify atomic A completion/B start → undo/reopen → simulate missing end and restart → resolve incomplete encounter.
- **E2E-006** (US-006): Show compact overlay across every phase → verify permitted fields, non-activation, hide/disable persistence, bounded overflow, keyboard controls, and recovery after overlay restart.
- **E2E-007** (US-007): Global shortcut → focused capture → type note → Enter → see current-match note within five seconds; repeat with Escape and injected save failure to verify no save and preserved text.
- **E2E-008** (US-008): Save free-text-only note → add user deck label, observed/suspected cards, contexts, and custom tags → edit after encounter → verify source/certainty/provenance.
- **E2E-009** (US-009): Finish encounter → skip optional review → edit note → delete/undo → delete past deadline → verify absence from history, search, backup fixture, and export.
- **E2E-010** (US-010): Confirm opponent/format → receive official-source fixture result → inspect provenance → confirm snapshot → start later encounter and refresh → verify separate dated snapshots and separate user label.
- **E2E-011** (US-011): Disconnect all external boundaries → restart → manually run encounter, notes, history, backup, restore preview, and export → reconnect → verify only future consented lookups run.
- **E2E-012** (US-012): Outside gameplay search by handle/alias/deck/observed/suspected/tag/date/note → page results and open chronology → enter gameplay → verify every history entry point is host-denied.
- **E2E-013** (US-013): Create duplicate profiles → preview merge/conflicts → choose primary → confirm alias lookup → add post-merge encounter → preview and apply unmerge assignments → verify provenance throughout.
- **E2E-014** (US-014): Choose destination/passphrase → acknowledge no recovery → create encrypted backup → verify completion/path and that plaintext canaries do not occur on disk.
- **E2E-015** (US-015): Select encrypted backup → wrong then correct passphrase → inspect preview → apply merge and rollback fixture → repeat with replace → kill at failpoint and verify complete old/new state.
- **E2E-016** (US-016): Choose full and selected-opponent export → accept unencrypted warning → verify human-readable ordering/provenance and that restore rejects the `.txt` file.
- **E2E-017** (US-017): Delete observation with undo → delete encounter/profile with scoped confirmation → erase notebook → verify search/backup/export absence and later detection creates fresh history only.
- **E2E-018** (US-018): Generate events containing canary private data → preview diagnostics → verify prohibited fields absent → save locally → verify zero network requests and safe failure for a forced redaction leak.
- **E2E-019** (US-019): Confirm complete public/user deck fixtures → inspect signature and k-NN/Unclassified explanations → verify no configuration controls → install signed classifier update → observe resumable reclassification and retained prior runs.

## Release Evidence Gates

- Automated suites above are green on the pinned toolchain, including Windows-only and recovery suites.
- Packaged manual evidence proves MTGO selection/detection on each supported client profile without prohibited access.
- Overlay never activates MTGO on automatic display and never flashes historical/public data when entering gameplay.
- Keyboard-only and screen-reader smoke checks cover onboarding, confirmation, phase correction, capture, history navigation, portability, privacy, and update consent.
- Visual checks at 1280×1024 and DPI 100/125/150/200% confirm `DESIGN.md` tokens, clipping, contrast, focus, and multi-monitor placement.
- Installer and updater artifacts verify code signatures; classifier resources verify asset signature/digest; tampered artifacts fail closed.
- A clean-machine Windows 10 22H2 and Windows 11 run validates WebView2 bootstrap, SQLCipher packaging, DPAPI, Windows OCR language failure messaging, tray shutdown, and rollback cleanup.
- The official MTGO deck adapter remains disabled unless the access-validation spike records documented permission, stable response fixtures, rate limits, and allowed data fields.
