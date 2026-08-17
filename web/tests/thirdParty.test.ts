import { describe, expect, it } from "vitest";
import { findForbiddenThirdParty } from "../src/content/copy";

describe("findForbiddenThirdParty", () => {
  it("flags tracker, font CDN, and ad hosts", () => {
    const html = [
      '<script src="https://www.googletagmanager.com/gtag/js"></script>',
      '<script src="https://www.google-analytics.com/analytics.js"></script>',
      '<link href="https://fonts.googleapis.com/css2?family=Inter" rel="stylesheet">',
      '<script src="https://plausible.io/js/script.js"></script>',
      '<script src="https://static.cloudflareinsights.com/beacon.min.js"></script>',
      '<script src="https://stats.g.doubleclick.net/dc.js"></script>',
    ].join("\n");

    expect(findForbiddenThirdParty(html)).toEqual([
      "googletagmanager.com",
      "google-analytics.com",
      "fonts.googleapis.com",
      "plausible.io",
      "cloudflareinsights.com",
      "doubleclick.net",
    ]);
  });

  it("accepts first-party markup", () => {
    expect(
      findForbiddenThirdParty(
        '<link rel="stylesheet" href="/fonts/inter.css"><script src="/download/status"></script>',
      ),
    ).toEqual([]);
  });
});
