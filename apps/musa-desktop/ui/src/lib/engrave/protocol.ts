/**
 * The messages that cross the worker boundary, and the layout vocabulary
 * shared by both sides (docs/interface/02-engraving.md §§1–4).
 */

/** Verovio layout options. Defaults are fixed in `options.ts`. */
export interface LayoutOptions {
  /** Page width in Verovio units (1/100 mm). */
  pageWidth: number;
  /** Page height in Verovio units. */
  pageHeight: number;
  /** Zoom as a percentage; a re-layout, never a CSS transform (§5). */
  zoom: number;
  /** System breaking. */
  breaks: "auto" | "encoded" | "none";
  /** Continuous view lets the page grow to its content. */
  adjustPageHeight: boolean;
}

/** What a load or relayout produced. */
export interface Layout {
  /** How many pages the score laid out to. */
  pages: number;
  /** One staff space in the page's own SVG user units. */
  staffSpace: number;
  /** Every engraved event id, in document order. */
  ids: string[];
  /** The generation this layout belongs to. */
  generation: number;
}

/** One rendered page. */
export interface PageSvg {
  /** The page number, 1-based. */
  page: number;
  /** Sanitized SVG markup. */
  svg: string;
  /** The page's own coordinate system, which the overlay layer adopts. */
  box: { width: number; height: number };
  /** The generation this page belongs to; stale ones are dropped. */
  generation: number;
}

export type Request =
  | { kind: "load"; generation: number; mei: string; options: LayoutOptions }
  | { kind: "relayout"; generation: number; options: LayoutOptions }
  | { kind: "page"; generation: number; page: number };

export type Response =
  | { kind: "layout"; id: number; layout: Layout }
  | { kind: "page"; id: number; page: PageSvg }
  | { kind: "error"; id: number; message: string };
