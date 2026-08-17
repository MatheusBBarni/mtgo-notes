import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

vi.mock("../../src/features/settings/OperationalSettings", () => ({
  OperationalSettings: () => (
    <section>
      Settings content
      <label>
        Settings draft
        <input />
      </label>
    </section>
  ),
}));
vi.mock("../../src/features/onboarding/DetectionOnboarding", () => ({
  DetectionOnboarding: () => <section>Detection content</section>,
}));
vi.mock("../../src/features/encounter/LiveEncounterControls", () => ({
  LiveEncounterControls: () => <section>Live match content</section>,
}));
vi.mock("../../src/features/decks/DeckEnrichmentPanel", () => ({
  DeckEnrichmentPanel: () => <section>Deck context content</section>,
}));
vi.mock("../../src/features/notebook/NotebookWorkspace", () => ({
  NotebookWorkspace: () => <section>Notebook content</section>,
}));
vi.mock("../../src/features/portability/PortabilityWorkspace", () => ({
  PortabilityWorkspace: () => <section>Backup and export content</section>,
}));

import { MainApp } from "../../src/main/MainApp";

describe("main workspace navigation", () => {
  test("groups the existing workspaces into one accessible tab set", async () => {
    const user = userEvent.setup();
    render(<MainApp />);

    const tabs = screen.getAllByRole("tab");
    expect(tabs).toHaveLength(6);
    expect(
      screen.getByRole("tablist", { name: "Workspace sections" }),
    ).toBeVisible();
    expect(screen.getByRole("tab", { name: "Settings" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByText("Settings content")).toBeVisible();
    await user.type(
      screen.getByRole("textbox", { name: "Settings draft" }),
      "keep me",
    );

    await user.click(screen.getByRole("tab", { name: "Notebook" }));

    expect(screen.getByRole("tab", { name: "Notebook" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByText("Notebook content")).toBeVisible();
    expect(
      screen.getByText("Settings content").closest("[data-inert]"),
    ).toHaveAttribute("data-inert", "true");

    await user.click(screen.getByRole("tab", { name: "Settings" }));

    expect(screen.getByRole("textbox", { name: "Settings draft" })).toHaveValue(
      "keep me",
    );
  });
});
