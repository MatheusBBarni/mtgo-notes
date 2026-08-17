import { describe, expect, it } from "vitest";
import { applyDownloadAvailability } from "../src/lib/downloadCta";

const markup = `
<a class="button-primary download-cta" href="/download/windows">Download for Windows</a>
<div class="download-empty" hidden>
  <button class="button-primary" type="button" disabled>Download for Windows</button>
  <p>A Windows build is not published yet.</p>
</div>
`;

describe("applyDownloadAvailability", () => {
  it("reveals the disabled empty state when the build is unavailable", () => {
    const html = applyDownloadAvailability(markup, false);
    expect(html).not.toContain('href="/download/windows"');
    expect(html).toContain("disabled");
    expect(html).toContain("A Windows build is not published yet.");
    expect(html).not.toMatch(/download-empty"[^>]*hidden/);
  });
});
