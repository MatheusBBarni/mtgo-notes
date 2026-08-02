import { invoke } from "@tauri-apps/api/core";

import type { BootstrapState, CommandResult, Settings } from "./contracts";

export function bootstrap(): Promise<CommandResult<BootstrapState>> {
  return invoke("bootstrap");
}

export function getSettings(): Promise<CommandResult<Settings>> {
  return invoke("get_settings");
}

export function updateSettings(request: {
  idempotencyKey: string;
  expectedRevision: number;
  settings: Settings;
}): Promise<CommandResult<Settings>> {
  return invoke("update_settings", { request });
}
