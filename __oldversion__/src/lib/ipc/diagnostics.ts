import { invoke } from "@tauri-apps/api/core";

import type { CommandResult } from "./contracts";
import { createIdempotencyKey } from "./notebook";

export type DiagnosticArtifactPreview = {
  fileName: string;
  fieldClasses: string[];
  eventCount: number;
  redactionCount: number;
  omitted: boolean;
  omissionCode?: string;
};

export type DiagnosticsPreview = {
  previewToken: string;
  artifacts: DiagnosticArtifactPreview[];
  totalEvents: number;
  totalRedactions: number;
  summarized: boolean;
  expiresAt: number;
};

export type DiagnosticsBundleResult = {
  fileName: string;
  artifactCount: number;
  eventCount: number;
  networkRequests: 0;
};

export type DiagnosticsPathSelection = {
  selectionToken: string;
  fileName: string;
};

export function previewDiagnostics() {
  return invoke<CommandResult<DiagnosticsPreview>>("preview_diagnostics");
}

export function selectDiagnosticsPath() {
  return invoke<CommandResult<DiagnosticsPathSelection | null>>(
    "select_diagnostics_path",
  );
}

export function createDiagnostics(
  previewToken: string,
  selectionToken: string,
) {
  return invoke<CommandResult<DiagnosticsBundleResult>>("create_diagnostics", {
    request: {
      idempotencyKey: createIdempotencyKey(),
      previewToken,
      selectionToken,
    },
  });
}

export function cancelDiagnostics(previewToken: string) {
  return invoke<CommandResult<boolean>>("cancel_diagnostics", {
    request: { previewToken },
  });
}
