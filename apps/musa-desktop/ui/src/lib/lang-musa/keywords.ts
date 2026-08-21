/**
 * Every keyword's own documentation, as the language wrote it.
 *
 * The words are `keyword-docs.json`, generated out of `keywords.rs` in
 * `musa-syntax` — the one table the lexer, the language server, and this
 * tooltip all read — by the same generator that writes `spellings.json`, so
 * a keyword added to the language arrives here as a stale-fixture failure,
 * not as a silent gap. What this module adds is only the reading: a lookup
 * by word, and the doc split into the prose and its example so the tooltip
 * can set each in its own face.
 */

import docs from "../session/generated/keyword-docs.json";

/** One keyword's documentation. */
export interface KeywordDoc {
  spelling: string;
  /** One clause, for lists and tooltips. */
  summary: string;
  /** Plain English and one `musa` example, fenced. */
  doc: string;
}

const DOCS = new Map<string, KeywordDoc>((docs as KeywordDoc[]).map((doc) => [doc.spelling, doc]));

/** The keyword's documentation, or undefined for a word that is not a keyword. */
export function keywordDoc(word: string): KeywordDoc | undefined {
  return DOCS.get(word);
}

/** A doc taken apart: what is said, and the example that shows it. */
export interface DocParts {
  prose: string;
  example: string | null;
}

const FENCE = "\n\n```musa\n";

/** Split a doc along its fence. A doc with no example is all prose. */
export function docParts(doc: string): DocParts {
  const at = doc.indexOf(FENCE);
  if (at === -1) return { prose: doc, example: null };
  const rest = doc.slice(at + FENCE.length);
  const end = rest.indexOf("\n```");
  return {
    prose: doc.slice(0, at),
    example: end === -1 ? rest : rest.slice(0, end),
  };
}

/** One run of prose: plain text, or a code span from the doc's backticks. */
export interface ProseRun {
  text: string;
  code: boolean;
}

/**
 * The prose as runs, so the tooltip can set the doc's `code spans` in the
 * editor's own face. Not markdown: the docs carry backticks and nothing
 * else that needs setting.
 */
export function proseRuns(prose: string): ProseRun[] {
  const runs: ProseRun[] = [];
  let rest = prose;
  while (rest.length > 0) {
    const open = rest.indexOf("`");
    if (open === -1) {
      runs.push({ text: rest, code: false });
      break;
    }
    const close = rest.indexOf("`", open + 1);
    if (close === -1) {
      runs.push({ text: rest, code: false });
      break;
    }
    if (open > 0) runs.push({ text: rest.slice(0, open), code: false });
    runs.push({ text: rest.slice(open + 1, close), code: true });
    rest = rest.slice(close + 1);
  }
  return runs;
}
