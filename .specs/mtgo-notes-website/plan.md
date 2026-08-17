# Requirements Document: MTGO Opponent Notes website

**Source:** [GitHub issue #21](https://github.com/MatheusBBarni/mtgo-notes/issues/21), `.compozy/tasks/mtgo-notes-website/`, `CONTEXT.md`, `DESIGN.md`, grill-me 2026-04-07.

**Approved:** 2026-04-07

## Objective

Give the Windows companion a public face: a static brochure that explains a private, local-first notes app (not a tracker, not an MTGO client, not a Videre clone) and lets a visitor download the latest Windows x64 zip from this site via Cloudflare R2 — never as a GitHub-primary CTA and never as a Pages static asset.

**Pitch:** A private, local-first Windows companion that helps you remember MTGO opponents and review verifiable public context — without becoming an MTGO account client.

## Expected behavior

### Site

- Five routes, one layout: `/`, `/download`, `/how-it-works`, `/live-attach`, `/privacy`.
- Header: brand icon + wordmark **MTGO Opponent Notes**, nav to all five routes, skip link.
- Footer on every page: unofficial; not affiliated with WotC or Daybreak; GitHub source.
- Favicon + header mark from the existing notebook/card icon (copied into `web/public/`, never read from `__oldversion__/` at build).
  - Source: `__oldversion__/assets/icons/mtgo-notes-transparent.png` and `mtgo-notes.ico`.
  - Canonical site copies: `web/public/brand/icon.png`, `web/public/favicon.ico`.

### Home (first viewport)

- Compact white hero: pitch, primary Download CTA, tracker contrast (“your observations / not a board logger”).
- Tight 3-up beat row in that same first screen: **confirm opponent**, **fast capture**, **recall between games**.
- No full-bleed coral/forest/dark signature bands on Home.

### Download

- Same primary CTA as Home: **Download for Windows**, `href="/download/windows"`.
- When a build exists: show version, Windows 10 22H2 / Windows 11 x64, unzip `MTGONotes.App.exe`. Live attach optional; MTGO already logged in if used. No auto-updater claim.
- Small secondary text link to GitHub Releases (not the primary CTA).
- **Empty state (no zip):** visually a **disabled** near-black primary button (no href) plus **“A Windows build is not published yet.”** Direct `GET /download/windows` still 302s to `/download?available=0` so no-JS and status-fetch failure never 404. `/download?available=0` server-renders the disabled state.

### How it works

- Local notebook, confirm-before-persist, click-through overlay, history hidden during possible gameplay, consented public-deck preview never labeled “their current deck.”
- Cream placeholder frames captioned **Screenshot forthcoming**.

### Live attach

- Off = manual notes. On = read-only attach to an already-logged-in client. No `LogOn`, password, chat, queue, or concede. Same *class* of inspect as Videre; we read less.
- Required phrases: unofficial; Daybreak may terminate accounts / EULA discretion; **Not legal advice**; **Not affiliated**; **Not tournament-approved**.
- One **dark signature card** holds the risk callout.

### Privacy

- No signup, no telemetry, notes stay on the machine, backups are user-made, text export is unencrypted and warned.
- Plain white editorial. No signature cards. No visitor accounts, pixels, or third-party tracker/font CDNs.

### Download pipeline

- `GET /download/status` → always 200 `ReleaseStatus`.
- `GET /download/windows` → stream `latest.zip` or 302 `/download?available=0`. Other methods 405.
- Missing `latest.json` must not block a present zip. Meta without zip → `available: false`.
- Zip `Cache-Control: private, no-store`. Filename from meta, else `MTGONotes-win-x64.zip`.
- Tagged `release.yml` puts versioned zip + `latest.zip` + `latest.json`. GitHub Release still created first. Non-tag `workflow_dispatch` must not overwrite `latest`. R2 put failure fails the job.

## Visual / UX

- `DESIGN.md` tokens: canvas `#ffffff`, ink `#181d26`, hairline borders, scarce shadows, no gradients.
- Primary CTA = `button-primary`: near-black, white label, 16×24, **12px** corners (not `{rounded.pill}`).
- Inter self-hosted (ADR-004). Desktop-first, readable at 390px without horizontal body scroll.
- `CONTEXT.md` vocabulary table is mandatory.

## Edge cases

- First deploy has no zip → empty state is a launch requirement.
- Status fetch fails → href stays `/download/windows`; Function 302s on miss.
- JS disabled → same Function path; `?available=0` shows empty state.
- Zip without meta → still download; version label may be omitted.
- Meta without zip → unavailable.
- Versioned object only, no `latest` → unavailable.
- Forbidden copy (`tournament-safe`, `ban-proof`, `auto-updater`, “their current deck”, `account` for the player, `sync` as scrape) fails CI.
- No `.zip` in `web/` or `dist/`.
- Brand icon missing from output fails CI.

## Stack

- Astro static in `web/`. Cloudflare Pages project `mtgo-notes`, Node 22, output `dist`, no `@astrojs/cloudflare` adapter.
- Pages Functions: `web/functions/download/{windows,status}.ts`. Binding `RELEASES` → private R2 `mtgo-notes-releases`.
- Vitest (Functions + copy guards), `astro build` output assertions, Playwright with stubbed download routes.
- Site CI: `web.yml` on `web/**`. Windows `release.yml` does not run Astro tests.

## Constraints / non-goals

- No auth, CMS, app backend, telemetry, custom domain, auto-updater, in-browser attach, or zip-as-Pages-asset.
- No claims of official approval, tournament safety, or that bans are impossible.
- WinUI / Core / Data / Live are untouched.

## This pipeline vs operator

| Agent (this cycle) | Operator |
|---|---|
| `web/` site, Functions, tests, `web.yml`, `release.yml` R2 steps | Pages project, R2 bucket, `RELEASES` binding, `CLOUDFLARE_*` secrets |
| Empty-state production is a valid first publish | Live zip only after a later tagged release |

## Out of scope for this cycle

- Custom domain, live tagged zip, real product screenshots (placeholders only), sibling repo, Workers migration.

## User stories

- **US-001** — As an MTGO player, I can read what the companion does and why it is not a tracker.
- **US-002** — As a visitor, I can download the latest Windows build from a button on the site.
- **US-003** — As a cautious player, I can read how live attach works and what risk it carries before I turn it on.
- **US-004** — As a privacy-conscious player, I can confirm there is no signup and notes are not uploaded.
- **US-005** — As a returning visitor, I get the current zip (stable `latest` object), not a stale versioned filename.
- **US-006** — As a maintainer, a tagged GitHub Release automatically publishes that zip to R2 so I do not upload by hand.

## Language

| Use | Do not use |
|---|---|
| Player identity | User profile, account |
| Opponent profile | Account |
| Public player result / lookup | Sync, match history scrape |
| Imported public result | Editable / current deck |
| Tournament-conservative | Tournament-safe / approved |
