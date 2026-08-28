/**
 * Revising a group of notes with one musical command.
 *
 * The rules held to here are `03-interaction.md` §§1 and 4: the same shortcut
 * changes what is selected rather than writing after it, a command that
 * touches several notes states its consequence before it happens, and what
 * the composer accepts is the edit they read — once. What the accepted edit
 * does to the source is `musa-project`'s group-edit laws, not this file's.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";

/** Two authored whole notes in the upper strings. */
const FIRST = '.engraving .arriving [id="event-a"]';
const SECOND = '.engraving .arriving [id="event-b"]';
/** A note from the first `use sigh()`: transforming it is a question. */
const GENERATED = '.engraving .arriving [id="event-3"]';

/** The preview panel, which exists only while there is one. */
function preview(page: Page) {
  return page.getByRole("group", { name: "Transform the selection" });
}

/** Every transformation the interface has committed, in order. */
function committed(page: Page): Promise<Record<string, unknown>[]> {
  return page.evaluate(() => window.__musaGroupEdits);
}

async function clickNote(page: Page, selector: string, extend = false): Promise<void> {
  await page
    .locator(selector)
    .first()
    .click(extend ? { modifiers: ["Shift"] } : {});
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.goto("/");
  await engraved(page);
});

test("a duration key with a selection previews a transformation instead of writing one", async ({ page }) => {
  await clickNote(page, FIRST);
  await clickNote(page, SECOND, true);
  await page.keyboard.press("4");

  await expect(preview(page)).toBeVisible();
  await expect(preview(page)).toContainText("Write 1/4 on 2 notes");
  // Previewing changes nothing: the transaction is the second act.
  expect(await committed(page)).toHaveLength(0);
});

test("accepting it commits exactly the plan that was read, once", async ({ page }) => {
  await clickNote(page, FIRST);
  await clickNote(page, SECOND, true);
  await page.keyboard.press("4");
  await preview(page).getByRole("button", { name: "Accept" }).click();

  await expect.poll(() => page.evaluate(() => window.__musaGroupEdits.length)).toBe(1);
  expect((await committed(page))[0]).toMatchObject({ plan: 1 });
  // The panel goes with the plan it described.
  await expect(preview(page)).toHaveCount(0);
});

test("cancelling leaves the document alone", async ({ page }) => {
  await clickNote(page, FIRST);
  await page.keyboard.press("2");
  await expect(preview(page)).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(preview(page)).toHaveCount(0);
  expect(await committed(page)).toHaveLength(0);
});

test("a duration key with nothing selected does nothing at all", async ({ page }) => {
  await page.locator(".engraving").click({ position: { x: 4, y: 4 } });
  await page.keyboard.press("4");

  await expect(preview(page)).toHaveCount(0);
  expect(await committed(page)).toHaveLength(0);
});

test("the vertical keys move the whole selection", async ({ page }) => {
  await clickNote(page, FIRST);
  await clickNote(page, SECOND, true);
  await page.keyboard.press("Alt+ArrowUp");

  await expect(preview(page)).toContainText("Move 2 notes a step");
});

test("transposing asks for an interval beside the notes it would move", async ({ page }) => {
  await clickNote(page, FIRST);
  await page.keyboard.press("t");

  const field = page.getByRole("textbox", { name: "Interval" });
  await expect(field).toBeFocused();
  await field.fill("down m3");
  await field.press("Enter");

  await expect(preview(page)).toContainText("Transpose 1 notes by down m3");
});

test("a transformation of generated music says what else it changes", async ({ page }) => {
  await clickNote(page, GENERATED);
  await page.keyboard.press("Alt+ArrowUp");

  // The motif spells that note twice over, so the preview names the motif and
  // counts both occurrences before anything happens (`04-provenance.md` §4).
  await expect(preview(page)).toContainText("sigh");
  await expect(preview(page)).toContainText("2 occurrences");
  await expect(preview(page).getByRole("button", { name: "Just these occurrences" })).toBeVisible();
});

test("a rectangle dragged over the page collects notes across voices", async ({ page }) => {
  // The one selection that crosses staves (§1): the composer drew a box, and
  // which staff each note sits on is not part of what they said.
  const box = await page.locator(".engraving .arriving").first().boundingBox();
  if (!box) throw new Error("nothing is engraved");
  await page.mouse.move(box.x + 2, box.y + 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width - 2, box.y + box.height - 2, { steps: 8 });
  await page.mouse.up();

  await page.keyboard.press("4");
  await expect(preview(page)).toBeVisible();
  const said = (await preview(page).textContent()) ?? "";
  expect(said).toMatch(/Write 1\/4 on \d+ notes/);
  const notes = Number(/on (\d+) notes/.exec(said)?.[1] ?? "0");
  expect(notes).toBeGreaterThan(1);
});
