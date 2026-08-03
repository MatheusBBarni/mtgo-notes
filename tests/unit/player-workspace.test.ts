import {
  FIRST_USE_PLAYER_VIEW,
  isValidPlayerWorkspaceView,
  playerWorkspaceReducer,
} from "../../src/features/player/usePlayerWorkspace";
import type { PlayerWorkspaceView } from "../../src/lib/ipc/player";

function viewAt(
  revision: number,
  state: PlayerWorkspaceView["lookup"]["state"] = "idle",
): PlayerWorkspaceView {
  return {
    ...FIRST_USE_PLAYER_VIEW,
    revision,
    lookup: {
      state,
      message: state === "idle" ? "Ready." : `Lookup ${state}.`,
      candidates: [],
    },
  };
}

describe("Player workspace replacement state", () => {
  test("starts with a non-mutating first-use projection", () => {
    expect(FIRST_USE_PLAYER_VIEW.identity).toBeNull();
    expect(FIRST_USE_PLAYER_VIEW.sources).toHaveLength(0);
    expect(FIRST_USE_PLAYER_VIEW.lookup.state).toBe("idle");
    expect(FIRST_USE_PLAYER_VIEW.evidence.items).toHaveLength(0);
  });

  test.each([1, 2])("ignores revision %i when it is not newer", (revision) => {
    const current = viewAt(2);
    const state = { view: current, needsRefresh: false };

    expect(
      playerWorkspaceReducer(state, {
        type: "replace",
        view: viewAt(revision),
      }),
    ).toBe(state);
  });

  test("accepts the next complete projection and replaces lookup state", () => {
    const current = viewAt(2);
    const next = viewAt(3, "empty");
    next.evidence = {
      items: [
        {
          id: "evidence-1",
          playerIdentityId: "player-1",
          kind: "official_published_decklist",
          provenanceMode: "user_attested_official_source",
          providerId: "official",
          attributionUrl: "https://example.test/deck",
          lookupNickname: "Teichou_Aisu",
          sourceNickname: "Teichou_Aisu",
          exactMatchRule: "exact",
          scope: {},
          observedAt: 1,
          importedAt: 2,
          sourceKey: "source-1",
          sourceDigest: "a".repeat(64),
          previewDigest: "b".repeat(64),
          payload: {},
          selectedFields: { source_nickname: true },
          cards: [],
        },
      ],
    };

    const replaced = playerWorkspaceReducer(
      { view: current, needsRefresh: false, error: "old" },
      { type: "replace", view: next },
    );

    expect(replaced.view.lookup.state).toBe("empty");
    expect(replaced.view.evidence.items).toHaveLength(1);
    expect(replaced.needsRefresh).toBe(false);
    expect(replaced.error).toBeUndefined();
  });

  test("marks a revision gap for snapshot refresh without clearing evidence", () => {
    const current = viewAt(2, "candidates");
    current.evidence = { items: [{ id: "retained" } as never] };
    const state = { view: current, needsRefresh: false };

    const next = playerWorkspaceReducer(state, {
      type: "replace",
      view: viewAt(4, "degraded"),
    });

    expect(next.needsRefresh).toBe(true);
    expect(next.view.evidence.items[0]?.id).toBe("retained");
    expect(next.error).toContain("Refreshing");
  });

  test("rejects malformed projections and keeps the last safe view", () => {
    const current = viewAt(2, "candidates");
    current.evidence = { items: [{ id: "retained" } as never] };
    const state = { view: current, needsRefresh: false };

    expect(isValidPlayerWorkspaceView({ revision: -1 })).toBe(false);
    const next = playerWorkspaceReducer(state, {
      type: "replace",
      view: { revision: 3 } as PlayerWorkspaceView,
    });

    expect(next.needsRefresh).toBe(true);
    expect(next.view.evidence.items[0]?.id).toBe("retained");
    expect(next.error).toBe("Player view unavailable.");
  });
});
