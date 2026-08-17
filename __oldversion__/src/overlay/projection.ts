import type { InternalPhase } from "../lib/ipc/contracts";
import type { ReplacementEvent } from "../lib/ipc/events";

export type OverlayObservation = {
  id: string;
  text: string;
  editable: boolean;
};

export type OverlayView = {
  phase: InternalPhase;
  confirmedHandle?: string;
  currentObservations: OverlayObservation[];
  historicalObservations: OverlayObservation[];
  publicSnapshot?: {
    label: string;
    format: string;
    publishedAt: number;
    sourceText: string;
    available: boolean;
  };
  historyEditable: boolean;
  needsIdentityResolution: boolean;
};

export const NEUTRAL_OVERLAY_VIEW: OverlayView = {
  phase: "idle",
  currentObservations: [],
  historicalObservations: [],
  historyEditable: false,
  needsIdentityResolution: false,
};

export function applyOverlayReplacement(
  current: OverlayView,
  event: ReplacementEvent<OverlayView>,
): OverlayView {
  if (
    event.payload.phase === "in_game_restricted" ||
    event.payload.phase === "candidate" ||
    event.payload.phase === "completion_pending" ||
    event.payload.phase === "incomplete"
  ) {
    return {
      ...event.payload,
      historicalObservations: [],
      publicSnapshot: undefined,
    };
  }
  return event.payload ?? current;
}
