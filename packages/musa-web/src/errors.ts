/**
 * The byte↔character conversion and the error excerpt.
 * Compiler spans are UTF-8 byte offsets; JS strings are UTF-16. The
 * conversion lives here — once, explicitly, and tested against multi-byte
 * source — and nowhere else in the package.
 */

import type { MusaDiagnostic, MusaLabel } from "./types";

/**
 * The UTF-16 offset of a UTF-8 byte offset in `source`. Iteration is by code
 * point, so a surrogate pair and a four-byte character both count correctly.
 */
export function byteToCharOffset(source: string, byteOffset: number): number {
  const encoder = new TextEncoder();
  let bytes = 0;
  let chars = 0;
  for (const ch of source) {
    const width = encoder.encode(ch).length;
    if (bytes + width > byteOffset) break;
    bytes += width;
    chars += ch.length;
  }
  return chars;
}

/** The one line an error box shows, and the offending span inside it. */
export interface ErrorExcerpt {
  /** The line's text (no trailing newline). */
  line: string;
  /** 1-based, as a person counts lines. */
  lineNumber: number;
  /** UTF-16 offsets of the label's span, clamped into this line. */
  spanStart: number;
  spanEnd: number;
}

/**
 * The excerpt for a label: the line its span starts on, with the span's
 * offsets relative to that line. A span covering whole lines or running off
 * one clamps to the line, so the underline always lands somewhere visible.
 */
export function excerptFor(source: string, label: MusaLabel): ErrorExcerpt {
  const start = byteToCharOffset(source, label.start);
  const end = byteToCharOffset(source, label.end);
  const lineStart = source.lastIndexOf("\n", start - 1) + 1;
  const nextNewline = source.indexOf("\n", start);
  const lineEnd = nextNewline === -1 ? source.length : nextNewline;
  return {
    line: source.slice(lineStart, lineEnd),
    lineNumber: source.slice(0, lineStart).split("\n").length,
    spanStart: Math.max(start, lineStart) - lineStart,
    spanEnd: Math.max(Math.min(end, lineEnd), Math.max(start, lineStart)) - lineStart,
  };
}

const ERROR_STYLE_CLASS = "musa-error";

/**
 * The error box that goes where the score would be: the offending line with
 * the span underlined, the message, and the stable code. Never silent, never
 * console-only. Built with the target's own document so it works inside
 * shadow roots.
 */
export function errorBox(
  doc: Document,
  source: string,
  diagnostics: MusaDiagnostic[],
): HTMLElement {
  const box = doc.createElement("div");
  box.className = ERROR_STYLE_CLASS;
  box.setAttribute("role", "alert");

  const first = diagnostics.find((diagnostic) => diagnostic.severity === "error") ?? diagnostics[0];
  if (first === undefined) return box;

  const label = first.labels.find((candidate) => candidate.primary) ?? first.labels[0];
  if (label !== undefined) {
    const excerpt = excerptFor(source, label);
    const pre = doc.createElement("pre");
    pre.className = "musa-error-source";
    pre.append(
      excerpt.line.slice(0, excerpt.spanStart),
      underline(doc, excerpt.line.slice(excerpt.spanStart, excerpt.spanEnd)),
      excerpt.line.slice(excerpt.spanEnd),
    );
    box.append(pre);
  }

  const message = doc.createElement("p");
  message.className = "musa-error-message";
  message.textContent = first.message;
  const code = doc.createElement("span");
  code.className = "musa-error-code";
  code.textContent = first.code;
  message.append(" ", code);
  box.append(message);
  return box;
}

function underline(doc: Document, text: string): HTMLElement {
  const span = doc.createElement("span");
  span.className = "musa-error-span";
  span.textContent = text;
  return span;
}
