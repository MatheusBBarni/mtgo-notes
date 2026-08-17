import { useState, type FormEvent } from "react";

import {
  selectPortabilityPath,
  startExport,
  type ExportResult,
  type PathSelection,
} from "../../lib/ipc/portability";
import {
  Button,
  Checkbox,
  Panel,
  RadioGroupField,
  TextField,
} from "../../ui/primitives";
import { OperationProgress } from "../operations/OperationProgress";

type ExportPanelProps = {
  onError: (message?: string) => void;
  onStatus: (message: string) => void;
};

export function ExportPanel({ onError, onStatus }: ExportPanelProps) {
  const [selection, setSelection] = useState<PathSelection>();
  const [scope, setScope] = useState<"complete" | "selected">("complete");
  const [profileId, setProfileId] = useState("");
  const [acknowledged, setAcknowledged] = useState(false);
  const [confirmEmpty, setConfirmEmpty] = useState(false);
  const [overwrite, setOverwrite] = useState(false);
  const [unsavedEditsResolved, setUnsavedEditsResolved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<ExportResult>();
  const [activeOperation, setActiveOperation] =
    useState<ExportResult["operation"]>();
  const displayedOperation = activeOperation ?? result?.operation;

  async function choose() {
    const selected = await selectPortabilityPath("export_destination");
    if (!selected.ok) onError(selected.error.message);
    else if (selected.data) setSelection(selected.data);
  }

  async function create(event: FormEvent) {
    event.preventDefault();
    if (!selection) return;
    setBusy(true);
    onError(undefined);
    const exported = await startExport(
      {
        selectionToken: selection.token,
        scope:
          scope === "complete"
            ? "complete_notebook"
            : { selected_opponent: { profileId } },
        plaintextAcknowledged: acknowledged,
        confirmEmpty,
        unsavedEditsResolved,
        overwrite,
      },
      (operationId) =>
        setActiveOperation({
          id: operationId,
          kind: "export_snapshot",
          idempotencyKey: operationId,
          state: "requested",
          requestedAt: Date.now(),
          completed: 0,
          total: 0,
          revision: 1,
        }),
    );
    setBusy(false);
    if (!exported.ok) {
      setActiveOperation(undefined);
      onError(exported.error.message);
      return;
    }
    setResult(exported.data);
    setActiveOperation(undefined);
    setSelection(undefined);
    onStatus(
      `Plaintext export ${exported.data.destinationName} completed: ${exported.data.opponentCount} opponents, ${exported.data.encounterCount} encounters.`,
    );
  }

  return (
    <Panel label="One-way text export">
      <form className="ui-stack" onSubmit={create}>
        <h3>Readable text export</h3>
        <p className="portability-warning" role="note">
          Privacy warning: the resulting UTF-8 .txt file is unencrypted and
          cannot be imported or used to restore the notebook.
        </p>
        <RadioGroupField
          className="portability-options"
          label="Export scope"
          name="export-scope"
          onChange={setScope}
          options={[
            { label: "Complete notebook", value: "complete" },
            { label: "One selected opponent", value: "selected" },
          ]}
          value={scope}
        />
        {scope === "selected" ? (
          <TextField
            label="Selected opponent profile ID"
            onChange={(event) => setProfileId(event.currentTarget.value)}
            required
            value={profileId}
          />
        ) : null}
        <Button onClick={choose} type="button" variant="secondary">
          Choose text destination
        </Button>
        <p aria-live="polite" className="notebook-hint">
          {selection?.displayName ?? "No destination selected."}
        </p>
        <Checkbox
          checked={acknowledged}
          className="portability-check"
          onChange={setAcknowledged}
        >
          I understand this file is readable outside the application.
        </Checkbox>
        <Checkbox
          checked={unsavedEditsResolved}
          className="portability-check"
          onChange={setUnsavedEditsResolved}
        >
          I saved or discarded any unsaved notebook edits.
        </Checkbox>
        <Checkbox
          checked={confirmEmpty}
          className="portability-check"
          onChange={setConfirmEmpty}
        >
          Create the file even if the selected scope is empty.
        </Checkbox>
        <Checkbox
          checked={overwrite}
          className="portability-check"
          onChange={setOverwrite}
        >
          Replace an existing text export at this destination.
        </Checkbox>
        <Button
          busy={busy}
          disabled={!selection || !acknowledged || !unsavedEditsResolved}
          type="submit"
        >
          Export unencrypted text
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
