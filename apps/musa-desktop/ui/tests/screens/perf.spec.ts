/**
 * The budgets this prompt owns, asserted as measurements
 * (`06-performance.md` §1–§2): B1, B2, B6, B7, B8, B10, B11, B12.
 *
 * The harness drives the Vite dev server with the stubbed shell, which §2
 * sanctions where a window is impractical. That means these numbers are the
 * *interface's* share of each budget: the debounce, the worker, the paint.
 * The core's share of B1 — compiling the source — is not in them, and the
 * budget is only fully proved on a platform where a real window can be
 * automated. What the interface must never do is add to it, and that is what
 * a regression here would show.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { toggleSource, rewrite } from "./source";
import { stubShell } from "./shell";

/** The debounce the budget is stated relative to (`06-performance.md` §1). */
const SETTLE_MS = 180;

const TRIALS = 20;

/**
 * Report a budget's measured p95 where a *passing* run can be read.
 *
 * An assertion message is only printed when the assertion fails, which makes
 * a green suite say "within budget" and nothing else. §2 asks for
 * measurements rather than verdicts — "a regression that misses B2 by 40 ms
 * says so" — and the same is true of headroom: a prompt deciding whether to
 * build a faster path needs to know it is at 30% of the budget or at 95%, and
 * should not have to break a test to find out. One line per budget, on
 * stdout, which every reporter carries.
 */
function record(budget: string, measured: number, unit = "ms"): number {
  process.stdout.write(`  ${budget}: p95 ${Math.round(measured)} ${unit}\n`);
  return measured;
}

function p95(samples: number[]): number {
  const sorted = [...samples].sort((a, b) => a - b);
  const at = Math.min(Math.ceil(sorted.length * 0.95) - 1, sorted.length - 1);
  return sorted[Math.max(at, 0)] ?? Number.NaN;
}

/** A mark's time since navigation started, or `NaN` if it never happened. */
async function at(page: Page, moment: string): Promise<number> {
  return page.evaluate(
    (name) => performance.getEntriesByName(`musa:${name}`, "mark")[0]?.startTime ?? Number.NaN,
    moment,
  );
}

/**
 * Wait until the interface has stopped putting ink on the leaf.
 *
 * `musa:score` is marked by every page that arrives, which includes the
 * neighbour pages the observer renders in the background (`02-engraving.md`
 * §7) — so "a score mark exists" is not "the thing I just did has finished".
 * A trial that starts before the previous one is quiet inherits its work and
 * measures two gestures as one; a trial that ends on the *previous* trial's
 * background page measures none. Both were happening, in strict alternation
 * (see B8 below).
 *
 * Settling on silence rather than on a count keeps that out of every trial
 * without the test needing to know how many pages a layout will render.
 */
async function quiet(page: Page): Promise<void> {
  await page.waitForFunction(async () => {
    const drawn = (): number => performance.getEntriesByName("musa:score", "mark").length;
    const before = drawn();
    await new Promise((settle) => setTimeout(settle, 150));
    return drawn() === before;
  });
}

/**
 * The first ink that is this gesture's: the earliest `musa:score` mark after
 * the mark the gesture itself left. Marks are cleared before each trial, so
 * `from` is unambiguous; what is not unambiguous without this is whether the
 * score mark being read came before the gesture or after it.
 */
async function after(page: Page, from: string): Promise<number> {
  return page.evaluate((name) => {
    const gesture = performance.getEntriesByName(`musa:${name}`, "mark")[0]?.startTime;
    if (gesture === undefined) return Number.NaN;
    const ink = performance
      .getEntriesByName("musa:score", "mark")
      .find((mark) => mark.startTime > gesture);
    return ink === undefined ? Number.NaN : ink.startTime - gesture;
  }, from);
}

