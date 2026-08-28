/**
 * The marks the performance budgets are measured from
 * (`docs/rules/desktop/06-frame-budgets.md` §2).
 *
 * `performance.mark` is cheap, but it is not free and it is not needed in a
 * release build, so the marks are behind a flag: `?perf=1`, which the headless
 * harness sets. Nothing in the application reads them back — they exist so a
 * budget can be asserted as a measurement rather than as an impression.
 */

/** The moments the budget table names. */
export type Moment =
  /** The shell frame is on screen: margins, title, transport (B6). */
  | "shell"
  /** The first engraved page is on screen (B7, B2, B8). */
  | "score"
  /** A keystroke started the debounce (B1). */
  | "edit"
  /** A zoom step was asked for, so a re-layout is owed (B8). */
  | "zoom"
  /** A snapshot arrived, so diagnostics are current (B1). */
  | "snapshot"
  /** A note was chosen, by pointer or by key (B3, B4). */
  | "select"
  /** The selection halo is measured and drawn (B3). */
  | "halo"
  /** The inspector is showing the chosen note's facts (B4). */
  | "inspector"
  /** An origin segment was clicked, so a new selection is owed (B9). */
  | "origin"
  /** Origin view was asked for, so the lens is owed (B9). */
  | "lens"
  /*
   * Review's own moments (`06-frame-budgets.md` §7). That section asks for
   * three quantities to be *measured* before the table names a threshold, so
   * these exist to produce the measurement rather than to assert a number
   * nobody has taken yet.
   */
  /** A review gesture was made, so a replacement reading is owed. */
  | "reviewAct"
  /** The A or B audition was asked for. */
  | "reviewAudition"
  /** The replacement reading is on the leaf. */
  | "reviewDrawn"
  /** Focus is back on the note the composer was reading. */
  | "reviewFocus"
  /*
   * The rest of the keyboard workflow (`06-frame-budgets.md` §7). One take
   * goes Capture or Keep that → a proposal → group revisions → kept, and each
   * arrow is a quantity that section asks to be measured on the real path
   * rather than on a function.
   */
  /** A capture door was pressed, so a first proposal is owed. */
  | "capture"
  /** A group command was issued, so a preview is owed. */
  | "group"
  /** The group preview is on screen. */
  | "groupDrawn"
  /** The accepted phrase was asked for, so a revision and ink are owed. */
  | "keep"
  /** An undo was asked for, so the previous ink is owed. */
  | "undo";

const enabled =
  typeof globalThis.location !== "undefined" && new URLSearchParams(globalThis.location.search).has("perf");

/** Record that a budgeted moment just happened. */
export function mark(moment: Moment): void {
  if (!enabled) return;
  performance.mark(`musa:${moment}`);
}
