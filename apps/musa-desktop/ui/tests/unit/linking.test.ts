/**
 * The two-way link between the page and the text, over a source that is not
 * ASCII.
 *
 * Every span here crossed the Rust/TypeScript boundary, and the two sides do
 * not measure a string the same way: Rust counts bytes, JavaScript counts
 * UTF-16 code units, and they agree only while the text stays under U+007F.
 * `unicode-fixture.musa` does not — its title, two of its section names, and
 * its comments hold em dashes, accents, a flat sign, and a treble clef — so a
 * span that arrived in bytes lands further and further from the text it names
 * as the file goes on.
 *
 * The assertions are therefore all of one shape: take a span the interface
 * would act on, cut the source with it, and read what came out. That is
 * exactly what CodeMirror does with the same numbers when it puts the caret
 * somewhere or draws a mark, and it needs no offsets written down here to
 * go stale. See `docs/interface/03-interaction.md` §7.
 */

import { describe, expect, it } from "vitest";

import snapshotJson from "../../fixtures/unicode-fixture.snapshot.json";
import { Workspace } from "../../src/lib/state/selection.svelte";
import type { ProjectSnapshot, Span } from "../../src/lib/state/snapshot";

const snapshot = snapshotJson as unknown as ProjectSnapshot;
const source = snapshot.source;

function workspace(): Workspace {
  return new Workspace(() => snapshot);
}

/** The text a span names, as the editor's document would cut it. */
function text(span: Span): string {
  return source.slice(span.start, span.end);
}

/** A caret at the first occurrence of `needle`, as the editor would report it. */
function caretAt(needle: string): number {
  const at = source.indexOf(needle);
  expect(at, `\`${needle}\` is not in the fixture`).toBeGreaterThan(-1);
  return at;
}

it("the fixture is the reason this file exists", () => {
  expect(source).toMatch(/[^\u0000-\u007f]/);
  // …and the divergence has to accumulate before the notes, or the spans
  // below would be right by accident.
  expect(source.indexOf("—")).toBeLessThan(caretAt("voice lead"));
});

describe("text to page", () => {
  it("a caret in a note chooses that note", () => {
    const at = caretAt("f5/4");
    const events = workspace().eventsForSpan({ start: at, end: at + 1 });
    expect(events).toHaveLength(1);

    const chosen = workspace().snapshot?.score?.events.find((event) => event.id === events[0]);
    expect(text(chosen?.origin.span as Span)).toBe("f5/4");
  });

  it("a caret in the whitespace between statements chooses nothing", () => {
    const at = caretAt("\n\n    motif");
    expect(workspace().eventsForSpan({ start: at, end: at + 1 })).toEqual([]);
  });
});

describe("page to text", () => {
  it("a chosen note marks the text it was written as", () => {
    const chosen = snapshot.score?.events.find((event) => !event.origin.generated);
    expect(chosen).toBeDefined();
    const space = workspace();
    space.select(chosen?.id ?? "");
    const spans = space.sourceSpans(false);
    expect(spans).toHaveLength(1);
    expect(text(spans[0] as Span)).toMatch(/^[a-g](?:##|bb|[#bn])?-?[0-9]+\/\d+$/);
  });

  it("a note from a motif marks where it is declared and where it was used", () => {
    const occurrence = snapshot.score?.occurrences[0];
    expect(occurrence).toBeDefined();
    const space = workspace();
    space.select(occurrence?.events[0] ?? "");

    expect(text(occurrence?.declaration as Span)).toMatch(/^motif sigh\(/);
    expect(text(occurrence?.useSite as Span)).toMatch(/^use sigh\(/);
    // Origin view marks both of those and the note itself.
    expect(space.sourceSpans(false)).toHaveLength(3);
  });
});

describe("the outline", () => {
  it("opens the source at the statement that wrote each row", () => {
    const rows = snapshot.score?.outline ?? [];
    expect(rows.length).toBeGreaterThan(0);
    for (const row of rows) {
      expect(text(row.span), `outline row ${row.name}`).toMatch(/^(section|phrase) "/);
    }
  });

  it("names sections whose names are themselves not ASCII", () => {
    const names = (snapshot.score?.outline ?? []).map((row) => row.name);
    expect(names).toContain("Réponse — variée");
  });
});

describe("diagnostics", () => {
  it("every span the gutter would underline lands on a token", () => {
    for (const diagnostic of snapshot.diagnostics) {
      if (!diagnostic.span) continue;
      expect(text(diagnostic.span)).not.toMatch(/^\s/);
    }
  });
});