test.describe("launch", () => {
  test("B6: the shell paints before the score, within 400 ms", async ({ page }) => {
    const shell: number[] = [];
    await stubShell(page);
    for (let trial = 0; trial < TRIALS; trial += 1) {
      await page.goto("/?perf=1");
      await page.waitForFunction(
        () => performance.getEntriesByName("musa:shell", "mark").length > 0,
      );
      shell.push(await at(page, "shell"));
    }
    expect(record("B6", p95(shell)), `shell painted at p95 ${Math.round(p95(shell))} ms`).toBeLessThanOrEqual(
      400,
    );
  });

  test("B7: the score is on the leaf within 1.5 s", async ({ page }) => {
    const score: number[] = [];
    await stubShell(page);
    for (let trial = 0; trial < TRIALS; trial += 1) {
      await page.goto("/?perf=1");
      await engraved(page);
      score.push(await at(page, "score"));
    }
    expect(record("B7", p95(score)), `score painted at p95 ${Math.round(p95(score))} ms`).toBeLessThanOrEqual(
      1500,
    );
  });

  test("B6 before B7: the frame never waits on the engraver", async ({ page }) => {
    await stubShell(page);
    await page.goto("/?perf=1");
    await engraved(page);
    expect(await at(page, "shell")).toBeLessThan(await at(page, "score"));
  });
});

test("B1: a keystroke reaches diagnostics within 120 ms of the debounce", async ({ page }) => {
  await stubShell(page);
  await page.goto("/?perf=1");
  await engraved(page);
  await toggleSource(page);

  const samples: number[] = [];
  for (let trial = 0; trial < TRIALS; trial += 1) {
    await page.evaluate(() => performance.clearMarks());
    // Every trial alternates between two sources so that each one is a real
    // change; setting the text the document already has compiles nothing.
    await rewrite(page, trial % 2 === 0 ? `piece "A" {}` : `piece "B" {}`);
    await page.waitForFunction(
      () => performance.getEntriesByName("musa:snapshot", "mark").length > 0,
    );
    samples.push((await at(page, "snapshot")) - (await at(page, "edit")) - SETTLE_MS);
  }
  expect(
    record("B1", p95(samples)),
    `diagnostics at p95 ${Math.round(p95(samples))} ms after settling`,
  ).toBeLessThanOrEqual(120);
});

/**
 * Selection: the most frequent action in the application, and the one budget
 * that forbids a round trip outright (`06-performance.md` §3.2).
 */
test.describe("selection", () => {
  /**
   * Distinct events, in engraved order. A tied note is drawn as several
   * elements with the same id, so clicking "the next element" is not
   * necessarily clicking a different note — and a trial that re-selects what
   * is already selected measures nothing.
   */
  async function distinctNotes(page: Page): Promise<string[]> {
    return page.evaluate(() => [
      ...new Set(
        [...document.querySelectorAll('.engraving g[id^="event-"]')].map((element) =>
          element.id.replace(/-t\d+$/, ""),
        ),
      ),
    ]);
  }

  test("B3: the halo is drawn in the frame the click happened in", async ({ page }) => {
    await stubShell(page);
    await page.goto("/?perf=1");
    await engraved(page);
    const notes = await distinctNotes(page);

    const samples: number[] = [];
    for (let trial = 0; trial < TRIALS; trial += 1) {
      await page.evaluate(() => performance.clearMarks());
      // Two different notes, alternately: re-selecting what is already
      // selected is not an interaction, and would measure nothing.
      await page
        .locator(`[id="${(trial % 2 === 0 ? notes[1] : notes[2]) ?? ""}"]`)
        .first()
        .click({ force: true });
      await page.waitForFunction(
        () => performance.getEntriesByName("musa:halo", "mark").length > 0,
      );
      samples.push((await at(page, "halo")) - (await at(page, "select")));
    }
    expect(
      record("B3", p95(samples)),
      `halo drawn at p95 ${Math.round(p95(samples))} ms after the click`,
    ).toBeLessThanOrEqual(16);
  });

  test("B4: the inspector is populated within 100 ms", async ({ page }) => {
    await stubShell(page);
    await page.goto("/?perf=1");
    await engraved(page);
    const notes = await distinctNotes(page);

    const samples: number[] = [];
    for (let trial = 0; trial < TRIALS; trial += 1) {
      await page.evaluate(() => performance.clearMarks());
      // Two different notes, alternately: re-selecting what is already
      // selected is not an interaction, and would measure nothing.
      await page
        .locator(`[id="${(trial % 2 === 0 ? notes[1] : notes[2]) ?? ""}"]`)
        .first()
        .click({ force: true });
      await page.waitForFunction(
        () => performance.getEntriesByName("musa:inspector", "mark").length > 0,
      );
      samples.push((await at(page, "inspector")) - (await at(page, "select")));
    }
    expect(
      record("B4", p95(samples)),
      `inspector populated at p95 ${Math.round(p95(samples))} ms`,
    ).toBeLessThanOrEqual(100);
  });

  /**
   * B9, the budget itself: Origin view enters in 120 ms and reflows nothing.
   *
   * The clock runs from the key going down to the frame the brackets are
   * measured in — the last thing the lens owes. The reflow half of the budget
   * is asserted in `origin.spec.ts`, by measuring the notes.
   */
  test("B9: Origin view opens within 120 ms of the key", async ({ page }) => {
    await stubShell(page);
    await page.goto("/?perf=1");
    await engraved(page);
    await page.getByRole("application", { name: "Engraved score" }).focus();

    const samples: number[] = [];
    for (let trial = 0; trial < TRIALS; trial += 1) {
      await page.evaluate(() => performance.clearMarks());
      await page.keyboard.down("o");
      await page.waitForFunction(
        () => performance.getEntriesByName("musa:halo", "mark").length > 0,
      );
      samples.push((await at(page, "halo")) - (await at(page, "lens")));
      await page.keyboard.up("o");
    }
    expect(
      record("B9", p95(samples)),
      `Origin view opened at p95 ${Math.round(p95(samples))} ms`,
    ).toBeLessThanOrEqual(120);
  });

  /**
   * The same rule for the Origin row's segments: following one is an ink
   * change too, and must cost like one.
   */
  test("B9: following an origin segment costs no more than an ink change", async ({ page }) => {
    await stubShell(page);
    await page.goto("/?perf=1");
    await engraved(page);
    await page.locator('.engraving g[id^="event-"]').first().click({ force: true });
    const segment = page.locator(".inspector button.segment").first();
    await expect(segment).toBeVisible();

    const samples: number[] = [];
    for (let trial = 0; trial < TRIALS; trial += 1) {
      await page.evaluate(() => performance.clearMarks());
      await segment.click();
      await page.waitForFunction(
        () => performance.getEntriesByName("musa:halo", "mark").length > 0,
      );
      samples.push((await at(page, "halo")) - (await at(page, "origin")));
    }
    expect(
      record("B9-follow", p95(samples)),
      `origin followed at p95 ${Math.round(p95(samples))} ms`,
    ).toBeLessThanOrEqual(120);
  });
});

