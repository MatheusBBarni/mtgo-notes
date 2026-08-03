export const HOST_EVENT_NAMES = [
  "encounter://state-v1",
  "overlay://view-v1",
  "capture://draft-v1",
  "notebook://view-v1",
  "operation://progress-v1",
  "provider://status-v1",
  "classifier://progress-v1",
  "update://status-v1",
  "player://workspace-v1",
] as const;

export type HostEventName = (typeof HOST_EVENT_NAMES)[number];

export type ReplacementEvent<T> = {
  name: HostEventName;
  version: { major: 1 };
  revision: number;
  payload: T;
};

export type FailClosedEventHandlers = {
  clearSensitiveView: () => void;
  requestBootstrap: () => void;
};

export function replacementEvent<T>(
  name: HostEventName,
  revision: number,
  payload: T,
): ReplacementEvent<T> {
  return {
    name,
    version: { major: 1 },
    revision,
    payload,
  };
}

export function acceptReplacementEvent<T>(
  candidate: unknown,
  handlers: FailClosedEventHandlers,
): ReplacementEvent<T> | null {
  if (!isSupportedReplacementEvent<T>(candidate)) {
    handlers.clearSensitiveView();
    handlers.requestBootstrap();
    return null;
  }

  return candidate;
}

export function acceptNewerReplacementEvent<T>(
  candidate: unknown,
  lastRevision: number,
  handlers: FailClosedEventHandlers,
): ReplacementEvent<T> | null {
  const accepted = acceptReplacementEvent<T>(candidate, handlers);
  if (!accepted || accepted.revision <= lastRevision) {
    return null;
  }
  return accepted;
}

function isSupportedReplacementEvent<T>(
  candidate: unknown,
): candidate is ReplacementEvent<T> {
  if (typeof candidate !== "object" || candidate === null) {
    return false;
  }

  const event = candidate as Partial<ReplacementEvent<T>>;
  return (
    typeof event.name === "string" &&
    HOST_EVENT_NAMES.includes(event.name) &&
    event.version?.major === 1 &&
    Number.isSafeInteger(event.revision) &&
    typeof event.payload === "object" &&
    event.payload !== null
  );
}
