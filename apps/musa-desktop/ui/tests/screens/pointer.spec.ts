/**
 * Pointer editing: a drag on a note is an edit to one token.
 *
 * The rule every test here holds to is `03-interaction.md` §2's: a gesture
 * replaces one token with one value, and it is measured from the note it
 * started on rather than from the page. So the assertions are about the
 * command the interface issued, which is the same command the keyboard
 * issues — what that command does to the source is `musa-project`'s editing
 * laws, not this file's.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";
import { source, text, toggleSource } from "./source";

/** An authored whole note in the upper strings: `a4 1;`. */
const AUTHORED = '.engraving .arriving [id="event-c"]';
/** The note below it, so a sideways drag has somewhere to go. */
const NEXT = '.engraving .arriving [id="event-d"]';
/** A note from the first `use sigh()`: editing it is a question, not an edit. */
const GENERATED = '.engraving .arriving [id="event-3"]';

/** Every edit the interface has asked the core for, in order. */
function edits(page: Page): Promise<Record<string, unknown>[]> {
  return page.evaluate(() => window.__musaEdits);
}

/**
 * One staff space, in screen pixels.
 *
 * Measured off the engraving itself — the gap between two of the five staff
 * lines — because every distance in this file is a musical one, and a musical
 * distance in pixels depends on the zoom the page happened to lay out at.
 */
async function staffSpace(page: Page): Promise<number> {
  return page.evaluate(() => {
    const staff = document.querySelector(".arriving g.staff");
    const lines = [...(staff?.querySelectorAll(":scope > path") ?? [])]
      .map((line) => line.getBoundingClientRect().y)
      .sort((a, b) => a - b);
    for (let index = 1; index < lines.length; index += 1) {
      const gap = (lines[index] ?? 0) - (lines[index - 1] ?? 0);
      if (gap > 0.5) return gap;
    }
    throw new Error("no staff lines on the page");
  });
}

/** The shapes a candidate draws, which is one per gesture; its label is not one. */
function ghost(page: Page) {
  return page.locator(".overlay .candidate:not(.label)");
}

/**
 * Where a note is on the screen.
 *
 * A page that re-engraves cross-fades, so the ink under the pointer during
 * the swap is the ink that is leaving. The box is re-read until it settles,
 * which is a fact about driving the engraving rather than about the gesture.
 */
async function boxOf(
  page: Page,
  selector: string,
): Promise<{ x: number; y: number; width: number; height: number }> {
  const note = page.locator(selector).first();
  let settled: { x: number; y: number; width: number; height: number } | null =
    null;
  await expect(async () => {
    await expect(note).toHaveCount(1, { timeout: 250 });
    const box = await note.boundingBox();
    expect(box?.width ?? 0).toBeGreaterThan(0);
    settled = box;
  }).toPass({ timeout: 5000 });
  if (!settled) throw new Error(`${selector} is not on the page`);
  return settled;
}

/** Press in the middle of a note, or — for a renotation — on its right edge. */
async function pressOn(
  page: Page,
  selector: string,
  edge = false,
): Promise<void> {
  const box = await boxOf(page, selector);
  const x = edge ? box.x + box.width - 1 : box.x + box.width / 2;
  await page.mouse.move(x, box.y + box.height / 2);
  await page.mouse.down();
}

/**
 * Press on a note and drag it, until the gesture is really under way.
 *
 * A press that lands while the page is re-engraving lands on the ink that is
 * leaving, and the gesture never starts. That is a fact about driving the
 * engraving from a test rather than about the gesture, so it is retried in
 * one place instead of being waited for in seven.
 */
async function dragOn(page: Page, selector: string, dx: number, dy: number): Promise<void> {
  await expect(async () => {
    await page.mouse.up();
    await pressOn(page, selector, dx !== 0 && dy === 0);
    await moveBy(page, dx, dy);
    await expect(ghost(page)).toHaveCount(1, { timeout: 500 });
  }).toPass({ timeout: 10_000 });
}

/** Move by a musical distance, in steps of a few pixels, as a hand does. */
async function moveBy(page: Page, dx: number, dy: number): Promise<void> {
  const at = await page.evaluate(() => ({
    x: window.__musaPointer.x,
    y: window.__musaPointer.y,
  }));
  await page.mouse.move(at.x + dx, at.y + dy, { steps: 6 });
}

/**
 * Where the pointer is, which Playwright does not report. Tracked in the page
 * so a drag can be described as "two steps up" rather than in absolutes.
 */
async function trackPointer(page: Page): Promise<void> {
  await page.addInitScript(() => {
    window.__musaPointer = { x: 0, y: 0 };
    addEventListener(
      "pointermove",
      (event: PointerEvent) => {
        window.__musaPointer = { x: event.clientX, y: event.clientY };
      },
      true,
    );
  });
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await trackPointer(page);
  await page.goto("/");
  await engraved(page);
});

