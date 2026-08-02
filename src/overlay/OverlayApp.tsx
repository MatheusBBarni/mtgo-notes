import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";

import { openCapture, setOverlayInteraction } from "../lib/ipc/capture";
import {
  confirmOpponent,
  type EncounterStateView,
  type OpponentCandidate,
} from "../lib/ipc/encounter";
import {
  acceptNewerReplacementEvent,
  type ReplacementEvent,
} from "../lib/ipc/events";
import { Button, Panel, StatusLabel } from "../ui/primitives";
import {
  applyOverlayReplacement,
  NEUTRAL_OVERLAY_VIEW,
  type OverlayView,
} from "./projection";

export function OverlayApp() {
  const [view, setView] = useState<OverlayView>(NEUTRAL_OVERLAY_VIEW);
  const [candidate, setCandidate] = useState<OpponentCandidate>();
  const [expanded, setExpanded] = useState(false);
  const [error, setError] = useState<string>();
  const overlayRevision = useRef(0);
  const encounterRevision = useRef(0);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let unlistenEncounter: (() => void) | undefined;
    void listen<ReplacementEvent<OverlayView>>(
      "overlay://view-v1",
      (incoming) => {
        const accepted = acceptNewerReplacementEvent<OverlayView>(
          incoming.payload,
          overlayRevision.current,
          {
            clearSensitiveView: () => setView(NEUTRAL_OVERLAY_VIEW),
            requestBootstrap: () =>
              setError("Overlay state changed. Reopen it."),
          },
        );
        if (accepted) {
          overlayRevision.current = accepted.revision;
          setView((current) => applyOverlayReplacement(current, accepted));
        }
      },
    ).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    void listen<ReplacementEvent<EncounterStateView>>(
      "encounter://state-v1",
      (incoming) => {
        const accepted = acceptNewerReplacementEvent<EncounterStateView>(
          incoming.payload,
          encounterRevision.current,
          {
            clearSensitiveView: () => setCandidate(undefined),
            requestBootstrap: () =>
              setError("Live context changed. Reopen the overlay."),
          },
        );
        if (accepted) {
          encounterRevision.current = accepted.revision;
          setCandidate(accepted.payload.candidate);
        }
      },
    ).then((stop) => {
      if (disposed) stop();
      else unlistenEncounter = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
      unlistenEncounter?.();
    };
  }, []);

  async function changeInteraction(next: boolean) {
    const result = await setOverlayInteraction(next);
    if (result.ok) {
      setExpanded(next);
      setError(undefined);
    } else {
      setError(result.error.message);
    }
  }

  return (
    <main
      aria-label="Opponent overlay"
      className="app-shell app-shell--compact"
      data-expanded={expanded}
    >
      <Panel label="Opponent overlay">
        <div className="ui-stack">
          <StatusLabel
            kind="phase"
            label={`${view.phase.replaceAll("_", " ")}${
              view.confirmedHandle ? ` — ${view.confirmedHandle}` : ""
            }`}
          />
          {error ? <StatusLabel kind="error" label={error} /> : null}
          {view.needsIdentityResolution ? (
            <p>Resolve the active opponent in the main window.</p>
          ) : null}
          {candidate ? (
            <div className="ui-stack" role="status">
              <p>Detected opponent: {candidate.displayHandle}</p>
              <Button
                onClick={async () => {
                  const result = await confirmOpponent(candidate);
                  if (result.ok) {
                    setCandidate(undefined);
                    setError(undefined);
                  } else {
                    setError(result.error.message);
                  }
                }}
              >
                Confirm {candidate.displayHandle}
              </Button>
            </div>
          ) : null}
          {view.currentObservations.slice(0, 3).map((observation) => (
            <p key={observation.id}>{observation.text}</p>
          ))}
          {expanded && view.historicalObservations.length ? (
            <section aria-label="Permitted historical observations">
              <h2>Prior notes</h2>
              {view.historicalObservations.slice(0, 5).map((observation) => (
                <p key={observation.id}>{observation.text}</p>
              ))}
            </section>
          ) : null}
          {expanded && view.publicSnapshot ? (
            <p>
              {view.publicSnapshot.label} · {view.publicSnapshot.format} ·{" "}
              {view.publicSnapshot.sourceText}
            </p>
          ) : null}
          {!view.confirmedHandle ? (
            <p>
              Ready for a confirmed opponent. Historical data is not loaded.
            </p>
          ) : null}
          <div className="ui-actions">
            <Button
              aria-label="Open quick capture"
              onClick={async () => {
                const result = await openCapture();
                if (!result.ok && result.error.code !== "already_open") {
                  setError(result.error.message);
                }
              }}
            >
              Capture note
            </Button>
            <Button
              aria-expanded={expanded}
              onClick={() => void changeInteraction(!expanded)}
              variant="secondary"
            >
              {expanded ? "Collapse overlay" : "Expand overlay"}
            </Button>
            <Button
              aria-label="Hide opponent overlay"
              onClick={() => void getCurrentWindow().hide()}
              variant="secondary"
            >
              Hide
            </Button>
          </div>
        </div>
      </Panel>
    </main>
  );
}
