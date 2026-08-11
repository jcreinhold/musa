/**
 * The Compose workspace, photographed.
 *
 * These four images are what "the design is settled" means for the rest of
 * the sequence: if a later change makes the chrome louder, the leaf duller,
 * or the engraving looser, the diff says so before anyone has to notice it.
 */

import { expect, test } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";
import { pane, toggleSource } from "./source";

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

// The source showing is the other half of the workspace, and the thing the
// margins and the leaf have to make room for (`01-visual-language.md` §7).
// Photographed at both widths because what gives way differs: at 1440 the
// column holds its measure, at 1100 it trades characters for the page.
for (const size of SIZES) {
  for (const theme of THEMES) {
    test(`compose with source ${size.name} ${theme}`, async ({ page }) => {
      await page.setViewportSize({ width: size.width, height: size.height });
      await page.emulateMedia({ colorScheme: theme });
      // Showing the source is a command, and commands come from the shell.
      await stubShell(page);
      await page.goto("/");
      await engraved(page);
      await toggleSource(page);
      await engraved(page);
      await expect(page).toHaveScreenshot(`compose-source-${size.name}-${theme}.png`);
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
  // Sized, like its neighbours: the point clicked below is 600px down the
  // page, and whether that point is on screen depends on how many rows the
  // top margin took (`01-visual-language.md` §7).
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await engraved(page);
  await page.locator('.engraving [id="event-4"] use').click({ force: true });
  await expect(page.locator(".overlay rect.selection")).toHaveCount(1);

  await page.locator(".engraving").click({ position: { x: 40, y: 600 } });
  await expect(page.locator(".overlay rect.selection")).toHaveCount(0);
});

test("the source column pushes the leaf sideways and never covers it", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await stubShell(page);
  await page.goto("/");
  await engraved(page);
  const before = await page.locator(".stage").boundingBox();

  await toggleSource(page);
  await engraved(page);
  const after = await page.locator(".stage").boundingBox();
  const column = await pane(page).boundingBox();

  // It takes width, which this screen has spare, and gives back height,
  // which the page needs (`01-visual-language.md` §7).
  expect(after?.width ?? 0).toBeLessThan(before?.width ?? 0);
  expect(after?.height ?? 0).toBe(before?.height ?? 0);

  // And it stands beside the page rather than over it.
  expect((column?.x ?? 0) + (column?.width ?? 0)).toBeLessThanOrEqual(after?.x ?? 0);
});
