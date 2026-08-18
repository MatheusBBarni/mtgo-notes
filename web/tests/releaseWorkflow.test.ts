import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const repoRoot = join(import.meta.dirname, "../..");
const workflow = readFileSync(
  join(repoRoot, ".github/workflows/release.yml"),
  "utf8",
);

describe("release publish wiring", () => {
  it("publishes the zip on GitHub Releases and not to R2", () => {
    expect(workflow).toContain("softprops/action-gh-release");
    expect(workflow).toContain("MTGONotes-");
    expect(workflow).toContain("win-x64.zip");
    expect(workflow).not.toContain("wrangler");
    expect(workflow).not.toContain("r2 object put");
    expect(workflow).not.toContain("CLOUDFLARE_");
  });
});
