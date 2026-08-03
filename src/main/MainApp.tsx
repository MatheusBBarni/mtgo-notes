import { Tabs } from "@heroui/react";
import { useState } from "react";

import { NotebookWorkspace } from "../features/notebook/NotebookWorkspace";
import { DeckEnrichmentPanel } from "../features/decks/DeckEnrichmentPanel";
import { PortabilityWorkspace } from "../features/portability/PortabilityWorkspace";
import { OperationalSettings } from "../features/settings/OperationalSettings";
import { DetectionOnboarding } from "../features/onboarding/DetectionOnboarding";
import { LiveEncounterControls } from "../features/encounter/LiveEncounterControls";
import { PlayerWorkspace } from "../features/player/PlayerWorkspace";
import { StatusLabel } from "../ui/primitives";

type WorkspaceView =
  | "settings"
  | "detection"
  | "encounter"
  | "decks"
  | "notebook"
  | "portability"
  | "player";

const WORKSPACE_VIEWS: readonly {
  label: string;
  value: WorkspaceView;
}[] = [
  { label: "Settings", value: "settings" },
  { label: "Detection", value: "detection" },
  { label: "Live match", value: "encounter" },
  { label: "Deck context", value: "decks" },
  { label: "Notebook", value: "notebook" },
  { label: "Backup & export", value: "portability" },
  { label: "Player", value: "player" },
];

export function MainApp() {
  const [view, setView] = useState<WorkspaceView>("settings");

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
      <Tabs
        className="main-workspace"
        onSelectionChange={(key) => setView(key as WorkspaceView)}
        selectedKey={view}
        variant="secondary"
      >
        <Tabs.ListContainer className="main-workspace-nav-container">
          <Tabs.List
            aria-label="Workspace sections"
            className="main-workspace-nav"
          >
            {WORKSPACE_VIEWS.map((item) => (
              <Tabs.Tab id={item.value} key={item.value}>
                {item.label}
                <Tabs.Indicator />
              </Tabs.Tab>
            ))}
          </Tabs.List>
        </Tabs.ListContainer>
        <Tabs.Panel
          className="main-workspace-panel"
          id="settings"
          shouldForceMount
        >
          <OperationalSettings />
        </Tabs.Panel>
        <Tabs.Panel
          className="main-workspace-panel"
          id="detection"
          shouldForceMount
        >
          <DetectionOnboarding />
        </Tabs.Panel>
        <Tabs.Panel
          className="main-workspace-panel"
          id="encounter"
          shouldForceMount
        >
          <LiveEncounterControls />
        </Tabs.Panel>
        <Tabs.Panel
          className="main-workspace-panel"
          id="decks"
          shouldForceMount
        >
          <DeckEnrichmentPanel />
        </Tabs.Panel>
        <Tabs.Panel
          className="main-workspace-panel"
          id="notebook"
          shouldForceMount
        >
          <NotebookWorkspace />
        </Tabs.Panel>
        <Tabs.Panel
          className="main-workspace-panel"
          id="portability"
          shouldForceMount
        >
          <PortabilityWorkspace />
        </Tabs.Panel>
        <Tabs.Panel
          className="main-workspace-panel"
          id="player"
          shouldForceMount
        >
          <PlayerWorkspace />
        </Tabs.Panel>
      </Tabs>
    </main>
  );
}
