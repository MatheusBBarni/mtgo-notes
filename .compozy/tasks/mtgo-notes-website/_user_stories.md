# User Stories: MTGO Opponent Notes website

Canonical behavior catalog for the brochure site. Companion to `_prd.md`; consumed by `_techspec.md` for component mapping and `_tests.md` for the coverage matrix.

## Personas

- **Notes-first MTGO player** — Wants the Windows zip and a 30-second explanation.
- **Comparator** — Needs “not Videre / not a board logger” immediately.
- **Risk-sensitive player** — Will bounce if live attach is hidden or oversold.
- **Privacy-conscious player** — Needs confirmation there is no signup and notes stay local.
- **Maintainer** — Tags a release and expects R2 + the site button to update without a manual upload.

## Story Index

| ID | Feature Area | Persona | Story |
|---|---|---|---|
| US-001 | Brochure | Notes-first / Comparator | Understand what the companion does and why it is not a tracker |
| US-002 | Download | Notes-first player | Download the latest Windows build from a button on the site |
| US-003 | Live attach | Risk-sensitive player | Read how live attach works and what risk it carries |
| US-004 | Privacy | Privacy-conscious player | Confirm there is no signup and notes are not uploaded |
| US-005 | Download freshness | Returning visitor | Receive the current zip via the stable `latest` object |
| US-006 | Release publish | Maintainer | A tagged GitHub Release publishes the zip to R2 automatically |

## Brochure

### US-001: Understand what the companion does and why it is not a tracker

**As a** notes-first MTGO player or comparator, **I want** to read the product pitch and the tracker contrast on one screen, **so that** I know this is a private notes companion and not a board logger.

Acceptance criteria:

- AC-1: Given I open `/`, when the first viewport loads, then I see the pitch, the primary Download CTA, and exactly three beats: confirm opponent, fast capture, recall between games.
- AC-2: Given I read Home, when I look for product contrast, then the copy states that the companion stores *my* observations and does not log the board.
- AC-3: Given I scan the first screen, when I compare it to Videre, then the site does not present itself as a Videre clone; any Videre mention is comparison only.
- AC-4: Given any page, when I read the footer, then I see unofficial / not affiliated with WotC or Daybreak, and a link to the GitHub source.
- AC-5: Given any page, when I look at chrome, then the notebook/card brand icon is the favicon and appears in the header.

Edge cases:

- EC-1: Visitor reads only the hero and CTA → the tracker contrast is still visible without scrolling past the first screen.
- EC-2: Copy accidentally uses forbidden vocabulary (`account` for the player, `sync`, `tournament-safe`, `current deck`) → the page fails the copy contract.
- EC-3: Brand icon file is missing from the static output → favicon and header mark must not silently fall back to a generic browser icon without a test failure.
- EC-4: Viewport is a phone-width window → Home remains readable; CTA stays usable; layout does not require horizontal scroll for body copy.

## Download

### US-002: Download the latest Windows build from a button on the site

**As a** visitor, **I want** a Download button on this site that gives me the Windows zip, **so that** I never have to open GitHub to install.

Acceptance criteria:

- AC-1: Given a published `latest` object, when I click the primary CTA on Home or Download, then the browser downloads the zip from a Cloudflare URL (`/download/windows`), not from `github.com/.../releases`.
- AC-2: Given no published object, when I view Home or Download, then the CTA shows an empty state and is not a dead link or a raw 404.
- AC-3: Given I open `/download`, when a build is available, then I see the version string, Windows 10 22H2 / Windows 11 x64 requirements, and unzip/`MTGONotes.App.exe` instructions.
- AC-4: Given I open `/download`, when I look for GitHub, then I may see a secondary “also on GitHub Releases” control; it is not the primary CTA.
- AC-5: Given Download copy, when I look for updater claims, then the site does not claim an auto-updater.

Edge cases:

- EC-1: Direct GET `/download/windows` while R2 is empty → 302 to `/download?available=0`, never a raw 404 body as the visitor-facing outcome.
- EC-2: `/download/status` fetch fails in the browser → the CTA still targets `/download/windows`; the Function handles a miss with the empty-state redirect.
- EC-3: Visitor has JavaScript disabled → clicking the CTA still reaches the Function; a miss lands on the Download empty state via query param.
- EC-4: Zip exists but `latest.json` is missing → the Function still streams the zip; the version label may be omitted rather than blocking the download.
- EC-5: Primary CTA `href` is a versioned filename or a GitHub release asset URL → contract fail.

