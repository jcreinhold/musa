/**
 * The Compose workspace, photographed.
 *
 * These four images are what "the design is settled" means for the rest of
 * the sequence: if a later prompt makes the chrome louder, the leaf duller,
 * or the engraving looser, the diff says so before anyone has to notice it.
 */

import { expect, test } from "@playwright/test";

import { engraved } from "./engraved";

const SIZES = [
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1100x720", width: 1100, height: 720 },
] as const;

const THEMES = ["light", "dark"] as const;

for (const size of SIZES) {
  for (const theme of THEMES) {
    test(`compose ${size.name} ${theme}`, async ({ page }) => {
      await page.setViewportSize({ width: size.width, height: size.height });
      await page.emulateMedia({ colorScheme: theme });
      await page.goto("/");
      await engraved(page);
      await expect(page).toHaveScreenshot(`compose-${size.name}-${theme}.png`);
    });
  }
}

test("selecting a note halos it and fills the inspector", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");
  await engraved(page);

  // Selection is applied by the frontend from the clicked element's xml:id,
  // in the same frame and with no round trip (`03-interaction.md` §2).
  const note = page.locator('.engraving [id="event-4"] use');
  await note.click({ force: true });

  await expect(page.locator(".overlay rect.selection")).toHaveCount(1);
  await expect(page.locator(".inspector")).toContainText("A4");
  await expect(page.locator(".inspector")).toContainText("2:3");
  // The Origin row is present whether or not the lens is held (§3).
  await expect(page.locator(".inspector")).toContainText("sigh()");
  await expect(page).toHaveScreenshot("compose-selection-light.png");
});

test("clicking empty leaf clears the selection", async ({ page }) => {
  await page.goto("/");
  await engraved(page);
  await page.locator('.engraving [id="event-4"] use').click({ force: true });
  await expect(page.locator(".overlay rect.selection")).toHaveCount(1);

  await page.locator(".engraving").click({ position: { x: 40, y: 600 } });
  await expect(page.locator(".overlay rect.selection")).toHaveCount(0);
});

test("the drawer pushes the leaf rather than covering it", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await engraved(page);
  const before = await page.locator(".engraving").boundingBox();

  await page.getByRole("button", { name: /Source/ }).click();
  const after = await page.locator(".engraving").boundingBox();

  expect(before?.height ?? 0).toBeGreaterThan(after?.height ?? 0);
});
