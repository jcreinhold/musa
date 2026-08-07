/**
 * The application against a shell: opening, editing, breaking, and playing.
 *
 * The behaviour under test is `05-states.md` §4 — what happens while the
 * source is invalid — because it is the one that decides whether the app is
 * pleasant to edit in, and it is not visible in any Rust test.
 */

import { expect, test } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";

const LEAF = ".stage > div";

/**
 * What is engraved, by identity: every event on the page, in order. Verovio
 * mints fresh symbol-definition ids on each load and the leaf re-lays out
 * whenever the drawer changes height, so neither the markup nor the geometry
 * is the invariant — the music is. These ids come from the MEI
 * (`02-engraving.md` §5), so an unchanged list is an unchanged score.
 */
async function engravedEvents(page: import("@playwright/test").Page): Promise<string> {
  return page.evaluate(() =>
    [...document.querySelectorAll('.engraving g[id^="event-"]')]
      .map((element) => element.id)
      .join(" "),
  );
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ colorScheme: "light" });
});

test("a piece opens engraved, with its source behind the drawer", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  await expect(page.locator("h1")).toHaveText("Glass Mountain");
  await expect(page.getByRole("textbox", { name: "Source" })).toHaveCount(0);

  await page.getByRole("button", { name: /Source/ }).click();
  await expect(page.getByRole("textbox", { name: "Source" })).toHaveValue(/^piece /);
});

test("breaking the source keeps the score and says how far behind it is", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  // Opened first, so the leaf has already settled at its shorter height: the
  // drawer pushes the leaf, and that re-layout is not what is under test.
  await page.getByRole("button", { name: /Source/ }).click();
  await engraved(page);
  const before = await engravedEvents(page);
  const edge = await page.locator(LEAF).evaluate((leaf) => getComputedStyle(leaf).borderTopColor);

  const source = page.getByRole("textbox", { name: "Source" });
  await source.fill("piece \"Glass Mountain\" {");

  // The drawer opened itself the first time, and the message names the
  // revision on screen and counts the problems.
  await expect(page.getByRole("status")).toContainText(/Showing revision \d+/);
  await expect(page.getByRole("status")).toContainText(/1\s+problem/);
  expect(await engravedEvents(page)).toBe(before);
  expect(before).not.toBe("");

  const stale = await page.locator(LEAF).evaluate((leaf) => getComputedStyle(leaf).borderTopColor);
  expect(stale).not.toBe(edge);

  // Diagnostics are in the drawer, in the compiler's own words.
  await expect(page.locator(".diagnostics li")).toHaveCount(1);
});

test("fixing the source removes the message without announcing it", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  await page.getByRole("button", { name: /Source/ }).click();
  const source = page.getByRole("textbox", { name: "Source" });
  await source.fill("piece \"Glass Mountain\" {");
  await expect(page.getByRole("status")).toBeVisible();

  await source.fill("piece \"Glass Mountain\" {}");
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

  await expect(page.locator(".panes")).toHaveCount(0);
  await page.evaluate(() => window.__musaEmit("musa://command", "view.drawer"));
  await expect(page.locator(".panes")).toHaveCount(1);
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
