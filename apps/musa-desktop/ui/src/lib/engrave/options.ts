/**
 * Layout defaults — docs/interface/02-engraving.md §4. Fixed here so they are
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
): Pick<LayoutOptions, "pageWidth" | "pageHeight"> {
  const units = (UNITS_PER_PIXEL * 100) / Math.max(zoom, 1);
  return {
    pageWidth: Math.max(Math.round(width * units), 500),
    pageHeight: Math.max(Math.round(height * units), 500),
  };
}

/** The Verovio option object for a layout. */
export function verovioOptions(options: LayoutOptions): Record<string, unknown> {
  return {
    font: "Bravura",
    breaks: options.breaks,
    pageWidth: options.pageWidth,
    pageHeight: options.pageHeight,
    adjustPageHeight: options.adjustPageHeight,
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
    header: "none",
    footer: "none",
  };
}