/**
 * The large case: 100 bars in four parts (`tests/fixtures/large-score.musa`).
 * This is the workload B2 and B8 are stated on, because a budget met only on
 * a sixteen-bar sketch is not a budget.
 */
test.describe("the large score", () => {
  // A 100-bar layout is real work, and each trial does it twenty times.
  test.slow();

  test("B2: an edit is re-engraved within 400 ms of the keystroke", async ({ page }) => {
    await stubShell(page, "large-score");
    await page.goto("/?perf=1");
    await engraved(page);
    await toggleSource(page);
    await quiet(page);

    const samples: number[] = [];
    // B2 split where a fix would have to land: what the round trip costs, and
    // what the engraver costs. Prompt 50 needed this to decide whether a
    // faster semantic core could move B2 at all — it cannot, and a number is
    // the only way to know that without guessing.
    const round: number[] = [];
    const engrave: number[] = [];
    for (let trial = 0; trial < TRIALS; trial += 1) {
      await page.evaluate(() => performance.clearMarks());
      await rewrite(page, trial % 2 === 0 ? `piece "A" {}` : `piece "B" {}`);
      // Wait for the ink this keystroke asked for, not for ink. A background
      // page left over from the trial before satisfies "a score mark exists"
      // without satisfying the thing `after` reads, and then the sample is
      // `NaN` and the whole run fails on a number nobody measured. B8 waits
      // this way already; B2 did not, and failed roughly one full run in two.
      await page.waitForFunction(() => {
        const gesture = performance.getEntriesByName("musa:edit", "mark")[0];
        return (
          gesture !== undefined &&
          performance
            .getEntriesByName("musa:score", "mark")
            .some((mark) => mark.startTime > gesture.startTime)
        );
      });
      // The 180 ms debounce is inside this number, as the budget states it:
      // what the composer waits is from the keystroke, not from the compile.
      // The ink read is the ink that came *after* the keystroke, for the
      // reason `after` gives: the first two trials of this loop used to land
      // on the opening layout's background pages and report 79 ms and −26 ms.
      const [edit, snapshot] = [await at(page, "edit"), await at(page, "snapshot")];
      const drawn = await after(page, "edit");
      samples.push(drawn);
      round.push(snapshot - edit - SETTLE_MS);
      engrave.push(edit + drawn - snapshot);
      await quiet(page);
    }
    expect(Math.min(...samples)).toBeGreaterThan(SETTLE_MS);
    expect(
      record("B2", p95(samples)),
      `re-engraved at p95 ${Math.round(p95(samples))} ms after the keystroke`,
    ).toBeLessThanOrEqual(400);
    record("B2 round trip", p95(round));
    record("B2 engraving", p95(engrave));
    expect(p95(samples)).toBeGreaterThanOrEqual(SETTLE_MS);
  });

  /**
   * One step, and one step only.
   *
   * This trial loop settles before it starts and reads the ink that came
   * *after* the click, because for a long time it did neither and the number
   * it produced was not a zoom step. The samples said so plainly, in a
   * four-trial cycle that repeated all the way down a run:
   *
   * ```
   * 194  -19  288  117  180  -20  287  117  181  -18  281  114  …
   * ```
   *
   * The negative trials ended on a background neighbour page left over from
   * the trial before and measured nothing at all; the trials after them
   * inherited the layout those had walked away from and measured two zoom
   * steps as one. A quarter of every run was double-counted, and p95 — by
   * construction — reported one of the doubles. Isolated, the same build
   * measures 117–194 ms, and that is the number this asserts.
   */
  test("B8: a zoom step is re-laid out within 250 ms", async ({ page }) => {
    await stubShell(page, "large-score");
    await page.goto("/?perf=1");
    await engraved(page);
    await quiet(page);

    const samples: number[] = [];
    for (let trial = 0; trial < TRIALS; trial += 1) {
      await page.evaluate(() => performance.clearMarks());
      // In and out alternately, so no trial runs off the end of the ladder
      // and measures a step that never happened.
      await page.getByRole("button", { name: trial % 2 === 0 ? "Zoom in" : "Zoom out" }).click();
      await page.waitForFunction(() => {
        const zoom = performance.getEntriesByName("musa:zoom", "mark")[0];
        return (
          zoom !== undefined &&
          performance
            .getEntriesByName("musa:score", "mark")
            .some((mark) => mark.startTime > zoom.startTime)
        );
      });
      samples.push(await after(page, "zoom"));
      // The neighbour pages this step set going are not the next step's
      // problem: waiting for them here is what keeps each sample one gesture.
      await quiet(page);
    }
    expect(
      record("B8", p95(samples)),
      `re-laid out at p95 ${Math.round(p95(samples))} ms after the step`,
    ).toBeLessThanOrEqual(250);
    // A step that measured nothing is not a fast step. Guarding the floor is
    // what stops this loop from quietly going back to timing stale ink.
    expect(Math.min(...samples)).toBeGreaterThan(0);
  });
});

