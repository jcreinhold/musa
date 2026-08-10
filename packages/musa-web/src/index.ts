/**
 * `@musa/web` — typeset musa scores in the browser (and in Node). Rust owns
 * MEI; the strings this package returns are projections, never edited here.
 */

export { parse, render } from "./api";
export { configure, type WebConfig } from "./configure";
export type { LayoutOptions } from "musa-engrave";
export type { MusaDiagnostic, MusaLabel, RenderOptions, RenderResult } from "./types";
