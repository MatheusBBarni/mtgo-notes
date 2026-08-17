import { useState, type FormEvent } from "react";

import {
  selectPortabilityPath,
  startBackup,
  type BackupResult,
  type PathSelection,
} from "../../lib/ipc/portability";
import { Button, Checkbox, Panel, TextField } from "../../ui/primitives";
import { OperationProgress } from "../operations/OperationProgress";

type BackupPanelProps = {
  onError: (message?: string) => void;
  onStatus: (message: string) => void;
};

export function BackupPanel({ onError, onStatus }: BackupPanelProps) {
  const [selection, setSelection] = useState<PathSelection>();
  const [passphrase, setPassphrase] = useState("");
  const [acknowledged, setAcknowledged] = useState(false);
  const [confirmEmpty, setConfirmEmpty] = useState(false);
  const [overwrite, setOverwrite] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<BackupResult>();
  const [activeOperation, setActiveOperation] =
    useState<BackupResult["operation"]>();
  const displayedOperation = activeOperation ?? result?.operation;

  async function choose() {
    onError(undefined);
    const selected = await selectPortabilityPath("backup_destination");
    if (!selected.ok) {
      onError(selected.error.message);
    } else if (selected.data) {
      setSelection(selected.data);
      onStatus(`Backup destination selected: ${selected.data.displayName}`);
    }
  }

  async function create(event: FormEvent) {
    event.preventDefault();
    if (!selection) return;
    setBusy(true);
    onError(undefined);
    const pendingBackup = startBackup(
      {
        selectionToken: selection.token,
        passphrase,
        passphraseAcknowledged: acknowledged,
        confirmEmpty,
        overwrite,
      },
      (operationId) =>
        setActiveOperation({
          id: operationId,
          kind: "backup_snapshot",
          idempotencyKey: operationId,
          state: "requested",
          requestedAt: Date.now(),
          completed: 0,
          total: 0,
          revision: 1,
        }),
    );
    setPassphrase("");
    const backup = await pendingBackup;
    setBusy(false);
    if (!backup.ok) {
      setActiveOperation(undefined);
      onError(backup.error.message);
      return;
    }
    setResult(backup.data);
    setActiveOperation(undefined);
    setSelection(undefined);
    onStatus(
      `Encrypted backup ${backup.data.destinationName} completed with ${backup.data.manifest.recordCount} logical records.`,
    );
  }

  return (
    <Panel label="Encrypted notebook backup">
      <form className="ui-stack" onSubmit={create}>
        <h3>Encrypted backup</h3>
        <p className="notebook-hint">
          The Rust host creates an authenticated portable archive. No notebook
          plaintext, database key, or Windows-bound secret is written to it.
        </p>
        <Button onClick={choose} type="button" variant="secondary">
          Choose backup destination
        </Button>
        <p aria-live="polite" className="notebook-hint">
          {selection?.displayName ?? "No destination selected."}
        </p>
        <TextField
          autoComplete="new-password"
          label="Backup passphrase"
          minLength={8}
          onChange={(event) => setPassphrase(event.currentTarget.value)}
          required
          type="password"
          value={passphrase}
        />
        <Checkbox
          checked={acknowledged}
          className="portability-check"
          onChange={setAcknowledged}
        >
          I understand this passphrase cannot be recovered if I forget it.
        </Checkbox>
        <Checkbox
          checked={confirmEmpty}
          className="portability-check"
          onChange={setConfirmEmpty}
        >
          Create the archive even if the notebook has no profiles.
        </Checkbox>
        <Checkbox
          checked={overwrite}
          className="portability-check"
          onChange={setOverwrite}
        >
          Replace an existing backup at this destination.
        </Checkbox>
        <Button
          busy={busy}
          disabled={!selection || !acknowledged}
          type="submit"
        >
          Create encrypted backup
        </Button>
      </form>
      {displayedOperation ? (
        <OperationProgress
          key={displayedOperation.id}
          operation={displayedOperation}
          onError={onError}
        />
      ) : null}
    </Panel>
  );
}
