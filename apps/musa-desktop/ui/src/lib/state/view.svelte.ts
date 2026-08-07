/**
 * How a piece is laid out on the leaf, remembered per piece
 * (`docs/interface/02-engraving.md` §4).
 *
 * Per piece rather than globally, because the choice belongs to the music: a
 * single-line sketch is written in continuous view and a four-part score is
 * read in pages, and a composer who works on both should not have to re-choose
 * every time they switch. It is a preference, not a document property — it
 * lives beside the application, never in the `.musa` source.
 */

import type { ViewMode } from "../engrave/options";
import { ZOOM_STEPS } from "../engrave/options";

const STORE = "musa:view-modes";

/** Local storage, when there is one and it is allowed. */
function shelf(): Storage | undefined {
  try {
    return globalThis.localStorage ?? undefined;
  } catch {
    // A hardened WebView can refuse storage outright. A forgotten preference
    // is not worth a broken screen.
    return undefined;
  }
}

function load(): Record<string, ViewMode> {
  try {
    const stored: unknown = JSON.parse(shelf()?.getItem(STORE) ?? "{}");
    if (typeof stored !== "object" || stored === null) return {};
    return Object.fromEntries(
      Object.entries(stored as Record<string, unknown>).filter(
        ([, mode]) => mode === "page" || mode === "continuous",
      ),
    ) as Record<string, ViewMode>;
  } catch {
    return {};
  }
}

export class ViewPreferences {
  #modes = $state<Record<string, ViewMode>>(load());

  /** Page, until this piece was last read another way. */
  mode(piece: string): ViewMode {
    return this.#modes[piece] ?? "page";
  }

  choose(piece: string, mode: ViewMode): void {
    this.#modes = { ...this.#modes, [piece]: mode };
    try {
      shelf()?.setItem(STORE, JSON.stringify(this.#modes));
    } catch {
      // Same as above: the choice still holds for this session.
    }
  }
}

/**
 * The zoom step a settled pinch lands on (§5).
 *
 * A pinch is continuous and the zoom ladder is not, so the gesture picks the
 * rung nearest to where the fingers left off. Landing on a real step is what
 * keeps zoom a re-layout: there is no in-between scale to hold as a transform.
 */
export function stepForPinch(step: number, factor: number): number {
  const from = ZOOM_STEPS[step] ?? 100;
  const wanted = from * factor;
  return ZOOM_STEPS.reduce(
    (best, level, index) =>
      Math.abs(level - wanted) < Math.abs((ZOOM_STEPS[best] ?? 100) - wanted) ? index : best,
    step,
  );
}
