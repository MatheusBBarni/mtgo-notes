import { useEffect, useState, type FormEvent } from "react";

import {
  applyPortabilityRollback,
  applyRestore,
  confirmPortabilityRollback,
  discardPortabilityRollback,
  listPortabilityRollbacks,
  previewRestore,
  selectPortabilityPath,
  type PathSelection,
  type RestoreMode,
  type RestorePreview,
  type RestoreResult,
  type RollbackView,
} from "../../lib/ipc/portability";
import { cancelOperation } from "../../lib/ipc/operations";
import {
  Button,
  Panel,
  RadioGroupField,
  StatusLabel,
  TextField,
} from "../../ui/primitives";
import { OperationProgress } from "../operations/OperationProgress";

type RestorePanelProps = {
  onError: (message?: string) => void;
  onStatus: (message: string) => void;
};

export function RestorePanel({ onError, onStatus }: RestorePanelProps) {
  const [selection, setSelection] = useState<PathSelection>();
  const [passphrase, setPassphrase] = useState("");
  const [preview, setPreview] = useState<RestorePreview>();
  const [mode, setMode] = useState<RestoreMode>("merge");
  const [result, setResult] = useState<RestoreResult>();
  const [activeOperation, setActiveOperation] =
    useState<RestorePreview["operation"]>();
  const [rollbacks, setRollbacks] = useState<RollbackView[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void refreshRollbacks();
  }, []);

  async function refreshRollbacks() {
    const available = await listPortabilityRollbacks();
    if (available.ok) setRollbacks(available.data);
  }

  async function choose() {
    const selected = await selectPortabilityPath("restore_source");
    if (!selected.ok) onError(selected.error.message);
    else if (selected.data) {
      setSelection(selected.data);
      setPreview(undefined);
    }
  }

  async function buildPreview(event: FormEvent) {
    event.preventDefault();
    if (!selection) return;
    setBusy(true);
    onError(undefined);
    const pendingPreview = previewRestore(
      selection.token,
      passphrase,
      (operationId) =>
        setActiveOperation({
          id: operationId,
          kind: "restore_merge",
          idempotencyKey: operationId,
          state: "requested",
          requestedAt: Date.now(),
          completed: 0,
          total: 0,
          revision: 1,
        }),
    );
    setPassphrase("");
    const staged = await pendingPreview;
    setBusy(false);
    if (!staged.ok) {
      setActiveOperation(undefined);
      onError(staged.error.message);
      return;
    }
    setPreview(staged.data);
    setActiveOperation(undefined);
    onStatus(
      "Restore authenticated and staged; the live notebook is unchanged.",
    );
  }

  async function apply() {
    if (!preview) return;
    setBusy(true);
    const applied = await applyRestore(preview.token, mode);
    setBusy(false);
    if (!applied.ok) {
      onError(applied.error.message);
      setPreview(undefined);
      return;
    }
    setResult(applied.data);
    setPreview(undefined);
    setSelection(undefined);
    await refreshRollbacks();
    onStatus(
      `${mode === "merge" ? "Merge" : "Replace"} restore completed with encrypted rollback available.`,
    );
  }

  async function cancelPreview() {
    if (!preview) return;
    const cancelled = await cancelOperation(preview.operation.id);
    if (!cancelled.ok) {
      onError(cancelled.error.message);
      return;
    }
    setPreview(undefined);
    onStatus("Staged restore cancelled; the live notebook is unchanged.");
  }

  async function actOnRollback(
    rollback: RollbackView,
    action: "apply" | "discard",
  ) {
    const confirmed = await confirmPortabilityRollback(rollback.id);
    if (!confirmed.ok) {
      onError(confirmed.error.message);
      return;
    }
    const completed =
      action === "apply"
        ? await applyPortabilityRollback(
            rollback.id,
            confirmed.data.confirmationToken,
          )
        : await discardPortabilityRollback(
            rollback.id,
            confirmed.data.confirmationToken,
          );
    if (!completed.ok) onError(completed.error.message);
    else {
      await refreshRollbacks();
      onStatus(
        action === "apply"
          ? "Encrypted rollback applied atomically."
          : "Encrypted rollback discarded.",
      );
    }
  }

  return (
    <Panel label="Staged restore and rollback">
      <form className="ui-stack" onSubmit={buildPreview}>
        <h3>Restore encrypted backup</h3>
        <p className="notebook-hint">
          Authentication, schema checks, and SQLCipher staging finish before
          merge or replace becomes available.
        </p>
        <Button onClick={choose} type="button" variant="secondary">
          Choose encrypted backup
        </Button>
        <p aria-live="polite" className="notebook-hint">
          {selection?.displayName ?? "No backup selected."}
        </p>
        <TextField
          autoComplete="current-password"
          label="Backup passphrase"
          onChange={(event) => setPassphrase(event.currentTarget.value)}
          required
          type="password"
          value={passphrase}
        />
        <Button busy={busy} disabled={!selection} type="submit">
          Authenticate and preview
        </Button>
      </form>

      {preview ? (
        <div className="restore-preview">
          <StatusLabel kind="source" label="Authenticated staging preview" />
          <dl className="count-grid">
            <div>
              <dt>Profiles</dt>
              <dd>{preview.diff.profiles}</dd>
            </div>
            <div>
              <dt>Encounters</dt>
              <dd>{preview.diff.encounters}</dd>
            </div>
            <div>
              <dt>Observations</dt>
              <dd>{preview.diff.observations}</dd>
            </div>
            <div>
              <dt>Conflicts retained</dt>
              <dd>{preview.diff.conflicts}</dd>
            </div>
            <div>
              <dt>Deleted records blocked</dt>
              <dd>{preview.diff.tombstoneSkips}</dd>
            </div>
          </dl>
          <RadioGroupField
            className="portability-options"
            label="Restore behavior"
            name="restore-mode"
            onChange={setMode}
            options={[
              {
                label: "Merge without resurrecting deleted records",
                value: "merge",
              },
              {
                label: "Replace notebook atomically",
                value: "replace",
              },
            ]}
            value={mode}
          />
          <div className="ui-actions">
            <Button busy={busy} onClick={apply} variant="destructive">
              Confirm {mode} restore
            </Button>
            <Button onClick={cancelPreview} variant="secondary">
              Cancel staged restore
            </Button>
          </div>
        </div>
      ) : null}

      {activeOperation ? (
        <OperationProgress
          key={activeOperation.id}
          operation={activeOperation}
          onError={onError}
        />
      ) : null}

      {result ? (
        <OperationProgress
          key={result.operation.id}
          operation={result.operation}
          onError={onError}
        />
      ) : null}

      <div className="rollback-list">
        <h4>Encrypted restore rollbacks</h4>
        {rollbacks.length === 0 ? (
          <p className="notebook-empty">No retained restore rollback.</p>
        ) : (
          <ul>
            {rollbacks.map((rollback) => (
              <li key={rollback.id}>
                <span>
                  {rollback.mode} restore · expires{" "}
                  {new Date(rollback.expiresAt).toLocaleString()}
                </span>
                <div className="ui-actions">
                  <Button
                    onClick={() => actOnRollback(rollback, "apply")}
                    variant="destructive"
                  >
                    Apply rollback
                  </Button>
                  <Button
                    onClick={() => actOnRollback(rollback, "discard")}
                    variant="secondary"
                  >
                    Discard rollback
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Panel>
  );
}
