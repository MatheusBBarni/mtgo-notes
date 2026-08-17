# Product Requirements Document: MTGO Opponent Notes website

Canonical V1 product contract. Source: [GitHub issue #21](https://github.com/MatheusBBarni/mtgo-notes/issues/21). Downstream TechSpec, tasks, and implementation treat the rules below as binding.

## Overview

MTGO Opponent Notes has a Windows companion and GitHub Releases, but no public face. A visitor cannot tell what the product is, how it differs from a tracker, or how to get a build without reading the repo.

V1 ships a static marketing/docs site plus a first-party download: the latest Windows zip lives on Cloudflare R2, and the site’s Download button is a Cloudflare URL that fetches that object. The site explains a private, local-first notes companion. It is not a web app, not an MTGO client, and not a Videre clone.

**Pitch:** A private, local-first Windows companion that helps you remember MTGO opponents and review verifiable public context — without becoming an MTGO account client.

## Goals

After this ships, a visitor can:

- Understand the product in one screen: confirm an opponent, capture a note in under five seconds, recall history between games (not during).
- Download the latest Windows x64 zip from **this site**, without opening GitHub.
- See that live attach is optional, unofficial, read-only, and can be turned off.
- See that notes stay on their machine: no account, no telemetry.

The system guarantees:

- The primary Download CTA is a Cloudflare URL backed by R2, never a hardcoded GitHub commit.
- GitHub Releases still exist as the CI archive; they are not the visitor’s main download.
- Copy never claims Daybreak approval, tournament safety, or that bans are impossible.
- Vocabulary matches `CONTEXT.md`.

## User stories

- **US-001** — As an MTGO player, I can read what the companion does and why it is not a tracker.
- **US-002** — As a visitor, I can download the latest Windows build from a button on the site.
- **US-003** — As a cautious player, I can read how live attach works and what risk it carries before I turn it on.
- **US-004** — As a privacy-conscious player, I can confirm there is no signup and notes are not uploaded.
- **US-005** — As a returning visitor, I get the current zip (stable `latest` object), not a stale versioned filename.
- **US-006** — As a maintainer, a tagged GitHub Release automatically publishes that zip to R2 so I do not upload by hand.

## Core features

### Brochure site

Static pages covering:

1. **Home** — Pitch, primary Download CTA, three beats only (confirm opponent, fast capture, recall between games). Contrast: we store *your* observations; we do not log the board.
2. **Download** — Same R2-backed button, version string, requirements (Windows 10 22H2 / Windows 11 x64; unzip `MTGONotes.App.exe`). Live attach optional; MTGO must already be logged in if used. Optional small “also on GitHub Releases” text. No auto-updater claim.
3. **How it works** — Local notebook, confirm-before-persist, click-through overlay, history hidden during possible gameplay, consented public-deck preview never labeled “their current deck.”
4. **Live attach** — Off = manual notes. On = read-only attach to an already-logged-in client. No `LogOn`, password, chat, queue, or concede. Same *class* of process inspect as Videre; we read less. Daybreak may still terminate accounts. “Not legal advice.” “Not affiliated.” “Not tournament-approved.”
5. **Privacy** — No signup, no telemetry. User-made backups. Text export is unencrypted and warned.

Footer on every page: unofficial; not affiliated with WotC or Daybreak; source on GitHub.

Brand mark: the existing notebook/card icon (`__oldversion__/assets/icons/mtgo-notes-transparent.png` and `mtgo-notes.ico`) is the favicon and appears in site chrome.

### First-party Windows download

The issue is **not done** if the site only links to GitHub.

1. After a successful tagged release, CI uploads `MTGONotes-<version>-win-x64.zip` to R2 (versioned key **and** stable `latest` key).
2. Home and Download CTAs use a Cloudflare URL (Pages Function) that serves the `latest` object.
3. If no object exists yet, the CTA shows an empty state — never a 404 or a dead link.

## Business rules

### Language

| Use | Do not use |
|---|---|
| Player identity | User profile, account |
| Opponent profile | Account |
| Public player result / lookup | Sync, match history scrape |
| Imported public result | Editable / current deck |
| Tournament-conservative | Tournament-safe / approved |

### Download and storage

1. Visitors download from Cloudflare, not from `github.com/.../releases` as the primary CTA.
2. The zip must not be a Cloudflare **Pages** static asset (Pages max file size is **25 MiB**; the WinUI zip will exceed it).
3. The zip **must** live in R2 Standard storage.
4. R2 keeps a versioned object and a stable latest object; latest is what the site button hits.
5. GitHub Releases remain the build record and long-term archive so R2 does not hoard every old build.
6. Binaries are never committed to the site repo.
7. Manual R2 uploads are not the supported long-term path; `.github/workflows/release.yml` publishes to R2 after a successful tagged build.

### Claims and risk

1. The site must not claim official approval, tournament safety, or that account termination is impossible.
2. Live attach copy must state: optional, read-only, no login, unofficial, EULA discretion, not legal advice.
3. Public deck data must not be described as the opponent’s current deck.

### Privacy

1. No visitor accounts, no cloud notebook, no analytics/telemetry pixels, no third-party tracker scripts.
2. The site does not accept or host other players’ notes.

## User experience

**Personas**

- Notes-first MTGO player: wants the zip and a 30-second explanation.
- Comparator: needs “not Videre / not a board logger” immediately.
- Risk-sensitive player: will bounce if live attach is hidden or oversold.

**Primary flow**

1. Land on Home.
2. Read pitch + three beats.
3. Click Download → receive the latest zip from R2.
4. Optionally read Live attach / Privacy before installing.

**Visual**

Follow `DESIGN.md`: canvas `#ffffff`, ink `#181d26`, hairline borders, near-black pill primary CTA, outlined secondary, scarce shadows, no gradients. Desktop-first, readable on a phone. Real screenshots when available; placeholders OK for first publish. Type uses Inter as the licensed-safe substitute for Haas (see TechSpec ADR-004).

## High-level technical constraints

- **Site host:** Cloudflare Pages (preview + production). Static site.
- **Binary host:** Cloudflare R2, Standard class (free-tier eligible). Not Pages assets.
- **Download URL:** Cloudflare Pages Function → R2.
- **Build source:** existing GitHub Release workflow produces the zip; extend it to upload to R2.
- **Repo:** `web/` in this repository.
- No auth, CMS, or app backend for V1.

## Non-goals

- User accounts, cloud sync, web notebook
- Hosting other players’ notes or public dossiers
- Metagame / collection / trade / replay pages
- In-browser live attach
- Shipping the zip as a Pages static file
- Claiming official safety or “ban-proof”
- Auto-updater
- Custom domain (Pages can add it later; not required to close this PRD)

## Decisions already made

- Brochure site + first-party download, not a web product.
- Cloudflare Pages for the site; R2 for the zip; site button points at R2.
- GitHub Releases stay as the CI archive.
- Videre is a layout comparison only.
- `CONTEXT.md` vocabulary and `DESIGN.md` visual system are mandatory.
- Static Astro in `web/`; private R2; Pages Function at `/download/windows`.
- Separate routes for Home, Download, How it works, Live attach, Privacy.
- Inter ships instead of Haas Grotesk.
- Existing app icon is favicon and on-page brand mark.

## Acceptance

- [ ] PRD-complete site on Cloudflare Pages (preview + production).
- [ ] Pages cover Home, Download, How it works, Live attach, Privacy.
- [ ] Release workflow uploads the zip to R2 (versioned + `latest`).
- [ ] Home and Download CTAs use the Cloudflare download URL; clicking downloads the app.
- [ ] Empty state if no zip exists yet.
- [ ] GitHub Release still created; not the primary CTA.
- [ ] Live-attach copy includes unofficial / EULA discretion / not legal advice. No “tournament-safe.”
- [ ] Vocabulary and `DESIGN.md` constraints held.
- [ ] No telemetry, auth, or third-party trackers.
- [ ] Brand icon is the favicon and appears in site chrome.
