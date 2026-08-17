import { useEffect, useState, type FormEvent } from "react";

import {
  cancelDiagnostics,
  createDiagnostics,
  previewDiagnostics,
  selectDiagnosticsPath,
  type DiagnosticsPathSelection,
  type DiagnosticsPreview,
} from "../../lib/ipc/diagnostics";
import { createIdempotencyKey } from "../../lib/ipc/notebook";
import {
  checkClassifierUpdate,
  checkUpdate,
  installClassifierUpdate,
  installUpdate,
  type ClassifierUpdateView,
  type SignedRelease,
} from "../../lib/ipc/updates";
import { getSettings, updateSettings } from "../../lib/ipc/client";
import type { Settings } from "../../lib/ipc/contracts";
import { Button, Checkbox, Panel, StatusLabel } from "../../ui/primitives";

const PRIVATE_DEFAULTS: Settings = {
  schemaVersion: 1,
  providerAccessEnabled: false,
  overlayEnabled: true,
  trayEnabled: true,
  launchWithWindows: false,
  updateChecksEnabled: false,
  classifierUpdateChecksEnabled: false,
  diagnosticsEnabled: false,
};

export function OperationalSettings() {
  const [settings, setSettings] = useState(PRIVATE_DEFAULTS);
  const [revision, setRevision] = useState(1);
  const [status, setStatus] = useState("Privacy defaults are active.");
  const [error, setError] = useState<string>();
  const [preview, setPreview] = useState<DiagnosticsPreview>();
  const [diagnosticsPath, setDiagnosticsPath] =
    useState<DiagnosticsPathSelection>();
  const [release, setRelease] = useState<SignedRelease>();
  const [classifier, setClassifier] = useState<ClassifierUpdateView>();
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void getSettings().then((result) => {
      if (result.ok) {
        setSettings(result.data);
        setRevision(result.revision);
      } else {
        setError(result.error.message);
      }
    });
  }, []);

  async function save(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(undefined);
    const result = await updateSettings({
      idempotencyKey: createIdempotencyKey(),
      expectedRevision: revision,
      settings,
    });
    setBusy(false);
    if (!result.ok) {
      setError(result.error.message);
      return;
    }
    setSettings(result.data);
    setRevision(result.revision);
    setStatus("Your independent privacy and startup choices were saved.");
  }

  async function buildPreview() {
    setBusy(true);
    setError(undefined);
    const result = await previewDiagnostics();
    setBusy(false);
    if (!result.ok) {
      setError(result.error.message);
      return;
    }
    setPreview(result.data);
    setStatus("Diagnostics were redacted and previewed locally.");
  }

  async function saveBundle() {
    if (!preview || !diagnosticsPath) return;
    setBusy(true);
    const result = await createDiagnostics(
      preview.previewToken,
      diagnosticsPath.selectionToken,
    );
    setBusy(false);
    if (!result.ok) {
      setError(result.error.message);
      return;
    }
    setPreview(undefined);
    setDiagnosticsPath(undefined);
    setStatus(
      `Saved ${result.data.fileName} locally. Nothing was uploaded or shared.`,
    );
  }

  async function chooseBundleLocation() {
    setBusy(true);
    setError(undefined);
    const result = await selectDiagnosticsPath();
    setBusy(false);
    if (!result.ok) {
      setError(result.error.message);
      return;
    }
    if (result.data) {
      setDiagnosticsPath(result.data);
      setStatus(`Diagnostics destination selected: ${result.data.fileName}.`);
    }
  }

  async function cancelPreview() {
    if (!preview) return;
    await cancelDiagnostics(preview.previewToken);
    setPreview(undefined);
    setStatus("Diagnostic bundle creation was cancelled.");
  }

  async function findApplicationUpdate() {
    setBusy(true);
    const result = await checkUpdate();
    setBusy(false);
    if (!result.ok) {
      setError(result.error.message);
      return;
    }
    setRelease(result.data);
    setStatus("A signed application update is ready for review.");
  }

  async function confirmApplicationUpdate() {
    if (!release) return;
    setBusy(true);
    const result = await installUpdate(release.version, true);
    setBusy(false);
    if (!result.ok) {
      setError(result.error.message);
      return;
    }
    setRelease(undefined);
    setStatus(`Application update stage: ${result.data.stage}.`);
  }

  async function findClassifierUpdate() {
    setBusy(true);
    const result = await checkClassifierUpdate();
    setBusy(false);
    if (!result.ok) {
      setError(result.error.message);
      return;
    }
    setClassifier(result.data);
    setStatus("Signed classifier assets are ready for review.");
  }

  async function confirmClassifierUpdate() {
    setBusy(true);
    const result = await installClassifierUpdate(true);
    setBusy(false);
    if (!result.ok) {
      setError(result.error.message);
      return;
    }
    setClassifier(undefined);
    setStatus(
      `Classifier reclassification queued: ${result.data.completed}/${result.data.total}.`,
    );
  }

  const offline =
    typeof navigator !== "undefined" && navigator.onLine === false;

  return (
    <section
      className="operational-settings"
      aria-labelledby="operations-title"
    >
      <div className="section-heading">
        <div>
          <h2 id="operations-title">Privacy, offline use, and updates</h2>
          <p className="muted-copy">
            The notebook is local and account-free. Each external or startup
            behavior is controlled separately; diagnostics are never uploaded.
          </p>
        </div>
        <StatusLabel
          kind={error ? "error" : offline ? "incomplete" : "source"}
          label={error ?? (offline ? "Offline · local features ready" : status)}
        />
      </div>

      <div className="operational-grid">
        <Panel label="Independent choices">
          <form className="ui-stack" onSubmit={save}>
            <p className="muted-copy">
              Provider access sends only disclosed confirmed fields. Application
              and classifier checks send only release metadata after opt-in.
            </p>
            <Choice
              checked={settings.providerAccessEnabled}
              label="Allow disclosed provider access"
              onChange={(checked) =>
                setSettings({ ...settings, providerAccessEnabled: checked })
              }
            />
            <Choice
              checked={settings.overlayEnabled}
              disabled={busy}
              label="Show the opponent overlay"
              onChange={(checked) =>
                setSettings({ ...settings, overlayEnabled: checked })
              }
            />
            <Choice
              checked={settings.updateChecksEnabled}
              label="Check for signed application updates"
              onChange={(checked) =>
                setSettings({ ...settings, updateChecksEnabled: checked })
              }
            />
            <Choice
              checked={settings.classifierUpdateChecksEnabled}
              label="Check for signed classifier asset updates"
              onChange={(checked) =>
                setSettings({
                  ...settings,
                  classifierUpdateChecksEnabled: checked,
                })
              }
            />
            <Choice
              checked={settings.launchWithWindows}
              label="Launch with Windows"
              onChange={(checked) =>
                setSettings({ ...settings, launchWithWindows: checked })
              }
            />
            <Choice
              checked={settings.trayEnabled}
              label="Keep the companion available in the tray"
              onChange={(checked) =>
                setSettings({ ...settings, trayEnabled: checked })
              }
            />
            <Choice
              checked={settings.diagnosticsEnabled}
              label="Allow local private diagnostic bundle creation"
              onChange={(checked) =>
                setSettings({ ...settings, diagnosticsEnabled: checked })
              }
            />
            <Button busy={busy} type="submit">
              Save choices
            </Button>
          </form>
        </Panel>

        <Panel label="Signed updates">
          <div className="ui-stack">
            <p className="muted-copy">
              Checks are manual here. Downloads and passive installation require
              a second explicit confirmation.
            </p>
            <div className="ui-actions">
              <Button
                disabled={!settings.updateChecksEnabled}
                onClick={findApplicationUpdate}
                variant="secondary"
              >
                Check application update
              </Button>
              <Button
                disabled={!settings.classifierUpdateChecksEnabled}
                onClick={findClassifierUpdate}
                variant="secondary"
              >
                Check classifier update
              </Button>
            </div>
            {release ? (
              <div className="update-confirmation" role="status">
                <h3>Application {release.version}</h3>
                <p>{release.releaseNotes}</p>
                <p>{release.classifierChangeSummary}</p>
                <Button onClick={confirmApplicationUpdate}>
                  Confirm download and install
                </Button>
              </div>
            ) : null}
            {classifier ? (
              <div className="update-confirmation" role="status">
                <h3>Classifier {classifier.classifierVersion}</h3>
                <p>Formats: {classifier.formats.join(", ")}</p>
                <p className="muted-copy">
                  Assets remain app-owned and read-only. Prior classification
                  runs remain available.
                </p>
                <Button onClick={confirmClassifierUpdate}>
                  Confirm activation and reclassification
                </Button>
              </div>
            ) : null}
          </div>
        </Panel>

        <Panel label="Private diagnostics">
          <div className="ui-stack">
            <p className="muted-copy">
              Preview lists every local artifact, allowed field class, omission,
              and redaction count. Handles, notes, cards, OCR text, URLs, paths,
              and secrets are excluded.
            </p>
            <Button
              disabled={!settings.diagnosticsEnabled}
              onClick={buildPreview}
              variant="secondary"
            >
              Preview redacted diagnostics
            </Button>
            {preview ? (
              <div className="diagnostics-preview">
                <h3>Local preview</h3>
                <ul>
                  {preview.artifacts.map((artifact) => (
                    <li key={artifact.fileName}>
                      <strong>{artifact.fileName}</strong>
                      <span>
                        {artifact.omitted
                          ? `Omitted: ${artifact.omissionCode}`
                          : `${artifact.eventCount} events · ${artifact.redactionCount} redactions`}
                      </span>
                    </li>
                  ))}
                </ul>
                <Button onClick={chooseBundleLocation} variant="secondary">
                  Choose bundle location
                </Button>
                <p className="muted-copy" role="status">
                  {diagnosticsPath?.fileName ?? "No destination selected."}
                </p>
                <div className="ui-actions">
                  <Button disabled={!diagnosticsPath} onClick={saveBundle}>
                    Create local bundle
                  </Button>
                  <Button onClick={cancelPreview} variant="secondary">
                    Cancel
                  </Button>
                </div>
              </div>
            ) : null}
          </div>
        </Panel>
      </div>
    </section>
  );
}

function Choice({
  checked,
  disabled = false,
  label,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  label: string;
  onChange: (checked: boolean) => void;
}) {
  return (
    <Checkbox
      checked={checked}
      className="settings-choice"
      disabled={disabled}
      onChange={onChange}
    >
      {label}
    </Checkbox>
  );
}
