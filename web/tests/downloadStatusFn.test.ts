import { describe, expect, it } from "vitest";
import { onRequest, onRequestGet } from "../functions/download/status";
import { fakeJson, fakeObject, fakeR2 } from "./helpers/fakeR2";

const latestZip = fakeObject(new Uint8Array([0x50, 0x4b, 0x03, 0x04]));
const latestMeta = fakeJson({
  version: "0.2.1",
  filename: "MTGONotes-0.2.1-win-x64.zip",
  sha256: "abc",
  uploadedAt: "2026-08-17T00:00:00.000Z",
});

describe("GET /download/status", () => {
  it("reports unavailable when latest.zip is missing", async () => {
    const response = await onRequestGet({
      request: new Request("https://mtgo-notes.pages.dev/download/status"),
      env: {
        RELEASES: fakeR2({
          "releases/windows/MTGONotes-0.2.1-win-x64.zip": latestZip,
        }),
      },
    });

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ available: false });
  });

  it("reports the published version when zip and meta exist", async () => {
    const response = await onRequestGet({
      request: new Request("https://mtgo-notes.pages.dev/download/status"),
      env: {
        RELEASES: fakeR2({
          "releases/windows/latest.zip": latestZip,
          "releases/windows/latest.json": latestMeta,
        }),
      },
    });

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({
      available: true,
      version: "0.2.1",
      filename: "MTGONotes-0.2.1-win-x64.zip",
      uploadedAt: "2026-08-17T00:00:00.000Z",
    });
  });

  it("reports available without a version when meta is missing", async () => {
    const response = await onRequestGet({
      request: new Request("https://mtgo-notes.pages.dev/download/status"),
      env: {
        RELEASES: fakeR2({
          "releases/windows/latest.zip": latestZip,
        }),
      },
    });

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ available: true });
  });

  it("reports unavailable when meta exists without a zip", async () => {
    const response = await onRequestGet({
      request: new Request("https://mtgo-notes.pages.dev/download/status"),
      env: {
        RELEASES: fakeR2({
          "releases/windows/latest.json": latestMeta,
        }),
      },
    });

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ available: false });
  });

  it("reports unavailable when R2 throws", async () => {
    const response = await onRequestGet({
      request: new Request("https://mtgo-notes.pages.dev/download/status"),
      env: {
        RELEASES: {
          async get() {
            throw new Error("r2 unavailable");
          },
        },
      },
    });

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ available: false });
  });

  it("rejects non-GET methods", async () => {
    const response = await onRequest({
      request: new Request("https://mtgo-notes.pages.dev/download/status", {
        method: "POST",
      }),
      env: { RELEASES: fakeR2() },
    });

    expect(response.status).toBe(405);
  });
});
