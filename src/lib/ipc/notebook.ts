import { invoke } from "@tauri-apps/api/core";

import type { CommandResult } from "./contracts";

export type CardCertainty = "observed" | "suspected";

export type CardObservationInput = {
  oracleId?: string;
  displayName: string;
  quantity: number;
  certainty: CardCertainty;
  context?: string;
};

export type CardObservationView = CardObservationInput & {
  oracleId: string;
};

export type TagView = {
  id: string;
  displayLabel: string;
  normalizedLabel: string;
};

export type ObservationDetail = {
  id: string;
  encounterId: string;
  text: string;
  encounterStartedAt: number;
  createdAt: number;
  editedAt?: number;
  revision: number;
  cards: CardObservationView[];
  tags: TagView[];
  userDeckLabel?: string;
  source: "player_observation";
};

export type OpponentProfile = {
  id: string;
  primaryHandle: string;
  normalizedHandle: string;
  createdAt: number;
  revision: number;
  deletedAt?: number;
};

export type OpponentAlias = {
  id: string;
  profileId: string;
  displayHandle: string;
  normalizedHandle: string;
  provenance: string;
};

export type ProfileAggregate = {
  profile: OpponentProfile;
  aliases: OpponentAlias[];
};

export type HistoryHit = {
  entityType: string;
  entityId: string;
  sortMs: number;
  content: string;
};

export type HistoryPage = {
  items: HistoryHit[];
  nextCursor?: string;
  replacement: true;
};

export type EncounterSummary = {
  id: string;
  format: string;
  startedAt: number;
  endedAt?: number;
  status: string;
  phase: string;
  source: string;
  incompleteReason?: string;
  revision: number;
  observationCount: number;
};

export type LastDeckSeen = {
  label: string;
  sourceClass: "public" | "user";
  sourceLabel: string;
  format: string;
  seenAt: number;
  confirmed: boolean;
};

export type ProfileDetail = {
  profile: ProfileAggregate;
  encounters: EncounterSummary[];
  lastDeckSeen?: LastDeckSeen;
  canonicalProfileId?: string;
};

export type EncounterDetail = {
  summary: EncounterSummary;
  profileId: string;
  observations: ObservationDetail[];
};

export type IdentityCounts = {
  profiles: number;
  aliases: number;
  encounters: number;
  observations: number;
  decks: number;
};

export type MergePreview = {
  primaryProfileId: string;
  secondaryProfileId: string;
  primaryHandle: string;
  secondaryHandle: string;
  expectedPrimaryRevision: number;
  expectedSecondaryRevision: number;
  affected: IdentityCounts;
  conflicts: string[];
  conflictCount: number;
  conflictDetailsBounded: boolean;
  irreversibleConsequences: string[];
  planToken: string;
};

export type MergeResult = {
  mergeId: string;
  canonicalProfileId: string;
  canonicalRevision: number;
  reversible: boolean;
};

export type UnmergePreview = {
  mergeId: string;
  primaryProfileId: string;
  secondaryProfileId: string;
  restoredEncounters: number;
  restoredDecks: number;
  postMergeEncounters: number;
  postMergeDecks: number;
  proposedPostMergeAssignment: "retain_with_primary";
  planToken: string;
};

export type DeletionEntityType =
  "observation" | "encounter" | "profile" | "notebook";

export type DeletionPreview = {
  entityType: DeletionEntityType;
  entityId: string;
  displayName: string;
  counts: {
    profiles: number;
    aliases: number;
    encounters: number;
    observations: number;
    decks: number;
    publicSnapshots: number;
  };
  dependencies: string[];
  confirmation: string;
  scopeToken: string;
};

export type DeletionResult = {
  entityType: DeletionEntityType;
  entityId: string;
  requestedAt: number;
  undoDeadline: number;
  undoToken: string;
  tombstoneState: string;
};

export function createIdempotencyKey(now = Date.now()): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  let timestamp = BigInt(now);
  for (let index = 5; index >= 0; index -= 1) {
    bytes[index] = Number(timestamp & 0xffn);
    timestamp >>= 8n;
  }
  bytes[6] = (bytes[6] & 0x0f) | 0x70;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20),
  ].join("-");
}

