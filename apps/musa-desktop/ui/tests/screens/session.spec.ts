/**
 * The application against a shell: opening, editing, breaking, and playing.
 *
 * The behaviour under test is `05-states.md` §4 — what happens while the
 * source is invalid — because it is the one that decides whether the app is
 * pleasant to edit in, and it is not visible in any Rust test.
 */

import { expect, test } from "@playwright/test";

import { engraved } from "./engraved";
import { pane, source, toggleSource, rewrite } from "./source";
import { stubShell } from "./shell";

const LEAF = ".stage > div";

/**
 * What is engraved, by identity: every event on the page, in order. Verovio
 * mints fresh symbol-definition ids on each load and the leaf re-lays out
 * whenever the source column changes its width, so neither the markup nor the geometry
 * is the invariant — the music is. These ids come from the MEI
 * (`02-engraving.md` §5), so an unchanged set is an unchanged score.
 *
 * Deduplicated because a page mid-swap holds the outgoing engraving under the
 * incoming one for 90 ms, and both carry the same ids (§6).
 */
async function engravedEvents(page: import("@playwright/test").Page): Promise<string> {
  return page.evaluate(() =>
    [...new Set([...document.querySelectorAll('.engraving g[id^="event-"]')].map((element) => element.id))]
      .sort()
      .join(" "),
  );
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ colorScheme: "light" });
});

test("a piece opens engraved, with its source not yet shown", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  await expect(page.locator("h1")).toHaveText("Glass Mountain");
  await expect(page.getByRole("textbox", { name: "Source" })).toHaveCount(0);

  await toggleSource(page);
  await expect(page.getByRole("textbox", { name: "Source" })).toContainText("piece");
});

/**
 * Opening a second piece.
 *
 * A revision counts within one document, and the piece that arrives is at its
 * own revision 0 — lower than whatever the edited piece had reached. An
 * interface that compares the two numbers without asking which piece they
 * belong to concludes the new score is stale and shows the old one, which is
 * indistinguishable from Open being broken.
 */
test("opening a piece replaces the one that has been edited", async ({ page }) => {
  await page.goto("/");
  await engraved(page);
  await toggleSource(page);

  // Several edits, so the piece on screen is well past revision 0.
  await rewrite(page, 'piece "Glass Mountain" {}');
  await rewrite(page, 'piece "Glass Mountain" { }');
  await expect(page.locator("h1")).toHaveText("Glass Mountain");

  await page.evaluate(() => window.__musaEmit("musa://command", "file.open"));

  await expect(page.locator("h1")).toHaveText("Annotated");
  // The text is the new piece's, not the draft that was typed into the old one.
  await expect(source(page)).toContainText("The annotation layer");
});

test("opening a piece drops the selection the old one left behind", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  await page.locator('.engraving .arriving [id="event-c"]').first().click();
  await expect(page.locator(".overlay rect.selection")).toHaveCount(1);

  await page.evaluate(() => window.__musaEmit("musa://command", "file.open"));
  await expect(page.locator("h1")).toHaveText("Annotated");
  await expect(page.locator(".overlay rect.selection")).toHaveCount(0);
});

test("breaking the source keeps the score and says how far behind it is", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  // Opened first, so the leaf has already settled at its narrower width: the
  // column pushes the leaf, and that re-layout is not what is under test.
  await toggleSource(page);
  await engraved(page);
  const before = await engravedEvents(page);
  const edge = await page.locator(LEAF).evaluate((leaf) => getComputedStyle(leaf).borderTopColor);

  await rewrite(page, 'piece "Glass Mountain" {');

  // The column opened itself the first time, and the message names the
  // revision on screen and counts the problems.
  await expect(page.getByRole("status")).toContainText(/Showing revision \d+/);
  await expect(page.getByRole("status")).toContainText(/1\s+problem/);
  expect(await engravedEvents(page)).toBe(before);
  expect(before).not.toBe("");

  const stale = await page.locator(LEAF).evaluate((leaf) => getComputedStyle(leaf).borderTopColor);
  expect(stale).not.toBe(edge);

  // Diagnostics are in the source column, in the compiler's own words.
  await expect(page.locator(".diagnostics li")).toHaveCount(1);
});

/**
 * A diagnostic that knows its repair offers it, and applying it is an ordinary
 * edit — the source compiles again and the problem is gone.
 */
test("a diagnostic with one certain fix offers it, and applying it works", async ({ page }) => {
  await page.goto("/");
  await engraved(page);
  await toggleSource(page);
  await rewrite(page, 'piece "Glass Mountain" {');

  const problem = page.locator(".diagnostics li").first();
  // The place is a line and a column, never a byte offset.
  await expect(problem.locator(".where")).toHaveText(/^\d+:\d+$/);
  await expect(problem.locator(".label")).toHaveText("it goes here");

  // The core writes the title in lower case; the control is sentence case.
  await problem.getByRole("button", { name: "Add }" }).click();
  await expect(page.locator(".diagnostics li")).toHaveCount(0);
  await expect(page.getByRole("status")).toHaveCount(0);
});

test("fixing the source removes the message without announcing it", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  await toggleSource(page);
  await rewrite(page, 'piece "Glass Mountain" {');
  await expect(page.getByRole("status")).toBeVisible();

  await rewrite(page, 'piece "Glass Mountain" {}');
  await expect(page.getByRole("status")).toHaveCount(0);
});

test("play reaches the shell and the transport follows", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  const stop = page.getByRole("button", { name: "Stop" });
  await expect(stop).toBeDisabled();
  await page.getByRole("button", { name: "Play" }).click();
  await expect(stop).toBeEnabled();
});

test("a menu selection runs the same command the interface does", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  await expect(pane(page)).toHaveCount(0);
  await toggleSource(page);
  await expect(pane(page)).toHaveCount(1);
});

test("the position event moves the readout without a snapshot", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  await page.evaluate(() =>
    window.__musaEmit("musa://position", {
      playing: true,
      positionFrames: 44_100 * 3,
      totalFrames: 44_100 * 10,
      sampleRate: 44_100,
      loopRegion: null,
    }),
  );
  await expect(page.locator(".time")).toHaveText("0:03");
});

test("the view mode is a choice the piece keeps", async ({ page }) => {
  await page.goto("/");
  await engraved(page);
  await expect(page.locator(".stage.continuous")).toHaveCount(0);

  await page.getByRole("button", { name: "Continuous" }).click();
  await expect(page.locator(".stage.continuous")).toHaveCount(1);
  await expect(page.getByRole("button", { name: "Continuous" })).toHaveAttribute("aria-pressed", "true");

  // Remembered per piece (§4): reopening the same score reopens the view it
  // was left in, and a preference that did not survive a reload would not be
  // one.
  await page.reload();
  await engraved(page);
  await expect(page.locator(".stage.continuous")).toHaveCount(1);
});
