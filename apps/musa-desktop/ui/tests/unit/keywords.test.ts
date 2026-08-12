/**
 * The tooltip reads the language's own keyword documentation.
 *
 * The drift law lives in Rust — the generator fails when `keyword-docs.json`
 * falls behind `keywords.rs` — so what is left to prove here is the reading:
 * every keyword the lexer spells has a doc to show, and the doc comes apart
 * into prose and example the way the tooltip sets it.
 */

import { describe, expect, it } from "vitest";

import {
  docParts,
  keywordDoc,
  proseRuns,
} from "../../src/lib/lang-musa/keywords";
import docs from "../../src/lib/session/generated/keyword-docs.json";
import spellings from "../../src/lib/session/generated/spellings.json";

describe("keyword documentation", () => {
  it("covers every keyword the lexer spells", () => {
    const keywords = (spellings as [string, string][])
      .filter(([, kind]) => kind === "keyword" || kind === "use")
      .map(([spelling]) => spelling);
    expect(keywords.length).toBe(docs.length);
    for (const spelling of keywords) {
      expect(keywordDoc(spelling), spelling).toBeDefined();
    }
  });

  it("answers nothing for a word that is not a keyword", () => {
    expect(keywordDoc("sigh")).toBeUndefined();
    expect(keywordDoc("Hz")).toBeUndefined();
  });

  it("splits a doc into prose and example along the fence", () => {
    const tempo = keywordDoc("tempo");
    const { prose, example } = docParts(tempo?.doc ?? "");
    expect(prose).toContain("A tempo statement is a speed");
    expect(prose).not.toContain("```");
    expect(example).toContain("tempo 1/4 = 96;");
    expect(example).not.toContain("```");
  });

  it("reads prose with no example as all prose", () => {
    expect(docParts("just prose")).toEqual({
      prose: "just prose",
      example: null,
    });
  });

  it("takes the doc's backticks as code spans", () => {
    expect(proseRuns("write `use sigh(e5);` here")).toEqual([
      { text: "write ", code: false },
      { text: "use sigh(e5);", code: true },
      { text: " here", code: false },
    ]);
    expect(proseRuns("no spans")).toEqual([{ text: "no spans", code: false }]);
    expect(proseRuns("an unbalanced ` tick")).toEqual([
      { text: "an unbalanced ` tick", code: false },
    ]);
  });
});