test("B10: nothing is scheduled while the transport is stopped", async ({ page }) => {
  await stubShell(page);
  // Counted from the page's own side, because "no timers" is a claim about
  // the code, not about a CPU sample taken through an automation harness.
  await page.addInitScript(() => {
    const counts = { frames: 0, intervals: 0, timeouts: 0 };
    Object.defineProperty(window, "__musaSchedules", { value: counts });
    const frame = window.requestAnimationFrame.bind(window);
    const interval = window.setInterval.bind(window);
    const timeout = window.setTimeout.bind(window);
    window.requestAnimationFrame = (callback) => {
      counts.frames += 1;
      return frame(callback);
    };
    window.setInterval = ((...args: Parameters<typeof interval>) => {
      counts.intervals += 1;
      return interval(...args);
    }) as typeof window.setInterval;
    window.setTimeout = ((...args: Parameters<typeof timeout>) => {
      counts.timeouts += 1;
      return timeout(...args);
    }) as typeof window.setTimeout;
  });
  await page.goto("/");
  await engraved(page);

  const before = await page.evaluate(() => ({ ...window.__musaSchedules }));
  await page.waitForTimeout(1000);
  const after = await page.evaluate(() => ({ ...window.__musaSchedules }));

  expect(after.frames - before.frames, "animation frames requested while idle").toBe(0);
  expect(after.intervals - before.intervals, "intervals started while idle").toBe(0);
  expect(after.timeouts - before.timeouts, "timeouts started while idle").toBe(0);
});

