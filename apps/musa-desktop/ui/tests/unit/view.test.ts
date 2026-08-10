/**
 * The view preferences of `02-engraving.md` §§4–5: the mode a piece is read
 * in, and where a pinch lands on the zoom ladder.
 */

import { beforeEach, describe, expect, it } from "vitest";

import { ZOOM_STEPS } from "musa-engrave";
import { ViewPreferences, stepForPinch } from "../../src/lib/state/view.svelte";

const HUNDRED = ZOOM_STEPS.indexOf(100);

/**
 * The unit suite runs in node, which has no `localStorage`. This is the
 * smallest thing that is one — enough to prove the preference is written and
 * read back, which is the contract; the browser's own implementation is not
 * under test.
 */
function shelf(): Storage {
  const entries = new Map<string, string>();
  return {
    get length() {
      return entries.size;
    },
    clear: () => entries.clear(),
    getItem: (key: string) => entries.get(key) ?? null,
    key: (index: number) => [...entries.keys()][index] ?? null,
    removeItem: (key: string) => void entries.delete(key),
    setItem: (key: string, value: string) => void entries.set(key, value),
  };
}

describe("the view mode", () => {
  beforeEach(() => {
    Object.defineProperty(globalThis, "localStorage", { value: shelf(), configurable: true });
  });

  it("is pages until a piece was last read another way", () => {
    const views = new ViewPreferences();
    expect(views.mode("glass-mountain.musa")).toBe("page");
    views.choose("glass-mountain.musa", "continuous");
    expect(views.mode("glass-mountain.musa")).toBe("continuous");
  });

  it("is remembered per piece, not per application", () => {
    const views = new ViewPreferences();
    views.choose("sketch.musa", "continuous");
    expect(views.mode("quartet.musa")).toBe("page");
  });

  it("outlives the session it was chosen in", () => {
    new ViewPreferences().choose("sketch.musa", "continuous");
    expect(new ViewPreferences().mode("sketch.musa")).toBe("continuous");
  });

  it("survives a store holding something that is not a mode", () => {
    globalThis.localStorage?.setItem("musa:view-modes", '{"sketch.musa":"sideways"}');
    expect(new ViewPreferences().mode("sketch.musa")).toBe("page");
  });
});

describe("a settled pinch", () => {
  it("lands on the nearest rung of the ladder, never between rungs", () => {
    const landed = stepForPinch(HUNDRED, 1.3);
    expect(ZOOM_STEPS[landed]).toBe(125);
    expect(ZOOM_STEPS).toContain(ZOOM_STEPS[stepForPinch(HUNDRED, 0.62)]);
  });

  it("holds still for a gesture too small to change the step", () => {
    expect(stepForPinch(HUNDRED, 1.02)).toBe(HUNDRED);
  });

  it("cannot walk off either end of the ladder", () => {
    expect(stepForPinch(0, 0.1)).toBe(0);
    expect(stepForPinch(ZOOM_STEPS.length - 1, 10)).toBe(ZOOM_STEPS.length - 1);
  });
});
