import { invoke } from "@tauri-apps/api/core";

import type { CommandResult } from "./contracts";
import { createIdempotencyKey } from "./notebook";

export type DeckZone = "main" | "sideboard";

export type DeckCard = {
  oracleId: string;
  displayName: string;
  zone: DeckZone;
  quantity: number;
  basicLand?: boolean;
};

export type RequestBinding = {
  encounterGeneration: number;
  requestToken: string;
  confirmedHandle: string;
  format: string;
};

export type InteractiveLookup = {
  status: "interactive_required";
  accessMode: "interactive_required";
  officialUrl: string;
  binding: RequestBinding;
};

export type DeckCandidate = {
  provider: "official_mtgo";
  event: string;
  format: string;
  publicationDate: number;
  sourceUrl: string;
  providerLabel?: string;
  responseToken: string;
  encounterGeneration: number;
  cards: DeckCard[];
};

export type ClassificationMethod = "signature" | "knn" | "unsupported";

export type ClassificationRun = {
  id: string;
  deckRevisionId: string;
  classifierVersion: string;
  classifierDigest: string;
  resultId: string;
  resultName: string;
  method: ClassificationMethod;
  confidence: number;
  explanation: {
    summary: string;
    matchedSignatureCards: string[];
    neighbors: {
      corpusId: string;
      archetypeId: string;
      similarity: number;
    }[];
    decisiveRule?: string;
  };
  status: string;
  createdAt: number;
};

export type DeckDetails = {
  deckId: string;
  deckRevisionId: string;
  revisionNumber: number;
  canonicalDigest: string;
  complete: boolean;
  format: string;
  sourceClass: "public" | "user";
  providerLabel?: string;
  userLabel?: string;
  cards: DeckCard[];
  publicSnapshot?: {
    id: string;
    encounterId: string;
    provider: string;
    event: string;
    format: string;
    publicationDate: number;
    sourceUrl: string;
    confirmed: boolean;
    sourceToken: string;
    createdAt: number;
  };
  currentClassification?: ClassificationRun;
  classificationHistory: ClassificationRun[];
};

export function setDeckProviderConsent(granted: boolean) {
  return invoke<
    CommandResult<{
      providerId: string;
      consentGranted: boolean;
      accessMode: "interactive_required";
      disclosedFields: ["confirmed_handle", "format"];
      automaticAccessEnabled: false;
    }>
  >("set_deck_provider_consent", {
    request: { granted, idempotencyKey: createIdempotencyKey() },
  });
}

export function lookupOfficialDeck(request: {
  encounterId: string;
  encounterGeneration: number;
  requestToken: string;
}) {
  return invoke<CommandResult<InteractiveLookup>>("lookup_official_deck", {
    request,
  });
}

export function confirmPublicSnapshot(request: {
  encounterId: string;
  candidate: DeckCandidate;
  activeGeneration: number;
  activeFormat: string;
}) {
  return invoke<CommandResult<DeckDetails>>("confirm_public_snapshot", {
    request: { ...request, idempotencyKey: createIdempotencyKey() },
  });
}

export function saveCompleteDeck(request: {
  deckId?: string;
  profileId: string;
  format: string;
  userLabel?: string;
  cards: DeckCard[];
}) {
  return invoke<CommandResult<DeckDetails>>("save_complete_deck", {
    request: {
      deck: request,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function getDeckDetails(deckId: string) {
  return invoke<CommandResult<DeckDetails>>("get_deck_details", {
    request: { deckId },
  });
}

export function openOfficialDeckPage(url: string) {
  return invoke<CommandResult<null>>("open_official_deck_page", {
    request: { url },
  });
}
