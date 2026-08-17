import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  assertAllowedCopy,
  findForbiddenThirdParty,
} from "../../src/content/copy";

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

  it("renders the Home first screen", () => {
    const home = html("index.html");
    expect(home).toMatch(/private, local-first/i);
    expect(home).toContain("confirm opponent");
    expect(home).toContain("fast capture");
    expect(home).toContain("recall between games");
    expect(home).toMatch(/board logger/i);
    expect(home).toContain('href="/download/windows"');
    expect(home).toContain("Download for Windows");
  });

  it("renders Download requirements, empty state, and a first-party CTA", () => {
    const page = html("download/index.html");
    expect(page).toContain("Windows 10 22H2");
    expect(page).toContain("Windows 11");
    expect(page).toContain("MTGONotes.App.exe");
    expect(page).toContain("A Windows build is not published yet.");
    expect(page).toContain("GitHub Releases");
    expect(page).not.toContain(
      "github.com/MatheusBBarni/mtgo-notes/releases/download/",
    );
    expect(page).toContain('href="/download/windows"');
    expect(page).toMatch(/<button[^>]*disabled[^>]*>Download for Windows<\/button>/);
  });

  it("renders live-attach risk copy in a dark card", () => {
    const page = html("live-attach/index.html");
    expect(page).toMatch(/optional/i);
    expect(page).toMatch(/read-only/i);
    expect(page).toContain("LogOn");
    expect(page).toContain("Not legal advice");
    expect(page).toContain("Not affiliated");
    expect(page).toContain("Not tournament-approved");
    expect(page).toMatch(/unofficial/i);
    expect(page).not.toContain("tournament-safe");
    expect(page).not.toContain("ban-proof");
    expect(page).toContain("hero-card-dark");
  });

  it("renders How it works and Privacy promises", () => {
    const how = html("how-it-works/index.html");
    expect(how).toMatch(/confirm before/i);
    expect(how).toMatch(/hidden during possible gameplay/i);
    expect(how).not.toContain("their current deck");
    expect(how).toContain("Screenshot forthcoming");

    const privacy = html("privacy/index.html");
    expect(privacy).toMatch(/no signup/i);
    expect(privacy).toMatch(/telemetry/i);
    expect(privacy).toMatch(/unencrypted/i);
  });

  it("keeps copy clean, first-party, and Inter self-hosted", () => {
    const documents = routes.map((file) => html(file)).join("\n");
    expect(() => assertAllowedCopy(documents)).not.toThrow();
    expect(findForbiddenThirdParty(documents)).toEqual([]);
    const fonts = [...walk(dist)].filter(
      (file) => file.endsWith(".woff2") || file.endsWith(".woff"),
    );
    expect(fonts.length).toBeGreaterThan(0);
  });

  it("ships the brand files and no zip", () => {
    expect(existsSync(join(dist, "favicon.ico"))).toBe(true);
    expect(existsSync(join(dist, "brand/icon.png"))).toBe(true);
    expect(existsSync(dist)).toBe(true);
    const zips = [...walk(dist)].filter((file) => file.endsWith(".zip"));
    expect(zips).toEqual([]);
  });
});
