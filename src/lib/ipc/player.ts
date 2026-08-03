import { invoke } from "@tauri-apps/api/core";

import type { CommandResult, InternalPhase } from "./contracts";
import { createIdempotencyKey } from "./notebook";

export type PlayerRoute =
  "census_mocs" | "official_mtgo_browser" | "mtg_top8_browser";
export type ProviderAvailability =
  | "disabled"
  | "ready"
  | "invalid"
  | "expired"
  | "cooldown"
  | "busy"
  | "unavailable";

export type PlayerIdentityView = {
  id: string;
  displayNickname: string;
  normalizedNickname: string;
  createdAt: number;
  updatedAt: number;
  revision: number;
};

export type PlayerSourceStatus = {
  route: PlayerRoute;
  availability: ProviderAvailability;
  consentGranted: boolean;
  disclosureVersion?: string;
  retryAt?: number;
};

export type PlayerCardView = {
  oracleId: string;
  displayName: string;
  zone: "main" | "sideboard" | "companion" | "other";
  quantity: number;
  basicLand: boolean;
};

export type PlayerClassificationView = {
  classifierVersion: string;
  classifierDigest: string;
  resultId: string;
  resultName: string;
  method: "signature" | "knn" | "unsupported";
  confidence: number;
  createdAt: number;
};

export type PlayerEvidenceView = {
  id: string;
  playerIdentityId: string;
  kind: "mocs_leaderboard_entry" | "official_published_decklist";
  provenanceMode: "provider_observed" | "user_attested_official_source";
  providerId: string;
  attributionUrl: string;
  canonicalSourceUrl?: string;
  lookupNickname: string;
  sourceNickname: string;
  exactMatchRule: string;
  scope: Record<string, unknown>;
  observedAt: number;
  importedAt: number;
  sourceKey: string;
  sourceDigest: string;
  previewDigest: string;
  payload: Record<string, unknown>;
  selectedFields: Record<string, boolean>;
  supersedesEvidenceId?: string;
  cards: PlayerCardView[];
  classification?: PlayerClassificationView;
};

export type PlayerEvidencePage = {
  items: PlayerEvidenceView[];
  nextCursor?: string;
};

export type PlayerLookupView = {
  state: "idle" | "loading" | "candidates" | "empty" | "degraded" | "cancelled";
  message: string;
  candidates: PlayerCandidateView[];
  operationKey?: string;
  errorCode?: string;
};

export type PlayerCandidateView = {
  token?: string;
  sourceKey: string;
  sourceDigest: string;
  previewDigest: string;
  lookupNickname: string;
  sourceNickname: string;
  payload: Record<string, unknown>;
  approvedFields: string[];
};

export type PlayerManualPreviewView = {
  token: string;
  playerIdentityId: string;
  identityRevision: number;
  evidence: PlayerEvidenceView;
  approvedFields: string[];
};

export type PlayerDeletionView = {
  token: string;
  digest: string;
  target: "identity" | "evidence" | "empty_outcome";
  playerIdentityId: string;
  identityRevision: number;
  counts: {
    evidence: number;
    cards: number;
    selections: number;
    classifications: number;
    emptyOutcomes: number;
    consents: number;
  };
  expiresAt: number;
};

export type PlayerWorkspaceView = {
  revision: number;
  identity: PlayerIdentityView | null;
  sources: PlayerSourceStatus[];
  lookup: PlayerLookupView;
  evidence: PlayerEvidencePage;
  deletion: PlayerDeletionView | null;
};

export type ManualEvidenceInput = {
  eventTitle: string;
  eventDate: string;
  format: string;
  placement?: string;
  record?: string;
  sourceNickname: string;
  attributionUrl: string;
  contents: "reference_only" | "complete_deck";
  cards: PlayerCardView[];
};

export type PlayerPageRequest = { cursor?: string; limit?: number };

export function operationKey(): string {
  return createIdempotencyKey();
}

export function getPlayerWorkspace(
  page: PlayerPageRequest = {},
): Promise<CommandResult<PlayerWorkspaceView>> {
  return invoke("get_player_workspace", { page });
}

export function savePlayerIdentity(request: {
  displayNickname: string;
  expectedRevision?: number;
  idempotencyKey: string;
}): Promise<CommandResult<PlayerIdentityView>> {
  return invoke("save_player_identity", { request });
}

export function grantPlayerConsent(request: {
  route: PlayerRoute;
  disclosureVersion: string;
  fieldsDigest: string;
  idempotencyKey: string;
}): Promise<CommandResult<PlayerSourceStatus>> {
  return invoke("set_public_provider_consent", {
    request: { ...request, granted: true },
  });
}

export function revokePlayerConsent(request: {
  route: PlayerRoute;
  idempotencyKey: string;
}): Promise<CommandResult<PlayerSourceStatus>> {
  return invoke("set_public_provider_consent", {
    request: { ...request, granted: false },
  });
}

export function startPlayerLookup(request: {
  identityRevision: number;
  consentVersion: string;
  fieldsDigest: string;
  operationKey: string;
  phase?: InternalPhase;
}): Promise<CommandResult<PlayerLookupView>> {
  return invoke("start_public_result_lookup", { request });
}

export function cancelPlayerLookup(
  operationKeyValue: string,
): Promise<CommandResult<PlayerLookupView>> {
  return invoke("cancel_public_result_lookup", {
    request: { operationKey: operationKeyValue },
  });
}

export function refreshPlayerResults(request: {
  identityRevision: number;
  operationKey: string;
}): Promise<CommandResult<PlayerLookupView>> {
  return invoke("refresh_public_results", { request });
}

export function openPlayerSource(request: {
  route: Exclude<PlayerRoute, "census_mocs">;
  operationKey: string;
}): Promise<CommandResult<{ opened: boolean }>> {
  return invoke("open_public_source", { request });
}

export function previewManualPlayerEvidence(request: {
  input: ManualEvidenceInput;
  identityRevision: number;
  operationKey: string;
}): Promise<CommandResult<PlayerManualPreviewView>> {
  return invoke("create_manual_evidence_preview", { request });
}

export function importPlayerEvidence(request: {
  token: string;
  previewDigest: string;
  selectedFields: Record<string, boolean>;
  operationKey: string;
}): Promise<CommandResult<PlayerEvidenceView>> {
  return invoke("import_public_result", { request });
}

export function updatePlayerSelection(request: {
  evidenceId: string;
  expectedRevision: number;
  selectedFields: Record<string, boolean>;
  operationKey: string;
}): Promise<CommandResult<PlayerEvidenceView>> {
  return invoke("update_evidence_selection", { request });
}

export function previewPlayerDeletion(request: {
  target: PlayerDeletionView["target"];
  targetId?: string;
  operationKey: string;
}): Promise<CommandResult<PlayerDeletionView>> {
  return invoke("preview_player_deletion", { request });
}

export function confirmPlayerDeletion(request: {
  token: string;
  digest: string;
  operationKey: string;
}): Promise<CommandResult<PlayerDeletionView>> {
  return invoke("confirm_player_deletion", { request });
}
