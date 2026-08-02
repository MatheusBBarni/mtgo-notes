import { invoke } from "@tauri-apps/api/core";

import type { CommandResult } from "./contracts";

export type OperationKind =
  | "backup_snapshot"
  | "export_snapshot"
  | "migration"
  | "restore_merge"
  | "restore_replace"
  | "purge"
  | "rollback_apply";

export type OperationState =
  | "requested"
  | "running"
  | "awaiting_confirmation"
  | "committing"
  | "completed"
  | "failed"
  | "cancelled"
  | "recoverable";

export type OperationRecord = {
  id: string;
  kind: OperationKind;
  idempotencyKey: string;
  state: OperationState;
  requestedAt: number;
  completedAt?: number;
  completed: number;
  total: number;
  rollbackLocation?: string;
  revision: number;
};

export type OperationProgressEvent = {
  name: "operation://progress-v1";
  version: { major: 1 };
  revision: number;
  payload: OperationRecord;
};

export function cancelOperation(operationId: string) {
  return invoke<CommandResult<OperationRecord>>("cancel_operation", {
    request: { operationId },
  });
}

export function getOperation(operationId: string) {
  return invoke<CommandResult<OperationRecord>>("get_operation", {
    request: { operationId },
  });
}

export function progressPercent(operation: OperationRecord): number {
  if (operation.total <= 0) return 0;
  return Math.min(
    100,
    Math.max(0, Math.round((operation.completed / operation.total) * 100)),
  );
}
