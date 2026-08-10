/**
 * Configuration for `@musa/web`. Set it before the first `parse`/`render`;
 * the MathJax config-before-load convention, available both as this function
 * and as a `window.MusaWeb` object set before the module loads (read once at
 * import, then `configure` takes over).
 */

export interface WebConfig {
  /**
   * Where the musa wasm module is served from. Defaults to the `wasm/`
   * directory beside the package's module — right for anything serving the
   * package as-is; set it when a bundler relocates assets.
   */
  wasmUrl?: string;
  /**
   * A CSS selector for additional snippet containers, beyond the built-in
   * forms (`text/musa` script tags, `<musa-score>`, `[data-musa]`).
   */
  selector?: string;
  /** Typeset the whole document on load, without an explicit call. */
  autoStart?: boolean;
  /** After each batch, the elements that were typeset. */
  onTypeset?: (elements: Element[]) => void;
  /** A snippet or the environment failed; the box is already shown. */
  onError?: (error: Error, element: Element | null) => void;
  /**
   * Fires for clicks on event-mapped notation. The id is the compiler's
   * EventId hex — tie-piece suffixes already stripped.
   */
  onEventClick?: (eventId: string, target: SVGElement, context: MusaElementContext) => void;
  /** Entering an event fires its id; leaving the last one fires null. */
  onEventHover?: (eventId: string | null, target: SVGElement | null) => void;
}

/** The page-side context an event callback receives about its score. */
export interface MusaElementContext {
  /** The `<musa-score>` or container this event lives in. */
  element: Element;
  /** The snippet's source text. */
  source: string;
}

let config: WebConfig = {};

export function configure(next: WebConfig): void {
  config = { ...config, ...next };
}

/** The configuration as it stands. Internal; the surface is `configure`. */
export function getConfig(): WebConfig {
  return config;
}
