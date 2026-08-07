/**
 * The fixtures this prototype runs on.
 *
 * They are imported, never fetched: roadmap §14.8 asks for an application
 * that works with zero setup and no network, and the smoke test holds the
 * page to exactly that. Every file here is written by a test in `musa-project`
 * — the snapshot from the type it serializes, the `.mei` from the real export
 * path — so none of them can drift from the core without a red test.
 */

import counterpointMei from "../../../fixtures/counterpoint.mei?raw";
import glassMountainMei from "../../../fixtures/glass-mountain.mei?raw";
import largeScoreMei from "../../../fixtures/large-score.mei?raw";
import twinkleMei from "../../../fixtures/twinkle.mei?raw";
import glassMountainSnapshot from "../../../fixtures/glass-mountain.snapshot.json";
import type { ProjectSnapshot } from "./snapshot";

export interface Fixture {
  key: string;
  /** The title as the score itself spells it. */
  title: string;
  mei: string;
  /** Present only where a full snapshot was serialized for this score. */
  snapshot?: ProjectSnapshot;
}

const GLASS_MOUNTAIN: Fixture = {
  key: "glass-mountain",
  title: "Glass Mountain",
  mei: glassMountainMei,
  snapshot: glassMountainSnapshot as unknown as ProjectSnapshot,
};

export const FIXTURES: readonly Fixture[] = [
  GLASS_MOUNTAIN,
  { key: "counterpoint", title: "Counterpoint Study", mei: counterpointMei },
  { key: "twinkle", title: "Twinkle", mei: twinkleMei },
  // The large-case workload of `06-performance.md` §1 — 100 bars in four
  // parts. It is here so the budgets can be measured through the real screen
  // rather than through a harness that skips it.
  { key: "large-score", title: "Large Score", mei: largeScoreMei },
];

/** The named fixture, or Glass Mountain — the score the Compose screen shows. */
export function fixture(key: string | null): Fixture {
  return FIXTURES.find((candidate) => candidate.key === key) ?? GLASS_MOUNTAIN;
}
