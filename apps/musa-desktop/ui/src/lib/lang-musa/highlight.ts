/**
 * How musa source is set.
 *
 * `01-visual-language.md` §3: the source is set, not dumped, and §5's rule
 * about colour holds here too — ink weight does the work and there is exactly
 * one accent. `use` takes `--plate`, because that is where generated material
 * comes from, and the same hue means the same thing on the page and in the
 * text. Nothing else is coloured.
 */

import { HighlightStyle } from "@codemirror/language";
import { Tag, tags } from "@lezer/highlight";

import type { TokenClass } from "./tokenize";

/** The tag each token class is highlighted under. */
export const CLASS_TAGS: Record<TokenClass, Tag> = {
  comment: tags.comment,
  keyword: tags.keyword,
  use: tags.definitionKeyword,
  pitch: tags.literal,
  duration: tags.number,
  number: tags.integer,
  text: tags.string,
  unit: tags.unit,
  name: tags.name,
  punctuation: tags.punctuation,
  invalid: tags.invalid,
};

/**
 * The style itself, in the interface's own tokens.
 *
 * Every colour here is a variable, so the source follows the theme the rest
 * of the application follows without a second palette to keep in step.
 */
export const musaHighlighting = HighlightStyle.define([
  { tag: tags.comment, color: "var(--ink-muted)", fontStyle: "italic" },
  { tag: tags.keyword, color: "var(--ink)", fontWeight: "500" },
  // The one accent: `use` is the generated-material site.
  { tag: tags.definitionKeyword, color: "var(--plate)", fontWeight: "500" },
  { tag: tags.literal, color: "var(--ink)" },
  { tag: tags.number, color: "var(--ink)" },
  { tag: tags.integer, color: "var(--ink-muted)" },
  { tag: tags.string, color: "var(--ink-muted)", fontStyle: "italic" },
  { tag: tags.unit, color: "var(--ink-muted)", fontStyle: "italic" },
  { tag: tags.name, color: "var(--ink-muted)" },
  { tag: tags.punctuation, color: "var(--ink-muted)" },
  { tag: tags.invalid, color: "var(--chalk)" },
]);
