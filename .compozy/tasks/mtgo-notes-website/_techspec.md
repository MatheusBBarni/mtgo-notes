# Technical Specification: MTGO Opponent Notes website

## Executive Summary

V1 is a **static Astro** brochure in `web/`, hosted on **Cloudflare Pages**, plus two **Pages Functions** that read a **private R2** bucket. Astro prerenders five routes to `dist`. The Windows zip never enters that output: tagged `release.yml` uploads a versioned object, `latest.zip`, and `latest.json` to R2. Home and Download point at `/download/windows`, which streams the latest zip or 302s to an empty state. No `@astrojs/cloudflare` adapter, no visitor auth, no telemetry, no third-party scripts.

Primary trade-off: stay on Pages (PRD) with static output instead of Workers + Assets. That keeps preview URLs and Git deploys, and avoids the Astro 6 adapter’s Pages drop because this site does not SSR. Inter substitutes for Haas. The existing notebook/card icon is the favicon and header mark.

## System Architecture

### Component Overview

| Component | Location | Responsibility and boundary | Stories |
|---|---|---|---|
| `WebSite` | `web/` | Astro static app. Owns routes, tokens, copy, brand files. Never contains the zip. | US-001–US-005 |
| `BaseLayout` | `web/src/layouts/BaseLayout.astro` | Document shell, favicon, Inter, `DESIGN.md` CSS tokens, skip link. | US-001, US-004 |
| `SiteHeader` | `web/src/components/SiteHeader.astro` | Brand icon + wordmark, nav to all five routes. | US-001, US-003 |
| `SiteFooter` | `web/src/components/SiteFooter.astro` | Unofficial / not affiliated / GitHub source on every page. | US-001, US-003 |
| `BrandMark` | `web/src/components/BrandMark.astro` | Renders `/brand/icon.png`. Used in header and any identity lockup. | US-001 |
| `DownloadCta` | `web/src/components/DownloadCta.astro` | Primary control. Default `href="/download/windows"`. Inline script reads `/download/status` to show version or empty state. | US-002, US-005 |
| `Copy` | `web/src/content/copy.ts` | Single source for pitch, beats, risk phrases, forbidden-term list consumers. | US-001–US-004 |
| `DownloadWindowsFn` | `web/functions/download/windows.ts` | `GET` streams `latest.zip` or 302s to `/download?available=0`. | US-002, US-005 |
| `DownloadStatusFn` | `web/functions/download/status.ts` | `GET` always 200 `ReleaseStatus`. | US-002, US-005 |
| `ReleaseKeys` | `web/src/lib/releaseKeys.ts` (shared with Functions via relative import or duplicated constant file under `web/lib/`) | R2 key layout. | US-005, US-006 |
| `ReleasePublish` | `.github/workflows/release.yml` | After a tagged GitHub Release, puts versioned zip, `latest.zip`, `latest.json`. | US-006 |
| `PagesProject` | Cloudflare dashboard + `web/wrangler.jsonc` | Project `mtgo-notes`, root `web`, output `dist`, R2 binding `RELEASES` → `mtgo-notes-releases`. | all |

Story ownership:

| Story | Primary technical owner | Supporting components |
|---|---|---|
| US-001 | `WebSite` pages `/`, `/how-it-works` | `BaseLayout`, `SiteHeader`, `SiteFooter`, `BrandMark`, `Copy` |
| US-002 | `DownloadCta` + `DownloadWindowsFn` | `/download`, `DownloadStatusFn` |
| US-003 | `/live-attach` | `Copy`, `SiteHeader` |
| US-004 | `/privacy` | `BaseLayout` (no third-party tags) |
| US-005 | `DownloadWindowsFn` + `ReleaseKeys` | `DownloadStatusFn`, `ReleasePublish` |
| US-006 | `ReleasePublish` | R2 bucket `mtgo-notes-releases` |

### Runtime and Data Flow

1. Cloudflare Pages builds `web/` on git push (`npm ci && npm run build`) and publishes `dist` plus `functions/`.
2. A visitor hits a static HTML route. Header, footer, and icon are in the document. No analytics tags.
3. `DownloadCta` ships as a real link to `/download/windows`. After load, a few lines of JS `fetch('/download/status')`:
   - `available: true` → keep the href, show `version`.
   - `available: false` → replace the control with empty-state text (no href that 404s).
   - fetch error → leave the original href (Function 302s on miss).
4. `GET /download/windows` uses `context.env.RELEASES.get(latestZipKey)`.
   - Hit: stream `obj.body` as `application/zip` with `Content-Disposition: attachment; filename="<from latest.json or MTGONotes-win-x64.zip>"` and `Cache-Control: private, no-store`.
   - Miss: `302 Location: /download?available=0`.