export function createProfile(handle: string) {
  return invoke<CommandResult<ProfileAggregate>>("create_profile", {
    request: { handle, idempotencyKey: createIdempotencyKey() },
  });
}

export function addAlias(profileId: string, handle: string) {
  return invoke<CommandResult<ProfileAggregate>>("add_alias", {
    request: {
      profileId,
      handle,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function updateProfile(
  profileId: string,
  handle: string,
  expectedRevision: number,
) {
  return invoke<CommandResult<ProfileAggregate>>("update_profile", {
    request: {
      profileId,
      handle,
      expectedRevision,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function saveObservation(request: {
  encounterId: string;
  text: string;
  cards?: CardObservationInput[];
  tags?: string[];
  userDeckLabel?: string;
}) {
  return invoke<CommandResult<ObservationDetail>>("save_observation", {
    request: {
      cards: [],
      tags: [],
      ...request,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function searchHistory(request: {
  text: string;
  cursor?: string;
  pageSize?: number;
  filters?: {
    entityTypes?: string[];
    dateFrom?: number;
    dateTo?: number;
    certainty?: CardCertainty;
  };
}) {
  return invoke<CommandResult<HistoryPage>>("search_history", {
    request: {
      pageSize: 50,
      filters: { entityTypes: [] },
      ...request,
    },
  });
}

export function getProfile(id: string) {
  return invoke<CommandResult<ProfileDetail>>("get_profile", {
    request: { id },
  });
}

export function getEncounter(id: string) {
  return invoke<CommandResult<EncounterDetail>>("get_encounter", {
    request: { id },
  });
}

export function updateObservation(
  observationId: string,
  text: string,
  expectedRevision: number,
) {
  return invoke<CommandResult<ObservationDetail>>("update_observation", {
    request: {
      observationId,
      text,
      expectedRevision,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function setCardObservations(
  observationId: string,
  cards: CardObservationInput[],
  expectedRevision: number,
) {
  return invoke<CommandResult<ObservationDetail>>("set_card_observations", {
    request: {
      observationId,
      cards,
      expectedRevision,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function setTendencyTags(
  observationId: string,
  tags: string[],
  expectedRevision: number,
) {
  return invoke<CommandResult<ObservationDetail>>("set_tendency_tags", {
    request: {
      observationId,
      tags,
      expectedRevision,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function previewMerge(
  leftProfileId: string,
  rightProfileId: string,
  primaryProfileId: string,
) {
  return invoke<CommandResult<MergePreview>>("preview_merge", {
    request: { leftProfileId, rightProfileId, primaryProfileId },
  });
}

export function applyMerge(preview: MergePreview) {
  return invoke<CommandResult<MergeResult>>("apply_merge", {
    request: { preview, idempotencyKey: createIdempotencyKey() },
  });
}

export function previewUnmerge(mergeId: string) {
  return invoke<CommandResult<UnmergePreview>>("preview_unmerge", {
    request: { mergeId },
  });
}

export function applyUnmerge(preview: UnmergePreview) {
  return invoke<CommandResult<MergeResult>>("apply_unmerge", {
    request: { preview, idempotencyKey: createIdempotencyKey() },
  });
}

export function previewDeletion(
  entityType: DeletionEntityType,
  entityId: string,
) {
  return invoke<CommandResult<DeletionPreview>>("preview_deletion", {
    request: { entityType, entityId },
  });
}

export function requestDeletion(
  preview: DeletionPreview,
  confirmation: string,
) {
  return invoke<CommandResult<DeletionResult>>("request_deletion", {
    request: {
      preview,
      confirmation,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function undoDeletion(result: DeletionResult) {
  return invoke<
    CommandResult<{
      entityType: DeletionEntityType;
      entityId: string;
      restored: boolean;
    }>
  >("undo_deletion", {
    request: {
      entityType: result.entityType,
      entityId: result.entityId,
      undoToken: result.undoToken,
    },
  });
}
