import { describe, expect, it } from "vitest";
import { ReleaseKeys } from "../src/lib/releaseKeys";
import { shouldPublishLatest } from "../src/lib/releasePublish";

describe("ReleaseKeys", () => {
  it("names the versioned and latest Windows objects", () => {
    expect(ReleaseKeys.versioned("0.2.1")).toBe(
      "releases/windows/MTGONotes-0.2.1-win-x64.zip",
    );
    expect(ReleaseKeys.latestZip).toBe("releases/windows/latest.zip");
    expect(ReleaseKeys.latestMeta).toBe("releases/windows/latest.json");
  });
});

describe("shouldPublishLatest", () => {
  it("publishes latest only for git tags", () => {
    expect(shouldPublishLatest("tag")).toBe(true);
    expect(shouldPublishLatest("branch")).toBe(false);
  });
});
