import { invoke } from "@tauri-apps/api/core";

import type { CommandResult } from "./contracts";
import type { ClassificationRun } from "./decks";

export type ReclassificationProgress = {
  jobId: string;
  classifierVersion: string;
  cursor?: string;
  completed: number;
  total: number;
  state: "requested" | "running" | "paused" | "completed" | "failed";
};

export function getClassification(deckRevisionId: string) {
  return invoke<CommandResult<ClassificationRun[]>>("get_classification", {
    request: { deckRevisionId },
  });
}

export function startReclassification() {
  return invoke<CommandResult<ReclassificationProgress>>(
    "start_reclassification",
  );
}
