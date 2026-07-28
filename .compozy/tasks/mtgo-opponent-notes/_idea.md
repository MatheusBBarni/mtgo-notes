# MTGO Opponent Notes

## Overview

MTGO Opponent Notes is a private, local-first Windows companion for Magic: The Gathering Online players. It helps an individual player record observations during a match and recall them when encountering the same opponent again.

V1 will validate a focused opponent-memory loop: dated encounters, free-form and structured observations, historical deck recall, and a capture surface that records a useful note in under five seconds. It will launch with tournament-conservative disclosure behavior. Casual full-history access during games remains the first planned expansion.

### Summary / Differentiator

The product is **opponent memory, not another deck tracker**. Existing tools emphasize match telemetry, deck statistics, and automated scouting. This product preserves subjective knowledge that automation cannot reliably infer, such as play tendencies, risk tolerance, and recurring interaction patterns.

## Problem

MTGO games run on a chess clock. Players must remember revealed cards, inferred archetypes, and behavioral observations without spending meaningful match time writing notes. When they encounter the same opponent later, useful knowledge from previous matches is usually lost or scattered across generic documents.

Tools such as [mymtgo](https://docs.mymtgo.com/gameplay/opponent-scout-window.html) and [Extra Turn](https://extraturn.gg/) already provide match tracking, opponent history, and overlays. They do not foreground rapid, first-person observations such as “plays around removal,” “keeps risky hands,” or “represented card X in this position.”

The companion must also maintain a conservative policy boundary. The [MTGO EULA](https://www.mtgo.com/eula) gives Daybreak broad discretion over third-party software affecting gameplay or collecting game information. V1 will therefore use manual input and avoid MTGO process, memory, file, and log access.

### Market Data

Hasbro reports more than 50 million lifetime Magic players, $1.72 billion in 2025 Magic revenue, and 13 million registered digital players for Arena. The Arena figure must not be treated as MTGO adoption data. No credible official MTGO active-user count was found. [Hasbro Magic investor data](https://investor.hasbro.com/magic-gathering)

MTGO continues to offer leagues, queues, scheduled events, prizes, and qualification paths, supporting an active competitive use case despite the missing participation figures. [MTGO events](https://www.mtgo.com/events)

## Core Features

| # | Feature | Priority | Description |
|---|---|---|---|
| F1 | Opponent Encounter Ledger | Critical | Maintain a private timeline of dated encounters and user-authored observations for each opponent. |
| F2 | Rapid Capture | Critical | Open a focused capture surface through a keyboard-first action and save a useful observation in under five seconds. Prototype a mini-window and compact overlay before selecting the launch surface. |
| F3 | Tournament-Conservative Disclosure | Critical | Show historical notes before matches, between games, and afterward. During games, expose only observations created in the current match. Do not claim official tournament safety. |
| F4 | Structured Observations | High | Record deck identity, observed cards, and play tendencies alongside free-form notes without making structured entry mandatory. |
| F5 | Historical Recall | High | Retrieve the opponent timeline and a clearly dated “last deck seen” summary without presenting historical information as current truth. |
| F6 | Profile Correction | High | Let users correct observations and merge mistakenly duplicated opponent profiles while preserving encounter provenance. |
| F7 | Encrypted Backup and Restore | Medium | Create and restore an explicit encrypted local backup without readable export, cloud synchronization, or sharing. |

## KPIs

| KPI | Target | How to Measure |
|---|---:|---|
| Median observation capture time | `< 5 seconds` | Measure locally from capture opening to save during opt-in beta usability sessions. |
| First-session activation | `≥ 70%` | Percentage of pilot users who record one observation within their first three matches. |
| Repeat usage | `≥ 50%` | Percentage of weekly pilot users who use the companion in at least three play sessions. |
| Repeat-opponent recall | `≥ 60%` | Percentage of identified repeat encounters in which stored history is opened at an allowed phase. |
| Profile quality | `≥ 80%` | Percentage of active opponent profiles containing a deck identity and at least one card or tendency observation. |
| Integration safety guardrail | `100%` | Every V1 release passes a checklist confirming no MTGO process, memory, file, or log access. |

## Feature Assessment

| Criteria | Question | Score |
|---|---|---|
| **Impact** | How much more valuable does this make the product? | Strong |
| **Reach** | What percentage of users would this affect? | Maybe |
| **Frequency** | How often would users encounter this value? | Strong |
| **Differentiation** | Does this set us apart or match competitors? | Strong |
| **Defensibility** | Is it easy to copy or does it compound? | Maybe |
| **Feasibility** | Can we build it? | Strong |

**Leverage type:** Compounding Feature

The accumulated private encounter history becomes more useful over time and creates personal switching cost, although competitors could reproduce the basic workflow.

## Council Insights

- **Recommended approach:** Validate one trustworthy opponent-memory loop with local dated records, rapid manual capture, and tournament-conservative disclosure.
- **Key trade-offs:** An overlay could improve capture speed but adds another product hypothesis. Two launch modes provide flexibility but increase policy risk and mode confusion.
- **Position evolution:** The Architect accepted an overlay-less first slice if capture remains UI-independent. The Security Advocate and Pragmatic Engineer converged on encrypted backup without readable export.
- **Risks identified:** Unsupported compliance assumptions, accidental historical-note disclosure, stale deck information, opponent identity fragmentation, data loss, overlay focus problems, and scope drift into automated scouting.
- **Stretch goal:** Expand into a Personal MTGO Memory Engine covering opponent knowledge, archetype lessons, matchup preparation, and post-session review.

## Out of Scope (V1)

- **Casual full-dossier mode** — planned as the first expansion after validating the conservative disclosure boundary.
- **Automated MTGO identification or data collection** — process hooks, memory inspection, file access, and log parsing increase policy risk.
- **Cloud accounts and synchronization** — unnecessary for validating the local memory loop and increases privacy exposure.
- **Shared or crowdsourced opponent profiles** — creates abuse, privacy, moderation, and competitive-integrity risks.
- **Predictive deck inference or strategic recommendations** — could misrepresent stale observations and drift into gameplay assistance.
- **Readable export and third-party integrations** — portable opponent dossiers increase exposure; V1 supports encrypted backup only.
- **General match and win-rate tracking** — established competitors already address this category.

## Architecture Decision Records

- [ADR-001: Stage the Opponent-Memory V1 Around a Tournament-Conservative Core](adrs/adr-001.md) — Launch a local encounter ledger with conservative disclosure and evidence-driven capture-surface selection.

## Open Questions

- Can Daybreak provide written clarification about manual companion notes and overlays?
- Can a focused mini-window meet the five-second target, or is an always-on-top overlay required?
- How should users indicate match phases without reading MTGO state?
- How should renamed or mistyped opponent handles be reconciled?
- What local encryption and recovery model best protects backups without creating unrecoverable data?
- Should the later TechSpec select Tauri or Electrobun based on Windows focus behavior, overlay reliability, packaging, accessibility, and team familiarity?
