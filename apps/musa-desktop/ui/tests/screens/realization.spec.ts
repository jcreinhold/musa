/**
 * The realization, in the page (prompt 76).
 *
 * An open work is one the composer cannot read off the source alone: the
 * source says `repeat 2 to 6`, and how many times it actually ran is a fact
 * about *this reading*. Prompt 66 shipped the reading and conceded that the
 * interface said nothing about it. These are the three things it now says —
 * what is free, what was chosen, and how to change it — and the one thing it
 * must not say, which is anything at all about a determinate piece.
 *
 * `loop-lengths.musa` is the piece under test: one question, asked once and
 * written in three voices, so a decision that came apart per voice would show
 * up here as three rows rather than one.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";

const sheet = (page: Page) => page.getByRole("dialog", { name: "Settings" });

/** The performance in force, as the field states it. */
const performance = (page: Page) => sheet(page).getByRole("spinbutton", { name: "Performance" });

const inspector = (page: Page) => page.locator(".inspector");

/** The events the engraver has actually put on the page right now. */
function rendered(page: Page): Promise<string[]> {
  return page.evaluate(() =>
    [...document.querySelectorAll('.engraving .arriving g[id^="event-"]')].map(
      (element) => element.id,
    ),
  );
}

/**
 * Wait until what is on the page is the reading in force.
 *
 * A new performance is a new score to lay out, and the engraver crosses the
 * old page and the new one over (`02-engraving.md` §6): for 120 ms there are
 * two. Clicking before that settles clicks the score being replaced, which is
 * a race and not a test.
 */
async function settled(page: Page): Promise<void> {
  await page.waitForTimeout(250);
}

/** The same, for a reading whose music is genuinely different. */
async function reengraved(page: Page, before: string[]): Promise<void> {
  await settled(page);
  await expect.poll(() => rendered(page)).not.toEqual(before);
}

/** Open the settings sheet the way the menu and the palette both do. */
async function settings(page: Page): Promise<void> {
  await page.evaluate(() => window.__musaEmit("musa://command", "settings.open"));
  await expect(sheet(page)).toBeVisible();
}

/** Pick a note that was played under the open repeat, and one that was not. */
const UNDER = '.engraving .arriving [id="event-10"] use';
const OUTSIDE = '.engraving .arriving [id="event-20"] use';

test.describe("an open work", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await stubShell(page, "open-form");
    await page.goto("/");
    await engraved(page);
  });

  test("settings names the performance, and gives a way to another one", async ({ page }) => {
    await settings(page);
    await expect(performance(page)).toHaveValue("4");

    // "New performance", not "reseed": what changes is which reading you are
    // hearing, and a composer thinks in performances.
    await sheet(page).getByRole("button", { name: "New performance" }).click();
    await expect(performance(page)).toHaveValue("5");
  });

  test("a new performance is a different reading of the same source", async ({ page }) => {
    await page.locator(UNDER).click({ force: true });
    await expect(inspector(page)).toContainText("2 passes");

    const before = await rendered(page);
    await settings(page);
    await sheet(page).getByRole("button", { name: "New performance" }).click();
    await page.keyboard.press("Escape");
    await reengraved(page, before);

    // The selection is let go of, and this is why: an event id is a position
    // in the score, so a piece read again renumbers them and the note that
    // was selected is not the note that id now names. Picking again is the
    // honest gesture, and it is what a composer does anyway.
    await expect(inspector(page)).not.toContainText("2 passes");
    await page.locator(UNDER).click({ force: true });
    await expect(inspector(page)).toContainText("6 passes");
  });

  test("a performance you liked is a number you can type back", async ({ page }) => {
    await settings(page);
    await performance(page).fill("8");
    await performance(page).blur();
    await expect(performance(page)).toHaveValue("8");
  });

  test("the inspector says what the note was played under", async ({ page }) => {
    await page.locator(UNDER).click({ force: true });

    // Both halves are the core's own words: what was left open, and what this
    // reading decided. The frontend spells neither (`03-interaction.md` §7).
    await expect(inspector(page)).toContainText("Decision");
    await expect(inspector(page)).toContainText("the first choice");
    await expect(inspector(page)).toContainText("2 passes");
  });

  /**
   * The question asked once and written in three voices is one decision. A
   * reading that let the kick run twice and the hats six times would not be a
   * reading of this piece, and the fixture is the piece that would catch it.
   */
  test("one question is one row, however many voices write it", async ({ page }) => {
    await page.locator(UNDER).click({ force: true });
    await expect(inspector(page).getByText("the first choice")).toHaveCount(1);
  });

  test("a note no open construct covers has no decision to show", async ({ page }) => {
    await page.locator(OUTSIDE).click({ force: true });
    await expect(inspector(page)).not.toContainText("Decision");
  });

  test("a kept decision holds across a new performance", async ({ page }) => {
    await page.locator(UNDER).click({ force: true });
    await inspector(page).getByRole("button", { name: "keep this one" }).click();

    // The offer becomes a state, in the same word and the same place.
    await expect(inspector(page).getByRole("button", { name: "kept" })).toBeVisible();

    await settings(page);
    await sheet(page).getByRole("button", { name: "New performance" }).click();
    await page.keyboard.press("Escape");

    // The number moved and the music did not, which is what keeping one is
    // for. (That a kept decision survives a real recompile is `musa-project`'s
    // law; what is asserted here is that the page says so.)
    await settled(page);
    await page.locator(UNDER).click({ force: true });
    await expect(inspector(page)).toContainText("2 passes");
    await expect(inspector(page).getByRole("button", { name: "kept" })).toBeVisible();
  });
});

/**
 * The requirement that is easiest to fail by accident: everything above is
 * conditional on the piece having asked something, and a determinate score
 * must not gain a control, a row, or a word.
 */
test.describe("a determinate piece", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await stubShell(page);
    await page.goto("/");
    await engraved(page);
  });

  test("says nothing about performances", async ({ page }) => {
    await settings(page);
    await expect(performance(page)).toHaveCount(0);
    await expect(sheet(page)).not.toContainText("New performance");
  });

  test("and nothing about decisions", async ({ page }) => {
    await page.locator('.engraving [id="event-4"] use').click({ force: true });
    await expect(inspector(page)).toContainText("Origin");
    await expect(inspector(page)).not.toContainText("Decision");
  });
});
