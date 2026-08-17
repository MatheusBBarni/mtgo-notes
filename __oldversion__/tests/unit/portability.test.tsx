import { render, screen } from "@testing-library/react";

type InvokeArguments = {
  request: Record<string, unknown>;
};

const invokeMock = vi.hoisted(() =>
  vi.fn<(command: string, arguments_?: InvokeArguments) => Promise<unknown>>(),
);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import { PortabilityWorkspace } from "../../src/features/portability/PortabilityWorkspace";
import {
  applyRestore,
  previewRestore,
  startBackup,
  startExport,
} from "../../src/lib/ipc/portability";
import { progressPercent } from "../../src/lib/ipc/operations";

describe("portability IPC and UI contracts", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ ok: true, data: [] });
  });

  test("IT-220/IT-223: backup and export send opaque selections, acknowledgements, and typed scopes", async () => {
    await startBackup({
      selectionToken: "opaque-backup-selection",
      passphrase: "host-owned-secret",
      passphraseAcknowledged: true,
    });
    expect(invokeMock.mock.lastCall?.[0]).toBe("start_backup");
    const backupRequest = invokeMock.mock.lastCall?.[1]?.request;
    expect(backupRequest).toMatchObject({
      selectionToken: "opaque-backup-selection",
      passphrase: "host-owned-secret",
      passphraseAcknowledged: true,
      confirmEmpty: false,
      overwrite: false,
    });
    expect(typeof backupRequest?.idempotencyKey).toBe("string");

    await startExport({
      selectionToken: "opaque-export-selection",
      scope: { selected_opponent: { profileId: "profile-id" } },
      plaintextAcknowledged: true,
      unsavedEditsResolved: true,
    });
    expect(invokeMock.mock.lastCall?.[0]).toBe("start_export");
    expect(invokeMock.mock.lastCall?.[1]?.request).toMatchObject({
      selectionToken: "opaque-export-selection",
      scope: { selected_opponent: { profileId: "profile-id" } },
      plaintextAcknowledged: true,
      unsavedEditsResolved: true,
    });
    expect(JSON.stringify(invokeMock.mock.calls)).not.toContain(
      "Users/example/private",
    );
  });

  test("IT-221/IT-222: restore preview and apply are separate token-bound commands", async () => {
    await previewRestore("opaque-source", "secret");
    expect(invokeMock.mock.lastCall?.[0]).toBe("preview_restore");
    const previewRequest = invokeMock.mock.lastCall?.[1]?.request;
    expect(previewRequest).toMatchObject({
      selectionToken: "opaque-source",
      passphrase: "secret",
    });
    expect(typeof previewRequest?.operationId).toBe("string");
    expect(typeof previewRequest?.idempotencyKey).toBe("string");

    await applyRestore("expiring-preview", "replace");
    expect(invokeMock.mock.lastCall?.[0]).toBe("apply_restore");
    const applyRequest = invokeMock.mock.lastCall?.[1]?.request;
    expect(applyRequest).toMatchObject({
      previewToken: "expiring-preview",
      mode: "replace",
    });
    expect(typeof applyRequest?.idempotencyKey).toBe("string");
  });

  test("IT-268: progress projection clamps monotonic terminal percentages", () => {
    const base = {
      id: "operation",
      kind: "backup_snapshot" as const,
      idempotencyKey: "idempotency",
      state: "running" as const,
      requestedAt: 1,
      completed: 4,
      total: 10,
      revision: 2,
    };
    expect(progressPercent(base)).toBe(40);
    expect(progressPercent({ ...base, completed: 12 })).toBe(100);
    expect(progressPercent({ ...base, completed: 0, total: 0 })).toBe(0);
  });

  test("E2E-014/E2E-015/E2E-016: accessible controls disclose passphrase, staging, rollback, and plaintext consequences", () => {
    render(<PortabilityWorkspace />);

    expect(
      screen.getByRole("region", { name: "Backup, restore, and export" }),
    ).toBeVisible();
    expect(screen.getByText(/passphrase cannot be recovered/i)).toBeVisible();
    expect(
      screen.getByText(/SQLCipher staging finish before merge or replace/i),
    ).toBeVisible();
    expect(screen.getByRole("note")).toHaveTextContent(
      /resulting UTF-8 .txt file is unencrypted/i,
    );
    expect(
      screen.getByRole("button", { name: "Authenticate and preview" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("heading", { name: "Encrypted restore rollbacks" }),
    ).toBeVisible();
  });
});
