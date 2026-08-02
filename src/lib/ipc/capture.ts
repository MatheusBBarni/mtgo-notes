import { invoke } from "@tauri-apps/api/core";

import type { CommandResult } from "./contracts";
import { createIdempotencyKey } from "./notebook";

export type CaptureDraftView = {
  encounterId: string;
  windowInstance: string;
  text: string;
  revision: number;
};

export function openCapture() {
  return invoke<CommandResult<CaptureDraftView>>("open_capture", {
    request: { idempotencyKey: createIdempotencyKey() },
  });
}

export function discardDraft(encounterId: string, windowInstance: string) {
  return invoke<CommandResult<CaptureDraftView>>("discard_draft", {
    request: {
      encounterId,
      windowInstance,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function setOverlayInteraction(expanded: boolean) {
  return invoke<CommandResult<{ expanded: boolean; clickThrough: boolean }>>(
    "set_overlay_interaction",
    {
      request: { expanded },
    },
  );
}
