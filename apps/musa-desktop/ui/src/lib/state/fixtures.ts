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
import annotatedMei from "../../../fixtures/annotated.mei?raw";
import openFormMei from "../../../fixtures/open-form.mei?raw";
import openFormAgainMei from "../../../fixtures/open-form-again.mei?raw";
import glassMountainSnapshot from "../../../fixtures/glass-mountain.snapshot.json";
import annotatedSnapshot from "../../../fixtures/annotated.snapshot.json";
import openFormSnapshot from "../../../fixtures/open-form.snapshot.json";
import openFormAgainSnapshot from "../../../fixtures/open-form-again.snapshot.json";
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
  // The annotated piece: phrases, form markers, and a harmony lane, which is
  // what the outline pane is read against.
  {
    key: "annotated",
    title: "Annotated",
    mei: annotatedMei,
    snapshot: annotatedSnapshot as unknown as ProjectSnapshot,
  },
  // The open work, twice. The same source read under two performances: the
  // interface's only proof that a realization is a reading and not a
  // property of the file.
  {
    key: "open-form",
    title: "Loop Lengths",
    mei: openFormMei,
    snapshot: openFormSnapshot as unknown as ProjectSnapshot,
  },
  {
    key: "open-form-again",
    title: "Loop Lengths, again",
    mei: openFormAgainMei,
    snapshot: openFormAgainSnapshot as unknown as ProjectSnapshot,
  },
  { key: "counterpoint", title: "Counterpoint Study", mei: counterpointMei },
  { key: "twinkle", title: "Twinkle", mei: twinkleMei },
  // The large-case workload of `06-frame-budgets.md` §1 — 100 bars in four
  // parts. It is here so the budgets can be measured through the real screen
  // rather than through a harness that skips it.
  { key: "large-score", title: "Large Score", mei: largeScoreMei },
];

/** The named fixture, or Glass Mountain — the score the Compose screen shows. */
export function fixture(key: string | null): Fixture {
  return FIXTURES.find((candidate) => candidate.key === key) ?? GLASS_MOUNTAIN;
}
