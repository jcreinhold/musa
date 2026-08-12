/**
 * The editor reads musa the way the compiler does.
 *
 * The source editor tokenizes locally — highlighting cannot wait on a round
 * trip — which is exactly the situation a second, drifting copy of a language
 * grows out of. So its reading of every example is compared, token for token,
 * against the real lexer's reading of the same file, written out by
 * `cargo test -p musa-desktop`.
 */

import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { CLASS_TAGS } from "../../src/lib/lang-musa/highlight";
import { TOKEN_CLASSES, tokenize, type Token } from "../../src/lib/lang-musa/tokenize";

const EXAMPLES = fileURLToPath(new URL("../../../../../examples/", import.meta.url));
const LEXED = fileURLToPath(new URL("../../fixtures/lexed/", import.meta.url));

const pieces = readdirSync(LEXED)
  .filter((name) => name.endsWith(".json"))
  .map((name) => name.replace(/\.json$/, ""));

const sourceOf = (piece: string) => readFileSync(`${EXAMPLES}${piece}.musa`, "utf8");

describe.each(pieces)("%s", (piece) => {
  it("tokenizes exactly as the compiler's lexer does", () => {
    const lexed = JSON.parse(readFileSync(`${LEXED}${piece}.json`, "utf8")) as Token[];
    expect(tokenize(sourceOf(piece))).toEqual(lexed);
  });
});

it("is checked against a source that is not ASCII", () => {
  // The comparison above is between offsets the lexer counted in bytes and
  // offsets this tokenizer counts in UTF-16 code units — two numbers that are
  // equal for every ASCII file and diverge for every other one
  // (`03-interaction.md` §7.1). If every example were ASCII, the suite would
  // pass whether or not the two sides agreed about the measure.
  // eslint-disable-next-line no-control-regex -- the ASCII range is the point: the fixture must contain a non-ASCII piece
  expect(pieces.some((piece) => /[^\u0000-\u007f]/.test(sourceOf(piece)))).toBe(true);
});

describe("half-typed source", () => {
  it("reads what the composer is in the middle of, without flashing at them", () => {
    expect(tokenize('piece "glass')).toEqual([
      { class: "keyword", start: 0, end: 5 },
      // A string with no closing quote is what typing one looks like; it is
      // the compiler's job to say so, not the editor's, but it must not be
      // read as code either.
      { class: "invalid", start: 6, end: 12 },
    ]);
    expect(tokenize("/* still writing")).toEqual([{ class: "comment", start: 0, end: 16 }]);
    // A block comment outlives the line it opened on.
    expect(tokenize("/* two\nlines */ c4")).toEqual([
      { class: "comment", start: 0, end: 15 },
      { class: "pitch", start: 16, end: 18 },
    ]);
  });
});

describe("the class table", () => {
  it("covers every class the lexer can produce", () => {
    // A class with no tag is text set as if it were prose — the one failure
    // mode that deriving the highlighting from the token list rules out.
    for (const name of TOKEN_CLASSES) {
      expect(CLASS_TAGS[name], name).toBeDefined();
    }
  });

  it("is that table and nothing else", () => {
    expect(Object.keys(CLASS_TAGS).sort()).toEqual([...TOKEN_CLASSES].sort());
  });
});
