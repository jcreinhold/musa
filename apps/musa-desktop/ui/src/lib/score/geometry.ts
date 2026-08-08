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

/**
 * The noteheads that draw `id`, rather than everything the engraver drew.
 *
 * A stem, a flag, and a ledger line are all part of an event's box and none of
 * them is where the note *is*. A mark meant to sit under the head has to ask
 * for the head, or a stem-down quarter gets its hairline two staff spaces
 * below the note it belongs to. Rests and anything else Verovio draws without
 * a head fall back to the whole element, which is the right answer for them.
 */
export function headsFor(container: ParentNode, id: string): Rect[] {
  const root = pageRoot(container);
  if (!root) return [];
  return elementsOf(container, id)
    .flatMap((element) => {
      const heads = [...element.querySelectorAll<SVGGraphicsElement>("g.notehead")];
      if (heads.length > 0) return heads;
      return "getBBox" in element ? [element as SVGGraphicsElement] : [];
    })
    .map((element) => boxOf(root, element))
    .filter((rect): rect is Rect => rect !== null && rect.width > 0 && rect.height > 0);
}

/** The smallest box containing both. */
function union(a: Rect, b: Rect): Rect {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  return {
    x,
    y,
    width: Math.max(a.x + a.width, b.x + b.width) - x,
    height: Math.max(a.y + a.height, b.y + b.height) - y,
  };
}

/**
 * A contiguous stretch of one expansion's output within one system, with the
 * system it landed in.
 *
 * Origin view brackets runs rather than events, because a composer reads "this
 * phrase came from `sigh()`", not "these five noteheads did"
 * (`04-provenance.md` §2). The system comes from the engraving's own structure
 * — Verovio's `g.system` — so the bracket sits in the margin the engraver laid
 * out rather than at a guessed offset.
 */
export interface Run {
  /** The system's own box: where its left margin is. */
  system: Rect;
  /** The union of this expansion's events inside that system. */
  run: Rect;
}

/** Where `ids` landed on this page, split by system, in system order. */
export function runsFor(container: ParentNode, ids: string[]): Run[] {
  const root = pageRoot(container);
  if (!root) return [];
  const runs = new Map<Element, Rect>();
  const systems = new Map<Element, Rect>();
  for (const id of ids) {
    for (const element of elementsOf(container, id)) {
      if (!(element instanceof SVGGraphicsElement)) continue;
      const rect = boxOf(root, element);
      if (rect === null || rect.width <= 0 || rect.height <= 0) continue;
      const owner = element.closest<SVGGraphicsElement>("g.system") ?? root;
      const known = runs.get(owner);
      runs.set(owner, known ? union(known, rect) : rect);
      if (!systems.has(owner)) {
        const box = boxOf(root, owner);
        if (box) systems.set(owner, box);
      }
    }
  }
  const found: Run[] = [];
  for (const [owner, run] of runs) {
    const system = systems.get(owner);
    if (system) found.push({ system, run });
  }
  return found.sort((a, b) => a.run.y - b.run.y);
}

/**
 * One run's bracket, in the page's own coordinates: a span over the notes one
 * expansion produced, with the terminals an editor would draw.
 */
export interface Bracket {
  /** The occurrence this brackets, so a click on it can select it. */
  id: string;
  /** The expansion path as one line: `transpose down P5 ▸ sigh()`. */
  label: string;
  x: number;
  y: number;
  width: number;
}

/** How far the bracket's terminals hang down, in staff spaces. */
export const TICK_SPACES = 0.5;

/** How far the bracket clears the run's ink, in staff spaces. */
export const CLEARANCE_SPACES = 1.2;

/** How far the bracket over-hangs the run at each end, in staff spaces. */
export const OVERHANG_SPACES = 0.4;

/** The bracket over one run of one expansion's output. */
export function bracketOver(
  id: string,
  label: string,
  run: Rect,
  staffSpace: number,
): Bracket {
  const over = staffSpace * OVERHANG_SPACES;
  return {
    id,
    label,
    x: run.x - over,
    y: run.y - staffSpace * CLEARANCE_SPACES,
    width: run.width + over * 2,
  };
}

/** The hover trace: one hairline from a note out to its bracket. */
export interface Trace {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

/**
 * The trace from a note up to its bracket's left terminal. One line, drawn
 * instantly (`04-provenance.md` §2).
 */
export function traceTo(bracket: Bracket, rect: Rect): Trace {
  return { x1: rect.x + rect.width / 2, y1: rect.y, x2: bracket.x, y2: bracket.y };
}

/**
 * The bracket of `id` this note belongs to: the one whose span contains it,
 * or failing that the nearest — a run and its bracket are the same notes, so
 * containment is the answer except at the rounding of a box edge.
 */
export function bracketNear(brackets: Bracket[], id: string, rect: Rect): Bracket | undefined {
  const mid = rect.x + rect.width / 2;
  const off = (bracket: Bracket) =>
    Math.max(bracket.x - mid, mid - (bracket.x + bracket.width), 0) + Math.abs(bracket.y - rect.y);
  return brackets
    .filter((bracket) => bracket.id === id)
    .reduce<Bracket | undefined>(
      (best, bracket) => (best === undefined || off(bracket) < off(best) ? bracket : best),
      undefined,
    );
}

/**
 * Everything drawn over one page.
 *
 * Marks are per page, not per score: each page has its own coordinate system,
 * and a box measured on one page would land somewhere arbitrary on another.
 */
export interface Marks {
  selection: Rect[];
  hover: Rect[];
  /**
   * The focus (prompt 52): a hairline under every notehead the focused
   * statement spelled. Measured on the heads rather than on the event boxes,
   * and never grown — a halo says *chosen*, a hairline says *this one*.
   */
  focus: Rect[];
  playing: Rect[];
  caret: Rect | null;
  loop: { from: Rect; to: Rect } | null;
  /** Systems a diagnostic points at, flashed once (`05-states.md` §5). */
  flash: Rect[];
  /** Origin view's margin brackets, empty unless the lens is held. */
  brackets: Bracket[];
  trace: Trace | null;
}

/** A page with nothing on it — the shared empty value, allocated once. */
export const NOTHING: Marks = Object.freeze({
  selection: [],
  hover: [],
  focus: [],
  playing: [],
  caret: null,
  loop: null,
  flash: [],
  brackets: [],
  trace: null,
});

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
