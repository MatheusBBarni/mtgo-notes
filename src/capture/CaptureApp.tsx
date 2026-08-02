import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState, type FormEvent } from "react";

import { discardDraft, type CaptureDraftView } from "../lib/ipc/capture";
import {
  acceptNewerReplacementEvent,
  type ReplacementEvent,
} from "../lib/ipc/events";
import { saveObservation } from "../lib/ipc/notebook";
import { Button, StatusLabel, TextAreaField } from "../ui/primitives";

export function CaptureApp() {
  const [encounterId, setEncounterId] = useState("");
  const [windowInstance, setWindowInstance] = useState("");
  const [text, setText] = useState("");
  const [error, setError] = useState<string>();
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);
  const formRef = useRef<HTMLFormElement>(null);
  const draftRevision = useRef(0);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<ReplacementEvent<CaptureDraftView>>(
      "capture://draft-v1",
      (incoming) => {
        const accepted = acceptNewerReplacementEvent<CaptureDraftView>(
          incoming.payload,
          draftRevision.current,
          {
            clearSensitiveView: () => {
              setEncounterId("");
              setWindowInstance("");
              setText("");
            },
            requestBootstrap: () =>
              setError("Capture state changed. Invoke the shortcut again."),
          },
        );
        if (accepted) {
          draftRevision.current = accepted.revision;
          setEncounterId(accepted.payload.encounterId);
          setWindowInstance(accepted.payload.windowInstance);
          setText(accepted.payload.text);
          setError(undefined);
          requestAnimationFrame(() => {
            document
              .querySelector<HTMLTextAreaElement>("#capture-observation")
              ?.focus();
          });
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
    setBusy(true);
    setError(undefined);
    const result = await saveObservation({
      encounterId,
      text,
      cards: [],
      tags: [],
    });
    setBusy(false);
    if (!result.ok) {
      setError(
        `${result.error.message} Your text is preserved for retry or copying.`,
      );
      return;
    }
    setText("");
    setSaved(true);
  }

  async function dismiss() {
    if (encounterId && windowInstance) {
      const result = await discardDraft(encounterId, windowInstance);
      if (!result.ok && result.error.code !== "not_found") {
        setError(result.error.message);
        return;
      }
    }
    setText("");
    setError(undefined);
    setSaved(false);
    await getCurrentWindow().hide();
  }

  return (
    <main className="app-shell app-shell--compact">
      <form
        aria-label="Quick observation capture"
        className="ui-stack"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            void dismiss();
          } else if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            formRef.current?.requestSubmit();
          }
        }}
        onSubmit={submit}
        ref={formRef}
      >
        {error ? <StatusLabel kind="error" label={error} /> : null}
        {saved ? <StatusLabel kind="source" label="Observation saved" /> : null}
        <input name="encounterId" type="hidden" value={encounterId} />
        <TextAreaField
          autoFocus
          className="capture-textarea"
          inputId="capture-observation"
          label="Observation"
          maxLength={4000}
          onChange={(event) => {
            setText(event.currentTarget.value);
            setSaved(false);
          }}
          placeholder="What did you notice?"
          required
          value={text}
        />
        <p>
          Add optional card and tendency details in the main notebook later.
        </p>
        <div className="ui-actions">
          <Button
            busy={busy}
            disabled={!encounterId || !text.trim()}
            type="submit"
          >
            Save observation
          </Button>
          <Button onClick={() => void dismiss()} variant="secondary">
            Cancel
          </Button>
        </div>
      </form>
    </main>
  );
}
