/**
 * What the workspace knows about provenance, against the canonical case.
 *
 * `glass-mountain.musa` is the score `04-provenance.md` §1 is written about:
 * ten of the violin's notes come from two occurrences of one five-note motif,
 * one of them transposed down a fifth. Every fact here came from the core —
 * the point of these tests is that the frontend reads it correctly and
 * derives nothing of its own (`03-interaction.md` §7).
 */

import { describe, expect, it } from "vitest";

import { fixture } from "../../src/lib/state/fixtures";
import { Workspace } from "../../src/lib/state/selection.svelte";
import type { ProjectSnapshot } from "../../src/lib/state/snapshot";

const snapshot = fixture("glass-mountain").snapshot as ProjectSnapshot;

function workspace(): Workspace {
  return new Workspace(() => snapshot);
}

describe("occurrences", () => {
  it("are two expansions of one motif, one of them transformed", () => {
    const [first, second] = workspace().occurrences;
    expect(workspace().occurrences).toHaveLength(2);
    expect(first?.path).toEqual(["sigh()"]);
    expect(second?.path).toEqual(["transpose down P5", "sigh()"]);
    expect(first?.motif).toBe("sigh");
    expect(second?.motif).toBe("sigh");
    // Two occurrences, never one: identity is the whole expansion path, so
    // the same motif used twice is two things a composer can act on.
    expect(first?.id).not.toBe(second?.id);
  });

  it("each account for five notes, and between them for all ten", () => {
    const [first, second] = workspace().occurrences;
    expect(first?.events).toHaveLength(5);
    expect(second?.events).toHaveLength(5);
    expect(new Set([...(first?.events ?? []), ...(second?.events ?? [])]).size).toBe(10);
  });

  it("point at the source: the declaration, and the use that ran", () => {
    const source = snapshot.source;
    for (const occurrence of workspace().occurrences) {
      const declaration = occurrence.declaration;
      expect(declaration).not.toBeNull();
      expect(source.slice(declaration?.start, declaration?.end)).toMatch(/^motif sigh/);
      expect(source.slice(occurrence.useSite.start, occurrence.useSite.end)).toBe("use sigh();");
      // The line the interface prints is the line the use statement is on.
      expect(source.slice(0, occurrence.useSite.start).split("\n")).toHaveLength(occurrence.line);
    }
  });
});

describe("selecting an occurrence", () => {
  it("takes everything that expansion produced, not the note clicked", () => {
    const space = workspace();
    const [first] = space.occurrences;
    space.select(first?.events[2] ?? "");
    expect(space.selected).toHaveLength(1);

    space.selectOccurrence(first?.id ?? null);
    expect(space.selected).toEqual(first?.events);
  });

  it("is what the innermost origin segment does", () => {
    const space = workspace();
    const [, second] = space.occurrences;
    space.select(second?.events[0] ?? "");
    // `transpose down P5 ▸ sigh()` — the second segment is the occurrence.
    space.selectOrigin(2);
    expect(space.selected).toEqual(second?.events);
  });

  it("reports honestly when there is no occurrence to select", () => {
    expect(workspace().selectOccurrence(null)).toBe(false);
    expect(workspace().selectOccurrence("occurrence-none")).toBe(false);
  });

  it("is reachable from any event of it, and from none other", () => {
    const space = workspace();
    const [first] = space.occurrences;
    for (const id of first?.events ?? []) expect(space.occurrenceOf(id)?.id).toBe(first?.id);
    const authored = (snapshot.score?.events ?? []).find((event) => !event.origin.generated);
    expect(space.occurrenceOf(authored?.id ?? null)).toBeUndefined();
  });
});

describe("a diagnostic's place in the music", () => {
  it("is the events whose source text encloses it", () => {
    const space = workspace();
    const event = snapshot.score?.events[0];
    const inside = { start: (event?.origin.span.start ?? 0) + 1, end: event?.origin.span.end ?? 0 };
    expect(space.eventsForSpan(inside)).toContain(event?.id);
  });

  it("is nothing at all when it points outside the music", () => {
    // A broken header has no system to flash, and says so with an empty list
    // rather than by guessing at the nearest note.
    expect(workspace().eventsForSpan({ start: 0, end: 1 })).toEqual([]);
    expect(workspace().eventsForSpan(null)).toEqual([]);
  });
});
