/**
 * How the composer reads and types.
 *
 * Both preferences are state of the *app*: they outlive the session, they
 * never reach the document, and a stored value that is not one of the four
 * steps is not a size.
 */

import { beforeEach, describe, expect, it } from "vitest";

import { Preferences, SOURCE_FLOOR, TEXT_SIZES } from "../../src/lib/session/preferences.svelte";

/** The unit suite runs in node, which has no `localStorage`. */
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

beforeEach(() => {
  Object.defineProperty(globalThis, "localStorage", {
    value: shelf(),
    configurable: true,
  });
});

describe("the text size", () => {
  it("is Normal until somebody says otherwise", () => {
    const preferences = new Preferences();
    preferences.start();
    expect(preferences.textSize).toBe("normal");
    expect(preferences.scale).toBe(1);
  });

  it("steps through the four sizes and no others", () => {
    const preferences = new Preferences();
    const walked = [preferences.textSize];
    for (let step = 0; step < 5; step += 1) {
      preferences.stepText(1);
      walked.push(preferences.textSize);
    }
    // Five steps up from Normal, and the ladder is three rungs long: it
    // stops at Larger rather than inventing a fifth size.
    expect(walked).toEqual(["normal", "large", "larger", "larger", "larger", "larger"]);
    expect(new Set(walked).size).toBeLessThanOrEqual(TEXT_SIZES.length);
  });

  it("cannot walk off the small end either", () => {
    const preferences = new Preferences();
    for (let step = 0; step < 4; step += 1) preferences.stepText(-1);
    expect(preferences.textSize).toBe("small");
  });

  it("outlives the session it was chosen in", () => {
    const chosen = new Preferences();
    chosen.stepText(1);
    const later = new Preferences();
    later.start();
    expect(later.textSize).toBe("large");
  });

  it("goes back to Normal, and stays there across a restart", () => {
    const chosen = new Preferences();
    chosen.stepText(1);
    chosen.resetText();
    const later = new Preferences();
    later.start();
    expect(later.textSize).toBe("normal");
  });

  it("falls back to Normal when the store holds something that is not a size", () => {
    // `localStorage` is writable by anything that can reach the origin, and a
    // scale of `999` is a broken window rather than a preference.
    globalThis.localStorage?.setItem("musa.text-size", "999");
    const preferences = new Preferences();
    preferences.start();
    expect(preferences.textSize).toBe("normal");
  });
});

describe("vim mode", () => {
  it("is off until it is asked for, and then stays on", () => {
    const chosen = new Preferences();
    chosen.start();
    expect(chosen.vim).toBe(false);
    chosen.toggleVim();

    const later = new Preferences();
    later.start();
    expect(later.vim).toBe(true);
  });

  it("goes back off, and that is remembered too", () => {
    const chosen = new Preferences();
    chosen.toggleVim();
    chosen.toggleVim();
    const later = new Preferences();
    later.start();
    expect(later.vim).toBe(false);
  });
});

describe("recent MIDI memory", () => {
  it("is on by default and remembers when the musician turns it off", () => {
    const chosen = new Preferences();
    chosen.start();
    expect(chosen.recentMidi).toBe(true);
    chosen.setRecentMidi(false);

    const later = new Preferences();
    later.start();
    expect(later.recentMidi).toBe(false);
  });
});

/**
 * The source column's width.
 *
 * It joins the other three because it is the same kind of thing — the app's,
 * not the document's — and because a column a composer widens once and has to
 * widen again next launch is not a preference.
 */
describe("the source width", () => {
  it("is the measure until it is dragged, which is stored as nothing", () => {
    const chosen = new Preferences();
    chosen.start();
    expect(chosen.sourceWidth).toBeNull();

    chosen.widenSource(700);

    const later = new Preferences();
    later.start();
    expect(later.sourceWidth).toBe(700);

    // Back to the default stores *no* width rather than the measure's pixels,
    // so the column follows the measure when the type size changes.
    later.resetSource();
    expect(globalThis.localStorage?.getItem("musa.source-width")).toBeNull();
    const after = new Preferences();
    after.start();
    expect(after.sourceWidth).toBeNull();
  });

  it("refuses to be dragged below the floor", () => {
    const preferences = new Preferences();
    preferences.widenSource(40);
    expect(preferences.sourceWidth).toBe(SOURCE_FLOOR);
  });

  it("ignores a stored width that is not one", () => {
    // The same reasoning as the text size: `localStorage` is writable by
    // anything that can reach the origin, and a column three characters wide
    // is a broken window rather than a preference.
    for (const junk of ["", "wide", "-1", "12"]) {
      globalThis.localStorage?.setItem("musa.source-width", junk);
      const preferences = new Preferences();
      preferences.start();
      expect(preferences.sourceWidth, junk).toBeNull();
    }
  });
});
