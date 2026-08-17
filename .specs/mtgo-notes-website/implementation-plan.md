# TDD Plan: MTGO Opponent Notes website

**Approved:** 2026-04-07  
**Requirements:** `.specs/mtgo-notes-website/plan.md`

`web/` does not exist yet. WinUI / Core / Data / Live stay untouched. Tests talk only to the seams below — not Astro internals, CSS class names, or a live Cloudflare account.

## Public interface

```ts
export interface ReleaseStatus {
  available: boolean;
  version?: string;
  filename?: string;
  uploadedAt?: string;
}

export const ReleaseKeys = {
  versioned: (version: string) =>
    `releases/windows/MTGONotes-${version}-win-x64.zip`,
  latestZip: "releases/windows/latest.zip",
  latestMeta: "releases/windows/latest.json",
} as const;

export const primaryDownloadHref = "/download/windows";
export function shouldPublishLatest(refType: string): boolean;
export function assertAllowedCopy(sample: string): void;
export function findForbiddenThirdParty(html: string): string[];
```

HTTP (Pages Functions, fake `R2Bucket` in tests):

| Method | Path | Observable result |
|---|---|---|
| GET | `/download/windows` | 200 zip stream **or** 302 `/download?available=0` |
| GET | `/download/status` | 200 `ReleaseStatus` (miss is `{ available: false }`, never 404) |
| other | both | 405 |

Visitor documents (Astro static output / Playwright):

- `GET /`, `/download`, `/how-it-works`, `/live-attach`, `/privacy`
- `DownloadCta`: default `<a href="/download/windows">Download for Windows</a>`; empty state = disabled button (no href) + “A Windows build is not published yet.”

## Seams (test only these)

1. **Download Functions** — HTTP request in, status/headers/body out. Fake R2 at the I/O boundary only.
2. **`ReleaseKeys` / `shouldPublishLatest`** — key strings and tag-only publish gate.
3. **`Copy` policy** — required/forbidden phrases and `primaryDownloadHref`.
4. **Built `dist/`** — routes, chrome, brand files, no zip, no third-party hosts.
5. **`release.yml` text** — tag-gated R2 puts of the three keys; no `continue-on-error`.
6. **Playwright document** — visitor journeys with stubbed `/download/*`.

## Deep modules

- `DownloadWindowsFn` / `DownloadStatusFn` — small HTTP contract, R2 miss/hit/meta drift.
- `Copy` + third-party denylist — small helpers, policy lives behind them.
- `shouldPublishLatest` — one boolean, keeps `latest` off `workflow_dispatch`.

Pages are shallow composition; we assert the built HTML, not component internals.

## Behaviors to test (in order)

1. **Tracer — empty R2 never 404s the zip.** `GET /download/windows` with missing `latest.zip` → 302 `Location: /download?available=0`.
2. **Available zip streams.** Hit + `latest.json` → 200 `application/zip`, disposition uses meta filename, `Cache-Control: private, no-store`.
3. **Meta is optional for the bytes.** Zip present, meta missing → still 200; generic `MTGONotes-win-x64.zip`. R2 throw → same 302, not 404. Non-GET → 405.
4. **Status is always 200 JSON.** Missing zip (ignore versioned-only) → `{ available: false }`. Zip+meta → full `ReleaseStatus`. Zip/no meta → `{ available: true }` without version. Meta/no zip or R2 throw → `{ available: false }`.
5. **Stable keys and tag-only latest.** `ReleaseKeys` literals; `shouldPublishLatest("tag")` true, `"branch"` false.
6. **Copy and CTA contract.** `primaryDownloadHref === "/download/windows"`; `assertAllowedCopy` rejects `tournament-safe`, `ban-proof`, `auto-updater`, “their current deck”; accepts required live-attach phrases.
7. **No third-party trackers/fonts.** `findForbiddenThirdParty` flags GTM, GA, Google Fonts, Plausible, Cloudflare Insights, DoubleClick.
8. **Brochure shell.** Five built routes; header nav + footer affiliation + brand icon/favicon on every page; no `*.zip` in `dist/`.
9. **Home first screen.** Pitch, Download CTA to `/download/windows`, tracker contrast, three beats (confirm / fast capture / recall).
10. **Download empty state.** `/download?available=0` shows disabled primary (no href) + “A Windows build is not published yet.” Requirements copy + secondary GitHub text when available. No GitHub release-asset primary href.
11. **Live attach + How it works + Privacy.** Required risk phrases in the dark card; How it works confirm-before-persist / hidden history / not “their current deck” + “Screenshot forthcoming”; Privacy no-signup / telemetry / unencrypted-export; no forbidden terms in `dist` HTML; Inter self-hosted.
12. **Release publish wiring.** `release.yml` puts the three R2 keys only on tag; step is not `continue-on-error`. `web/` has no committed zip/exe.
13. **Visitor journeys (Playwright).** Home first-visit; stubbed available download stays on `/download/windows`; empty + status abort keep the no-404 contract; nav to live-attach/privacy; 390×844 Home does not overflow.

## Out of scope for this cycle

- Live Cloudflare project/bucket/secrets (operator).
- Real tagged zip in R2.
- Real product screenshots.
- Custom domain, Workers migration, WinUI changes.
- Testing CSS class names, Astro internals, or real R2.
