import { invoke } from "@tauri-apps/api/core";

import type { CommandResult } from "./contracts";
import type { ReclassificationProgress } from "./classifier";

export type SignedRelease = {
  version: string;
  releaseNotes: string;
  classifierChangeSummary: string;
  artifactDigest: string;
  metadataSignature: string;
  artifactSignature: string;
};

export type UpdateStage =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "verifying"
  | "awaiting_confirmation"
  | "installing"
  | "completed"
  | "failed";

export type UpdateStatus = {
  stage: UpdateStage;
  version?: string;
  errorCode?: string;
};

export type ClassifierUpdateView = {
  classifierVersion: string;
  digest: string;
  formats: string[];
};

export function checkUpdate() {
  return invoke<CommandResult<SignedRelease>>("check_update");
}

export function installUpdate(version: string, confirmed: boolean) {
  return invoke<CommandResult<UpdateStatus>>("install_update", {
    request: { version, confirmed },
  });
}

export function checkClassifierUpdate() {
  return invoke<CommandResult<ClassifierUpdateView>>("check_classifier_update");
}

export function installClassifierUpdate(confirmed: boolean) {
  return invoke<CommandResult<ReclassificationProgress>>(
    "install_classifier_update",
    { request: { confirmed } },
  );
}
