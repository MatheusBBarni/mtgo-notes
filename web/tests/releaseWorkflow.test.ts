import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const repoRoot = join(import.meta.dirname, "../..");
const workflow = readFileSync(
  join(repoRoot, ".github/workflows/release.yml"),
  "utf8",
);
const webRoot = join(repoRoot, "web");

function* walk(dir: string): Generator<string> {
  if (!existsSync(dir)) {
    return;
  }
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "dist" || name === ".astro") {
      continue;
    }
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      yield* walk(full);
    } else {
      yield full;
    }
  }
}

describe("release publish wiring", () => {
  it("uploads versioned and latest R2 objects only on tags", () => {
    expect(workflow).toMatch(/github\.ref_type == 'tag'/);
    expect(workflow).toContain("releases/windows/latest.zip");
    expect(workflow).toContain("releases/windows/latest.json");
    expect(workflow).toContain("releases/windows/MTGONotes-");
    expect(workflow).toContain("wrangler r2 object put");
    expect(workflow).not.toMatch(/continue-on-error:\s*true/);
  });

  it("does not keep Windows binaries in the site tree", () => {
    const binaries = [...walk(webRoot)].filter(
      (file) => file.endsWith(".zip") || file.endsWith(".exe"),
    );
    expect(binaries).toEqual([]);
  });
});
