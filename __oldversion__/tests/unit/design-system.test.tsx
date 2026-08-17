import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { designTokens } from "../../src/ui/tokens";
import { Button, StatusLabel, TextField } from "../../src/ui/primitives";

const globalStyles = readFileSync(
  resolve(process.cwd(), "src/ui/global.css"),
  "utf8",
);

describe("DesignSystem contract", () => {
  test("UT-105: token snapshot matches DESIGN.md", () => {
    expect(designTokens.colors).toMatchObject({
      primary: "#181d26",
      primaryActive: "#0d1218",
      ink: "#181d26",
      body: "#333840",
      muted: "#41454d",
      hairline: "#dddddd",
      borderStrong: "#9297a0",
      canvas: "#ffffff",
      surfaceSoft: "#f8fafc",
      surfaceStrong: "#e0e2e6",
      surfaceDark: "#181d26",
      surfaceDarkElevated: "#1d1f25",
      signatureCoral: "#aa2d00",
      signatureForest: "#0a2e0e",
      signatureCream: "#f5e9d4",
      signaturePeach: "#fcab79",
      signatureMint: "#a8d8c4",
      signatureYellow: "#f4d35e",
      signatureMustard: "#d9a441",
      onPrimary: "#ffffff",
      onDark: "#ffffff",
      link: "#1b61c9",
      linkActive: "#1a3866",
      info: "#254fad",
      infoBorder: "#458fff",
      success: "#006400",
      successBorder: "#39bf45",
    });
    expect(designTokens.fontFamily).toBe(
      '"Inter Variable", "Segoe UI", system-ui, sans-serif',
    );
    expect(designTokens.spacing).toEqual({
      xxs: "4px",
      xs: "8px",
      sm: "12px",
      md: "16px",
      lg: "24px",
      xl: "32px",
      xxl: "48px",
      section: "96px",
    });
    expect(designTokens.radii).toEqual({
      control: "6px",
      card: "10px",
      panel: "12px",
    });
  });

  test("UT-106: interactive and error states are visually distinct without gradients", () => {
    expect(designTokens.effects.gradient).toBe("none");
    expect(globalStyles).not.toMatch(/linear-gradient|radial-gradient/);
    expect(globalStyles).toContain('@import "tailwindcss"');
    expect(globalStyles).toContain('@import "@heroui/styles"');
    expect(globalStyles).toContain(".ui-button--primary[data-pressed");
    expect(globalStyles).toContain(":focus-visible");
    expect(globalStyles).toContain('[aria-invalid="true"]');

    render(
      <>
        <Button>Primary action</Button>
        <Button variant="secondary">Secondary action</Button>
        <Button variant="destructive">Destructive action</Button>
      </>,
    );

    expect(screen.getByRole("button", { name: "Primary action" })).toHaveClass(
      "button--primary",
      "ui-button--primary",
    );
    expect(
      screen.getByRole("button", { name: "Secondary action" }),
    ).toHaveClass("button--secondary", "ui-button--secondary");
    expect(
      screen.getByRole("button", { name: "Destructive action" }),
    ).toHaveClass("button--danger", "ui-button--destructive");

    expect(
      new Set([
        designTokens.colors.primary,
        designTokens.colors.primaryActive,
        designTokens.colors.destructive,
        designTokens.colors.surfaceStrong,
        designTokens.colors.errorSurface,
      ]).size,
    ).toBe(5);
  });

  test("UT-107: interactive primitives expose name, role, state, and visible focus", async () => {
    const user = userEvent.setup();
    render(
      <>
        <Button variant="secondary">Review privacy</Button>
        <Button busy>Saving</Button>
        <TextField error="Required" label="Opponent handle" />
      </>,
    );

    const review = screen.getByRole("button", {
      name: "Review privacy",
    });
    const saving = screen.getByRole("button", { name: "Saving" });
    const field = screen.getByRole("textbox", {
      name: "Opponent handle",
    });

    expect(saving).toBeDisabled();
    expect(saving).toHaveAttribute("aria-busy", "true");
    expect(field).toHaveAttribute("aria-invalid", "true");
    expect(screen.getByRole("alert")).toHaveTextContent("Error: Required");

    await user.tab();
    expect(review).toHaveFocus();
    expect(globalStyles).toContain(":focus-visible");
    expect(globalStyles).toContain("var(--color-focus)");
  });

  test("UT-108: semantic statuses remain understandable without color", () => {
    render(
      <>
        <StatusLabel kind="phase" label="In game" />
        <StatusLabel kind="certainty" label="Observed" />
        <StatusLabel kind="source" label="Local note" />
        <StatusLabel kind="error" label="Save failed" />
        <StatusLabel kind="incomplete" label="Incomplete" />
      </>,
    );

    for (const text of [
      "Phase: In game",
      "Certainty: Observed",
      "Source: Local note",
      "Error: Save failed",
      "Encounter: Incomplete",
    ]) {
      expect(screen.getByText(text)).toBeVisible();
    }

    expect(screen.getByRole("alert")).toHaveTextContent("Error: Save failed");
  });
});
