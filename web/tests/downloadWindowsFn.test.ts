import { describe, expect, it } from "vitest";
import { onRequestGet } from "../functions/download/windows";
import { fakeR2 } from "./helpers/fakeR2";

describe("GET /download/windows", () => {
  it("redirects to the empty-state page when latest.zip is missing", async () => {
    const response = await onRequestGet({
      request: new Request("https://mtgo-notes.pages.dev/download/windows"),
      env: { RELEASES: fakeR2() },
    });

    expect(response.status).toBe(302);
    expect(response.headers.get("Location")).toBe("/download?available=0");
  });
});
