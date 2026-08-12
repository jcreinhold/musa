/**
 * How the composer reads and types.
 *
 * Two decisions the app leaves to the composer: how large its text is, and
 * whether its editor is modal. Both are state of the *app* and
 * never of the document — they do not appear in the file, in a revision, or in
 * the undo history (`05-states.md`).
 *
 * Shaped like `ThemeChoice`, and deliberately beside it rather than folded
 * into it: same `start()`, same `localStorage`, same stamp-on-the-root. There
 * is one preference mechanism in this app, and this is it.
 */

/** The four steps. Chrome uses no size between them (§3). */
export type TextSize = "small" | "normal" | "large" | "larger";

/** Each step's multiplier over the scale `01-visual-language.md` §3 fixes. */
const SCALE: Record<TextSize, number> = {
  small: 0.85,
  normal: 1,
  large: 1.15,
  larger: 1.3,
};

/** In order, so a step is an index and stepping past the end stops there. */
export const TEXT_SIZES: readonly TextSize[] = [
  "small",
  "normal",
  "large",
  "larger",
];

const SIZE_KEY = "musa.text-size";
const VIM_KEY = "musa.vim";
const WIDTH_KEY = "musa.source-width";

/**
 * The narrowest the source column may be asked to go, in pixels.
 *
 * Still wider than nine in ten lines in the language (§8), which is the point:
 * a floor is not a collapse. `⌘'` is how the column goes away.
 */
export const SOURCE_FLOOR = 300;

function storedSize(): TextSize | null {
  const stored = globalThis.localStorage?.getItem(SIZE_KEY);
  return TEXT_SIZES.find((size) => size === stored) ?? null;
}

/** A stored width is a number at or above the floor, or it is not a width. */
function storedWidth(): number | null {
  const stored = Number(globalThis.localStorage?.getItem(WIDTH_KEY));
  return Number.isFinite(stored) && stored >= SOURCE_FLOOR ? stored : null;
}

export class Preferences {
  /** How large the frame's text is. Never the score's — that is zoom. */
  textSize = $state<TextSize>("normal");
  /** Whether the source column is modal. */
  vim = $state(false);
  /**
   * How wide the source column was left, in pixels. `null` is the measure.
   *
   * Stored as *nothing* rather than as the measure's pixel value, so a column
   * at the default follows the measure when the type size changes instead of
   * freezing at the pixels the measure happened to be.
   */
  sourceWidth = $state<number | null>(null);

  /** The multiplier the tokens are written against. */
  get scale(): number {
    return SCALE[this.textSize];
  }

  /**
   * Restore both, and stamp the size on the root.
   *
   * A stored value that is not one of the four steps is not a size, and the
   * app falls back to Normal rather than trusting it: `localStorage` is
   * writable by anything that can reach the origin, and a `--type-scale` of
   * `999` is a broken window rather than a preference.
   */
  start(): void {
    this.textSize = storedSize() ?? "normal";
    this.vim = globalThis.localStorage?.getItem(VIM_KEY) === "on";
    this.sourceWidth = storedWidth();
    this.apply();
  }

  /** `+1` a step larger, `-1` a step smaller; the ends are the ends. */
  stepText(by: number): void {
    const at = TEXT_SIZES.indexOf(this.textSize);
    const next =
      TEXT_SIZES[Math.min(Math.max(at + by, 0), TEXT_SIZES.length - 1)];
    if (next) this.#size(next);
  }

  resetText(): void {
    this.#size("normal");
  }

  /**
   * Take one of the four steps directly.
   *
   * The keys step and the sheet chooses, which is the difference between the
   * two ways in: a key is "a bit bigger", a row of four is "that one".
   */
  chooseText(size: TextSize): void {
    this.#size(size);
  }

  toggleVim(): void {
    this.setVim(!this.vim);
  }

  setVim(on: boolean): void {
    this.vim = on;
    globalThis.localStorage?.setItem(VIM_KEY, on ? "on" : "off");
  }

  /**
   * Ask for a source column this wide.
   *
   * Only the floor is enforced here. The ceiling belongs to the seam, which
   * measures what the page can spare at the moment of the gesture, and to the
   * layout, which holds the column narrower than the ask on a window with no
   * room for it. What is *stored* is the ask itself, so a width chosen on a
   * large display comes back whole (`01-visual-language.md` §7).
   */
  widenSource(width: number): void {
    this.sourceWidth = Math.round(Math.max(width, SOURCE_FLOOR));
    globalThis.localStorage?.setItem(WIDTH_KEY, String(this.sourceWidth));
  }

  /** Back to the measure, which is stored as no width at all. */
  resetSource(): void {
    this.sourceWidth = null;
    globalThis.localStorage?.removeItem(WIDTH_KEY);
  }

  #size(next: TextSize): void {
    this.textSize = next;
    globalThis.localStorage?.setItem(SIZE_KEY, next);
    this.apply();
  }

  /**
   * One custom property on the root, which every `--t-*-size` is written
   * against. Normal removes it rather than setting `1`, so the tokens' own
   * fallback is what a fresh profile reads and there is one default and not
   * two.
   */
  apply(): void {
    const root = globalThis.document?.documentElement;
    if (!root) return;
    if (this.textSize === "normal") root.style.removeProperty("--type-scale");
    else root.style.setProperty("--type-scale", String(this.scale));
  }
}
