/**
 * musa, as CodeMirror understands it.
 *
 * A stream language rather than a Lezer grammar: the parser that matters
 * already exists in Rust and is the one whose opinion counts, so what the
 * editor needs is enough structure to *set* the text — tokens, blocks,
 * comments — and nothing that could disagree with the core about meaning.
 *
 * Folding and indentation follow the formatter (`musa-language`'s: four
 * spaces, `{` trailing the declaration line), so formatting a file the editor
 * indented changes nothing.
 */

import { LanguageSupport, StreamLanguage, foldService, indentService, indentUnit } from "@codemirror/language";

import { CLASS_TAGS } from "./highlight";
import { read, type ReaderState } from "./tokenize";

/** What the formatter writes, so the editor writes the same. */
export const INDENT = "    ";

const musaStream = StreamLanguage.define<ReaderState>({
  name: "musa",
  startState: () => ({ inComment: false }),
  token(stream, state) {
    const step = read(stream.string, stream.pos, state);
    // A line ends a block comment's line-slice but not the comment itself;
    // the reader says which, and the state carries it to the next line.
    state.inComment = step.state.inComment;
    stream.pos = Math.max(step.end, stream.pos + 1);
    return step.token?.class ?? null;
  },
  tokenTable: CLASS_TAGS,
  languageData: {
    commentTokens: { line: "//", block: { open: "/*", close: "*/" } },
    closeBrackets: { brackets: ["{", "[", "(", '"'] },
    indentOnInput: /^\s*\}$/,
  },
});

/**
 * The formatter's rule, applied while typing: a line that opened a block
 * indents what follows it, and a `}` closes back to the level of the line
 * that opened it. Nothing else moves — a source the editor indented and one
 * the formatter wrote agree.
 */
const musaIndent = indentService.of((context, pos) => {
  const here = context.state.doc.lineAt(pos);
  const previous = here.number > 1 ? context.state.doc.line(here.number - 1).text : "";
  const base = /^\s*/.exec(previous)?.[0].length ?? 0;
  const opened = previous.trimEnd().endsWith("{") ? INDENT.length : 0;
  const closed = here.text.trimStart().startsWith("}") ? INDENT.length : 0;
  return Math.max(base + opened - closed, 0);
});

/**
 * Folding, by the block a `{` opens.
 *
 * Every foldable thing in musa — `piece`, `score`, `part`, `voice`, `motif`,
 * `transpose`, `overlay` — is a braced block, so the rule is the brace rather
 * than a list of keywords that would go stale the moment the language grew
 * one. Text inside a string is not counted, because a `{` in a title is not a
 * block.
 */
const musaFold = foldService.of((state, start, end) => {
  if (!state.doc.lineAt(start).text.trimEnd().endsWith("{")) return null;
  let depth = 0;
  for (let number = state.doc.lineAt(start).number; number <= state.doc.lines; number += 1) {
    const line = state.doc.line(number);
    let quoted = false;
    for (let at = 0; at < line.text.length; at += 1) {
      const char = line.text[at];
      if (char === '"') quoted = !quoted;
      else if (quoted) continue;
      else if (char === "{") depth += 1;
      else if (char === "}") {
        depth -= 1;
        if (depth === 0) return { from: end, to: line.from + at };
      }
    }
  }
  return null;
});

/** The language, with the editor settings that belong to it. */
export function musa(): LanguageSupport {
  return new LanguageSupport(musaStream, [indentUnit.of(INDENT), musaIndent, musaFold]);
}

export { CLASS_TAGS, musaHighlighting } from "./highlight";
export { docParts, keywordDoc, proseRuns, type DocParts, type KeywordDoc, type ProseRun } from "./keywords";
export { TOKEN_CLASSES, tokenize, type Token, type TokenClass } from "./tokenize";
