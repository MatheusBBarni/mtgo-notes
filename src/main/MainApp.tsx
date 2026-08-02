import { NotebookWorkspace } from "../features/notebook/NotebookWorkspace";
import { DeckEnrichmentPanel } from "../features/decks/DeckEnrichmentPanel";
import { PortabilityWorkspace } from "../features/portability/PortabilityWorkspace";
import { OperationalSettings } from "../features/settings/OperationalSettings";
import { DetectionOnboarding } from "../features/onboarding/DetectionOnboarding";
import { LiveEncounterControls } from "../features/encounter/LiveEncounterControls";
import { StatusLabel } from "../ui/primitives";

export function MainApp() {
  return (
    <main className="app-shell">
      <header className="app-header">
        <div>
          <h1 className="app-title">MTGO Opponent Notes</h1>
          <p className="app-subtitle">
            Your private, local notebook is ready. Automatic context remains off
            until you review and grant provider consent.
          </p>
        </div>
        <StatusLabel kind="phase" label="Ready" />
      </header>
      <OperationalSettings />
      <DetectionOnboarding />
      <LiveEncounterControls />
      <DeckEnrichmentPanel />
      <NotebookWorkspace />
      <PortabilityWorkspace />
    </main>
  );
}
