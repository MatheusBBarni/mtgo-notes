import { describe, expect, it } from "vitest";
import {
  assertAllowedCopy,
  primaryDownloadHref,
} from "../src/content/copy";

describe("copy contract", () => {
  it("exports the first-party download href", () => {
    expect(primaryDownloadHref).toBe("/download/windows");
  });

  it("rejects forbidden marketing claims", () => {
    expect(() => assertAllowedCopy("this is tournament-safe")).toThrow();
    expect(() => assertAllowedCopy("ban-proof companion")).toThrow();
    expect(() => assertAllowedCopy("includes an auto-updater")).toThrow();
    expect(() => assertAllowedCopy("this is their current deck")).toThrow();
  });

  it("accepts required live-attach phrases", () => {
    expect(() =>
      assertAllowedCopy(
        "Not legal advice. Not affiliated. Not tournament-approved. unofficial",
      ),
    ).not.toThrow();
  });
});
