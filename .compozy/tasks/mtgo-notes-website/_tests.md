# Test Specification: MTGO Opponent Notes website

Canonical test contract for the brochure site. Companion to `_techspec.md`.
Derived from `_user_stories.md` (behavior) and `_techspec.md` (components).

## Strategy

- Frameworks and harnesses: Vitest for Functions and pure TS (`ReleaseKeys`, copy guards). `astro build` plus file assertions for static output. Playwright for visitor journeys. Fake `R2Bucket` at the I/O boundary only.
- Execution: `web/package.json` scripts `test` (Vitest), `test:build` (build + output assertions), `test:e2e` (Playwright). Site CI (`web.yml`) runs those on `web/**` changes. Windows `release.yml` does not run Astro tests.
- Conventions: table-driven Function cases. Forbidden-copy and required-phrase lists live in `web/tests/fixtures/copy.json` and are the only source of those strings in tests. Do not hit real Cloudflare, R2, or GitHub from unit/e2e.

## Coverage Matrix

| Source | Behavior | Unit | Integration | E2E |
|---|---|---|---|---|
| US-001 | Pitch, three beats, not a tracker | UT-012, UT-013 | IT-001, IT-002, IT-003, IT-010 | E2E-001 |
| US-001.EC-1 | Tracker contrast in first screen | — | IT-002 | E2E-001 |
| US-001.EC-2 | Forbidden vocabulary | UT-012 | IT-008 | — |
| US-001.EC-3 | Missing brand icon | — | IT-003 | — |
| US-001.EC-4 | Phone-width Home | — | — | E2E-006 |
| US-002 | Download CTA from this site | UT-001–UT-006 | IT-004, IT-005 | E2E-002 |
| US-002.EC-1 | GET zip while R2 empty → 302 | UT-002 | — | E2E-003 |
| US-002.EC-2 | Status fetch fails, href remains | — | — | E2E-004 |
| US-002.EC-3 | No JS, miss lands on empty state | UT-002 | — | E2E-003 |
| US-002.EC-4 | Zip without latest.json still streams | UT-003 | — | — |
| US-002.EC-5 | CTA is not GitHub / versioned filename | UT-013 | IT-005 | E2E-002 |
| US-003 | Live attach risk copy | UT-012 | IT-006 | E2E-005 |
| US-003.EC-1 | Required risk phrase missing | UT-012 | IT-006 | — |
| US-003.EC-2 | Videre comparison-only | UT-012 | IT-006 | — |
| US-003.EC-3 | `/live-attach` linked from chrome | — | IT-001 | E2E-005 |
| US-004 | Privacy / no signup / no telemetry | UT-012, UT-014 | IT-007, IT-009 | E2E-005 |
| US-004.EC-1 | Third-party tracker/font CDN | UT-014 | IT-009 | — |
| US-004.EC-2 | Missing export warning | UT-012 | IT-007 | — |
| US-005 | Stable `latest` object | UT-007–UT-009 | IT-004 | E2E-002 |
| US-005.EC-1 | Versioned only → unavailable | UT-005 | — | — |
| US-005.EC-2 | no-store on zip response | UT-006 | — | — |
| US-006 | Tag publish writes three keys | UT-010, UT-011 | IT-011 | — |
| US-006.EC-1 | Manual upload is not the supported path | UT-011 | — | — |
| US-006.EC-2 | No zip committed under `web/` | — | IT-012 | — |
| US-006.EC-3 | No zip in `dist/` | — | IT-004 | — |
| `DownloadWindowsFn` | Zip stream / 302 / 405 | UT-001–UT-004, UT-015 | — | — |
| `DownloadStatusFn` | Always-200 status | UT-005, UT-007–UT-009, UT-016 | — | — |
| `ReleaseKeys` | Key layout | UT-010 | — | — |
| `Copy` | Required / forbidden phrases | UT-012 | IT-008 | — |
| `BrandMark` / favicon | Icon in output | — | IT-003 | E2E-001 |
| `BaseLayout` | No third-party tags | UT-014 | IT-009 | — |
| `ReleasePublish` | Tag-only latest, fail on put error | UT-011 | IT-011 | — |
| GET `/download/status` | Success + unavailable shapes | UT-007–UT-009 | — | E2E-002 |
| GET `/download/windows` | Success + miss + 405 | UT-001–UT-004, UT-015 | — | E2E-002, E2E-003 |

## Unit Tests

### `DownloadWindowsFn` (TechSpec: Core Interfaces / API Endpoints)

- **UT-001** (happy): `onRequestGet` with `RELEASES.get(latestZip)` returning a body and `latest.json` `{ version: "0.2.1", filename: "MTGONotes-0.2.1-win-x64.zip" }` — responds 200, `Content-Type: application/zip`, `Content-Disposition` contains `MTGONotes-0.2.1-win-x64.zip`, body is the object stream.
- **UT-002** (error): `onRequestGet` with `get(latestZip)` `null` — responds 302 with `Location` exactly `/download?available=0`.
- **UT-003** (happy): `onRequestGet` with zip present and `get(latestMeta)` `null` — responds 200 zip; `Content-Disposition` uses `MTGONotes-win-x64.zip` (or equivalent generic filename), does not throw.
- **UT-004** (error): `onRequestGet` when `RELEASES.get` throws — responds 302 `/download?available=0` (visitor still does not see a raw 404).
- **UT-006** (happy): successful zip response includes `Cache-Control: private, no-store`.
- **UT-015** (error): `onRequest` with method `POST` — responds 405.

### `DownloadStatusFn` (TechSpec: Core Interfaces)

