/**
 * How the composer reads and types (prompt 55).
 *
 * Two decisions the app used to make on the composer's behalf: how large its
 * text is, and whether its editor is modal. Both are state of the *app* and
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
export const TEXT_SIZES: readonly TextSize[] = ["small", "normal", "large", "larger"];

const SIZE_KEY = "musa.text-size";
const VIM_KEY = "musa.vim";

function storedSize(): TextSize | null {
  const stored = globalThis.localStorage?.getItem(SIZE_KEY);
  return TEXT_SIZES.find((size) => size === stored) ?? null;
}

export class Preferences {
  /** How large the frame's text is. Never the score's — that is zoom. */
  textSize = $state<TextSize>("normal");
  /** Whether the source column is modal. */
  vim = $state(false);

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
    this.apply();
  }

  /** `+1` a step larger, `-1` a step smaller; the ends are the ends. */
  stepText(by: number): void {
    const at = TEXT_SIZES.indexOf(this.textSize);
    const next = TEXT_SIZES[Math.min(Math.max(at + by, 0), TEXT_SIZES.length - 1)];
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
