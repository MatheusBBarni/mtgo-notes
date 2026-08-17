import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import {
  acceptReplacementEvent,
  replacementEvent,
} from "../../src/lib/ipc/events";
import {
  DOCUMENTED_WINDOW_PERMISSIONS,
  findCapabilityViolations,
  type CapabilityManifest,
} from "../../src/lib/security/capabilities";

function readCapability(name: "overlay" | "capture"): CapabilityManifest {
  return JSON.parse(
    readFileSync(
      resolve(process.cwd(), `src-tauri/capabilities/${name}.json`),
      "utf8",
    ),
  ) as CapabilityManifest;
}

describe("IPC event and capability contracts", () => {
  test("UT-118: unknown event major is rejected and requests safe bootstrap", () => {
    const clearSensitiveView = vi.fn();
    const requestBootstrap = vi.fn();
    const candidate = replacementEvent("overlay://view-v1", 9, {
      history: ["private"],
    });

    const result = acceptReplacementEvent(
      {
        ...candidate,
        version: { major: 2 },
      },
      { clearSensitiveView, requestBootstrap },
    );

    expect(result).toBeNull();
    expect(clearSensitiveView).toHaveBeenCalledOnce();
    expect(requestBootstrap).toHaveBeenCalledOnce();
  });

  test("UT-119: overlay and capture capabilities grant exactly their documented permissions", () => {
    for (const windowLabel of ["overlay", "capture"] as const) {
      const manifest = readCapability(windowLabel);

      expect(manifest.windows).toEqual([windowLabel]);
      expect(new Set(manifest.permissions)).toEqual(
        DOCUMENTED_WINDOW_PERMISSIONS[windowLabel],
      );
      expect(findCapabilityViolations(manifest)).toEqual([]);
    }
  });

  test.each([
    "*",
    "fs:default",
    "sql:default",
    "shell:allow-execute",
    "process:allow-restart",
    "http:default",
    "opener:default",
    "updater:allow-install",
    "global-shortcut:allow-register",
  ])(
    "UT-120: forbidden or wildcard capability %s fails policy lint",
    (permission) => {
      const violations = findCapabilityViolations({
        identifier: "unsafe",
        windows: ["main"],
        permissions: [permission],
      });

      expect(violations).not.toEqual([]);
    },
  );
});