- **UT-005** (error): zip missing, versioned key ignored — body is `{ "available": false }` and status 200.
- **UT-007** (happy): zip present and `latest.json` `{ version: "0.2.1", filename: "MTGONotes-0.2.1-win-x64.zip", uploadedAt: "2026-08-17T00:00:00.000Z" }` — 200 `{ available: true, version: "0.2.1", filename: "MTGONotes-0.2.1-win-x64.zip", uploadedAt: "2026-08-17T00:00:00.000Z" }`.
- **UT-008** (boundary): zip present, meta missing — 200 `{ available: true }` with `version` omitted.
- **UT-009** (error): meta present, zip missing — 200 `{ available: false }` (do not advertise a version you cannot serve).
- **UT-016** (error): `RELEASES.get` throws — 200 `{ available: false }`.

### `ReleaseKeys` (TechSpec: Data Models)

- **UT-010** (happy): `ReleaseKeys.versioned("0.2.1")` equals `releases/windows/MTGONotes-0.2.1-win-x64.zip`; `latestZip` equals `releases/windows/latest.zip`; `latestMeta` equals `releases/windows/latest.json`.

### `ReleasePublish` helpers (TechSpec: Integration Points / GitHub Releases)

- **UT-011** (boundary): a pure helper `shouldPublishLatest(refType)` returns `true` for `"tag"` and `false` for `"branch"` (covers tag-only latest; `workflow_dispatch` must not overwrite).

### `Copy` (TechSpec: Copy component)

- **UT-012** (error): `assertAllowedCopy(sample)` rejects strings containing `tournament-safe`, `ban-proof`, `auto-updater`, and the phrase `their current deck`; accepts the required live-attach phrases `Not legal advice`, `Not affiliated`, `Not tournament-approved`, and `unofficial`.
- **UT-013** (happy): `primaryDownloadHref` exported for the CTA is exactly `/download/windows`.

### `BaseLayout` policy helper

- **UT-014** (error): `findForbiddenThirdParty(html)` flags `googletagmanager.com`, `google-analytics.com`, `fonts.googleapis.com`, `plausible.io`, `cloudflareinsights.com` (visitor RUM), and `doubleclick.net`.

## Integration Tests

### Static build output

- **IT-001**: `astro build` emits `dist/index.html`, `dist/download/index.html`, `dist/how-it-works/index.html`, `dist/live-attach/index.html`, `dist/privacy/index.html`. Each HTML file contains the header nav links to those five paths and the footer affiliation sentence.
- **IT-002**: `dist/index.html` contains the three beat headings (confirm opponent, fast capture, recall between games) and a board-logger / tracker contrast sentence from `copy.ts`.
- **IT-003**: `dist/favicon.ico` exists; `dist/brand/icon.png` exists; every HTML file references `/favicon.ico` and `/brand/icon.png`.
- **IT-004**: `dist/` contains no `*.zip`. `dist/download/index.html` primary CTA `href` is `/download/windows`.
- **IT-005**: no built HTML file contains `github.com/MatheusBBarni/mtgo-notes/releases/download/` as a primary CTA href. A GitHub Releases URL may appear only on `/download` as secondary text/link.
- **IT-006**: `dist/live-attach/index.html` contains `optional`, `read-only`, `LogOn` (as something the app does **not** call), `Not legal advice`, `Not affiliated`, `Not tournament-approved`, and `unofficial`. It does not contain `tournament-safe` or `ban-proof`.
- **IT-007**: `dist/privacy/index.html` contains `no signup` (or equivalent from `copy.ts`), `telemetry`, and the unencrypted text-export warning.
- **IT-008**: concatenating all `dist/**/*.html` and running the forbidden-copy fixture fails the build test if any forbidden term from `copy.json` appears outside an allowed “do not say this” quotation.
- **IT-009**: no built HTML file includes a `script[src]` or `link[href]` whose host is in the UT-014 denylist. Inter is self-hosted (filename under `dist/_astro/` or `dist/fonts/`).
- **IT-010**: `dist/how-it-works/index.html` states confirm-before-persist, history hidden during possible gameplay, and never labels public data “their current deck.”

### Release workflow

- **IT-011**: `.github/workflows/release.yml` contains an R2 upload step gated on tag / `github.ref_type == 'tag'`, puts the three keys from `ReleaseKeys`, and does not `continue-on-error: true` on that step.
- **IT-012**: `web/` (excluding `node_modules` and fixtures) contains no `*.zip` and no `MTGONotes.App.exe`.

## End-to-End Tests

### First-visit understanding (US-001)

- **E2E-001**: Open `/` → see pitch, three beats, tracker contrast, header icon, primary Download control pointing at `/download/windows` → footer shows unofficial / not affiliated / GitHub.

### Successful download (US-002, US-005)

- **E2E-002**: Stub `GET /download/status` → `{ available: true, version: "0.2.1", filename: "MTGONotes-0.2.1-win-x64.zip" }` and stub `GET /download/windows` → 200 zip. Open `/` and `/download` → version `0.2.1` is visible on Download → click primary CTA → browser request path is `/download/windows`, not a GitHub URL.

### Empty state (US-002.EC-1, US-002.EC-3)

- **E2E-003**: Stub status `{ available: false }`. Open `/download?available=0` and `/` → empty-state copy is visible → primary control is not an href that 404s. Visiting `/download/windows` (unstubbed Function miss or stubbed 302) lands on `/download?available=0`.

### Status failure fallback (US-002.EC-2)

- **E2E-004**: Stub `/download/status` → network abort. Open `/` → primary CTA `href` remains `/download/windows`.

### Risk and privacy routes (US-003, US-004)

- **E2E-005**: From Home nav, open `/live-attach` then `/privacy` → required risk phrases and privacy promises are visible.

### Narrow viewport (US-001.EC-4)

- **E2E-006**: Viewport 390×844 on `/` → body copy does not overflow horizontally; Download CTA is visible and clickable.
