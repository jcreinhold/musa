/**
 * What Origin says about music the composer did not write note by note
 * (`docs/rules/desktop/08-elaboration.md` §§4–5).
 *
 * `events-splice.musa` is the hardest case the workbench has: every note in it
 * is generated, its expansion path runs through an event track quote, and one of its
 * steps is a claim about the passage rather than a place in the source.
 * `claim-not-a-measure.musa` is the other end — a piece that does not compile,
 * because the composer asserted something of a passage that is not true of it.
 */

import { describe, expect, it } from "vitest";

import eventsFixture from "../../fixtures/events-splice.snapshot.json";
import refusedFixture from "../../fixtures/refused-claim.snapshot.json";
import report from "../../fixtures/pivot-ambiguity.analysis.json";
import { Workspace } from "../../src/lib/state/selection.svelte";
import { applyFix, onlyFix } from "../../src/lib/state/fix";
import type { AnalysisFacts, ProjectSnapshot, StepFact } from "../../src/lib/state/snapshot";

const SPLICED = eventsFixture as unknown as ProjectSnapshot;
const REFUSED = refusedFixture as unknown as ProjectSnapshot;
const READING = report as unknown as AnalysisFacts;

function workspace(snapshot: ProjectSnapshot): Workspace {
  return new Workspace(() => snapshot);
}

describe("an expansion path through an event track quote", () => {
  const events = SPLICED.score?.events ?? [];

  it("is generated music, all of it", () => {
    expect(events).not.toHaveLength(0);
    for (const event of events) expect(event.origin.generated).toBe(true);
  });

  /*
   * The path is plural, and each step is a different kind of thing: a claim
   * about the passage, the term that was expanded, and the placement the quote
   * performed. Flattening the three into one word would be the interface
   * deciding which of them the composer meant.
   */
  it("names each step by what it is, not by where it happens to point", () => {
    const kinds = new Set(events.flatMap((event) => event.origin.path.map((step) => step.kind)));
    expect(kinds).toContain("occurrence");
    expect(kinds).toContain("splice");
    expect(kinds).toContain("assertion");
  });

  it("keeps every step it was given, rather than choosing a convenient one", () => {
    for (const event of events) expect(event.origin.path.length).toBeGreaterThan(1);
  });

  /*
   * `08-elaboration.md` §4: a step that is not a place has nothing to reveal.
   * A splice happened at a time in the event track term, not at an offset in the
   * file, and a row that pointed it at the nearest brace would be inventing a
   * source map the compiler declined to write.
   */
  it("carries a span only for the steps that are places", () => {
    const steps: StepFact[] = events.flatMap((event) => event.origin.path);
    const placed = steps.filter((step) => step.span !== null);
    expect(placed).not.toHaveLength(0);
    for (const step of placed) expect(step.kind).toBe("occurrence");
    expect(steps.some((step) => step.kind === "splice" && step.span === null)).toBe(true);
  });

  it("still selects the whole expansion from any note of it", () => {
    const space = workspace(SPLICED);
    const first = space.occurrences[0];
    expect(first).toBeDefined();
    space.selectOccurrence(first?.id ?? null);
    expect(space.selected).toEqual(first?.events);
  });
});

describe("a claim the compiler refused", () => {
  const [problem] = REFUSED.diagnostics;

  it("is reported at the passage the claim was written about", () => {
    expect(problem?.span).not.toBeNull();
    expect(REFUSED.source.slice(problem?.span?.start, problem?.span?.end)).toContain("assert");
  });

  it("keeps the piece off the stand: nothing was engraved from it", () => {
    expect(REFUSED.compiles).toBe(false);
  });

  /*
   * A refused claim is one of the few diagnostics that knows the repair, and
   * the repair is the compiler's own text (`05-states.md` §3). The interface
   * applies it; it never composes one.
   */
  it("offers the compiler's certain fix, and the fix is an edit to the source", () => {
    const fix = onlyFix(problem);
    expect(fix).not.toBeNull();
    expect(fix?.title).toBeTruthy();
    const repaired = applyFix(REFUSED.source, fix ?? { title: "", edits: [] });
    expect(repaired).not.toBe(REFUSED.source);
    expect(repaired.length).toBeGreaterThan(REFUSED.source.length);
  });
});

describe("a reading with more than one answer", () => {
  it("states its method and what it took for granted before anything it saw", () => {
    expect(READING.method).toBeTruthy();
    expect(READING.assumptions).not.toHaveLength(0);
  });

  /*
   * Two keys explain this passage and the reading says so. Nothing in the
   * interface may reduce that to one: an ambiguity resolved by the frontend is
   * a theoretical judgment the frontend has no standing to make
   * (`08-elaboration.md` §5).
   */
  it("keeps both readings of the pivot, each with its own standing", () => {
    const regions = READING.findings.filter((finding) => finding.code === "key-region");
    expect(regions.length).toBeGreaterThan(1);
    for (const finding of regions) expect(finding.standing).toBeTruthy();
  });

  it("says which evidence a finding does not have, rather than dropping it", () => {
    const grounded = READING.findings.filter((finding) => finding.grounds.length > 0);
    expect(grounded).not.toHaveLength(0);
    for (const finding of grounded) {
      for (const ground of finding.grounds) {
        expect(ground.criterion).toBeTruthy();
        expect(ground.cites).toBeTruthy();
      }
    }
  });

  it("is never a diagnostic: a finding has no severity to paint red", () => {
    for (const finding of READING.findings) {
      expect(finding).not.toHaveProperty("severity");
      expect(finding).not.toHaveProperty("fixes");
    }
  });
});
