/**
 * The `currentColor` sanitizer — docs/interface/02-engraving.md §3.
 *
 * Verovio 6 emits no per-element `fill`, but it does set `color="black"` on
 * the `.definition-scale` element and lets everything inherit from there.
 * That single attribute is what defeats theming, so it is removed and the CSS
 * cascade governs instead. `filter: invert()` on a score is forbidden: it
 * ruins glyph weight, breaks coloured annotation, and prints wrong.
 *
 * These are string transforms rather than DOM surgery so the worker can run
 * them before it ever builds a tree, and so they are testable without a
 * browser. Each one targets an attribute Verovio itself writes, and none of
 * them can touch an `xml:id` — the id map is the engraver's contract with
 * the editor and must survive verbatim.
 */

/** ` color="black"` inside the `<svg class="definition-scale">` open tag. */
const DEFINITION_SCALE_TAG = /<svg\b[^>]*\bclass="definition-scale"[^>]*>/g;
const BLACK_COLOUR_ATTRIBUTE = /\s+color="(?:black|#000|#000000)"/gi;

/** Verovio's serif fallback for score text; Academico comes from the tokens. */
const TIMES_FALLBACK = /\s+font-family="Times(?:,\s*serif)?"/g;

/**
 * A `fill=` or `stroke=` set to black anywhere in the tree.
 *
 * Only black is matched, which is how §3.5 is honoured: a colour the MEI
 * itself asked for is by construction not one of these literals, so it
 * survives untouched without needing an allowlist to be maintained.
 */
const BLACK_PAINT = /\s+(fill|stroke)="(?:black|#000|#000000)"/gi;

/** The `viewBox` of the `.definition-scale` element — the page's own units. */
const PAGE_VIEW_BOX =
  /<svg\b[^>]*\bclass="definition-scale"[^>]*\bviewBox="0 0 ([\d.]+) ([\d.]+)"/;

export function sanitize(svg: string): string {
  return svg
    .replace(DEFINITION_SCALE_TAG, (tag) =>
      tag.replace(BLACK_COLOUR_ATTRIBUTE, ""),
    )
    .replace(TIMES_FALLBACK, "")
    .replace(
      BLACK_PAINT,
      (_match, attribute: string) => ` ${attribute}="currentColor"`,
    );
}

/**
 * The page's own coordinate system, in which every overlay is drawn.
 *
 * Verovio nests a `.definition-scale` element whose `viewBox` is the page in
 * hundredths of a millimetre; the overlay layer adopts it verbatim so a halo
 * measured in staff spaces lands on the glyph at every zoom
 * (`01-visual-language.md` §4).
 */
export function pageBox(svg: string): { width: number; height: number } {
  const match = PAGE_VIEW_BOX.exec(svg);
  if (!match) return { width: 0, height: 0 };
  return { width: Number(match[1]), height: Number(match[2]) };
}

/**
 * Every `xml:id` in the document, in order. Used by the tests to assert the
 * sanitizer preserved the id map, and by nothing else.
 */
export function identifiers(svg: string): string[] {
  return [...svg.matchAll(/\bid="([^"]+)"/g)].map((match) => match[1]);
}
