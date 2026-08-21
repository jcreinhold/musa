/**
 * The printed front matter, read from the frontend's side.
 *
 * The MEI backend writes the page's head and foot itself rather than letting
 * Verovio invent one, and every line it writes carries an `xml:id` — so the
 * title on the page is findable the same way a notehead is, and clicking it
 * is the same gesture reaching a different field. This module is the other
 * half of that contract: the ids here and the `FRONT_*` constants in
 * `musa-notation`'s `mei.rs` are one list, and they change together.
 *
 * Only these five lines are editable on the page; part names, dynamics, and
 * tempo marks are not.
 */

import type { HeaderFieldDto } from "../session/generated/HeaderFieldDto";

/** The five printed lines, by the id the engraving gives each of them. */
const FIELDS: Record<string, HeaderFieldDto> = {
  "front-title": "title",
  "front-subtitle": "subtitle",
  "front-composer": "composer",
  "front-arranger": "arranger",
  "front-copyright": "copyright",
};

/** The field a printed line's id names, or null if the id names none. */
export function fieldForId(id: string | null | undefined): HeaderFieldDto | null {
  return (id && FIELDS[id]) || null;
}

/** The line of front matter an engraved element belongs to, if it is one. */
export function frontFieldOf(element: Element | null): HeaderFieldDto | null {
  return fieldForId(element?.closest('[id^="front-"]')?.getAttribute("id"));
}

/** Where that line is drawn, so an input can be put exactly over it. */
export interface FrontMatterAt {
  field: HeaderFieldDto;
  /** Its box in the score pane's own coordinates, scroll included. */
  left: number;
  top: number;
  width: number;
  height: number;
  /** The size and alignment the page set it in, so the input matches. */
  fontSize: number;
  align: "left" | "center" | "right";
}

/** The alignment Verovio drew a line at, read off the text it anchored. */
function alignOf(element: Element): FrontMatterAt["align"] {
  const anchor = element.closest("[text-anchor]")?.getAttribute("text-anchor");
  if (anchor === "middle") return "center";
  if (anchor === "end") return "right";
  return "left";
}

/**
 * Measure a line of front matter against its scroll container.
 *
 * The box comes from the DOM rather than from the layout, because the
 * engraving is an SVG scaled to the leaf and only the browser knows what that
 * came to. Nothing musical is computed here: this is where a control goes,
 * which is the same thing a halo's rectangle is.
 */
export function measureFront(container: HTMLElement, element: Element, field: HeaderFieldDto): FrontMatterAt {
  const box = element.getBoundingClientRect();
  const frame = container.getBoundingClientRect();
  return {
    field,
    left: box.left - frame.left + container.scrollLeft,
    top: box.top - frame.top + container.scrollTop,
    width: box.width,
    height: box.height,
    fontSize: renderedFontSize(element, box.width),
    align: alignOf(element),
  };
}

/**
 * The size the page drew this line at, in screen pixels.
 *
 * The engraving states its type in page units and is then scaled to the leaf,
 * so neither number alone is the answer: the ratio between the line's own
 * bounding box and the box the browser laid it out in is the magnification,
 * and the declared size times that is what the reader is actually seeing.
 */
function renderedFontSize(element: Element, renderedWidth: number): number {
  const inner = element.querySelector("[font-size]") ?? element;
  const declared = Number.parseFloat(globalThis.getComputedStyle(inner).fontSize);
  const measured = (element as SVGGraphicsElement).getBBox?.();
  const scale = measured && measured.width > 0 ? renderedWidth / measured.width : 1;
  return Number.isFinite(declared) ? declared * scale : 16;
}
