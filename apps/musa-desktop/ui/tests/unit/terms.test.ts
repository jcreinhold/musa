/**
 * What the source column can answer about a term
 * (`docs/interface/08-elaboration.md` §§1–3).
 *
 * `stdlib-basics.musa` is the case the workbench exists for: names the
 * composer declared, and names they only used — the latter declared in modules
 * that came with the compiler. Every fact here was resolved by the core; these
 * tests hold the frontend to reading it and to inventing nothing, which is why
 * they are written against offsets in the committed source rather than against
 * hand-built term lists.
 */

import { describe, expect, it } from "vitest";

import fixture from "../../fixtures/stdlib-basics.snapshot.json";
import {
  completions,
  definitionAt,
  nameAt,
  termAt,
  usesAt,
} from "../../src/lib/state/terms";
import type { ProjectSnapshot, Span } from "../../src/lib/state/snapshot";

const SNAPSHOT = fixture as unknown as ProjectSnapshot;
const KNOWN = { terms: SNAPSHOT.terms, names: SNAPSHOT.names };
const SOURCE = SNAPSHOT.source;

/** The first offset at which `text` is written, mid-word so the ends are not the test. */
function inside(text: string, from = 0): number {
  const at = SOURCE.indexOf(text, from);
  expect(at, `\`${text}\` is not in the fixture`).toBeGreaterThanOrEqual(0);
  return at + 1;
}

function quoted(span: Span): string {
  return SOURCE.slice(span.start, span.end);
}

describe("asking what a word is", () => {
  it("answers with the declaration, wherever the word was written", () => {
    // The declaration and a use are two spellings of one question.
    expect(termAt(KNOWN, inside("subject: Music"))?.name).toBe("subject");
    expect(termAt(KNOWN, inside("subject, shift"))?.name).toBe("subject");
  });

  it("answers for a term the composer never declared", () => {
    const term = termAt(KNOWN, inside("compose_music("));
    expect(term?.name).toBe("compose_music");
    expect(term?.site.where).toBe("library");
    // The signature is the module's own text, not a summary of it.
    expect(term?.signature).toContain("compose_music");
  });

  it("says nothing where the compiler resolved nothing", () => {
    expect(nameAt(KNOWN, inside("Standard Library Basics"))).toBeUndefined();
    expect(termAt(KNOWN, SOURCE.length)).toBeUndefined();
  });
});

describe("following a name", () => {
  it("goes to the span, when the declaration is in this document", () => {
    const site = definitionAt(KNOWN, inside("subject, shift"));
    expect(site?.where).toBe("open");
    expect(site?.where === "open" && quoted(site.span)).toBe("subject");
  });

  /*
   * A library declaration crosses as a module and two numbers rather than as a
   * span, because a span means "in the open document" everywhere else on the
   * wire (`musa_project::utf16`). The frontend's part of that contract is to
   * hand the numbers back untouched, which is what this pins.
   */
  it("goes to the module, when the declaration came with the compiler", () => {
    const site = definitionAt(KNOWN, inside("naturals(4)"));
    expect(site?.where).toBe("library");
    if (site?.where !== "library") throw new Error("unreachable");
    expect(site.uri).toBe("musa-stdlib:/std/list.musa");
    expect(site.end).toBeGreaterThan(site.start);
  });

  it("has nowhere to go from a word that is not a name", () => {
    expect(definitionAt(KNOWN, inside('piece "Standard'))).toBeUndefined();
  });
});

describe("gathering the uses of a name", () => {
  it("takes the declaration and every use the resolver recorded", () => {
    const uses = usesAt(KNOWN, inside("subject: Music"));
    expect(uses.length).toBeGreaterThan(1);
    for (const use of uses) expect(quoted(use)).toBe("subject");
  });

  it("takes only the uses, for a name declared elsewhere", () => {
    const uses = usesAt(KNOWN, inside("naturals(4)"));
    expect(uses).not.toHaveLength(0);
    for (const use of uses) expect(quoted(use)).toBe("naturals");
  });

  it("is empty where there is no name, rather than being every match of the word", () => {
    expect(usesAt(KNOWN, inside('piece "Standard'))).toEqual([]);
  });
});

describe("the completion list", () => {
  it("is the core's list, in the core's order", () => {
    expect(completions(KNOWN)).toEqual(SNAPSHOT.terms);
  });

  it("offers the modules' terms beside the composer's own", () => {
    const offered = completions(KNOWN);
    expect(offered.some((term) => term.site.where === "library")).toBe(true);
    expect(offered.some((term) => term.site.where === "open")).toBe(true);
  });

  it("is empty rather than absent while nothing has compiled", () => {
    expect(completions(null)).toEqual([]);
  });
});
