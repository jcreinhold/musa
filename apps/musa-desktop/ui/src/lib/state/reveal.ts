import type { Span } from "./snapshot";

/**
 * A place in the source to go to.
 *
 * Whether going there takes the keyboard is part of the request, because it
 * depends on why it was asked for. A diagnostic is somewhere the composer is
 * going to *fix*, so the caret arrives ready to type (`05-states.md` §5).
 * Choosing a note on the page is not: the text follows along, but the hands
 * stay where they were.
 */
export interface Reveal {
  span: Span;
  focus: boolean;
}
