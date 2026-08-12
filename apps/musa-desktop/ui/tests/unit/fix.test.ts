import { describe, expect, it } from "vitest";

import { applyFix, asControl, labelOf, onlyFix, placeOf } from "../../src/lib/state/fix";
import type { Diagnostic, Fix } from "../../src/lib/state/snapshot";

function diagnostic(over: Partial<Diagnostic> = {}): Diagnostic {
  return {
    severity: "error",
    code: "syntax",
    message: "missing `;`",
    labels: [],
    help: null,
    note: null,
    fixes: [],
    span: null,
    ...over,
  };
}

const insert = (start: number, replacement: string): Fix => ({
  title: `add \`${replacement}\``,
  edits: [{ span: { start, end: start }, replacement }],
});

describe("applyFix", () => {
  it("inserts at an empty span", () => {
    expect(applyFix("clef treble", insert(11, ";"))).toBe("clef treble;");
  });

  it("replaces a range", () => {
    const fix: Fix = {
      title: "write `1400 Hz`",
      edits: [{ span: { start: 8, end: 12 }, replacement: "1400 Hz" }],
    };
    expect(applyFix("cutoff: 1400", fix)).toBe("cutoff: 1400 Hz");
  });

  it("applies several edits without invalidating their offsets", () => {
    const fix: Fix = {
      title: "close both",
      edits: [
        { span: { start: 1, end: 1 }, replacement: "}" },
        { span: { start: 3, end: 3 }, replacement: "}" },
      ],
    };
    expect(applyFix("a{b", fix)).toBe("a}{b}");
  });

  it("skips an edit that no longer fits the text", () => {
    // A fix held over from a revision the user has typed past. Clamping it
    // would put the character somewhere nobody asked for.
    expect(applyFix("short", insert(99, ";"))).toBe("short");
  });
});

describe("onlyFix", () => {
  it("returns the one fix", () => {
    expect(onlyFix(diagnostic({ fixes: [insert(0, ";")] }))?.title).toBe("add `;`");
  });

  it("refuses to choose between two", () => {
    expect(onlyFix(diagnostic({ fixes: [insert(0, ";"), insert(1, "}")] }))).toBeNull();
  });

  it("is null with none", () => {
    expect(onlyFix(diagnostic())).toBeNull();
  });
});

describe("placeOf and labelOf", () => {
  const labelled = diagnostic({
    labels: [
      {
        span: { start: 4, end: 5 },
        at: { line: 2, column: 9 },
        text: "second",
        primary: false,
      },
      {
        span: { start: 0, end: 1 },
        at: { line: 12, column: 5 },
        text: "it goes here",
        primary: true,
      },
    ],
  });

  it("reads the primary label wherever it is in the list", () => {
    expect(placeOf(labelled)).toBe("12:5");
    expect(labelOf(labelled)).toBe("it goes here");
  });

  it("says nothing when the problem is the whole file", () => {
    expect(placeOf(diagnostic())).toBeNull();
    expect(labelOf(diagnostic())).toBeNull();
  });
});

describe("asControl", () => {
  it("sets a terminal-spelled title in sentence case", () => {
    expect(asControl("add `;`")).toBe("Add `;`");
    expect(asControl("")).toBe("");
  });
});
