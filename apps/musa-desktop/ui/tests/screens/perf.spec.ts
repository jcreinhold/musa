/**
 * The budgets this prompt owns, asserted as measurements
 * (`06-performance.md` §1–§2): B1, B2, B6, B7, B8, B10.
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
      await page.waitForFunction(
        () => performance.getEntriesByName("musa:score", "mark").length > 0,
      );
      // The 180 ms debounce is inside this number, as the budget states it:
      // what the composer waits is from the keystroke, not from the compile.
      const [edit, snapshot, score] = [
        await at(page, "edit"),
        await at(page, "snapshot"),
        await at(page, "score"),
      ];
      samples.push(score - edit);
      round.push(snapshot - edit - SETTLE_MS);
      engrave.push(score - snapshot);
    }
    expect(
      record("B2", p95(samples)),
      `re-engraved at p95 ${Math.round(p95(samples))} ms after the keystroke`,
    ).toBeLessThanOrEqual(400);
    record("B2 round trip", p95(round));
    record("B2 engraving", p95(engrave));
    expect(p95(samples)).toBeGreaterThanOrEqual(SETTLE_MS);
  });

  test("B8: a zoom step is re-laid out within 250 ms", async ({ page }) => {
    await stubShell(page, "large-score");
    await page.goto("/?perf=1");
    await engraved(page);

    const samples: number[] = [];
    for (let trial = 0; trial < TRIALS; trial += 1) {
      await page.evaluate(() => performance.clearMarks());
      // In and out alternately, so no trial runs off the end of the ladder
      // and measures a step that never happened.
      await page.getByRole("button", { name: trial % 2 === 0 ? "Zoom in" : "Zoom out" }).click();
      await page.waitForFunction(
        () => performance.getEntriesByName("musa:score", "mark").length > 0,
      );
      samples.push((await at(page, "score")) - (await at(page, "zoom")));
    }
    expect(
      record("B8", p95(samples)),
      `re-laid out at p95 ${Math.round(p95(samples))} ms after the step`,
    ).toBeLessThanOrEqual(250);
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

declare global {
  interface Window {
    __musaSchedules: { frames: number; intervals: number; timeouts: number };
  }
}
