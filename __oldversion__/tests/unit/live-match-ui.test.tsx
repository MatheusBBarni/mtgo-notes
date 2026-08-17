import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { CaptureApp } from "../../src/capture/CaptureApp";
import { replacementEvent } from "../../src/lib/ipc/events";
import { OverlayApp } from "../../src/overlay/OverlayApp";
import {
  applyOverlayReplacement,
  type OverlayView,
} from "../../src/overlay/projection";

const mocks = vi.hoisted(() => ({
  hide: vi.fn<() => Promise<void>>(),
  invoke: vi.fn<(command: string) => Promise<unknown>>(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: mocks.listen,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    hide: mocks.hide,
  }),
}));

const overlayView = (
  phase: OverlayView["phase"],
  overrides: Partial<OverlayView> = {},
): OverlayView => ({
  phase,
  confirmedHandle: "VisibleOpponent",
  currentObservations: [],
  historicalObservations: [],
  historyEditable: false,
  needsIdentityResolution: false,
  ...overrides,
});

describe("Task 03 live-match renderer contracts", () => {
  beforeEach(() => {
    mocks.hide.mockReset();
    mocks.invoke.mockReset();
    mocks.listeners.clear();
    mocks.listen.mockReset();
    mocks.listen.mockImplementation(
      (name: string, listener: (event: { payload: unknown }) => void) => {
        mocks.listeners.set(name, listener);
        return Promise.resolve(vi.fn());
      },
    );
    mocks.invoke.mockResolvedValue({
      ok: true,
      data: true,
      revision: 1,
    });
  });

  test("UT-109: compact overlay bounds overflow and keeps capture and hide reachable", async () => {
    render(<OverlayApp />);
    await waitFor(() =>
      expect(mocks.listeners.get("overlay://view-v1")).toBeTypeOf("function"),
    );

    mocks.listeners.get("overlay://view-v1")?.({
      payload: replacementEvent(
        "overlay://view-v1",
        2,
        overlayView("pre_match", {
          currentObservations: Array.from({ length: 6 }, (_, index) => ({
            id: `observation-${index}`,
            text: `Visible note ${index}`,
            editable: false,
          })),
        }),
      ),
    });

    expect(await screen.findByText("Visible note 0")).toBeVisible();
    expect(screen.getByText("Visible note 2")).toBeVisible();
    expect(screen.queryByText("Visible note 3")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Open quick capture" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Hide opponent overlay" }),
    ).toBeVisible();
  });

  test("UT-110: restricted replacement removes historical data from view state and DOM", async () => {
    const permitted = overlayView("pre_match", {
      historicalObservations: [
        { id: "history", text: "Prior private tendency", editable: false },
      ],
      publicSnapshot: {
        label: "Official list",
        format: "Modern",
        publishedAt: 1,
        sourceText: "Official source",
        available: true,
      },
    });
    const restricted = applyOverlayReplacement(
      permitted,
      replacementEvent(
        "overlay://view-v1",
        3,
        overlayView("in_game_restricted", {
          historicalObservations: permitted.historicalObservations,
          publicSnapshot: permitted.publicSnapshot,
        }),
      ),
    );

    expect(restricted.historicalObservations).toEqual([]);
    expect(restricted.publicSnapshot).toBeUndefined();

    const user = userEvent.setup();
    render(<OverlayApp />);
    await waitFor(() =>
      expect(mocks.listeners.get("overlay://view-v1")).toBeTypeOf("function"),
    );
    mocks.listeners.get("overlay://view-v1")?.({
      payload: replacementEvent("overlay://view-v1", 2, permitted),
    });
    await user.click(
      await screen.findByRole("button", { name: "Expand overlay" }),
    );
    expect(await screen.findByText("Prior private tendency")).toBeVisible();

    mocks.listeners.get("overlay://view-v1")?.({
      payload: replacementEvent(
        "overlay://view-v1",
        3,
        overlayView("in_game_restricted", {
          historicalObservations: permitted.historicalObservations,
          publicSnapshot: permitted.publicSnapshot,
        }),
      ),
    });
    await waitFor(() =>
      expect(
        screen.queryByText("Prior private tendency"),
      ).not.toBeInTheDocument(),
    );
    expect(screen.queryByText("Official list")).not.toBeInTheDocument();
  });

  test("UT-112: failed save renders the fallback and preserves recoverable input", async () => {
    mocks.invoke.mockImplementation((command) => {
      if (command === "save_observation") {
        return Promise.resolve({
          ok: false,
          error: {
            code: "save_failed",
            message: "The observation could not be saved.",
            retryable: true,
          },
        });
      }
      return Promise.resolve({ ok: true, data: true, revision: 1 });
    });
    const user = userEvent.setup();
    render(<CaptureApp />);
    await waitFor(() =>
      expect(mocks.listeners.get("capture://draft-v1")).toBeTypeOf("function"),
    );
    mocks.listeners.get("capture://draft-v1")?.({
      payload: replacementEvent("capture://draft-v1", 4, {
        encounterId: "01900000-0000-7000-8000-000000000001",
        windowInstance: "01900000-0000-7000-8000-000000000002",
        text: "Recover this note",
        revision: 4,
      }),
    });

    const editor = await screen.findByRole("textbox", {
      name: "Observation",
    });
    expect(editor).toHaveValue("Recover this note");
    await user.type(editor, " after failure");
    await user.keyboard("{Enter}");

    expect(
      await screen.findByText(
        /The observation could not be saved\. Your text is preserved for retry or copying\./,
      ),
    ).toBeVisible();
    expect(editor).toHaveValue("Recover this note after failure");
    expect(mocks.hide).not.toHaveBeenCalled();
  });

  test("UT-101: user-invoked capture focuses the observation editor", async () => {
    render(<CaptureApp />);
    await waitFor(() =>
      expect(mocks.listeners.get("capture://draft-v1")).toBeTypeOf("function"),
    );
    mocks.listeners.get("capture://draft-v1")?.({
      payload: replacementEvent("capture://draft-v1", 1, {
        encounterId: "01900000-0000-7000-8000-000000000003",
        windowInstance: "01900000-0000-7000-8000-000000000004",
        text: "",
        revision: 1,
      }),
    });
    await waitFor(() =>
      expect(
        screen.getByRole("textbox", { name: "Observation" }),
      ).toHaveFocus(),
    );
  });
});
