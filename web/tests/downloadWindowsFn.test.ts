import { describe, expect, it } from "vitest";
import { onRequest, onRequestGet } from "../functions/download/windows";
import { fakeJson, fakeObject, fakeR2 } from "./helpers/fakeR2";

describe("GET /download/windows", () => {
  it("redirects to the empty-state page when latest.zip is missing", async () => {
    const response = await onRequestGet({
      request: new Request("https://mtgo-notes.pages.dev/download/windows"),
      env: { RELEASES: fakeR2() },
    });

    expect(response.status).toBe(302);
    expect(response.headers.get("Location")).toBe("/download?available=0");
  });

  it("streams the latest zip with the meta filename and no-store cache", async () => {
    const zipBytes = new Uint8Array([0x50, 0x4b, 0x03, 0x04]);
    const response = await onRequestGet({
      request: new Request("https://mtgo-notes.pages.dev/download/windows"),
      env: {
        RELEASES: fakeR2({
          "releases/windows/latest.zip": fakeObject(zipBytes),
          "releases/windows/latest.json": fakeJson({
            version: "0.2.1",
            filename: "MTGONotes-0.2.1-win-x64.zip",
            sha256: "abc",
            uploadedAt: "2026-08-17T00:00:00.000Z",
          }),
        }),
      },
    });

    expect(response.status).toBe(200);
    expect(response.headers.get("Content-Type")).toBe("application/zip");
    expect(response.headers.get("Content-Disposition")).toContain(
      "MTGONotes-0.2.1-win-x64.zip",
    );
    expect(response.headers.get("Cache-Control")).toBe("private, no-store");
    expect(new Uint8Array(await response.arrayBuffer())).toEqual(zipBytes);
  });

  it("streams the zip with a generic filename when latest.json is missing", async () => {
    const zipBytes = new Uint8Array([0x50, 0x4b, 0x03, 0x04]);
    const response = await onRequestGet({
      request: new Request("https://mtgo-notes.pages.dev/download/windows"),
      env: {
        RELEASES: fakeR2({
          "releases/windows/latest.zip": fakeObject(zipBytes),
        }),
      },
    });

    expect(response.status).toBe(200);
    expect(response.headers.get("Content-Disposition")).toContain(
      "MTGONotes-win-x64.zip",
    );
    expect(new Uint8Array(await response.arrayBuffer())).toEqual(zipBytes);
  });

  it("redirects to the empty-state page when R2 throws", async () => {
    const response = await onRequestGet({
      request: new Request("https://mtgo-notes.pages.dev/download/windows"),
      env: {
        RELEASES: {
          async get() {
            throw new Error("r2 unavailable");
          },
        },
      },
    });

    expect(response.status).toBe(302);
    expect(response.headers.get("Location")).toBe("/download?available=0");
  });

  it("rejects non-GET methods", async () => {
    const response = await onRequest({
      request: new Request("https://mtgo-notes.pages.dev/download/windows", {
        method: "POST",
      }),
      env: { RELEASES: fakeR2() },
    });

    expect(response.status).toBe(405);
  });
});
