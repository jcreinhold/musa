/**
 * The budgets this prompt owns, asserted as measurements
 * (`06-performance.md` §1–§2): B1, B6, B7, B10.
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
import { stubShell } from "./shell";

/** The debounce the budget is stated relative to (`06-performance.md` §1). */
const SETTLE_MS = 180;

const TRIALS = 20;

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
    expect(p95(shell), `shell painted at p95 ${Math.round(p95(shell))} ms`).toBeLessThanOrEqual(400);
  });

  test("B7: the score is on the leaf within 1.5 s", async ({ page }) => {
    const score: number[] = [];
    await stubShell(page);
    for (let trial = 0; trial < TRIALS; trial += 1) {
      await page.goto("/?perf=1");
      await engraved(page);
      score.push(await at(page, "score"));
    }
    expect(p95(score), `score painted at p95 ${Math.round(p95(score))} ms`).toBeLessThanOrEqual(
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
  await page.getByRole("button", { name: /Source/ }).click();
  const source = page.getByRole("textbox", { name: "Source" });

  const samples: number[] = [];
  for (let trial = 0; trial < TRIALS; trial += 1) {
    await page.evaluate(() => performance.clearMarks());
    // Every trial alternates between two sources so that each one is a real
    // change; setting the text the document already has compiles nothing.
    await source.fill(trial % 2 === 0 ? `piece "A" {}` : `piece "B" {}`);
    await page.waitForFunction(
      () => performance.getEntriesByName("musa:snapshot", "mark").length > 0,
    );
    samples.push((await at(page, "snapshot")) - (await at(page, "edit")) - SETTLE_MS);
  }
  expect(p95(samples), `diagnostics at p95 ${Math.round(p95(samples))} ms after settling`)
    .toBeLessThanOrEqual(120);
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
