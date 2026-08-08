/**
 * How the composer reads and types (prompt 55).
 *
 * Two preferences, and the two boundaries that make them safe: the text size
 * is the *frame's* and never the score's, and vim mode is the source column's
 * and never the application's. Each of these tests is one of those boundaries
 * being held.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";
import { pane, source, text, toggleSource } from "./source";

/** Run a command the way the native menu and the palette both do. */
async function run(page: Page, id: string): Promise<void> {
  await page.evaluate((command) => window.__musaEmit("musa://command", command), id);
}

/** The computed size of a piece of the frame's own type, in pixels. */
async function frameType(page: Page): Promise<number> {
  return page
    .locator("nav.workspaces button")
    .first()
    .evaluate((node) => Number.parseFloat(globalThis.getComputedStyle(node).fontSize));
}

/**
 * How much of the page one staff takes.
 *
 * A ratio rather than a height, because the leaf is fitted to whatever room
 * the chrome leaves and larger chrome legitimately leaves less: the page gets
 * smaller, uniformly, exactly as it does when the window is resized. What
 * must not change is the engraving *inside* it — a staff that took a larger
 * share of its page would be a second zoom.
 */
async function staffShare(page: Page): Promise<number> {
  const staff = await page.locator(".engraving .page svg g.staff").first().boundingBox();
  const leaf = await page.locator(".engraving .page").first().boundingBox();
  return (staff?.height ?? 0) / (leaf?.height ?? 1);
}

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await stubShell(page);
  await page.goto("/");
  await engraved(page);
});

test("a step larger is the frame's type, and only the frame's", async ({ page }) => {
  const before = await frameType(page);
  const share = await staffShare(page);

  await run(page, "view.text.larger");
  await expect.poll(() => frameType(page)).toBeGreaterThan(before);

  // The score's size is zoom, it is a re-layout, and it already has a control.
  // A text-size preference that also grew the staves would be a second zoom
  // that disagrees with the first (`02-engraving.md` §5).
  //
  // Polled, because the claim is about where the engraving lands and not about
  // the moment in between: taller chrome resizes the leaf before the layout it
  // asked for comes back, and read in that gap the ratio is last layout's staff
  // over this layout's page — a number that belongs to neither.
  await expect.poll(() => staffShare(page)).toBeCloseTo(share, 4);

  await run(page, "view.text.reset");
  await expect.poll(() => frameType(page)).toBeCloseTo(before, 1);
});

test("the ladder has four rungs and stops at both ends", async ({ page }) => {
  const normal = await frameType(page);
  for (let step = 0; step < 5; step += 1) await run(page, "view.text.larger");
  const largest = await frameType(page);
  for (let step = 0; step < 5; step += 1) await run(page, "view.text.smaller");
  const smallest = await frameType(page);

  expect(largest / normal).toBeCloseTo(1.3, 2);
  expect(smallest / normal).toBeCloseTo(0.85, 2);
});

test("the size outlives the window it was chosen in", async ({ page }) => {
  const before = await frameType(page);
  await run(page, "view.text.larger");
  await expect.poll(() => frameType(page)).toBeGreaterThan(before);

  await page.reload();
  await engraved(page);
  expect(await frameType(page)).toBeCloseTo(before * 1.15, 1);
});

test("with vim off, Esc still puts the source column away", async ({ page }) => {
  await toggleSource(page);
  await source(page).click();
  await page.keyboard.press("Escape");
  // Nothing is selected after the click landed in the text, so the second
  // Escape is the one that closes it — the first clears the selection.
  await page.keyboard.press("Escape");
  await expect(pane(page)).toHaveCount(0);
});

test("with vim on, Esc leaves insert mode and the column stays", async ({ page }) => {
  await run(page, "view.vim");
  await toggleSource(page);
  await source(page).click();

  // The mode says which mode it is in, which is the one thing worse to omit
  // than the mode itself.
  await expect(page.locator(".cm-vim-panel")).toContainText("NORMAL");

  await page.keyboard.press("i");
  await expect(page.locator(".cm-vim-panel")).toContainText("INSERT");
  await page.keyboard.press("Escape");

  // Back to normal mode, and the column is still there: `Esc` belongs to vim
  // while vim mode is on and the caret is in the source, and nowhere else.
  await expect(page.locator(".cm-vim-panel")).toContainText("NORMAL");
  await expect(pane(page)).toHaveCount(1);
});

test("vim's undo is the project's undo, not a second history", async ({ page }) => {
  await run(page, "view.vim");
  await toggleSource(page);
  await source(page).click();

  const before = await page.evaluate(() => window.__musaRevision);
  await page.keyboard.press("i");
  await page.keyboard.insertText("// a note to self\n");
  await expect.poll(() => page.evaluate(() => window.__musaRevision)).toBeGreaterThan(before);
  await page.keyboard.press("Escape");

  await page.keyboard.press("u");
  // The revision went back, which is the whole point: `u` and `⌘Z` are one
  // undo over one document, and a vim history beside the project's would
  // disagree with it about what the document is.
  await expect.poll(() => page.evaluate(() => window.__musaRevision)).toBe(before);
});

test("`:w` is the project's save", async ({ page }) => {
  await run(page, "view.vim");
  await toggleSource(page);
  await source(page).click();
  await page.keyboard.press("i");
  await page.keyboard.insertText("// x\n");
  await page.keyboard.press("Escape");

  await page.keyboard.press(":");
  await page.keyboard.insertText("w");
  await page.keyboard.press("Enter");

  await expect.poll(() => page.evaluate(() => window.__musaSaves)).toBeGreaterThan(0);
});

test("the mode outlives the window it was turned on in", async ({ page }) => {
  await run(page, "view.vim");
  await page.reload();
  await engraved(page);
  await toggleSource(page);
  await source(page).click();
  await expect(page.locator(".cm-vim-panel")).toContainText("NORMAL");
});

test("vim mode changes the keys and nothing about the document", async ({ page }) => {
  await toggleSource(page);
  const before = await text(page);
  await run(page, "view.vim");
  expect(await text(page)).toBe(before);
});

test("the frame at Larger, photographed", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await run(page, "view.text.larger");
  await run(page, "view.text.larger");
  await expect(page).toHaveScreenshot("larger-1440x900-light.png");
});

test("the frame at Larger, in the dark", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await engraved(page);
  await run(page, "view.text.larger");
  await run(page, "view.text.larger");
  await expect(page).toHaveScreenshot("larger-1440x900-dark.png");
});
