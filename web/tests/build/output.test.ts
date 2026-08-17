import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const dist = join(import.meta.dirname, "../../dist");

const routes = [
  "index.html",
  "download/index.html",
  "how-it-works/index.html",
  "live-attach/index.html",
  "privacy/index.html",
] as const;

function html(file: string): string {
  return readFileSync(join(dist, file), "utf8");
}

function* walk(dir: string): Generator<string> {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      yield* walk(full);
    } else {
      yield full;
    }
  }
}

describe("brochure build output", () => {
  it("emits the five brochure routes with header, footer, and brand", () => {
    for (const file of routes) {
      expect(existsSync(join(dist, file)), file).toBe(true);
      const document = html(file);
      expect(document).toContain('href="/"');
      expect(document).toContain('href="/download"');
      expect(document).toContain('href="/how-it-works"');
      expect(document).toContain('href="/live-attach"');
      expect(document).toContain('href="/privacy"');
      expect(document).toMatch(/not affiliated/i);
      expect(document).toContain("/favicon.ico");
      expect(document).toContain("/brand/icon.png");
    }
  });

  it("ships the brand files and no zip", () => {
    expect(existsSync(join(dist, "favicon.ico"))).toBe(true);
    expect(existsSync(join(dist, "brand/icon.png"))).toBe(true);
    expect(existsSync(dist)).toBe(true);
    const zips = [...walk(dist)].filter((file) => file.endsWith(".zip"));
    expect(zips).toEqual([]);
  });
});