5. `/download` reads `Astro.url.searchParams.get('available') === '0'` so the empty state works without JS.
6. On `push: tags: v*`, `release.yml` builds the WinUI zip, creates the GitHub Release, then `wrangler r2 object put` for the three keys. Non-tag `workflow_dispatch` skips those puts.

```
Visitor → Pages (HTML/CSS/icon)
       → GET /download/status → Function → R2 latest.json / head latest.zip
       → GET /download/windows → Function → stream R2 latest.zip
Tag CI → GitHub Release (archive)
       → R2 versioned zip + latest.zip + latest.json
```

## Implementation Design

### Core Interfaces

Primary language for this feature is TypeScript. `ReleaseStatus` is the type every download surface depends on.

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

export interface LatestMeta {
  version: string;
  filename: string;
  sha256: string;
  uploadedAt: string;
}

export interface PagesEnv {
  RELEASES: R2Bucket;
}
```

Function contract (Pages `PagesFunction<PagesEnv>`):

```ts
// GET /download/status  → 200 ReleaseStatus
// GET /download/windows → 200 application/zip | 302 /download?available=0
// Other methods         → 405
```

`DownloadWindowsFn` reads `latest.json` only to name the file. A missing meta file must not block a present zip. A present meta file with a missing zip is `available: false`.

Shared error rule: Functions never return a bare `404` HTML/text body for the zip route. Status is always 200 JSON.

### Data Models

R2 objects (no database):

| Key | Body | Writer |
|---|---|---|
| `releases/windows/MTGONotes-<version>-win-x64.zip` | zip bytes | `ReleasePublish` |
| `releases/windows/latest.zip` | copy of that zip | `ReleasePublish` |
| `releases/windows/latest.json` | `LatestMeta` | `ReleasePublish` |

`LatestMeta` example:

```json
{
  "version": "0.2.1",
  "filename": "MTGONotes-0.2.1-win-x64.zip",
  "sha256": "<hex>",
  "uploadedAt": "2026-08-17T00:00:00.000Z"
}
```

No visitor-identifying storage. No cookies.

### API Endpoints

| Method | Path | Request | Success | Failure |
|---|---|---|---|---|
| GET | `/download/status` | none | 200 `ReleaseStatus` | none (miss → `{ "available": false }`) |
| GET | `/download/windows` | none | 200 zip stream | 302 `/download?available=0` |
| * | both | — | — | 405 |

No auth headers. No CORS configuration (same origin).

## Integration Points

### Cloudflare Pages

- Dashboard project `mtgo-notes`, production branch `main` (or the default protected branch).
- Root directory `web`. Build command `npm ci && npm run build`. Output `dist`. Node **22**.
- Preview deployments from PRs that touch `web/` or the Functions.
- R2 binding: variable `RELEASES`, bucket `mtgo-notes-releases`, production and preview (preview may share the bucket; it only reads).

### Cloudflare R2

- Bucket `mtgo-notes-releases`, Standard class, **private**.
- Auth for CI: `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID`.
- No public bucket access, no custom domain on the bucket.

### GitHub Releases

- Existing `softprops/action-gh-release` step stays. It remains the long-term archive.
- R2 publish is an additional step, not a replacement.
- Order: tests → publish zip → GitHub Release → R2 puts → fail job if any R2 put fails.

### Brand assets

- Source today: `__oldversion__/assets/icons/mtgo-notes-transparent.png`, `mtgo-notes.ico`.
- Canonical site copies: `web/public/brand/icon.png`, `web/public/favicon.ico`.
- Implementation copies the files once; the site build must not read `__oldversion__/`.

## Impact Analysis

| Component | Impact Type | Description and Risk | Required Action |
|---|---|---|---|
| `web/` | new | Entire site + Functions. Low risk to the WinUI app. | Scaffold and deploy independently |
| `.github/workflows/release.yml` | modified | Adds Node + wrangler R2 puts on tags. Risk: secret missing fails a release job after GH release exists. | Add secrets; keep GH release step before R2; fail on R2 error |
| Cloudflare account | new | Pages project + R2 bucket + API token | Create before first production deploy |
| `MTGONotes.App` / Core / Data / Live | none | No code change | None |
| `__oldversion__/` | none | Icon is copied out, not referenced at build | Copy once |

## Testing Approach

Concrete cases live in `_tests.md`.

- **Unit (Vitest):** `DownloadWindowsFn`, `DownloadStatusFn`, `ReleaseKeys`, copy-forbidden-term helper. Fake `R2Bucket` only.
- **Integration:** `astro build` output assertions (routes, favicon, icon, no zip in `dist`, no third-party script hosts, required phrases). `wrangler pages dev` optional for Function+asset wiring when a local R2 fixture is present.
- **E2E (Playwright):** `astro preview` plus Playwright `route` stubs for `/download/status` and `/download/windows`. One path uses a real Function harness with an in-memory R2 fake if the unit tests do not already cover the 302.

CI for the site is a new `web.yml` (or a job in an existing workflow) that runs on `web/**` changes: `npm ci`, `npm test`, `npm run build`, output assertions. Windows `release.yml` stays on `windows-latest` and does not run Astro tests.

## Development Sequencing

### Build Order

1. `web/` Astro scaffold, `wrangler.jsonc`, tokens CSS, Inter, copied icon/favicon — no download yet.
2. `BaseLayout`, header, footer, `Copy`, five routes with PRD copy.
3. `ReleaseKeys` + Functions + Vitest fakes.
4. `DownloadCta` status enhancement + `?available=0`.
5. `ReleasePublish` steps + documented secrets.
6. Create Pages project, R2 bucket, binding; first production deploy (empty-state is valid).
7. Playwright + copy/output gates in `web.yml`.

### Technical Dependencies

- Cloudflare account permission to create Pages + R2.
- GitHub secrets `CLOUDFLARE_ACCOUNT_ID`, `CLOUDFLARE_API_TOKEN`.
- Node 22 for Pages and local site CI.
- Brand PNG/ICO exist (they do).

## Monitoring and Observability

- No visitor analytics, no pixels, no third-party RUM.
- Operators use the Cloudflare dashboard: Pages deploy status, Function invocation/error logs for `/download/*`.
- Release workflow failure (including R2 put) is the alert that `latest` did not update.
- Do not log object bytes or visitor IPs into the site codebase. Default Cloudflare request logs are account-side, not shipped telemetry.

## Technical Considerations

### Key Decisions

- **Static Astro in `web/` on Pages.** Rationale: matches the PRD host, needs no SSR. Trade-off: not Workers-first. Alternatives: sibling repo, Workers + Assets, hand-rolled HTML — see [ADR-001](adrs/adr-001.md).
- **Private R2 + Pages Functions.** Rationale: zip exceeds 25 MiB; empty state needs a first-party miss path. Trade-off: stream large objects through the Function. Alternatives: public R2, standalone Worker, presigned URLs — see [ADR-002](adrs/adr-002.md).
- **Five routes, one layout.** Rationale: Home stays one screen; attach/privacy stay bookmarkable. See [ADR-003](adrs/adr-003.md).
- **Inter, not Haas.** Rationale: no license. See [ADR-004](adrs/adr-004.md).
- **Icon is first-class.** Favicon + header `BrandMark` from the existing artwork.

### Known Risks

- **First deploy has no zip.** Likelihood high. Mitigation: empty state is a launch requirement, not an afterthought.
- **R2 put fails after GitHub Release.** Likelihood low. Mitigation: fail the workflow; latest stays on the previous object.
- **`workflow_dispatch` overwrites latest.** Mitigation: R2 puts are tag-only.
- **Zip accidentally added to `public/`.** Mitigation: output test forbids `.zip` under `dist/`.
- **Forbidden marketing claims regress.** Mitigation: copy fixture + HTML grep in CI.
- **Pages product direction.** Cloudflare is pushing Workers. Mitigation: static `dist` + two tiny Functions are a cheap later migrate (see Cloudflare Pages → Workers guide). Do not migrate in V1.

### Pages / Wrangler snapshot

```jsonc
{
  "name": "mtgo-notes",
  "pages_build_output_dir": "dist",
  "compatibility_date": "2026-08-17",
  "r2_buckets": [
    { "binding": "RELEASES", "bucket_name": "mtgo-notes-releases" }
  ]
}
```

```text
web/
  astro.config.mjs          # static, trailingSlash: 'never', site: https://mtgo-notes.pages.dev
  wrangler.jsonc
  functions/download/windows.ts
  functions/download/status.ts
  public/favicon.ico
  public/brand/icon.png
  src/layouts/BaseLayout.astro
  src/components/{SiteHeader,SiteFooter,BrandMark,DownloadCta}.astro
  src/content/copy.ts
  src/lib/releaseKeys.ts
  src/pages/{index,download,how-it-works,live-attach,privacy}.astro
  src/styles/tokens.css
  tests/
```

## Architecture Decision Records

- [ADR-001: Static Astro in `web/` on Cloudflare Pages](adrs/adr-001.md) — site location, generator, brand files, no adapter
- [ADR-002: Private R2 plus a Pages Function for the Windows download](adrs/adr-002.md) — bucket, keys, stream vs miss, tag-only publish
- [ADR-003: Five brochure routes behind one layout](adrs/adr-003.md) — IA and shared CTA
- [ADR-004: Ship Inter instead of Haas Grotesk](adrs/adr-004.md) — licensed-safe type, no font CDN
