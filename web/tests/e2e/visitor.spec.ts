import { expect, test } from "@playwright/test";

const releases = "https://github.com/MatheusBBarni/mtgo-notes/releases";

test("visitor understands the product on Home", async ({ page }) => {
  await page.goto("/mtgo-notes");
  await expect(page.getByRole("heading", { name: "MTGO Opponent Notes" })).toBeVisible();
  await expect(page.getByText(/private, local-first/i)).toBeVisible();
  await expect(page.getByText("confirm opponent")).toBeVisible();
  await expect(page.getByText(/board logger/i)).toBeVisible();
  await expect(page.getByRole("link", { name: "Download for Windows" })).toHaveAttribute(
    "href",
    releases,
  );
  await expect(page.getByText(/not affiliated/i)).toBeVisible();
});

test("Download page points at GitHub Releases", async ({ page }) => {
  await page.goto("/mtgo-notes/download");
  await expect(page.getByText("Windows 10 22H2")).toBeVisible();
  await expect(page.getByRole("link", { name: "Download for Windows" })).toHaveAttribute(
    "href",
    releases,
  );
});

test("navigates to live attach and privacy", async ({ page }) => {
  await page.goto("/mtgo-notes");
  await page.getByRole("navigation").getByRole("link", { name: "Live attach" }).click();
  await expect(page.getByText("Not legal advice")).toBeVisible();
  await page.getByRole("navigation").getByRole("link", { name: "Privacy" }).click();
  await expect(page.getByText(/no signup/i)).toBeVisible();
});

test("theme toggle switches between light and dark", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/mtgo-notes");
  const toggle = page.getByRole("button", { name: /use dark theme/i });
  await expect(toggle).toBeVisible();
  await toggle.click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.getByRole("button", { name: /use light theme/i }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
});

test("Home does not overflow at phone width", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/mtgo-notes");
  const overflowed = await page.evaluate(
    () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
  );
  expect(overflowed).toBe(false);
  await expect(page.getByRole("link", { name: "Download for Windows" })).toBeVisible();
});
