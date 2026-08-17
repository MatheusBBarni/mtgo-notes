import { expect, test } from "@playwright/test";

test("visitor understands the product on Home", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "MTGO Opponent Notes" })).toBeVisible();
  await expect(page.getByText(/private, local-first/i)).toBeVisible();
  await expect(page.getByText("confirm opponent")).toBeVisible();
  await expect(page.getByText(/board logger/i)).toBeVisible();
  await expect(page.getByRole("link", { name: "Download for Windows" })).toHaveAttribute(
    "href",
    "/download/windows",
  );
  await expect(page.getByText(/not affiliated/i)).toBeVisible();
});

test("available download stays on the first-party href", async ({ page }) => {
  await page.route("**/download/status", async (route) => {
    await route.fulfill({
      json: {
        available: true,
        version: "0.2.1",
        filename: "MTGONotes-0.2.1-win-x64.zip",
      },
    });
  });
  await page.route("**/download/windows", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/zip",
      body: "PK",
    });
  });

  await page.goto("/download");
  await expect(page.getByText("0.2.1")).toBeVisible();
  await expect(page.getByRole("link", { name: "Download for Windows" })).toHaveAttribute(
    "href",
    "/download/windows",
  );
});

test("empty state is not a dead 404 link", async ({ page }) => {
  await page.route("**/download/status", async (route) => {
    await route.fulfill({ json: { available: false } });
  });

  await page.goto("/download?available=0");
  await expect(page.getByText("A Windows build is not published yet.")).toBeVisible();
  await expect(page.getByRole("link", { name: "Download for Windows" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Download for Windows" })).toBeDisabled();
});

test("keeps the first-party href when status fetch fails", async ({ page }) => {
  await page.route("**/download/status", async (route) => {
    await route.abort();
  });

  await page.goto("/");
  await expect(page.getByRole("link", { name: "Download for Windows" })).toHaveAttribute(
    "href",
    "/download/windows",
  );
});

test("navigates to live attach and privacy", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("navigation").getByRole("link", { name: "Live attach" }).click();
  await expect(page.getByText("Not legal advice")).toBeVisible();
  await page.getByRole("navigation").getByRole("link", { name: "Privacy" }).click();
  await expect(page.getByText(/no signup/i)).toBeVisible();
});

test("Home does not overflow at phone width", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  const overflowed = await page.evaluate(
    () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
  );
  expect(overflowed).toBe(false);
  await expect(page.getByRole("link", { name: "Download for Windows" })).toBeVisible();
});
