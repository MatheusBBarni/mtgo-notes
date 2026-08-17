import {
  acceptNewerReplacementEvent,
  replacementEvent,
} from "../../src/lib/ipc/events";
import {
  applyOverlayReplacement,
  type OverlayView,
} from "../../src/overlay/projection";

const handlers = {
  clearSensitiveView: vi.fn(),
  requestBootstrap: vi.fn(),
};

describe("Task 03 replacement event contracts", () => {
  beforeEach(() => {
    handlers.clearSensitiveView.mockReset();
    handlers.requestBootstrap.mockReset();
  });

  test("IT-265: encounter replacements accept only strictly increasing revisions", () => {
    const current = replacementEvent("encounter://state-v1", 9, {
      encounter: { phase: "pre_match" },
    });
    expect(acceptNewerReplacementEvent(current, 8, handlers)?.revision).toBe(9);
    expect(acceptNewerReplacementEvent(current, 9, handlers)).toBeNull();
    expect(
      acceptNewerReplacementEvent(
        replacementEvent("encounter://state-v1", 8, {
          encounter: { phase: "between_games" },
        }),
        9,
        handlers,
      ),
    ).toBeNull();
    expect(handlers.clearSensitiveView).not.toHaveBeenCalled();
    expect(handlers.requestBootstrap).not.toHaveBeenCalled();
  });

  test("IT-266: restricted overlay replacement cannot be undone by a stale permitted event", () => {
    const permitted: OverlayView = {
      phase: "pre_match",
      confirmedHandle: "Opponent",
      currentObservations: [],
      historicalObservations: [
        { id: "private", text: "Historical note", editable: false },
      ],
      historyEditable: false,
      needsIdentityResolution: false,
    };
    const restrictedEvent = replacementEvent("overlay://view-v1", 11, {
      ...permitted,
      phase: "in_game_restricted" as const,
    });
    const restricted = applyOverlayReplacement(permitted, restrictedEvent);
    expect(restricted.historicalObservations).toEqual([]);

    const stale = acceptNewerReplacementEvent(
      replacementEvent("overlay://view-v1", 10, permitted),
      restrictedEvent.revision,
      handlers,
    );
    expect(stale).toBeNull();
    expect(restricted.historicalObservations).toEqual([]);
  });

  test("IT-267: capture replacement carries one exact encounter/window claim and draft", () => {
    const event = replacementEvent("capture://draft-v1", 4, {
      encounterId: "encounter-1",
      windowInstance: "capture-1",
      text: "recoverable draft",
      revision: 4,
    });
    expect(acceptNewerReplacementEvent(event, 3, handlers)?.payload).toEqual({
      encounterId: "encounter-1",
      windowInstance: "capture-1",
      text: "recoverable draft",
      revision: 4,
    });
  });

  test("IT-269: provider status replacement contains status but no native window handle", () => {
    const event = replacementEvent("provider://status-v1", 5, {
      providerId: "windows_visible_mtgo",
      consentGranted: true,
      available: true,
      paused: false,
      generation: 2,
      selectedWindow: {
        authorized: true,
        visible: true,
        minimized: false,
      },
      manualAvailable: true,
    });
    const serialized = JSON.stringify(
      acceptNewerReplacementEvent(event, 4, handlers)?.payload,
    );
    expect(serialized).not.toContain("nativeHandle");
    expect(serialized).not.toContain("opponent");
  });
});
