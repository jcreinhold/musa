/**
 * The shared worker engraver (prompt 139): the render layer's entire surface
 * to the desktop UI and to `@musa/web` (docs/interface/02-engraving.md §1).
 *
 * No component outside this package touches a Verovio toolkit, an MEI string,
 * or a raw SVG string. Rust owns MEI; this is a projection, not a model.
 */

export type { Engraver, EngraverOptions } from "./engraver";
export { createEngraver } from "./engraver";
export type { Box, Layout, LayoutOptions, PageSvg } from "./protocol";
export {
  CONTINUOUS_WIDTH,
  DEFAULT_LAYOUT,
  ZOOM_STEPS,
  modeOptions,
  pageFor,
  pixelsPerUnit,
  verovioOptions,
  type ViewMode,
} from "./options";
export { identifiers, pageBox, sanitize } from "./sanitize";
