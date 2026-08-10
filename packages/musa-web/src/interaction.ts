/**
 * Provenance interaction (prompt 142): every engraved note knows its event.
 * One delegated listener per typeset target; the MEI `xml:id` contract
 * (`event-<hex>`, tie pieces `-tN`) does all the work — there is no
 * "unidentified object" state because unmapped elements simply do not fire.
 */

import { getConfig, type MusaElementContext } from "./configure";

export type { MusaElementContext };

/** The id contract, as a pattern: the capture is the EventId; `-tN` is a tie piece. */
const EVENT_ID = /^event-([0-9a-f]+)(?:-t\d+)?$/;

/** Where listeners are attached, for `highlight`: target → its host element. */
const targets = new Map<Element | ShadowRoot, Element>();

/**
 * Attach the delegated listeners to one typeset target. Shadow-root-aware:
 * the listener lives on the shadow root itself, so `event.target` is the
 * hit element, not a retargeted host. Idempotent per target.
 */
export function attachInteraction(target: Element | ShadowRoot, context: MusaElementContext): void {
  if (targets.has(target)) return;
  targets.set(target, context.element);

  target.addEventListener("click", (event) => {
    const hit = eventHit(event.target);
    if (hit === null) return;
    getConfig().onEventClick?.(hit.id, hit.element, context);
  });

  // Hover fires on transitions only: entering an event fires its id,
  // leaving the last one fires null.
  let hovering: string | null = null;
  target.addEventListener("pointerover", (event) => {
    const hit = eventHit(event.target);
    const id = hit?.id ?? null;
    if (id === hovering) return;
    hovering = id;
    getConfig().onEventHover?.(id, hit?.element ?? null);
  });
  target.addEventListener("pointerout", (event) => {
    if (hovering === null) return;
    const entering = eventHit((event as PointerEvent).relatedTarget);
    if (entering?.id === hovering) return;
    hovering = null;
    getConfig().onEventHover?.(null, null);
  });
}

/** The event an event target belongs to, walking up to the id-bearing group. */
function eventHit(target: EventTarget | null): { id: string; element: SVGElement } | null {
  let node = target as Element | null;
  while (node instanceof Element) {
    const match = EVENT_ID.exec(node.getAttribute("id") ?? "");
    if (match?.[1] !== undefined) return { id: match[1], element: node as unknown as SVGElement };
    node = node.parentElement;
  }
  return null;
}

/** Mark every mapped element of a freshly typeset target with `.musa-event`. */
export function markEvents(target: Element | ShadowRoot): void {
  for (const element of target.querySelectorAll('[id^="event-"]')) {
    if (EVENT_ID.test(element.getAttribute("id") ?? "")) element.classList.add("musa-event");
  }
}

/**
 * Add `.musa-event-active` to every rendered piece of an event — all tie
 * pieces, across the whole document or one root — or clear it with `null`.
 */
export function highlight(eventId: string | null, root: Document | Element = document): void {
  for (const [target, host] of targets) {
    if (!host.isConnected) {
      targets.delete(target);
      continue;
    }
    if (!inside(host, root)) continue;
    for (const element of target.querySelectorAll(".musa-event-active")) {
      element.classList.remove("musa-event-active");
    }
    if (eventId !== null) {
      const pieces = target.querySelectorAll(`[id="event-${eventId}"], [id^="event-${eventId}-t"]`);
      for (const piece of pieces) piece.classList.add("musa-event-active");
    }
  }
}

/** Whether a host element belongs to the highlight root. */
function inside(host: Element, root: Document | Element): boolean {
  if (root instanceof Document) return host.ownerDocument === root;
  return root === host || root.contains(host);
}
