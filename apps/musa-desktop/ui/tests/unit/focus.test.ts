/**
 * The shared focus, against the score `04-provenance.md` §1 is written about.
 *
 * `glass-mountain.musa` has both hard cases in one file: ten violin notes from
 * two occurrences of a five-note motif, and eight authored notes below them.
 * So the plural answers are real here — one `motif` line spells two notes, one
 * `use` line places five — and every one of them came from the core's
 * `origin.span` and `origin.definitionSpan` (`03-interaction.md` §7).
 */

import { describe, expect, it } from "vitest";

import { Focus } from "../../src/lib/state/focus.svelte";
import { fixture } from "../../src/lib/state/fixtures";
import type { ProjectSnapshot } from "../../src/lib/state/snapshot";

const snapshot = fixture("glass-mountain").snapshot as ProjectSnapshot;

function focus(): Focus {
  return new Focus(() => snapshot);
}

/** The event ids, by the shape of their origin, so the test reads as prose. */
const AUTHORED = "event-a";
const GENERATED = "event-0";
/** The same note of the motif, in the other occurrence. */
const SIBLING = "event-5";

/** A line wide enough to hold this statement, as the editor would report it. */
function line(span: { start: number; end: number }): {
  from: number;
  to: number;
} {
  return { from: span.start - 4, to: span.end };
}

function origin(id: string) {
  const event = (snapshot.score?.events ?? []).find(
    (candidate) => candidate.id === id,
  );
  if (!event) throw new Error(`no ${id} in the fixture`);
  return event.origin;
}

describe("focusing a note", () => {
  it("marks the one statement that wrote an authored note", () => {
    const shared = focus();
    shared.point(AUTHORED);
    const marked = shared.marked;
    expect(marked.events).toEqual([AUTHORED]);
    // It was written where it stands, so there is no second place to point at.
    expect(marked.definition).toBeNull();
    expect(marked.place).toEqual(origin(AUTHORED).span);
  });

  it("marks both places for a generated note, and they differ", () => {
    const shared = focus();
    shared.point(GENERATED);
    const marked = shared.marked;
    expect(marked.definition).toEqual(origin(GENERATED).definitionSpan);
    expect(marked.place).toEqual(origin(GENERATED).span);
    expect(marked.definition).not.toEqual(marked.place);
  });

  it("marks the siblings that same line spelled, in every occurrence", () => {
    const shared = focus();
    shared.point(GENERATED);
    // The motif is used twice, so one line of it spells two notes — and both
    // get the hairline. Showing one of them is the lie the editing choice
    // later has to correct with a number (`04-provenance.md` §4).
    expect(shared.marked.events).toEqual([GENERATED, SIBLING]);
  });

  it("marks nothing when the pointer is over blank paper", () => {
    const shared = focus();
    shared.point(GENERATED);
    shared.point(null);
    expect(shared.marked.events).toEqual([]);
  });
});

describe("focusing a line of the text", () => {
  it("resolves a motif line to every note it spelled", () => {
    const shared = focus();
    shared.pointLine(line(origin(GENERATED).definitionSpan));
    expect(shared.marked.events).toEqual([GENERATED, SIBLING]);
    // A line in the text is already the answer to "where"; there is nothing
    // to mark back at it.
    expect(shared.marked.definition).toBeNull();
    expect(shared.marked.place).toBeNull();
  });

  it("resolves a use line to the whole expansion", () => {
    const shared = focus();
    shared.pointLine(line(origin(GENERATED).span));
    expect(shared.marked.events).toEqual([
      "event-0",
      "event-1",
      "event-2",
      "event-3",
      "event-4",
    ]);
  });

  it("resolves an authored note's own line to that note", () => {
    const shared = focus();
    shared.pointLine(line(origin(AUTHORED).span));
    expect(shared.marked.events).toEqual([AUTHORED]);
  });

  it("resolves scaffolding to nothing", () => {
    const shared = focus();
    // The first line is the front matter: text, but not text that sounds.
    shared.pointLine({ from: 0, to: 24 });
    expect(shared.marked.events).toEqual([]);
  });

  it("answers for the line, not for the character under the pointer", () => {
    const shared = focus();
    const span = origin(GENERATED).definitionSpan;
    // The pointer past the last character of the line reports the line's end,
    // which no statement contains — the line is what is being pointed at, so
    // it is still the same answer.
    shared.pointLine({ from: span.start - 8, to: span.end });
    expect(shared.marked.events).toEqual([GENERATED, SIBLING]);
  });

  it("resolves a blank line to nothing", () => {
    const shared = focus();
    const between = origin(GENERATED).definitionSpan.end + 1;
    shared.pointLine({ from: between, to: between });
    expect(shared.marked.events).toEqual([]);
  });
});

describe("the focus itself", () => {
  it("is the pointer's while there is one and the keyboard's otherwise", () => {
    const shared = focus();
    shared.land(AUTHORED);
    expect(shared.marked.events).toEqual([AUTHORED]);
    shared.point(GENERATED);
    expect(shared.marked.events).toEqual([GENERATED, SIBLING]);
    // The pointer leaves and the keyboard's focus is still there — it was
    // never overwritten, only outranked.
    shared.leave();
    expect(shared.marked.events).toEqual([AUTHORED]);
  });

  it("does not linger once the pointer has left and nothing is selected", () => {
    const shared = focus();
    shared.point(GENERATED);
    shared.leave();
    expect(shared.marked.events).toEqual([]);
  });

  it("counts what a statement spelled, for the row that says so before anyone hovers", () => {
    const shared = focus();
    expect(shared.spelled(origin(GENERATED).definitionSpan)).toBe(2);
    expect(shared.spelled(origin(AUTHORED).definitionSpan)).toBe(1);
    expect(shared.spelled(undefined)).toBe(0);
  });
});
