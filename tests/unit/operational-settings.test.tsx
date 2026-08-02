import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

type InvokeArguments = {
  request?: Record<string, unknown>;
};

const invokeMock = vi.hoisted(() =>
  vi.fn<(command: string, arguments_?: InvokeArguments) => Promise<unknown>>(),
);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import { OperationalSettings } from "../../src/features/settings/OperationalSettings";
import {
  createDiagnostics,
  previewDiagnostics,
  selectDiagnosticsPath,
} from "../../src/lib/ipc/diagnostics";
import { checkUpdate, installUpdate } from "../../src/lib/ipc/updates";

const settings = {
  schemaVersion: 1,
  providerAccessEnabled: false,
  overlayEnabled: true,
  trayEnabled: true,
  launchWithWindows: false,
  updateChecksEnabled: false,
  classifierUpdateChecksEnabled: false,
  diagnosticsEnabled: false,
};

describe("Task 07 operational privacy surfaces", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((command) => {
      if (command === "get_settings") {
        return Promise.resolve({ ok: true, data: settings, revision: 1 });
      }
      return Promise.resolve({
        ok: false,
        error: {
          code: "update_unavailable",
          message: "No newer signed release is available.",
          retryable: false,
        },
      });
    });
  });

  test("independent network, startup, and diagnostics choices use private defaults", async () => {
    render(<OperationalSettings />);

    expect(
      await screen.findByRole("checkbox", {
        name: "Allow disclosed provider access",
      }),
    ).not.toBeChecked();
    expect(
      screen.getByRole("checkbox", {
        name: "Check for signed application updates",
      }),
    ).not.toBeChecked();
    expect(
      screen.getByRole("checkbox", {
        name: "Check for signed classifier asset updates",
      }),
    ).not.toBeChecked();
    expect(
      screen.getByRole("checkbox", { name: "Launch with Windows" }),
    ).not.toBeChecked();
    expect(
      screen.getByRole("checkbox", {
        name: "Allow local private diagnostic bundle creation",
      }),
    ).not.toBeChecked();
    expect(
      screen.getByRole("checkbox", {
        name: "Keep the companion available in the tray",
      }),
    ).toBeChecked();
    expect(
      screen.getByRole("checkbox", { name: "Show the opponent overlay" }),
    ).toBeChecked();
  });

  test("E2E-018: preview precedes local creation and no upload action exists", async () => {
    const user = userEvent.setup();
    invokeMock.mockImplementation((command) => {
      if (command === "get_settings") {
        return Promise.resolve({
          ok: true,
          revision: 1,
          data: { ...settings, diagnosticsEnabled: true },
        });
      }
      if (command === "preview_diagnostics") {
        return Promise.resolve({
          ok: true,
          revision: 1,
          data: {
            previewToken: "preview",
            artifacts: [
              {
                fileName: "events.jsonl",
                fieldClasses: ["eventCode", "timestamp"],
                eventCount: 2,
                redactionCount: 4,
                omitted: false,
              },
            ],
            totalEvents: 2,
            totalRedactions: 4,
            summarized: false,
            expiresAt: Date.now() + 60_000,
          },
        });
      }
      if (command === "select_diagnostics_path") {
        return Promise.resolve({
          ok: true,
          revision: 1,
          data: {
            selectionToken: "opaque-selection-token",
            fileName: "support.mtgodiag",
          },
        });
      }
      if (command === "create_diagnostics") {
        return Promise.resolve({
          ok: true,
          revision: 1,
          data: {
            fileName: "support.mtgodiag",
            artifactCount: 1,
            eventCount: 2,
            networkRequests: 0,
          },
        });
      }
      return Promise.resolve({ ok: true, revision: 1, data: true });
    });
    render(<OperationalSettings />);

    await user.click(
      await screen.findByRole("button", {
        name: "Preview redacted diagnostics",
      }),
    );
    expect(await screen.findByText("events.jsonl")).toBeVisible();
    expect(screen.getByText(/2 events · 4 redactions/i)).toBeVisible();
    expect(
      screen.queryByRole("button", { name: /upload|send|share/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Create local bundle" }),
    ).toBeDisabled();
    await user.click(
      screen.getByRole("button", { name: "Choose bundle location" }),
    );
    expect(await screen.findByText("support.mtgodiag")).toBeVisible();
    const create = screen.getByRole("button", { name: "Create local bundle" });
    expect(create).toBeEnabled();
    await user.click(create);
    const createCall = invokeMock.mock.calls.find(
      ([command]) => command === "create_diagnostics",
    );
    expect(createCall?.[1]?.request).toMatchObject({
      previewToken: "preview",
      selectionToken: "opaque-selection-token",
    });
  });

  test("E2E-019: classifier updates remain read-only and require confirmation", async () => {
    render(<OperationalSettings />);
    expect(
      await screen.findByRole("button", { name: "Check classifier update" }),
    ).toBeDisabled();
    for (const forbidden of [
      /edit classifier/i,
      /import asset/i,
      /delete definition/i,
      /choose asset/i,
    ]) {
      expect(
        screen.queryByRole("button", { name: forbidden }),
      ).not.toBeInTheDocument();
    }
  });

  test("typed diagnostics and updater IPC contain only explicit local actions", async () => {
    await previewDiagnostics();
    expect(invokeMock.mock.lastCall?.[0]).toBe("preview_diagnostics");
    await selectDiagnosticsPath();
    expect(invokeMock.mock.lastCall?.[0]).toBe("select_diagnostics_path");
    await createDiagnostics("preview", "opaque-selection-token");
    expect(invokeMock.mock.lastCall?.[0]).toBe("create_diagnostics");
    expect(invokeMock.mock.lastCall?.[1]?.request).toMatchObject({
      previewToken: "preview",
      selectionToken: "opaque-selection-token",
    });
    expect(invokeMock.mock.lastCall?.[1]?.request).not.toHaveProperty(
      "destination",
    );
    await checkUpdate();
    expect(invokeMock.mock.lastCall?.[0]).toBe("check_update");
    await installUpdate("0.2.0", true);
    expect(invokeMock.mock.lastCall?.[1]?.request).toEqual({
      version: "0.2.0",
      confirmed: true,
    });
    expect(JSON.stringify(invokeMock.mock.calls)).not.toContain("deviceId");
  });
});
