/**
 * Light or dark, following the system until the user says otherwise.
 *
 * The token system already answers `prefers-color-scheme`; this only exists
 * for the override, which it applies by stamping `data-theme` on the root —
 * the same attribute the review URLs use, so there is one mechanism rather
 * than two.
 */

export type Theme = "light" | "dark";

const KEY = "musa.theme";

function systemTheme(): Theme {
  return globalThis.matchMedia?.("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export class ThemeChoice {
  /** `null` means follow the system. */
  chosen = $state<Theme | null>(null);
  #system = $state<Theme>(systemTheme());

  get current(): Theme {
    return this.chosen ?? this.#system;
  }

  /** Start following the system and restore any previous override. */
  start(): () => void {
    const stored = globalThis.localStorage?.getItem(KEY);
    if (stored === "light" || stored === "dark") this.chosen = stored;
    this.apply();

    const query = globalThis.matchMedia?.("(prefers-color-scheme: dark)");
    const follow = (event: MediaQueryListEvent): void => {
      this.#system = event.matches ? "dark" : "light";
      this.apply();
    };
    query?.addEventListener("change", follow);
    return () => query?.removeEventListener("change", follow);
  }

  /** Switch to the other theme, which also makes the choice explicit. */
  toggle(): void {
    this.choose(this.current === "dark" ? "light" : "dark");
  }

  /**
   * Take one of the three states, `null` being *follow the system*.
   *
   * The third state was always here and was reachable by nothing: `toggle`
   * only ever moves between light and dark, so a composer who overrode the
   * theme once could not get back to following the system.
   */
  choose(theme: Theme | null): void {
    if (theme === null) return this.follow();
    this.chosen = theme;
    globalThis.localStorage?.setItem(KEY, theme);
    this.apply();
  }

  /** Go back to whatever the system says. */
  follow(): void {
    this.chosen = null;
    globalThis.localStorage?.removeItem(KEY);
    this.apply();
  }

  private apply(): void {
    const root = globalThis.document?.documentElement;
    if (!root) return;
    if (this.chosen === null) root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", this.chosen);
  }
}
