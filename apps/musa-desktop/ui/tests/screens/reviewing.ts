/**
 * Opening a take for review, for the tests that need one.
 *
 * The readings are the project's own: `musa-desktop`'s review generator
 * composes them from prompt 203's measured corpus and commits them, so a
 * fixture asks exactly what the transcriber asked and nothing more.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, type Page } from "@playwright/test";

import { engraved } from "./engraved";

const READINGS = JSON.parse(
  readFileSync(fileURLToPath(new URL("../../fixtures/reviews.json", import.meta.url)), "utf8"),
) as Record<string, Record<string, unknown>>;

/** The clean take, and one per ambiguity class the corpus exhibited. */
export type Reading = "straight-known" | "swing-known" | "rolled-and-block-chords" | "crossing-voices" | "rubato-free";

export function reading(which: Reading): Record<string, unknown> {
  return READINGS[which] ?? {};
}

/**
 * Open a take through the interface's own door: the capture line offers
 * Review exactly when there is a take, and pressing it is the only way in. A
 * test that reached past it would prove a screen nobody can get to.
 */
export async function openReview(page: Page, which: Reading): Promise<void> {
  await engraved(page);
  await page.evaluate((facts) => window.__musaReview(facts), reading(which));
  await page.evaluate(() =>
    window.__musaSet({
      midiPort: "Stub Keyboard",
      midiCapture: {
        state: "review",
        recentEnabled: false,
        recentEvents: 0,
        recentMicros: 0,
        recentTruncated: false,
        captureEvents: 12,
        captureMicros: 4_000_000,
        recentEventLimit: 4096,
        recentTimeLimitMicros: 30_000_000,
        captureEventLimit: 65_536,
        captureTimeLimitMicros: 600_000_000,
        unsupportedAuditionEvents: 0,
        losses: { queueOverflow: 0, refusedSysex: 0, malformed: 0, unsupportedSystem: 0 },
      },
    }),
  );
  await page.getByRole("button", { name: "Review the take" }).click();
  await expect(page.getByRole("heading", { name: "Review" })).toBeVisible();
}
