export const ERROR_CODES = [
  "acknowledgement_required",
  "already_open",
  "assets_invalid",
  "blank_observation",
  "cancel_unsafe",
  "candidate_stale",
  "consent_required",
  "deck_incomplete",
  "destination_unwritable",
  "disclosure_restricted",
  "explicit_correction_required",
  "format_unsupported",
  "identity_conflict",
  "internal_error",
  "input_too_long",
  "interactive_required",
  "invalid_backup",
  "invalid_card",
  "invalid_cursor",
  "invalid_handle",
  "invalid_request",
  "invalid_transition",
  "job_busy",
  "key_unavailable",
  "merge_conflict",
  "migration_failed",
  "no_active_encounter",
  "notebook_invalid",
  "not_found",
  "operation_busy",
  "ocr_language_missing",
  "overlay_unavailable",
  "provider_invalid_response",
  "provider_unavailable",
  "redaction_failed",
  "revision_conflict",
  "save_failed",
  "scope_mismatch",
  "signature_invalid",
  "stale_provider_result",
  "unauthorized_caller",
  "undo_expired",
  "unsafe_source",
  "update_unavailable",
  "window_not_found",
  "wrong_passphrase",
] as const;

export type ErrorCode = (typeof ERROR_CODES)[number];
export type CallerIdentity = "main" | "overlay" | "capture";

export type AppError = {
  code: ErrorCode;
  message: string;
  retryable: boolean;
  field?: string;
};

export type CommandResult<T> =
  { ok: true; data: T; revision: number } | { ok: false; error: AppError };

export type Settings = {
  schemaVersion: number;
  providerAccessEnabled: boolean;
  overlayEnabled: boolean;
  trayEnabled: boolean;
  launchWithWindows: boolean;
  updateChecksEnabled: boolean;
  classifierUpdateChecksEnabled: boolean;
  diagnosticsEnabled: boolean;
};

export type BootstrapState = {
  app: {
    name: string;
    version: string;
    localOnly: true;
  };
  settings: Settings;
  encounter: EncounterBootstrapView | null;
  caller: CallerIdentity;
};

export type InternalPhase =
  | "idle"
  | "candidate"
  | "pre_match"
  | "in_game_restricted"
  | "between_games"
  | "completion_pending"
  | "finished"
  | "incomplete";

export type EncounterBootstrapView = {
  id: string;
  phase: InternalPhase;
  revision: number;
};

export type MutationRequest = {
  idempotencyKey: string;
  expectedRevision: number;
};

export function commandSuccess<T>(data: T, revision: number): CommandResult<T> {
  return { ok: true, data, revision };
}

export function commandFailure<T = never>(error: AppError): CommandResult<T> {
  return { ok: false, error };
}

export function isValidIdempotencyKey(value: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
    value,
  );
}

export function validateMutationRequest(
  request: MutationRequest,
): AppError | null {
  const idempotencyKeyIsValid = isValidIdempotencyKey(request.idempotencyKey);
  if (
    !idempotencyKeyIsValid ||
    !Number.isSafeInteger(request.expectedRevision) ||
    request.expectedRevision < 0
  ) {
    return {
      code: "invalid_request",
      message: "A valid idempotency key and revision are required.",
      retryable: false,
      field: !idempotencyKeyIsValid ? "idempotencyKey" : "expectedRevision",
    };
  }

  return null;
}
