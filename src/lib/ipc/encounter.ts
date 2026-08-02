import { invoke } from "@tauri-apps/api/core";

import type { CommandResult, InternalPhase } from "./contracts";
import { createIdempotencyKey } from "./notebook";

export type EncounterCommandView = {
  id: string;
  profileId: string;
  primaryHandle: string;
  phase: InternalPhase;
  generation: number;
  revision: number;
  undoGroupId?: string;
  undoDeadline?: number;
};

export type OpponentCandidate = {
  displayHandle: string;
  normalizedHandle: string;
  providerSession: string;
  generation: number;
  sequence: number;
  provenance: "uia" | "ocr" | "manual";
};

export type EncounterStateView = {
  candidate?: OpponentCandidate;
  encounter?: EncounterCommandView;
};

export function confirmOpponent(
  candidate: OpponentCandidate,
  correctedHandle?: string,
) {
  return invoke<CommandResult<EncounterCommandView>>("confirm_opponent", {
    request: {
      providerSession: candidate.providerSession,
      candidateGeneration: candidate.generation,
      candidateSequence: candidate.sequence,
      correctedHandle,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function enterOpponent(handle: string) {
  return invoke<CommandResult<EncounterCommandView>>("enter_opponent", {
    request: { handle, idempotencyKey: createIdempotencyKey() },
  });
}

export function correctPhase(
  encounterId: string,
  phase: InternalPhase,
  expectedRevision: number,
) {
  return invoke<CommandResult<EncounterCommandView>>("correct_phase", {
    request: {
      encounterId,
      phase,
      expectedRevision,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function finishEncounter(encounterId: string) {
  return invoke<CommandResult<EncounterCommandView>>("finish_encounter", {
    request: { encounterId, idempotencyKey: createIdempotencyKey() },
  });
}

export function reopenEncounter(encounterId: string) {
  return invoke<CommandResult<unknown>>("reopen_encounter", {
    request: { encounterId, idempotencyKey: createIdempotencyKey() },
  });
}

export function undoTransition(undoGroupId: string) {
  return invoke<CommandResult<EncounterCommandView>>("undo_transition", {
    request: { undoGroupId, idempotencyKey: createIdempotencyKey() },
  });
}
