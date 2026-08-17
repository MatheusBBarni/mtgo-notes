import { acceptReplacementEvent } from "../../src/lib/ipc/events";

test("IT-272: unknown event major clears sensitive view and requests bootstrap", () => {
  let rendererState: {
    opponent: string | null;
    history: string[];
  } = {
    opponent: "SyntheticOpponent",
    history: ["private historical note"],
  };
  let bootstrapRequests = 0;

  const result = acceptReplacementEvent(
    {
      name: "overlay://view-v1",
      version: { major: 7 },
      revision: 14,
      payload: { history: ["must not merge"] },
    },
    {
      clearSensitiveView: () => {
        rendererState = { opponent: null, history: [] };
      },
      requestBootstrap: () => {
        bootstrapRequests += 1;
      },
    },
  );

  expect(result).toBeNull();
  expect(rendererState).toEqual({ opponent: null, history: [] });
  expect(bootstrapRequests).toBe(1);
});
