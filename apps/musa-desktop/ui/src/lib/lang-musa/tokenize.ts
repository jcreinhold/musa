/**
 * Reading musa text the way the compiler reads it.
 *
 * The editor cannot ask the core what a half-typed line is — highlighting
 * happens on every keystroke and the core answers on its own schedule — so it
 * has a tokenizer of its own. What keeps that from becoming a second, drifting
 * language is that it owns no vocabulary: the words and marks come from
 * `spellings.json`, written out of the real lexer, and its reading of every
 * example is compared token for token against the lexer's own
 * (`tests/unit/highlighting.test.ts`).
 *
 * The offsets below are JavaScript string indices — UTF-16 code units — which
 * is what CodeMirror wants and what the lexer's own offsets are restated in
 * before they are written to a fixture (`03-interaction.md` §7.1). Nothing
 * here converts anything; that is the point of the contract.
 *
 * One function does the reading — [`read`] — so the whole-document tokenizer
 * the tests use and the line-at-a-time one CodeMirror needs cannot disagree
 * about what a token is.
 */

import spellings from "../session/generated/spellings.json";
import classes from "../session/generated/token-classes.json";

/** What a token is, in the language's own vocabulary. */
export type TokenClass = (typeof classes)[number];

/** Every class the lexer can produce. */
export const TOKEN_CLASSES: readonly TokenClass[] = classes;

/** The words and marks the composer types literally. */
const SPELLED = new Map<string, TokenClass>(
  spellings as [string, TokenClass][],
);

/** The marks, longest first, so `->` is read before `-`. */
const MARKS = [...SPELLED.keys()]
  .filter((text) => !/^[A-Za-z]/.test(text))
  .sort((a, b) => b.length - a.length);

/** One token: what it is, and where. */
export interface Token {
  class: TokenClass;
  start: number;
  end: number;
}

/**
 * What the reader carries between tokens: only whether it is inside a block
 * comment, which is the one construct that outlives a line.
 */
export interface ReaderState {
  inComment: boolean;
}

/** What one read produced: a token, or whitespace, plus the state after it. */
export interface Read {
  /** Null for whitespace, which carries no ink. */
  token: Token | null;
  end: number;
  state: ReaderState;
}

const WORD = /[A-Za-z_]/;
const WORD_TAIL = /[A-Za-z_0-9]/;
const DIGIT = /[0-9]/;

const OPEN: ReaderState = { inComment: false };
const INSIDE: ReaderState = { inComment: true };

function at(source: string, index: number): string {
  return source[index] ?? "";
}

/**
 * A word, and what it turns out to be.
 *
 * The lexer's own order: a pitch (`g#4`, `a-1`) and an interval (`P5`) beat
 * the identifier pattern, and a spelled word beats both. Everything the
 * composer named is a name.
 */
function word(source: string, from: number): Token {
  let end = from;
  while (end < source.length && WORD.test(at(source, end))) end += 1;

  const letters = source.slice(from, end);
  const rest = source.slice(end);
  // A pitch carries its accidental and its octave — and the sign a negative
  // octave takes — so what follows the letters belongs to the same token. A
  // flat is a letter and rides along in `letters`; a sharp is not, so it is
  // picked up here.
  const sharps = /^(##|#)/.exec(rest);
  const afterSharps = sharps ? rest.slice(sharps[0].length) : rest;
  const octave = /^-?[0-9]+/.exec(afterSharps);
  const spelled = sharps
    ? /^[a-g]$/.test(letters)
    : /^[a-g](bb|[bn])?$/.test(letters);
  if (octave && spelled) {
    return {
      class: "pitch",
      start: from,
      end: end + (sharps?.[0].length ?? 0) + octave[0].length,
    };
  }
  const size = /^[0-9]+/.exec(rest);
  if (size && /^[PMm]$/.test(letters)) {
    return { class: "pitch", start: from, end: end + size[0].length };
  }
  // No literal read the word, so it is a name — and a name may carry digits
  // after its first letter, because musicians write words that do: `fmaj7`,
  // `sus4`, `drop2`. The digits are taken only here, after the literals have
  // had their turn, because `c4` is a pitch and taking them earlier would
  // spell it as a chord symbol.
  while (end < source.length && WORD_TAIL.test(at(source, end))) end += 1;
  return {
    class: SPELLED.get(source.slice(from, end)) ?? "name",
    start: from,
    end,
  };
}

/** A number: a rational, a float, or an integer, in the lexer's order. */
function number(source: string, from: number): Token {
  const match = /^[0-9]+\/[0-9]+|^[0-9]+\.[0-9]+|^[0-9]+/.exec(
    source.slice(from),
  );
  const text = match?.[0] ?? at(source, from);
  return {
    class: text.includes("/") ? "duration" : "number",
    start: from,
    end: from + text.length,
  };
}

/** A string literal. A newline ends it, unterminated, as it does for the lexer. */
function text(source: string, from: number): Token {
  let end = from + 1;
  let closed = false;
  while (end < source.length) {
    const here = at(source, end);
    if (here === "\n") break;
    if (here === "\\") {
      end += 2;
      continue;
    }
    end += 1;
    if (here === '"') {
      closed = true;
      break;
    }
  }
  return {
    class: closed ? "text" : "invalid",
    start: from,
    end: Math.min(end, source.length),
  };
}

/** The rest of a block comment, from `from`, given that one is open. */
function comment(source: string, from: number, opening: boolean): Read {
  const close = source.indexOf("*/", opening ? from + 2 : from);
  const end = close === -1 ? source.length : close + 2;
  return {
    token: { class: "comment", start: from, end },
    end,
    // An unclosed comment is still a comment: it is what the composer is in
    // the middle of typing, and marking it invalid would flash at them.
    state: close === -1 ? INSIDE : OPEN,
  };
}

/**
 * Read one token, starting at `from`.
 *
 * Never returns without advancing: an unreadable byte is one invalid token,
 * which is what the lexer does with it too.
 */
export function read(
  source: string,
  from: number,
  state: ReaderState = OPEN,
): Read {
  if (state.inComment) return comment(source, from, false);

  const here = at(source, from);
  if (/\s/.test(here)) {
    let end = from;
    while (end < source.length && /\s/.test(at(source, end))) end += 1;
    return { token: null, end, state };
  }

  if (source.startsWith("//", from)) {
    const line = source.indexOf("\n", from);
    const end = line === -1 ? source.length : line;
    return { token: { class: "comment", start: from, end }, end, state };
  }
  if (source.startsWith("/*", from)) return comment(source, from, true);

  if (here === '"') {
    const token = text(source, from);
    return { token, end: token.end, state };
  }
  if (WORD.test(here)) {
    const token = word(source, from);
    return { token, end: token.end, state };
  }
  if (DIGIT.test(here)) {
    const token = number(source, from);
    return { token, end: token.end, state };
  }

  const mark = MARKS.find((candidate) => source.startsWith(candidate, from));
  if (mark !== undefined) {
    const token: Token = {
      class: SPELLED.get(mark) ?? "punctuation",
      start: from,
      end: from + mark.length,
    };
    return { token, end: token.end, state };
  }

  return {
    token: { class: "invalid", start: from, end: from + 1 },
    end: from + 1,
    state,
  };
}

/** Every token in a piece of musa source, whitespace aside. */
export function tokenize(source: string): Token[] {
  const tokens: Token[] = [];
  let state = OPEN;
  let from = 0;
  while (from < source.length) {
    const step = read(source, from, state);
    if (step.token) tokens.push(step.token);
    state = step.state;
    from = step.end;
  }
  return tokens;
}
