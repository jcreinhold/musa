/**
 * Re-render without disruption (`02-engraving.md` §6, and the second half of
 * budget B2).
 *
 * The two claims that make an editor usable are that the page a composer is
 * reading does not move under them, and that it never blinks. Both are
 * measured here rather than described: the anchor note's position across the
 * swap, and a frame-by-frame count of what was on the leaf while it happened.
 */

import { expect, test } from "@playwright/test";

import { engraved } from "./engraved";
import { toggleSource, rewrite } from "./source";
import { stubShell } from "./shell";

/** How far the reader's anchor may drift across a re-engraving (§6). */
const DRIFT_PX = 2;

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.setViewportSize({ width: 1440, height: 900 });
});

test("a re-engraving neither moves the page nor blinks", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  // The source column is opened first and allowed to settle: it changes the leaf's
  // height, so it re-lays the score out for a reason that has nothing to do
  // with the edit under test.
  await toggleSource(page);
  await page.waitForTimeout(500);

  const anchor = page.locator('.engraving g[id^="event-"]').first();
  const id = await anchor.getAttribute("id");
  expect(id, "an engraved event to anchor to").not.toBeNull();
  const before = await anchor.boundingBox();

  // Marked so the test can prove the page really was replaced. A stability
  // test that passed because nothing re-rendered would be worse than none.
  await page.evaluate(() =>
    document
      .querySelector(".engraving .ink > svg")
      ?.setAttribute("data-before", ""),
  );

  // Watch every frame from before the edit until well after the swap. The
  // count is of engraved events on the leaf: a white frame is a frame where
  // it fell to zero, and the cross-fade exists precisely to prevent one.
  await page.evaluate(() => {
    const counts: number[] = [];
    Object.defineProperty(window, "__musaFrames", {
      value: counts,
      configurable: true,
    });
    const tick = () => {
      counts.push(
        document.querySelectorAll('.engraving g[id^="event-"]').length,
      );
      window.requestAnimationFrame(tick);
    };
    window.requestAnimationFrame(tick);
  });

  await rewrite(page, `piece "Glass Mountain" { }`);
  await expect(page.locator(".notice")).toHaveCount(0);
  await page.waitForTimeout(500);

  await expect(page.locator(".engraving .ink > svg[data-before]")).toHaveCount(
    0,
  );

  const frames: number[] = await page.evaluate(() => [...window.__musaFrames]);
  expect(frames.length, "frames observed").toBeGreaterThan(5);
  expect(
    Math.min(...frames),
    "engraved events on the leaf at the emptiest frame",
  ).toBeGreaterThan(0);

  const after = await page.locator(`[id="${id}"]`).first().boundingBox();
  expect(after, "the anchor is still engraved").not.toBeNull();
  expect(Math.abs((after?.x ?? 0) - (before?.x ?? 0))).toBeLessThan(DRIFT_PX);
  expect(Math.abs((after?.y ?? 0) - (before?.y ?? 0))).toBeLessThan(DRIFT_PX);
});

declare global {
  interface Window {
    __musaFrames: number[];
  }
}
