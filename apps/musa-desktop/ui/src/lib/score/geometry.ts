/**
 * Where an engraved event sits, in the page's own coordinate system.
 *
 * Overlays are never drawn by mutating Verovio's SVG — that fights the next
 * re-render and corrupts the id map (`02-engraving.md` §8). They are drawn in
 * a separate layer that shares the page's coordinates, which is also why they
 * can be measured in staff spaces and stay correct at every zoom.
 */

import { elementsOf } from "./ids";

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** The `.definition-scale` element, whose viewBox is the page's coordinates. */
function pageRoot(container: ParentNode): SVGGraphicsElement | null {
  return container.querySelector<SVGGraphicsElement>("svg.definition-scale");
}

/** One engraved element's box, expressed in the page's coordinates. */
function boxOf(root: SVGGraphicsElement, element: SVGGraphicsElement): Rect | null {
  const rootMatrix = root.getScreenCTM();
  const elementMatrix = element.getScreenCTM();
  if (!rootMatrix || !elementMatrix) return null;
  const toPage = rootMatrix.inverse().multiply(elementMatrix);
  const local = element.getBBox();
  const corners = [
    new DOMPoint(local.x, local.y),
    new DOMPoint(local.x + local.width, local.y),
    new DOMPoint(local.x, local.y + local.height),
    new DOMPoint(local.x + local.width, local.y + local.height),
  ].map((point) => point.matrixTransform(toPage));
  const xs = corners.map((point) => point.x);
  const ys = corners.map((point) => point.y);
  const x = Math.min(...xs);
  const y = Math.min(...ys);
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
}

/**
 * Every box that draws `id` — more than one when the event is tied across a
 * barline, so the halo follows the note wherever it sounds.
 */
export function boxesFor(container: ParentNode, id: string): Rect[] {
  const root = pageRoot(container);
  if (!root) return [];
  return elementsOf(container, id)
    .filter((element): element is SVGGraphicsElement => "getBBox" in element)
    .map((element) => boxOf(root, element))
    .filter((rect): rect is Rect => rect !== null && rect.width > 0 && rect.height > 0);
}

/** A box grown by `spaces` staff spaces on every side. */
export function pad(rect: Rect, spaces: number, staffSpace: number): Rect {
  const grow = spaces * staffSpace;
  return {
    x: rect.x - grow,
    y: rect.y - grow,
    width: rect.width + grow * 2,
    height: rect.height + grow * 2,
  };
}