test("a drag up two steps writes the note two steps up", async ({ page }) => {
  const space = await staffSpace(page);
  // Two diatonic steps is one staff space, whatever the zoom.
  await dragOn(page, AUTHORED, 0, -space);

  // Nothing is written until the pointer comes up: the candidate is drawn
  // over the unmoved engraving.
  expect(await edits(page)).toHaveLength(0);

  await page.mouse.up();
  await expect
    .poll(() => page.evaluate(() => window.__musaEdits.length))
    .toBe(1);
  expect((await edits(page))[0]).toMatchObject({
    kind: "changePitch",
    event: "event-c",
    pitch: "c5",
    mode: "editDefinition",
  });
  // And the candidate goes with the gesture.
  await expect(ghost(page)).toHaveCount(0);
});

test("a drag that comes back where it started writes nothing", async ({
  page,
}) => {
  const space = await staffSpace(page);
  await dragOn(page, AUTHORED, 0, -space);

  await moveBy(page, 0, space);
  await expect(ghost(page)).toHaveCount(0);
  await page.mouse.up();

  expect(await edits(page)).toHaveLength(0);
});

test("Esc mid-drag leaves the document exactly as it was", async ({ page }) => {
  await toggleSource(page);
  const before = await text(page);
  const space = await staffSpace(page);

  await dragOn(page, AUTHORED, 0, -space * 2);

  await page.keyboard.press("Escape");
  await expect(ghost(page)).toHaveCount(0);
  await page.mouse.up();

  expect(await edits(page)).toHaveLength(0);
  expect(await text(page)).toBe(before);
});

test("the source column stands the token the drag would write", async ({
  page,
}) => {
  await toggleSource(page);
  await expect(source(page)).toBeVisible();
  const before = await text(page);
  const space = await staffSpace(page);

  await dragOn(page, AUTHORED, 0, -space);

  // The token, standing where the token it would replace stands. It is drawn
  // over the text rather than typed into it, which is what makes the `Esc`
  // below free and the undo one step instead of one per pixel.
  await expect(page.locator(".cm-musa-candidate")).toHaveText("c5");
  expect(await edits(page)).toHaveLength(0);

  await page.keyboard.press("Escape");
  await page.mouse.up();
  expect(await text(page)).toBe(before);
});

test("a drag on generated music asks the same question a keystroke does", async ({
  page,
}) => {
  const space = await staffSpace(page);
  await dragOn(page, GENERATED, 0, -space);
  await page.mouse.up();

  const choice = page.getByRole("group", { name: "Editing generated music" });
  await expect(choice).toBeVisible();
  await expect(choice).toContainText("changes 2 occurrences, 2 notes");
  await expect(choice).toContainText("sigh()");
  expect(await edits(page)).toHaveLength(0);

  await choice.getByRole("button", { name: /Edit the motif/ }).click();
  await expect
    .poll(() => page.evaluate(() => window.__musaEdits.length))
    .toBe(1);
  expect((await edits(page))[0]).toMatchObject({
    kind: "changePitch",
    event: "event-3",
    pitch: "d5",
  });
});

test("a drag on the right edge renotates instead of respelling", async ({
  page,
}) => {
  const space = await staffSpace(page);
  // Two rungs shorter: a whole note becomes a half.
  await dragOn(page, AUTHORED, -space * 3, 0);
  await page.mouse.up();

  await expect
    .poll(() => page.evaluate(() => window.__musaEdits.length))
    .toBe(1);
  expect((await edits(page))[0]).toMatchObject({
    kind: "changeDuration",
    event: "event-c",
    duration: "1/2",
    mode: "editDefinition",
  });
});

test("a horizontal drag is still a range selection", async ({ page }) => {
  const from = await boxOf(page, AUTHORED);
  const to = await boxOf(page, NEXT);

  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(to.x + to.width / 2, from.y + from.height / 2, {
    steps: 8,
  });
  await page.mouse.up();

  await expect(page.locator(".overlay rect.selection")).toHaveCount(2);
  expect(await edits(page)).toHaveLength(0);
});

test("with entry armed, a click on an empty step writes a note there", async ({
  page,
}) => {
  await page.getByRole("button", { name: /^Notes/ }).click();
  const space = await staffSpace(page);
  const box = await boxOf(page, AUTHORED);

  // Two steps above the note beside it, on blank staff.
  await page.mouse.click(
    box.x + box.width * 2.5,
    box.y + box.height / 2 - space,
  );

  await expect
    .poll(() => page.evaluate(() => window.__musaEdits.length))
    .toBe(1);
  expect((await edits(page))[0]).toMatchObject({
    kind: "insertNote",
    note: { kind: "note", pitch: "c5", duration: "1/4" },
  });
});

test("a gesture in flight has no accessibility violations", async ({
  page,
}) => {
  const { default: AxeBuilder } = await import("@axe-core/playwright");
  const space = await staffSpace(page);
  await dragOn(page, AUTHORED, 0, -space);

  const { violations } = await new AxeBuilder({ page })
    .disableRules(["svg-img-alt"])
    .analyze();
  expect(violations.map((violation) => violation.id)).toEqual([]);
  await page.keyboard.press("Escape");
  await page.mouse.up();
});

declare global {
  interface Window {
    /** Where the pointer is, so a drag can be written as a musical distance. */
    __musaPointer: { x: number; y: number };
  }
}