### US-005: Receive the current zip via the stable `latest` object

**As a** returning visitor, **I want** the same Download button to always mean “current build”, **so that** I do not keep an old versioned filename.

Acceptance criteria:

- AC-1: Given two tagged releases, when I click Download, then I receive the object stored at the stable `latest` key, not a URL that embeds the previous version.
- AC-2: Given the Download page shows a version string, when a newer tag has published, then the status endpoint reports the newer version.

Edge cases:

- EC-1: Versioned object exists and `latest` does not → status reports unavailable; CTA uses the empty state.
- EC-2: `latest` is overwritten by a newer tag → subsequent downloads serve the new bytes; `Cache-Control` on the Function response is `private, no-store`.

## Live attach

### US-003: Read how live attach works and what risk it carries

**As a** risk-sensitive player, **I want** a dedicated Live attach page, **so that** I can decide before I turn it on.

Acceptance criteria:

- AC-1: Given I open `/live-attach`, when I read the page, then it states: optional; off means manual notes; on is read-only attach to an already-logged-in client; no `LogOn`, password, chat, queue, or concede.
- AC-2: Given that page, when I read risk copy, then I see unofficial, Daybreak EULA discretion / accounts may still be terminated, “Not legal advice,” “Not affiliated,” and “Not tournament-approved.”
- AC-3: Given any page, when I search the HTML, then I do not find “tournament-safe,” “tournament-approved” as a claim of safety, or “ban-proof.”

Edge cases:

- EC-1: Required risk phrase is missing from `/live-attach` → contract fail.
- EC-2: Videre is mentioned → it is only as a class-of-inspect comparison (“we read less”), never as affiliation or equivalence of product purpose.
- EC-3: Home hides live attach entirely → nav or How it works must still expose `/live-attach` as a first-class route.

## Privacy

### US-004: Confirm there is no signup and notes are not uploaded

**As a** privacy-conscious player, **I want** a Privacy page that states the local-first boundary, **so that** I know the site and the app do not take an account or my notebook.

Acceptance criteria:

- AC-1: Given I open `/privacy`, when I read the page, then I see: no signup, no telemetry, notes stay on the machine, backups are user-made, text export is unencrypted and warned.
- AC-2: Given the built site, when I inspect documents and network policy, then there are no visitor accounts, no analytics/telemetry pixels, and no third-party tracker scripts.
- AC-3: Given the site, when I look for a notes inbox, then the site does not accept or host other players’ notes.

Edge cases:

- EC-1: A third-party script host (analytics, tag manager, font CDN that phones home as a tracker) appears in built HTML → contract fail.
- EC-2: Privacy page omits the unencrypted-export warning → contract fail.

## How it works

How it works is not its own story ID; it supports US-001 and US-003.

- `/how-it-works` explains: local notebook, confirm-before-persist, click-through overlay, history hidden during possible gameplay, consented public-deck preview never labeled “their current deck.”

## Release publish

### US-006: A tagged GitHub Release publishes the zip to R2 automatically

**As a** maintainer, **I want** the existing tagged release workflow to upload versioned + `latest` objects, **so that** I never upload the zip to R2 by hand.

Acceptance criteria:

- AC-1: Given a successful `v*` tag build, when the workflow finishes, then R2 contains `releases/windows/MTGONotes-<version>-win-x64.zip`, `releases/windows/latest.zip`, and `releases/windows/latest.json`.
- AC-2: Given that same run, when I open GitHub Releases, then the GitHub Release still exists with the zip attached as the CI archive.
- AC-3: Given `workflow_dispatch` (non-tag), when the job runs, then it must not overwrite the production `latest` objects.
- AC-4: Given the R2 upload fails after the GitHub Release is created, when the workflow ends, then the job is failed so the miss is visible.

Edge cases:

- EC-1: Manual dashboard upload is not documented as the supported path; the workflow is the supported path.
- EC-2: Binaries are committed under `web/` → contract fail.
- EC-3: Zip is copied into Pages `dist/` → contract fail (25 MiB Pages limit).
