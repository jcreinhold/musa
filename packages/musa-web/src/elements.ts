/**
 * The `<musa-score>` element (prompt 149). Deliberately dumb: it carries the
 * source and hosts the shadow root, and `typeset()` does the work — batching
 * and idempotence are the scanner's job, not each element's.
 */

/** Register the element, once. Called at import; safe to call again. */
export function registerElements(): void {
  if (customElements.get("musa-score") !== undefined) return;
  customElements.define("musa-score", class extends HTMLElement {});
}
