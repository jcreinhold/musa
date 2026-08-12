/**
 * Applying a diagnostic's fix to the source.
 *
 * A fix arrives as data — a title and a list of replacements — so the terminal
 * and the app spend the same object. Applying one here rather than
 * asking the core to is not the frontend inventing anything: the spans were
 * computed in Rust, and what happens next is the same `setSource` any keystroke
 * takes. The source stays canonical (AGENTS.md); this produces new text for it.
 *
 * Offsets on the wire are UTF-16 code units, which is exactly what a JavaScript
 * string is indexed in (`crates/musa-project/src/utf16.rs`), so no translation
 * happens here and none should.
 */

import type { Diagnostic, Fix } from "./snapshot";

/**
 * `source` with `fix` applied.
 *
 * Edits are applied last-first so that each one's offsets still describe the
 * text it was computed against. An edit that falls outside the source — a fix
 * held over from a revision the user has since typed past — is skipped rather
 * than clamped, because a replacement at the wrong place is worse than none.
 */
export function applyFix(source: string, fix: Fix): string {
  const ordered = [...fix.edits].sort(
    (left, right) => right.span.start - left.span.start,
  );
  let text = source;
  for (const edit of ordered) {
    const { start, end } = edit.span;
    if (start < 0 || end < start || end > text.length) continue;
    text = text.slice(0, start) + edit.replacement + text.slice(end);
  }
  return text;
}

/**
 * The one fix a diagnostic offers, or null.
 *
 * Null when there are none and also when there are several: a control labelled
 * "the fix" that silently picks the first of two is how an editor applies the
 * wrong one. A ranked list of candidates is an editor feature and is not this.
 */
export function onlyFix(diagnostic: Diagnostic): Fix | null {
  return diagnostic.fixes.length === 1 ? (diagnostic.fixes[0] ?? null) : null;
}

/** Where a diagnostic points, as `12:5`, or null when it points nowhere. */
export function placeOf(diagnostic: Diagnostic): string | null {
  const label =
    diagnostic.labels.find((candidate) => candidate.primary) ??
    diagnostic.labels[0];
  if (!label) return null;
  return `${label.at.line}:${label.at.column}`;
}

/** The primary label's text, when there is one worth showing. */
export function labelOf(diagnostic: Diagnostic): string | null {
  const label =
    diagnostic.labels.find((candidate) => candidate.primary) ??
    diagnostic.labels[0];
  return label?.text ?? null;
}

/**
 * A fix title as a control label.
 *
 * The core writes ``add `;` `` because that is how a terminal footer reads.
 * On a page it is a button, and `05-states.md` §1 says sentence case. Same
 * decision as the backticks in `Ticked`: one string in the core, spelled for
 * the terminal, adjusted where it is drawn.
 */
export function asControl(title: string): string {
  return title.charAt(0).toUpperCase() + title.slice(1);
}
