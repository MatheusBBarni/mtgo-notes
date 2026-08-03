# Player workspace prototype

Wayfinder ticket: [Prototype the Player Workspace](https://github.com/MatheusBBarni/mtgo-notes/issues/7)

## Design anchor

- **Job:** Let the player save one local MTGO nickname, explicitly find public results, review exactly what will be retained, and preserve source-attributed evidence without implying MTGO account access.
- **Scene:** An MTGO player reviews public context between matches on a Windows desktop, with limited attention and no tolerance for an accidental network request or hidden evidence change.
- **Register:** Product.
- **Dials:** `VISUAL_VARIANCE=3`, `MOTION_INTENSITY=2`, `INFORMATION_DENSITY=8`.
- **Pattern:** One compact Player tab with progressive disclosure; no modal is needed for identity, consent, candidate review, empty, or failure states.

## State matrix

The prototype exposes these states through its review controls:

1. First use: no Player identity.
2. Consent needed: identity saved locally; Census remains off.
3. Ready: consent granted; explicit lookup available.
4. Loading: lookup visible and cancellable.
5. Candidates: exact-match candidates with selective fields and explicit import.
6. Empty: provider- and time-scoped absence with no claim of complete history.
7. Failure: provider-scoped degradation that preserves local evidence.
8. Nickname change: inline edit warning that old imports keep their lookup nickname.
9. Imported: durable evidence plus explicit non-destructive refresh.

Interactive controls include default, hover, active, focus-visible, disabled, busy, selected, error, and success treatments. The prototype uses semantic HTML, visible labels, a live status region, keyboard-operable tabs/buttons/checkboxes/details, and a reduced-motion branch.

## Review log

- Approved: compact two-column model with Player identity and source controls on the left, and lookup, candidate review, and saved evidence on the right.
- Approved: provider status remains visible in the source column; source-specific consent expands inline only when off or explicitly reviewed, with no modal or Settings detour.
- Approved: exact-match candidates use one selectable list; retained fields expand within each result; source identity and attribution are mandatory; one sticky summary bar owns the import action.
- Approved: scoped absence and provider failure use distinct inline states above saved evidence; neither alters imports, overclaims source coverage, nor triggers an automatic fallback.
- Approved: nickname editing stays inline; saving is local-only, starts no lookup, preserves historical lookup nicknames, and creates no aliasing or evidence relinking.
- Approved: refresh is explicit and non-destructive; it reuses the current nickname and consent, previews only new or changed evidence, identifies duplicates, and shares lookup loading, cancellation, empty, and failure states.
