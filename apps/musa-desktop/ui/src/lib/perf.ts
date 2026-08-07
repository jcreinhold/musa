/**
 * The marks the performance budgets are measured from
 * (`docs/interface/06-performance.md` §2).
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
  | "origin";

const enabled =
  typeof globalThis.location !== "undefined" &&
  new URLSearchParams(globalThis.location.search).has("perf");

/** Record that a budgeted moment just happened. */
export function mark(moment: Moment): void {
  if (!enabled) return;
  performance.mark(`musa:${moment}`);
}
