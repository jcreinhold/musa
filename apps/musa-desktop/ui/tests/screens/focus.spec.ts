/**
 * Linked reading: the two views share one focus.
 *
 * The question this answers is the one a musa score asks and a normal score
 * does not — *which line wrote this note, and which notes did this line
 * write* — and the answer has to arrive by looking, in both directions.
 *
 * `glass-mountain.musa` is the case: line 11 of the `motif` spells one note,
 * the motif is used twice, and so that one line spells two notes. Every
 * assertion here is against that fact.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { pane, source, toggleSource } from "./source";
import { stubShell } from "./shell";

/** The hairlines the focus draws on the page. */
function hairlines(page: Page) {
  return page.locator(".overlay line.focus");
}

/** The hairline the focus draws under the statement that spells. */
function spelling(page: Page) {
  return page.locator(".cm-musa-focus");
}

/** The line number the focus marks: the statement that placed the note. */
function placed(page: Page) {
  return page.locator(".cm-lineNumbers .cm-musa-focus-line");
}

/**
 * A note that came from the first `use sigh()` — the notehead itself, and a
 * quarter note's, because the centre of an event's box is blank paper beside
 * the stem and the centre of a half note's head is the hole in it.
 */
const GENERATED = '.engraving .arriving [id="event-3"] g.notehead';
/** The `b4/4` inside the motif: the line that spells it. */
const SPELLS = "b4/4";

/**
 * Point at a note.
 *
 * A page that re-engraves cross-fades, and a pointer that arrives during the
 * swap lands on the ink that is leaving — so the move is repeated until the
 * mark it asks for is there. That is a fact about driving the engraving from
 * a test, not about the focus, which is why it lives in one place.
 */
async function pointAt(page: Page, selector: string): Promise<void> {
  const note = page.locator(selector).first();
  await expect(note).toBeVisible();
  await expect(async () => {
    await page.mouse.move(0, 0);
    await note.hover({ force: true });
    await expect(hairlines(page)).not.toHaveCount(0, { timeout: 250 });
  }).toPass({ timeout: 5000 });
}

/** Focus the score pane, so the unmodified keys are the score's (§3). */
async function inScore(page: Page): Promise<void> {
  await page.getByRole("application", { name: "Engraved score" }).focus();
}

/** Where both views are scrolled to, so a hover can be shown not to move them. */
function scrolls(page: Page): Promise<number[]> {
  return page.evaluate(() =>
    [...document.querySelectorAll(".engraving, .cm-scroller")].map(
      (node) => node.scrollTop,
    ),
  );
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await engraved(page);
  await toggleSource(page);
  await expect(source(page)).toBeVisible();
});

test("pointing at a generated note marks both places that wrote it", async ({
  page,
}) => {
  await expect(spelling(page)).toHaveCount(0);
  await pointAt(page, GENERATED);

  // The statement inside the motif that spells it, and the `use` that placed
  // it. Two different questions, so two different marks.
  await expect(spelling(page)).toHaveText(SPELLS);
  await expect(placed(page)).toHaveText("23");
});

test("and marks the siblings that line spelled in the other occurrence", async ({
  page,
}) => {
  await pointAt(page, GENERATED);
  // The motif is used twice, so this line spelled two notes. Marking one of
  // them is the lie the editing choice later has to correct with a number.
  await expect(hairlines(page)).toHaveCount(2);
});

test("pointing at a line in the source marks the notes it produced", async ({
  page,
}) => {
  await expect(hairlines(page)).toHaveCount(0);
  await page.locator(".cm-line", { hasText: SPELLS }).first().hover();
  // The same answer from the other side: one line, two notes.
  await expect(hairlines(page)).toHaveCount(2);

  // A line in the text is already the answer to "where"; there is nothing to
  // mark back at it.
  await expect(spelling(page)).toHaveCount(0);
  await expect(placed(page)).toHaveCount(0);
});

test("pointing at a use statement marks the whole expansion", async ({
  page,
}) => {
  await page.locator(".cm-line", { hasText: "use sigh();" }).first().hover();
  // Five notes, one of them tied across a barline and therefore drawn twice.
  await expect(hairlines(page)).toHaveCount(6);
});

test("the focus follows the keyboard, so it is on for someone who never hovers", async ({
  page,
}) => {
  await inScore(page);
  await expect(spelling(page)).toHaveCount(0);
  await page.keyboard.press("ArrowRight");
  // The second note of the motif, and the line inside the motif that spells it.
  await expect(spelling(page)).toHaveText("rest/4");
  await expect(hairlines(page)).toHaveCount(2);

  // And it moves with the arrow keys rather than staying where it landed.
  await page.keyboard.press("ArrowRight");
  await expect(spelling(page)).toHaveText("c5/2");
});

test("the focus changes nothing else: not the selection, not the scroll", async ({
  page,
}) => {
  const before = await scrolls(page);
  await expect(page.locator(".overlay rect.selection")).toHaveCount(0);

  await pointAt(page, GENERATED);
  await expect(hairlines(page)).toHaveCount(2);

  // Attention is not a commitment: nothing is selected and neither view moved.
  await expect(page.locator(".overlay rect.selection")).toHaveCount(0);
  expect(await scrolls(page)).toEqual(before);

  await page.locator(".cm-line", { hasText: SPELLS }).first().hover();
  expect(await scrolls(page)).toEqual(before);
});

test("the focus does not survive leaving the leaf", async ({ page }) => {
  await pointAt(page, GENERATED);
  await expect(hairlines(page)).toHaveCount(2);

  // Off the page entirely, onto the margin. The last focus does not linger.
  await page.mouse.move(8, 8);
  await expect(hairlines(page)).toHaveCount(0);
  await expect(spelling(page)).toHaveCount(0);
});

test("a line that makes music on the page in view is ticked in the gutter", async ({
  page,
}) => {
  const ticked = page.locator(".cm-lineNumbers .cm-musa-sounds");
  // The five lines of the motif, the two `use` statements, and the eight
  // authored notes below them — and nothing in the studio block.
  await expect.poll(() => ticked.count()).toBeGreaterThan(0);
  const numbers = await ticked.allTextContents();
  expect(numbers).toContain("11");
  expect(numbers).toContain("23");
  expect(numbers).toContain("33");
  expect(numbers).not.toContain("53");
});

test("the origin row says how many notes the line spelled, before anyone hovers", async ({
  page,
}) => {
  const generated = page.locator(GENERATED).first();
  await expect(async () => {
    await generated.click({ force: true });
    await expect(page.locator(".inspector .kin")).toHaveText("2 notes", { timeout: 500 });
  }).toPass({ timeout: 5000 });

  // An authored note's line spelled exactly one, and a count of one is not a
  // fact worth printing.
  await page
    .locator('.engraving .arriving [id="event-a"]')
    .first()
    .click({ force: true });
  await expect(page.locator(".inspector .kin")).toHaveCount(0);
});

for (const theme of ["light", "dark"] as const) {
  // Set before the page opens rather than after: a theme change re-engraves,
  // and a pointer that has not moved since the swap is over the new ink
  // without ever having entered it.
  test.describe(`in ${theme}`, () => {
    test.use({ colorScheme: theme });

    test(`focus ${theme}`, async ({ page }) => {
      await expect(pane(page)).toBeVisible();
      await pointAt(page, GENERATED);
      await expect(hairlines(page)).toHaveCount(2);
      await expect(spelling(page)).toHaveText(SPELLS);
      await expect(page).toHaveScreenshot(`focus-${theme}.png`);
    });
  });
}
