# Official MTGO deck access validation

Decision date: 2026-07-28

Automatic access is disabled.

The implementation spike did not establish a documented or explicitly
permitted machine-readable official MTGO deck endpoint with stable response
semantics, published limits, and a confirmed redistributable field set. The
application therefore fails closed:

- `lookup` returns `interactive_required`.
- The only browser destination is an HTTPS page on `mtgo.com` or
  `www.mtgo.com`.
- The browser request contains only the player-confirmed opponent handle and
  format.
- The application does not scrape that page or any third-party site.
- A result is accepted only after the player supplies and confirms its official
  URL, provenance, format, complete decklist, encounter generation, and request
  token.

Enabling an automatic adapter requires a new reviewed spike recording the
official permission or documentation, response fixtures, rate limits,
redistributable fields, and a release change that switches the access decision.
