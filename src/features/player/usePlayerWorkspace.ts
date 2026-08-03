import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useReducer, useRef } from "react";

import {
  getPlayerWorkspace,
  type PlayerWorkspaceView,
} from "../../lib/ipc/player";
import {
  acceptNewerReplacementEvent,
  type ReplacementEvent,
} from "../../lib/ipc/events";

export const FIRST_USE_PLAYER_VIEW: PlayerWorkspaceView = {
  revision: 0,
  identity: null,
  sources: [],
  lookup: {
    state: "idle",
    message: "Ready for a Player identity.",
    candidates: [],
  },
  evidence: { items: [] },
  deletion: null,
};

export type PlayerWorkspaceState = {
  view: PlayerWorkspaceView;
  needsRefresh: boolean;
  error?: string;
};

export type PlayerWorkspaceAction =
  | { type: "replace"; view: PlayerWorkspaceView }
  | { type: "invalid" | "gap"; message: string }
  | { type: "error"; message: string }
  | { type: "clearError" };

export function playerWorkspaceReducer(
  state: PlayerWorkspaceState,
  action: PlayerWorkspaceAction,
): PlayerWorkspaceState {
  switch (action.type) {
    case "replace":
      if (!isValidPlayerWorkspaceView(action.view)) {
        return {
          ...state,
          needsRefresh: true,
          error: "Player view unavailable.",
        };
      }
      if (action.view.revision <= state.view.revision) return state;
      if (action.view.revision > state.view.revision + 1) {
        return {
          ...state,
          needsRefresh: true,
          error: "Player view changed. Refreshing.",
        };
      }
      return { view: action.view, needsRefresh: false, error: undefined };
    case "invalid":
    case "gap":
      return { ...state, needsRefresh: true, error: action.message };
    case "error":
      return { ...state, error: action.message };
    case "clearError":
      return { ...state, error: undefined };
  }
}

export function isValidPlayerWorkspaceView(
  value: unknown,
): value is PlayerWorkspaceView {
  if (typeof value !== "object" || value === null) return false;
  const view = value as Partial<PlayerWorkspaceView>;
  const revision = view.revision;
  return (
    Number.isSafeInteger(revision) &&
    typeof revision === "number" &&
    revision >= 0 &&
    (view.identity === null || typeof view.identity === "object") &&
    Array.isArray(view.sources) &&
    view.sources.length <= 3 &&
    Array.isArray(view.lookup?.candidates) &&
    view.lookup.candidates.length <= 10 &&
    Array.isArray(view.evidence?.items) &&
    view.evidence.items.length <= 100
  );
}

export function usePlayerWorkspace() {
  const [state, dispatch] = useReducer(playerWorkspaceReducer, {
    view: FIRST_USE_PLAYER_VIEW,
    needsRefresh: false,
  });
  const revision = useRef(0);

  const refresh = useCallback(async () => {
    try {
      const result = await getPlayerWorkspace();
      if (!result.ok) {
        dispatch({ type: "error", message: result.error.message });
        return;
      }
      revision.current = result.data.revision;
      dispatch({ type: "replace", view: result.data });
    } catch {
      dispatch({ type: "error", message: "Player view unavailable." });
    }
  }, []);

  useEffect(() => {
    let disposed = false;
    let stop: (() => void) | undefined;
    void refresh();
    void listen<ReplacementEvent<PlayerWorkspaceView>>(
      "player://workspace-v1",
      (incoming) => {
        const accepted = acceptNewerReplacementEvent<PlayerWorkspaceView>(
          incoming.payload,
          revision.current,
          {
            clearSensitiveView: () =>
              dispatch({
                type: "invalid",
                message: "Player view unavailable.",
              }),
            requestBootstrap: () => void refresh(),
          },
        );
        if (!accepted) return;
        if (accepted.revision > revision.current + 1) {
          dispatch({
            type: "gap",
            message: "Player view changed. Refreshing.",
          });
          void refresh();
          return;
        }
        revision.current = accepted.revision;
        dispatch({ type: "replace", view: accepted.payload });
      },
    )
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch(() => {
        if (!disposed) {
          dispatch({ type: "error", message: "Player updates unavailable." });
        }
      });
    return () => {
      disposed = true;
      stop?.();
    };
  }, [refresh]);

  return { ...state, refresh, dispatch };
}
