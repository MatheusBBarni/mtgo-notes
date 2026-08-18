import { describe, expect, it } from "vitest";
import { resolveTheme } from "../src/lib/theme";

describe("resolveTheme", () => {
  it("prefers an explicit stored theme", () => {
    expect(resolveTheme("dark", false)).toBe("dark");
    expect(resolveTheme("light", true)).toBe("light");
  });

  it("falls back to the system preference", () => {
    expect(resolveTheme(null, true)).toBe("dark");
    expect(resolveTheme("invalid", false)).toBe("light");
  });
});
