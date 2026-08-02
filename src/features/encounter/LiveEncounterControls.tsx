import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState, type FormEvent } from "react";

import { bootstrap } from "../../lib/ipc/client";
import {
  confirmOpponent,
  correctPhase,
  enterOpponent,
  finishEncounter,
  type EncounterCommandView,
  type EncounterStateView,
  type OpponentCandidate,
} from "../../lib/ipc/encounter";
import type { InternalPhase } from "../../lib/ipc/contracts";
import {
  acceptNewerReplacementEvent,
  type ReplacementEvent,
} from "../../lib/ipc/events";
import { Button, Panel, StatusLabel, TextField } from "../../ui/primitives";

const CORRECTABLE_PHASES: InternalPhase[] = [
  "pre_match",
  "in_game_restricted",
  "between_games",
  "completion_pending",
];

export function LiveEncounterControls() {
  const [encounter, setEncounter] = useState<EncounterCommandView>();
  const [candidate, setCandidate] = useState<OpponentCandidate>();
  const [handle, setHandle] = useState("");
  const [error, setError] = useState<string>();
  const encounterRevision = useRef(0);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void bootstrap().then((result) => {
      if (result.ok && result.data.encounter) {
        setEncounter({
          id: result.data.encounter.id,
          profileId: "",
          primaryHandle: "Confirmed opponent",
          phase: result.data.encounter.phase,
          generation: 0,
          revision: result.data.encounter.revision,
        });
      }
    });
    void listen<ReplacementEvent<EncounterStateView>>(
      "encounter://state-v1",
      (incoming) => {
        const accepted = acceptNewerReplacementEvent<EncounterStateView>(
          incoming.payload,
          encounterRevision.current,
          {
            clearSensitiveView: () => {
              setCandidate(undefined);
              setEncounter(undefined);
            },
            requestBootstrap: () =>
              setError("Live context changed. Refreshing safe state."),
          },
        );
        if (accepted) {
          encounterRevision.current = accepted.revision;
          setCandidate(accepted.payload.candidate);
          setEncounter(accepted.payload.encounter);
        }
      },
    ).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  async function submit(event: FormEvent) {
    event.preventDefault();
    const result = await enterOpponent(handle);
    if (result.ok) {
      setEncounter(result.data);
      setHandle("");
      setError(undefined);
    } else {
      setError(result.error.message);
    }
  }

  async function phase(phase: InternalPhase) {
    if (!encounter) return;
    const result = await correctPhase(encounter.id, phase, encounter.revision);
    if (result.ok) {
      setEncounter(result.data);
      setError(undefined);
    } else {
      setError(result.error.message);
    }
  }

  return (
    <Panel label="Live encounter">
      <div className="ui-stack">
        <StatusLabel
          kind="phase"
          label={
            encounter
              ? `${encounter.primaryHandle}: ${encounter.phase.replaceAll("_", " ")}`
              : "No active encounter"
          }
        />
        {error ? <StatusLabel kind="error" label={error} /> : null}
        {candidate ? (
          <div className="ui-stack" role="status">
            <p>
              Detected opponent: {candidate.displayHandle}. Confirm before any
              history is loaded.
            </p>
            <div className="ui-actions">
              <Button
                onClick={async () => {
                  const result = await confirmOpponent(candidate);
                  if (result.ok) {
                    setEncounter(result.data);
                    setCandidate(undefined);
                    setError(undefined);
                  } else {
                    setError(result.error.message);
                  }
                }}
              >
                Confirm {candidate.displayHandle}
              </Button>
              <Button
                onClick={() => setCandidate(undefined)}
                variant="secondary"
              >
                Dismiss candidate
              </Button>
            </div>
          </div>
        ) : null}
        {!encounter ? (
          <form className="ui-actions" onSubmit={submit}>
            <TextField
              label="Manual opponent"
              onChange={(event) => setHandle(event.currentTarget.value)}
              value={handle}
            />
            <Button disabled={!handle.trim()} type="submit">
              Start manual encounter
            </Button>
          </form>
        ) : (
          <>
            <div aria-label="Correct match phase" className="ui-actions">
              {CORRECTABLE_PHASES.map((value) => (
                <Button
                  key={value}
                  onClick={() => void phase(value)}
                  variant={value === encounter.phase ? "primary" : "secondary"}
                >
                  {value.replaceAll("_", " ")}
                </Button>
              ))}
            </div>
            <Button
              onClick={async () => {
                const result = await finishEncounter(encounter.id);
                if (result.ok) setEncounter(undefined);
                else setError(result.error.message);
              }}
              variant="secondary"
            >
              Finish encounter
            </Button>
          </>
        )}
      </div>
    </Panel>
  );
}
