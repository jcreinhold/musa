/**
 * `@musa/web` — typeset musa scores in the browser (and in Node). Rust owns
 * MEI; the strings this package returns are projections, never edited here.
 */

export { parse, render } from "./api";
export { configure, type MusaElementContext, type WebConfig } from "./configure";
export { highlight } from "./interaction";
export { typeset, type TypesetOptions } from "./typeset";
export type { LayoutOptions } from "musa-engrave";
export type { MusaDiagnostic, MusaLabel, RenderOptions, RenderResult } from "./types";

import { configure, type WebConfig } from "./configure";
import { registerElements } from "./elements";
import { typeset } from "./typeset";

declare global {
  interface Window {
    /** Config-before-load, the MathJax convention; read once at import. */
    MusaWeb?: WebConfig;
  }
}

// The browser-only conveniences: element registration, config-before-load,
// and opt-in auto-start. Node skips all three (its DOM is somebody else's).
if (typeof window !== "undefined" && typeof document !== "undefined") {
  registerElements();
  const preset = window.MusaWeb;
  if (preset !== undefined) configure(preset);

  // `document.currentScript` is only set for classic scripts (the CDN build);
  // module scripts use `window.MusaWeb = { autoStart: true }` instead.
  const script = document.currentScript;
  const autoStart = preset?.autoStart === true || script?.hasAttribute("data-musa-autostart") === true;
  if (autoStart) {
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", () => void typeset(), { once: true });
    } else {
      void typeset();
    }
  }
}
