import { useState } from "react";

import { StatusLabel } from "../../ui/primitives";
import { BackupPanel } from "../backup/BackupPanel";
import { ExportPanel } from "../export/ExportPanel";
import { RestorePanel } from "../restore/RestorePanel";

export function PortabilityWorkspace() {
  const [status, setStatus] = useState("Portability ready");
  const [error, setError] = useState<string>();

  return (
    <section
      aria-labelledby="portability-title"
      className="portability-workspace"
    >
      <div className="section-heading">
        <div>
          <h2 id="portability-title">Backup, restore, and export</h2>
          <p className="notebook-hint">
            File access, passphrases, encryption, staging, and rollback remain
            inside the trusted Rust host.
          </p>
        </div>
        <div aria-atomic="true" aria-live="polite">
          <StatusLabel
            kind={error ? "error" : "source"}
            label={error ?? status}
          />
        </div>
      </div>
      <div className="portability-grid">
        <BackupPanel onError={setError} onStatus={setStatus} />
        <RestorePanel onError={setError} onStatus={setStatus} />
        <ExportPanel onError={setError} onStatus={setStatus} />
      </div>
    </section>
  );
}
