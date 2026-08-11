/**
 * The engraved `xml:id` contract, read from the frontend's side.
 *
 * Every note, chord, and rest is engraved as `event-<hex>`; a tie
 * continuation repeats its event with a `-tN` suffix, because one event can
 * be drawn as several noteheads. Selection is by event, so the suffix is
 * stripped on the way in and restored on the way out — a tied note highlights
 * in every bar it sounds through, and clicking either half selects the same
 * thing.
 */

const TIE_CONTINUATION = /-t\d+$/;

/** The event id an engraved element belongs to, or null if it is not one. */
export function eventIdOf(element: Element | null): string | null {
  const owner = element?.closest('[id^="event-"]');
  const id = owner?.getAttribute("id");
  return id ? id.replace(TIE_CONTINUATION, "") : null;
}

/** Every engraved element that draws part of `id`, in document order. */
export function elementsOf(root: ParentNode, id: string): Element[] {
  return [...root.querySelectorAll(`[id="${id}"], [id^="${id}-t"]`)];
}
