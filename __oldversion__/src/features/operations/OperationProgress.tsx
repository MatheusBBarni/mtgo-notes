import { ProgressBar } from "@heroui/react";
import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import {
  cancelOperation,
  getOperation,
  progressPercent,
  type OperationRecord,
  type OperationProgressEvent,
} from "../../lib/ipc/operations";
import { Button, StatusLabel } from "../../ui/primitives";

type OperationProgressProps = {
  operation: OperationRecord;
  onError: (message: string) => void;
};

export function OperationProgress({
  operation,
  onError,
}: OperationProgressProps) {
  const [current, setCurrent] = useState(operation);
  const percent = progressPercent(current);
  const cancellable = [
    "requested",
    "running",
    "awaiting_confirmation",
  ].includes(current.state);

  useEffect(() => {
    if (!cancellable) return;
    const timer = window.setInterval(() => {
      void getOperation(current.id).then((result) => {
        if (result.ok) setCurrent(result.data);
      });
    }, 250);
    return () => window.clearInterval(timer);
  }, [cancellable, current.id]);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    void listen<OperationProgressEvent>("operation://progress-v1", (event) => {
      if (event.payload.payload.id === current.id) {
        setCurrent(event.payload.payload);
      }
    }).then((removeListener) => {
      if (disposed) removeListener();
      else unlisten = removeListener;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [current.id]);

  async function cancel() {
    const result = await cancelOperation(current.id);
    if (!result.ok) onError(result.error.message);
    else setCurrent(result.data);
  }

  return (
    <div
      aria-label="Portability operation progress"
      className="operation-progress"
    >
      <div className="section-heading">
        <StatusLabel kind="source" label={current.state.replaceAll("_", " ")} />
        <span>{percent}%</span>
      </div>
      <ProgressBar
        aria-label="Operation progress"
        className="w-full"
        value={percent}
      >
        <ProgressBar.Track>
          <ProgressBar.Fill />
        </ProgressBar.Track>
      </ProgressBar>
      {cancellable ? (
        <Button onClick={cancel} variant="secondary">
          Cancel safely
        </Button>
      ) : null}
    </div>
  );
}