/**
 * Reading an open work again, measured on its own (prompt 76).
 *
 * Deliberately not folded into B2. B2 is what an *edit* costs, and its
 * 400 ms includes the 180 ms the interface spends waiting for typing to
 * settle. A new performance is a click: nothing is being typed, so there is
 * nothing to wait for, and rolling it into B2 would hide a redraw that had
 * become slow behind a debounce it never pays. What is measured is the
 * redraw — from the snapshot arriving to the ink — because that is the part
 * this control adds, and the part a regression would land in.
 */
test("B11: a new reading is on the leaf within 250 ms of the snapshot", async ({
  page,
}) => {
  await stubShell(page, "open-form");
  await page.goto("/?perf=1");
  await engraved(page);
  await quiet(page);

  const sheet = page.getByRole("dialog", { name: "Settings" });
  await page.evaluate(() => window.__musaEmit("musa://command", "settings.open"));
  await expect(sheet).toBeVisible();
  const again = sheet.getByRole("button", { name: "New performance" });

  const samples: number[] = [];
  for (let trial = 0; trial < TRIALS; trial += 1) {
    await page.evaluate(() => performance.clearMarks());
    await again.click();
    await page.waitForFunction(() => {
      const arrived = performance.getEntriesByName("musa:snapshot", "mark")[0];
      return (
        arrived !== undefined &&
        performance
          .getEntriesByName("musa:score", "mark")
          .some((mark) => mark.startTime > arrived.startTime)
      );
    });
    samples.push(await after(page, "snapshot"));
    await quiet(page);
  }

  expect(
    record("B11", p95(samples)),
    `a new reading was drawn at p95 ${Math.round(p95(samples))} ms after its snapshot`,
  ).toBeLessThanOrEqual(250);
});

/**
 * Turning to another piece of the volume (prompt 85).
 *
 * Measured apart from B11 because that is a *re-reading of one score* and this
 * is a different score: a new title, a new part list, a new outline, a whole
 * page laid out from a different MEI. A piece opened for the first time pays
 * B7's cold number by construction — a file has to be read and compiled — and
 * is not asserted here; what this budget covers is turning *back*, where the
 * session is already in memory and nothing should touch the disk.
 *
 * The gesture is the margin's running order rather than the contents page,
 * because that list stays on screen either side of the turn, which is what
 * lets "the previous page is visible throughout" be checked at all.
 */
test("B12: a piece already opened is on the leaf within 400 ms", async ({ page }) => {
  await stubShell(page, "glass-mountain", "annotated", true);
  await page.goto("/?perf=1");
  await engraved(page);
  await quiet(page);

  const rows = page.getByRole("navigation", { name: "Contents" }).getByRole("button");
  // Open the second piece once, so both are in hand and every trial below is
  // a turn back rather than a first opening.
  await rows.nth(1).click();
  await engraved(page);
  await quiet(page);

  const samples: number[] = [];
  for (let trial = 0; trial < TRIALS; trial += 1) {
    await page.evaluate(() => performance.clearMarks());
    // Alternately, because turning to the piece already in hand turns nothing.
    await rows.nth(trial % 2).click();
    // Read before waiting, which is the only moment the claim is checkable:
    // the page turned away from is still on the leaf while the next one is
    // being laid out, so there is no frame with nothing on it.
    expect(await page.locator(".engraving svg").count()).toBeGreaterThan(0);
    await page.waitForFunction(() => {
      const arrived = performance.getEntriesByName("musa:snapshot", "mark")[0];
      return (
        arrived !== undefined &&
        performance
          .getEntriesByName("musa:score", "mark")
          .some((mark) => mark.startTime > arrived.startTime)
      );
    });
    samples.push(await after(page, "snapshot"));
    await quiet(page);
  }

  expect(
    record("B12", p95(samples)),
    `the piece turned to was drawn at p95 ${Math.round(p95(samples))} ms after its snapshot`,
  ).toBeLessThanOrEqual(400);
  // A turn that measured nothing is not a fast turn.
  expect(Math.min(...samples)).toBeGreaterThan(0);
});

declare global {
  interface Window {
    __musaSchedules: { frames: number; intervals: number; timeouts: number };
  }
}
