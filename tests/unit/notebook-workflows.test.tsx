import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const invokeMock = vi.hoisted(() =>
  vi.fn<
    (
      command: string,
      arguments_?: { request?: Record<string, unknown> },
    ) => Promise<unknown>
  >(),
);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import { NotebookWorkspace } from "../../src/features/notebook/NotebookWorkspace";

const success = (data: unknown) => ({ ok: true, data, revision: 1 });

describe("Task 04 visible notebook journeys", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  test("E2E-008: free text remains the only required note input while optional structure is preserved", async () => {
    invokeMock.mockResolvedValue(
      success({
        id: "observation-id",
        encounterId: "encounter-id",
        text: "Plays patiently around removal",
        encounterStartedAt: 1,
        createdAt: 2,
        revision: 4,
        cards: [],
        tags: [],
        source: "player_observation",
      }),
    );
    const user = userEvent.setup();
    render(<NotebookWorkspace />);

    await user.type(
      screen.getByRole("textbox", { name: "Encounter ID" }),
      "encounter-id",
    );
    await user.type(
      screen.getByRole("textbox", { name: "Observation" }),
      "Plays patiently around removal",
    );
    await user.click(screen.getByText("Optional structure"));
    await user.type(
      screen.getByRole("textbox", { name: "User-entered deck label" }),
      "Jeskai Control",
    );
    await user.type(
      screen.getByRole("textbox", { name: "Card name" }),
      "Subtlety",
    );
    await user.click(screen.getByRole("button", { name: /Card certainty/ }));
    await user.click(await screen.findByRole("option", { name: "Suspected" }));
    await user.type(
      screen.getByRole("textbox", { name: "Card context" }),
      "held up in game two",
    );
    await user.type(
      screen.getByRole("textbox", {
        name: "Custom tendency tags (comma-separated)",
      }),
      "patient, plays around removal",
    );
    await user.click(screen.getByRole("button", { name: "Save observation" }));

    await waitFor(() =>
      expect(
        invokeMock.mock.calls.some(
          ([command]) => command === "save_observation",
        ),
      ).toBe(true),
    );
    const saveCall = invokeMock.mock.calls.find(
      ([command]) => command === "save_observation",
    );
    expect(saveCall?.[1]?.request).toMatchObject({
      encounterId: "encounter-id",
      text: "Plays patiently around removal",
      userDeckLabel: "Jeskai Control",
      cards: [
        {
          displayName: "Subtlety",
          quantity: 1,
          certainty: "suspected",
          context: "held up in game two",
        },
      ],
      tags: ["patient", "plays around removal"],
    });
    expect(screen.getByRole("status")).toHaveTextContent(
      "Observation saved with encounter provenance.",
    );
  }, 15_000);

  test("E2E-012: offline history pages are replaced with an error-safe empty state when disclosure is denied", async () => {
    invokeMock
      .mockResolvedValueOnce(
        success({
          items: [
            {
              entityType: "observation",
              entityId: "note-1",
              sortMs: 1,
              content: "Known local history",
            },
          ],
          nextCursor: "opaque-cursor",
          replacement: true,
        }),
      )
      .mockResolvedValueOnce({
        ok: false,
        error: {
          code: "disclosure_restricted",
          message: "History is unavailable during gameplay.",
          retryable: false,
        },
      });
    const user = userEvent.setup();
    render(<NotebookWorkspace />);

    await user.click(screen.getByRole("tab", { name: "History" }));
    await user.type(
      screen.getByRole("textbox", {
        name: "Search handles, aliases, notes, decks, cards, or tags",
      }),
      "Known",
    );
    await user.click(screen.getByRole("button", { name: /Result type/ }));
    await user.click(
      await screen.findByRole("option", { name: "Observations" }),
    );
    await user.click(screen.getByRole("button", { name: /Card certainty/ }));
    await user.click(await screen.findByRole("option", { name: "Observed" }));
    await user.type(screen.getByLabelText("From date"), "2026-07-01");
    await user.type(screen.getByLabelText("Through date"), "2026-07-31");
    await user.click(
      screen.getByRole("button", { name: "Search local history" }),
    );
    expect(await screen.findByText("Known local history")).toBeVisible();
    expect(invokeMock.mock.calls[0]?.[1]?.request).toMatchObject({
      text: "Known",
      filters: {
        entityTypes: ["observation"],
        certainty: "observed",
        dateFrom: new Date("2026-07-01T00:00:00").getTime(),
        dateTo: new Date("2026-07-31T23:59:59.999").getTime(),
      },
    });

    await user.click(
      screen.getByRole("button", { name: "Load next stable page" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "History is unavailable during gameplay.",
    );
    expect(screen.queryByText("Known local history")).not.toBeInTheDocument();
    expect(screen.getByText("No results to show.")).toBeVisible();
  });

  test("E2E-013: merge and unmerge require previews that expose counts and post-merge assignments", async () => {
    invokeMock.mockImplementation((command) => {
      if (command === "preview_merge") {
        return Promise.resolve(
          success({
            primaryProfileId: "primary-id",
            secondaryProfileId: "secondary-id",
            primaryHandle: "Canonical",
            secondaryHandle: "Duplicate",
            expectedPrimaryRevision: 1,
            expectedSecondaryRevision: 1,
            affected: {
              profiles: 2,
              aliases: 3,
              encounters: 4,
              observations: 5,
              decks: 6,
            },
            conflicts: ["duplicate_alias:Duplicate"],
            conflictCount: 1,
            conflictDetailsBounded: false,
            irreversibleConsequences: [
              "Purged records cannot be restored by unmerge.",
            ],
            planToken: "merge-plan",
          }),
        );
      }
      if (command === "apply_merge") {
        return Promise.resolve(
          success({
            mergeId: "merge-id",
            canonicalProfileId: "primary-id",
            canonicalRevision: 2,
            reversible: true,
          }),
        );
      }
      if (command === "preview_unmerge") {
        return Promise.resolve(
          success({
            mergeId: "merge-id",
            primaryProfileId: "primary-id",
            secondaryProfileId: "secondary-id",
            restoredEncounters: 2,
            restoredDecks: 1,
            postMergeEncounters: 1,
            postMergeDecks: 1,
            proposedPostMergeAssignment: "retain_with_primary",
            planToken: "unmerge-plan",
          }),
        );
      }
      if (command === "apply_unmerge") {
        return Promise.resolve(
          success({
            mergeId: "merge-id",
            canonicalProfileId: "primary-id",
            canonicalRevision: 3,
            reversible: false,
          }),
        );
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    const user = userEvent.setup();
    render(<NotebookWorkspace />);

    await user.click(screen.getByRole("tab", { name: "Identity" }));
    await user.type(
      screen.getByRole("textbox", { name: "First profile ID" }),
      "primary-id",
    );
    await user.type(
      screen.getByRole("textbox", { name: "Second profile ID" }),
      "secondary-id",
    );
    await user.type(
      screen.getByRole("textbox", { name: "Primary profile ID" }),
      "primary-id",
    );
    await user.click(screen.getByRole("button", { name: "Preview merge" }));

    expect(await screen.findByText("Duplicate → Canonical")).toBeVisible();
    expect(
      screen.getByText("Purged records cannot be restored by unmerge."),
    ).toBeVisible();
    expect(screen.getByText("5")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Confirm merge" }));

    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "Profiles merged. Undo record: merge-id",
      ),
    );
    expect(
      screen.getByRole("textbox", { name: "Merge undo record ID" }),
    ).toHaveValue("merge-id");
    await user.click(
      screen.getByRole("button", { name: "Preview unmerge assignments" }),
    );
    expect(await screen.findByText("Unmerge assignment plan")).toBeVisible();
    expect(screen.getByText(/remain with the primary profile/)).toBeVisible();
    await user.click(
      screen.getByRole("button", { name: "Apply confirmed unmerge" }),
    );
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "Profiles restored according to the confirmed assignment plan.",
      ),
    );
  });

  test("E2E-017: scoped deletion requires exact confirmation and exposes undo recovery", async () => {
    invokeMock.mockImplementation((command) => {
      if (command === "preview_deletion") {
        return Promise.resolve(
          success({
            entityType: "profile",
            entityId: "profile-id",
            displayName: "Private Opponent",
            counts: {
              profiles: 1,
              aliases: 2,
              encounters: 3,
              observations: 4,
              decks: 1,
              publicSnapshots: 1,
            },
            dependencies: [],
            confirmation: "DELETE profile Private Opponent",
            scopeToken: "deletion-scope",
          }),
        );
      }
      if (command === "request_deletion") {
        return Promise.resolve(
          success({
            entityType: "profile",
            entityId: "profile-id",
            requestedAt: 1,
            undoDeadline: Date.now() + 10_000,
            undoToken: "undo-token",
            tombstoneState: "pending",
          }),
        );
      }
      if (command === "undo_deletion") {
        return Promise.resolve(
          success({
            entityType: "profile",
            entityId: "profile-id",
            restored: true,
          }),
        );
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    const user = userEvent.setup();
    render(<NotebookWorkspace />);

    await user.click(screen.getByRole("tab", { name: "Privacy" }));
    await user.type(
      screen.getByRole("textbox", { name: "Entity ID" }),
      "profile-id",
    );
    await user.click(screen.getByRole("button", { name: "Preview deletion" }));
    expect(
      await screen.findByText("Affected scope: Private Opponent"),
    ).toBeVisible();
    const confirm = screen.getByRole("button", {
      name: "Confirm scoped deletion",
    });
    expect(confirm).toBeDisabled();
    await user.type(
      screen.getByRole("textbox", {
        name: "Type exactly: DELETE profile Private Opponent",
      }),
      "DELETE profile Private Opponent",
    );
    expect(confirm).toBeEnabled();
    await user.click(confirm);
    expect(
      await screen.findByRole("button", { name: "Undo deletion" }),
    ).toBeVisible();
    expect(
      screen
        .getAllByRole("status")
        .some((status) =>
          status.textContent?.includes("Selected data is hidden now."),
        ),
    ).toBe(true);
    await user.click(screen.getByRole("button", { name: "Undo deletion" }));
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "Deletion undone; local search index restored.",
      ),
    );
  });
});
