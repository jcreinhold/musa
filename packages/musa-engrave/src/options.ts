/**
 * Layout defaults — docs/rules/desktop/02-engraving.md §4. Fixed here so they are
 * a decision rather than an accident.
 */

import type { LayoutOptions } from "./protocol";

/**
 * Verovio page units per CSS pixel at 100 % zoom.
 *
 * Page dimensions are tenths of a millimetre (A4 is `pageWidth: 2100`), and a
 * CSS pixel is 1/96 inch, so one pixel is 25.4/96 mm — 2.6458 units. Getting
 * this wrong does not fail; it silently engraves a score at the wrong rastral
 * size, which is why it is derived here rather than tuned by eye.
 */
const UNITS_PER_PIXEL = (25.4 / 96) * 10;

/**
 * How the score is broken into pages (§4).
 *
 * Page is the default — a leaf, per the thesis. Continuous lays the whole
 * piece out as one system and scrolls horizontally; it is what following the
 * playhead prefers, and what a single line is easiest to write in.
 */
export type ViewMode = "page" | "continuous";

/**
 * How wide a continuous layout is allowed to run before Verovio breaks it:
 * a hundred metres of paper, which is longer than any single system musa can
 * currently produce. The page is trimmed back to its content, so this is a
 * ceiling rather than a size.
 */
export const CONTINUOUS_WIDTH = 1_000_000;

/**
 * The layout a view mode implies.
 *
 * Continuous keeps the page's *height* — the height is what fixes the staff
 * size, and a continuous view that re-scaled the music would make the mode
 * toggle a zoom control. Only the width is given away, to the one system and
 * the horizontal scrollbar.
 */
export function modeOptions(
  mode: ViewMode,
): Pick<LayoutOptions, "breaks" | "adjustPageHeight" | "adjustPageWidth"> {
  return mode === "continuous"
    ? { breaks: "none", adjustPageHeight: false, adjustPageWidth: true }
    : { breaks: "auto", adjustPageHeight: false, adjustPageWidth: false };
}

/** The discrete zoom steps (§5). Zoom is a re-layout, never a transform. */
export const ZOOM_STEPS = [50, 75, 90, 100, 125, 150, 200] as const;

export const DEFAULT_LAYOUT: LayoutOptions = {
  pageWidth: 2100,
  pageHeight: 2970,
  zoom: 100,
  breaks: "auto",
  // Page view is the default — a leaf, per the thesis (§4). Letting the page
  // shrink to its content turns the sheet into a strip of music floating on
  // the surround, which is the continuous view and an explicit mode.
  adjustPageHeight: false,
  adjustPageWidth: false,
};

/**
 * The page size that fills a leaf of `width` × `height` CSS pixels.
 *
 * Zoom is expressed here, in the page, and not as a `scale` — that is what
 * makes it a re-layout rather than a transform (§5). Zooming in shrinks the
 * page the music is laid out on, so the same pixels hold less music at a
 * larger rastral size, and every hairline and beam is re-engraved at its
 * proper optical weight instead of being stretched.
 */
export function pageFor(
  width: number,
  height: number,
  zoom: number,
  mode: ViewMode = "page",
): Pick<LayoutOptions, "pageWidth" | "pageHeight"> {
  const units = (UNITS_PER_PIXEL * 100) / Math.max(zoom, 1);
  return {
    // Continuous is one system, so the width is the scroll rather than the
    // leaf; `adjustPageWidth` trims what this allows down to the music.
    pageWidth:
      mode === "continuous"
        ? CONTINUOUS_WIDTH
        : Math.max(Math.round(width * units), 500),
    pageHeight: Math.max(Math.round(height * units), 500),
  };
}

/**
 * CSS pixels per SVG user unit at a given zoom.
 *
 * Verovio states page sizes in tenths of a millimetre but draws in hundredths
 * — the `definition-scale` viewBox is ten times the page — so a caller that
 * sizes an element from a rendered box works in these units, not in page
 * units. Continuous view needs it: with no page width to fill, the scale is
 * the only thing that fixes how large the music is drawn.
 */
export function pixelsPerUnit(zoom: number): number {
  return zoom / 100 / (UNITS_PER_PIXEL * 10);
}

/** The Verovio option object for a layout. */
export function verovioOptions(
  options: LayoutOptions,
): Record<string, unknown> {
  // Verovio bounds page dimensions to [100, 100000] (a hundred metres was
  // never really on offer); out-of-bounds values are refused with a console
  // error. Clamp here so the continuous ceiling stays silent and every
  // caller — desktop and web — gets the same layout.
  const clampPage = (units: number) => Math.min(Math.max(units, 100), 100_000);
  return {
    font: "Bravura",
    breaks: options.breaks,
    pageWidth: clampPage(options.pageWidth),
    pageHeight: clampPage(options.pageHeight),
    adjustPageHeight: options.adjustPageHeight,
    adjustPageWidth: options.adjustPageWidth,
    // Zoom lives in the page size, never in `scale`: see `pageFor`.
    scale: 100,
    svgViewBox: true,
    svgRemoveXlink: true,
    svgHtml5: false,
    justifyVertically: true,
    unit: 11,
    spacingStaff: 8,
    spacingSystem: 10,
    spacingNonLinear: 0.55,
    spacingLinear: 0.25,
    // The page's own front matter: title and subtitle centred, composer and
    // arranger to the right, a running head after page 1. Turning this off is
    // what made the page read as a run of staves rather than as an edition.
    //
    // Encoded rather than automatic, because Verovio's automatic head is
    // anonymous — every id in it is generated per render, so nothing on it
    // can be traced back to the statement that put it there. The `<pgHead>`
    // the MEI backend writes carries `front-title`, `front-subtitle`,
    // `front-composer`, and `front-arranger`, which is what makes clicking the
    // piece's name the same machinery as clicking one of its notes (prompt
    // 54). Measured against the automatic head: same words, same positions,
    // running page number intact.
    header: "encoded",
    // The foot is the piece's copyright and nothing else. Verovio's automatic
    // footer is its own credit line, which is not a fact about this piece; the
    // encoded footer is the `<pgFoot>` the MEI backend writes, and a piece
    // that claims no copyright gets no footer at all.
    footer: "encoded",
    // A measure number at the head of every system, which is the modern
    // editorial default — not one on every bar, which is a proof-reading
    // copy. Verovio spells that as an interval of 0: 0 is per-system, and
    // any n > 0 is "repeat every n bars". Measured, not assumed — the
    // option's name reads like the opposite of what it does.
    mnumInterval: 0,
  };
}
