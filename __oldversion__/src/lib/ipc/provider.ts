import { invoke } from "@tauri-apps/api/core";

import type { CommandResult } from "./contracts";
import { createIdempotencyKey } from "./notebook";

export type SelectedWindowStatus = {
  authorized: boolean;
  visible: boolean;
  minimized: boolean;
};

export type ProviderStatus = {
  providerId: "windows_visible_mtgo";
  disclosureVersion: 1;
  disclosedFields: string[];
  consentGranted: boolean;
  available: boolean;
  paused: boolean;
  generation: number;
  selectedWindow?: SelectedWindowStatus;
  manualAvailable: true;
};

export type AuthorizedWindow = {
  nativeHandle: number;
  className: string;
  visibleTitle: string;
  selectedAt: number;
  visible: boolean;
  minimized: boolean;
  usableBounds: boolean;
};

export function listProviders() {
  return invoke<CommandResult<ProviderStatus[]>>("list_providers");
}

export function listMtgoWindows() {
  return invoke<CommandResult<AuthorizedWindow[]>>("list_mtgo_windows");
}

export function setProviderConsent(
  granted: boolean,
  disclosedFields: string[],
) {
  return invoke<CommandResult<ProviderStatus>>("set_provider_consent", {
    request: {
      providerId: "windows_visible_mtgo",
      disclosureVersion: 1,
      disclosedFields,
      granted,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function selectMtgoWindow(window: AuthorizedWindow) {
  return invoke<CommandResult<ProviderStatus>>("select_mtgo_window", {
    request: {
      ...window,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function pauseDetection(paused: boolean) {
  return invoke<CommandResult<ProviderStatus>>("pause_detection", {
    request: { paused, idempotencyKey: createIdempotencyKey() },
  });
}
