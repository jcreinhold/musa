/**
 * The score, driven without a mouse (`03-interaction.md` §3, §5, §6).
 *
 * Everything here is done the way a composer with no pointer would do it:
 * focus the pane, arrow through the music, open the palette, loop, play. A
 * feature that only works when clicked is not in this file, and that is the
 * point of the file.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { toggleSource, rewrite, source, text } from "./source";
import { stubShell } from "./shell";

/**
 * What one inspector row currently says. The rows are typographic — a label
 * over a value, not a form — so they are found by the label they print.
 */
async function field(page: Page, label: string): Promise<string> {
  const row = page.locator(`.inspector .row:has(.label:text-is("${label}")) .value`);
  return (await row.first().innerText()).trim();
}

/** Where the selection is, as bar:beat — it changes with every step. */
async function where(page: Page): Promise<string> {
  return field(page, "Position");
}

/** Focus the score pane, as `Tab` from the top of the document would. */
async function inScore(page: Page): Promise<void> {
  await page.getByRole("application", { name: "Engraved score" }).focus();
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.goto("/");
  await engraved(page);
});

test("arrows walk the voice and the inspector follows", async ({ page }) => {
  await inScore(page);
  // A freshly opened piece has nothing selected — the inspector is the piece
  // itself — so the first key is what puts the caret in the music.
  await page.keyboard.press("Home");
  const first = await where(page);

  await page.keyboard.press("ArrowRight");
  const second = await where(page);
  expect(second).not.toBe(first);

  await page.keyboard.press("ArrowLeft");
  expect(await where(page)).toBe(first);

  // Home and End are the ends of this voice, not of the score.
  await page.keyboard.press("End");
  const last = await where(page);
  await page.keyboard.press("Home");
  expect(await where(page)).toBe(first);
  expect(last).not.toBe(first);
});

test("the selected note is haloed and named for a screen reader", async ({ page }) => {
  await inScore(page);
  await page.keyboard.press("ArrowRight");

  await expect(page.locator(".overlay .selection")).toHaveCount(1);

  const described = await page.evaluate(() => {
    const pane = document.querySelector('[role="application"]');
    const id = pane?.getAttribute("aria-activedescendant") ?? "";
    return document.getElementById(id)?.getAttribute("aria-label") ?? "";
  });
  // A musical sentence, not a path element: pitch, duration, and where it is.
  expect(described).toMatch(/bar \d+ beat \d+/);
});

test("the arrows belong to the score, and the source keeps its own letters", async ({ page }) => {
  await toggleSource(page);
  await source(page).click();
  // `f` is the follow binding in the score; in the source it is an `f`, and
  // `o` — origin view — is an `o`.
  await page.keyboard.type("fof");
  await expect.poll(() => text(page)).toContain("fof");
});

test("⌘K opens the palette, which runs a command by name", async ({ page }) => {
  await page.keyboard.press("Meta+k");
  const palette = page.getByRole("dialog", { name: "Commands" });
  await expect(palette).toBeVisible();

  // Subsequence matching: `kbd` finds the keyboard sheet.
  await page.getByRole("combobox", { name: "Run a command" }).fill("keyb");
  await page.keyboard.press("Enter");
  await expect(palette).toBeHidden();
  await expect(page.getByRole("dialog", { name: "Keyboard" })).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "Keyboard" })).toBeHidden();
});

test("the keyboard sheet prints the bindings the map actually has", async ({ page }) => {
  await inScore(page);
  await page.keyboard.press("Shift+?");
  const sheet = page.getByRole("dialog", { name: "Keyboard" });
  await expect(sheet).toBeVisible();
  await expect(sheet.getByText("Next note")).toBeVisible();
  await expect(sheet.getByText("⌥→", { exact: true })).toBeVisible();
});

test("a bar is looped from the selection and marked in the margin", async ({ page }) => {
  await inScore(page);
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Shift+ArrowRight");
  await page.keyboard.press("l");

  await expect(page.locator(".overlay .loop").first()).toBeVisible();
  const region = await page.evaluate(() => window.__musaLoop);
  expect(region, "the loop the shell was asked for").not.toBeNull();

  await page.keyboard.press("l");
  await expect(page.locator(".overlay .loop")).toHaveCount(0);
});

test("space plays and stops without moving the selection", async ({ page }) => {
  await inScore(page);
  await page.keyboard.press("ArrowRight");
  const chosen = await where(page);

  await page.keyboard.press("Space");
  await expect(page.getByRole("button", { name: "Pause" })).toBeVisible();
  expect(await where(page)).toBe(chosen);

  await page.keyboard.press("Space");
  await expect(page.getByRole("button", { name: "Play" })).toBeVisible();
  expect(await where(page)).toBe(chosen);
});

test("the selection survives a re-engraving", async ({ page }) => {
  await inScore(page);
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  const chosen = await where(page);
  const halo = await page.locator(".overlay .selection").count();
  expect(halo).toBeGreaterThan(0);

  // A new score revision with the same events: the ids are unchanged, so the
  // selection is too (`02-engraving.md` §6).
  await toggleSource(page);
  await rewrite(page, `${await text(page)}\n`);
  await page.waitForTimeout(400);

  expect(await where(page)).toBe(chosen);
  // The same note, so the same halo: one box per notehead it is drawn as.
  await expect(page.locator(".overlay .selection")).toHaveCount(halo);
});
