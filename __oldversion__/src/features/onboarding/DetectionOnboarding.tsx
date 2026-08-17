import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import {
  acceptNewerReplacementEvent,
  type ReplacementEvent,
} from "../../lib/ipc/events";
import {
  listMtgoWindows,
  listProviders,
  pauseDetection,
  selectMtgoWindow,
  setProviderConsent,
  type AuthorizedWindow,
  type ProviderStatus,
} from "../../lib/ipc/provider";
import { Button, Checkbox, Panel, StatusLabel } from "../../ui/primitives";

const DISCLOSED_FIELDS = [
  "visible opponent handle",
  "visible match phase",
  "visible format, game, and result labels",
];

export function DetectionOnboarding() {
  const [provider, setProvider] = useState<ProviderStatus>();
  const [windows, setWindows] = useState<AuthorizedWindow[]>([]);
  const [selectedFields, setSelectedFields] =
    useState<string[]>(DISCLOSED_FIELDS);
  const [error, setError] = useState<string>();
  const providerRevision = useRef(0);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void refresh();
    void listen<ReplacementEvent<ProviderStatus>>(
      "provider://status-v1",
      (incoming) => {
        const accepted = acceptNewerReplacementEvent<ProviderStatus>(
          incoming.payload,
          providerRevision.current,
          {
            clearSensitiveView: () => {
              setProvider(undefined);
              setWindows([]);
            },
            requestBootstrap: () => void refresh(),
          },
        );
        if (accepted) {
          providerRevision.current = accepted.revision;
          setProvider(accepted.payload);
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

  async function refresh() {
    const [providers, visibleWindows] = await Promise.all([
      listProviders(),
      listMtgoWindows(),
    ]);
    if (providers.ok) setProvider(providers.data[0]);
    if (visibleWindows.ok) setWindows(visibleWindows.data);
  }

  async function consent(granted: boolean) {
    const result = await setProviderConsent(granted, selectedFields);
    if (result.ok) {
      setProvider(result.data);
      setError(undefined);
      await refresh();
    } else {
      setError(result.error.message);
    }
  }

  async function choose(window: AuthorizedWindow) {
    const result = await selectMtgoWindow(window);
    if (result.ok) {
      setProvider(result.data);
      setError(undefined);
    } else {
      setError(result.error.message);
    }
  }

  async function pause(paused: boolean) {
    const result = await pauseDetection(paused);
    if (result.ok) {
      setProvider(result.data);
      setError(undefined);
    } else {
      setError(result.error.message);
    }
  }

  return (
    <Panel label="Automatic match context">
      <div className="ui-stack">
        <p>
          With your permission, the companion reads only visible accessibility
          labels in the MTGO window you select. If a label is inaccessible, it
          may OCR only the disclosed visible crop. It never reads process
          memory, logs, files, hidden windows, or network traffic.
        </p>
        <StatusLabel
          kind={provider?.available ? "source" : "phase"}
          label={
            provider?.available
              ? "Selected MTGO window is available"
              : "Automatic context unavailable — manual entry remains available"
          }
        />
        {error ? <StatusLabel kind="error" label={error} /> : null}
        {!provider?.consentGranted ? (
          <>
            <fieldset className="ui-stack">
              <legend>Visible fields the companion may read</legend>
              {DISCLOSED_FIELDS.map((field) => (
                <Checkbox
                  checked={selectedFields.includes(field)}
                  key={field}
                  onChange={(selected) => {
                    setSelectedFields((current) =>
                      selected
                        ? [...new Set([...current, field])]
                        : current.filter((value) => value !== field),
                    );
                  }}
                >
                  {field}
                </Checkbox>
              ))}
            </fieldset>
            <Button
              disabled={!selectedFields.length}
              onClick={() => void consent(true)}
            >
              Allow visible-window detection
            </Button>
          </>
        ) : (
          <>
            <div aria-label="Visible MTGO windows" className="ui-actions">
              {windows.map((window) => (
                <Button
                  key={`${window.nativeHandle}-${window.visibleTitle}`}
                  onClick={() => void choose(window)}
                  variant="secondary"
                >
                  Select {window.visibleTitle}
                </Button>
              ))}
              <Button onClick={() => void refresh()} variant="secondary">
                Refresh windows
              </Button>
            </div>
            <div className="ui-actions">
              <Button
                onClick={() => void pause(!provider.paused)}
                variant="secondary"
              >
                {provider.paused ? "Resume detection" : "Pause detection"}
              </Button>
              <Button onClick={() => void consent(false)} variant="destructive">
                Revoke visible-window access
              </Button>
            </div>
          </>
        )}
      </div>
    </Panel>
  );
}
