import { invoke } from "@tauri-apps/api/core";

import type { CommandResult } from "./contracts";
import { createIdempotencyKey } from "./notebook";
import type { OperationRecord } from "./operations";

export type SelectionPurpose =
  "backup_destination" | "restore_source" | "export_destination";

export type PathSelection = {
  token: string;
  purpose: SelectionPurpose;
  displayName: string;
  expiresAt: number;
};

export type ClassifierProvenance = {
  version: string;
  digest: string;
};

export type ArchiveManifest = {
  formatVersion: number;
  createdAt: number;
  schemaMin: number;
  schemaMax: number;
  recordCount: number;
  tableCounts: Record<string, number>;
  tableHashes: Record<string, string>;
  classifierProvenance: ClassifierProvenance[];
};

export type RestoreMode = "merge" | "replace";

export type RestoreDiff = {
  importedRecords: number;
  exactDuplicates: number;
  conflicts: number;
  tombstoneSkips: number;
  profiles: number;
  encounters: number;
  observations: number;
};

export type RestorePreview = {
  operation: OperationRecord;
  token: string;
  expiresAt: number;
  archiveSha256: string;
  manifest: ArchiveManifest;
  diff: RestoreDiff;
  allowedModes: RestoreMode[];
};

export type RollbackView = {
  id: string;
  restoreOperationId: string;
  mode: RestoreMode;
  createdAt: number;
  expiresAt: number;
};

export type RollbackConfirmation = {
  rollback: RollbackView;
  confirmationToken: string;
  expiresAt: number;
};

export type BackupResult = {
  operation: OperationRecord;
  destinationName: string;
  manifest: ArchiveManifest;
};

export type RestoreResult = {
  operation: OperationRecord;
  mode: RestoreMode;
  importedRecords: number;
  exactDuplicates: number;
  conflicts: number;
  tombstoneSkips: number;
  rollback: RollbackView;
};

export type ExportResult = {
  operation: OperationRecord;
  destinationName: string;
  opponentCount: number;
  encounterCount: number;
  observationCount: number;
};

export type ExportScope =
  "complete_notebook" | { selected_opponent: { profileId: string } };

export function selectPortabilityPath(purpose: SelectionPurpose) {
  return invoke<CommandResult<PathSelection | null>>(
    "select_portability_path",
    {
      request: { purpose },
    },
  );
}

export function startBackup(
  request: {
    selectionToken: string;
    passphrase: string;
    passphraseAcknowledged: boolean;
    confirmEmpty?: boolean;
    overwrite?: boolean;
  },
  onStarted?: (operationId: string) => void,
) {
  const operationId = createIdempotencyKey();
  onStarted?.(operationId);
  return invoke<CommandResult<BackupResult>>("start_backup", {
    request: {
      ...request,
      operationId,
      confirmEmpty: request.confirmEmpty ?? false,
      overwrite: request.overwrite ?? false,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function previewRestore(
  selectionToken: string,
  passphrase: string,
  onStarted?: (operationId: string) => void,
) {
  const operationId = createIdempotencyKey();
  onStarted?.(operationId);
  return invoke<CommandResult<RestorePreview>>("preview_restore", {
    request: {
      operationId,
      idempotencyKey: createIdempotencyKey(),
      selectionToken,
      passphrase,
    },
  });
}

export function applyRestore(previewToken: string, mode: RestoreMode) {
  return invoke<CommandResult<RestoreResult>>("apply_restore", {
    request: {
      previewToken,
      mode,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function startExport(
  request: {
    selectionToken: string;
    scope: ExportScope;
    plaintextAcknowledged: boolean;
    confirmEmpty?: boolean;
    unsavedEditsResolved: boolean;
    overwrite?: boolean;
  },
  onStarted?: (operationId: string) => void,
) {
  const operationId = createIdempotencyKey();
  onStarted?.(operationId);
  return invoke<CommandResult<ExportResult>>("start_export", {
    request: {
      ...request,
      operationId,
      confirmEmpty: request.confirmEmpty ?? false,
      overwrite: request.overwrite ?? false,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function listPortabilityRollbacks() {
  return invoke<CommandResult<RollbackView[]>>("list_portability_rollbacks");
}

export function confirmPortabilityRollback(rollbackId: string) {
  return invoke<CommandResult<RollbackConfirmation>>(
    "confirm_portability_rollback",
    { request: { rollbackId } },
  );
}

export function applyPortabilityRollback(
  rollbackId: string,
  confirmationToken: string,
) {
  return invoke<CommandResult<RollbackView>>("apply_portability_rollback", {
    request: {
      rollbackId,
      confirmationToken,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}

export function discardPortabilityRollback(
  rollbackId: string,
  confirmationToken: string,
) {
  return invoke<CommandResult<RollbackView>>("discard_portability_rollback", {
    request: {
      rollbackId,
      confirmationToken,
      idempotencyKey: createIdempotencyKey(),
    },
  });
}
