---
id: 147
slug: provenance-interaction
status: done
depends_on: [146]
phase: 5
---

# Provenance Interaction: Every Note Knows Its Event

## Task

Wire the MEI `xml:id` contract through to the page: every engraved note, chord, and rest that Verovio
marks with an `event-<hex>` id becomes addressable from the embedding page — clickable, hoverable,
highlightable — identified by the `EventId` the compiler assigned. This is the feature that makes the
package musa rather than another notation renderer: the same provenance the desktop app's origin view
is built on, delivered as a DOM API.

## Read

- `crates/musa-render/src/mei.rs` header — the `xml:id` contract: `event-<hex>`, tie pieces as
  `event-<hex>-t2`, `-t3`, …; stripping the suffix yields the `EventId`. Layers are `layer-<s>-<l>` and
  are not event-mapped.
- Prompt 146 — where the SVG lands (shadow roots and sibling containers); interaction must cross that
  boundary deliberately.
- `docs/interface/` (origin view, linked reading) for the semantics; the web analog is a callback, not
  a port of the desktop UI.

## Design

```ts
// Added to MusaWebConfig and exported helpers.

export interface MusaWebConfig {
  // … prompt 146 fields …
  /** Fires for clicks on event-mapped notation. The id is the compiler's
      EventId hex — tie-piece suffixes already stripped. */
  onEventClick?: (eventId: string, target: SVGElement, source: MusaElementContext) => void;
  onEventHover?: (eventId: string | null, target: SVGElement | null) => void;
}

export interface MusaElementContext {
  element: Element;      // the <musa-score> or container this event lives in
  source: string;        // the snippet's source text
}

/** Add `.musa-event-active` to every rendered piece of an event (all tie
    pieces, across the whole document or one root). Pass null to clear. */
export function highlight(eventId: string | null, root?: Document | Element): void;
```

Mechanics, all consequences of the contract rather than new machinery:

1. **Delegation, not per-note listeners.** One click/hover listener per typeset root (shadow-root-aware
   — events retarget across the shadow boundary, so listeners attach to the shadow root itself). A hit
   target's id is matched against `^event-([0-9a-f]+)(?:-t\d+)?$`; the capture is the `EventId`, the
   suffix is discarded. Non-mapped elements (layers, barlines, text) simply do not fire — no
   "unidentified object" state exists.
2. **Tie pieces are one event.** `highlight` selects by the id *prefix*, so a tied note lights up as
   the single event it is — the behavior the `-tN` suffix scheme was designed for, now load-bearing on
   a second platform.
3. **Styling is classes, not inline styles.** `.musa-event` on mapped elements, `.musa-event-active`
   from `highlight`, both in the injected stylesheet with `:host`-aware rules for shadow roots;
   embedders override freely.
4. **Verification that the contract survives Verovio.** This prompt's first test is a guard, not a
   feature test: engrave a fixture and assert the SVG ids match the MEI `xml:id`s exactly. If a Verovio
   upgrade ever drops or rewrites ids, this test — not a user's page — is where it breaks. The guard
   runs against the pinned `verovio` version in CI.

No source-position mapping in this prompt: `EventId → source span` lives in the compiler's provenance
(`Origin`), and exposing it is a wasm-shell addition (a later prompt, when a caller needs it). The
callback receives the source text so embedders can do their own mapping meanwhile.

## Target

- `src/interaction.ts` (delegated listeners, id matching, highlight), stylesheet additions, exports.
- Playwright: click a note in a `<musa-score>` shadow root → callback with the exact hex id; click a
  tie's second piece → same id; `highlight` marks all pieces of a tied event and clears on `null`;
  barlines/staff lines never fire; callbacks fire per-container with the right `MusaElementContext`
  when two scores share a page.
- The contract guard: SVG ids ≡ MEI `xml:id`s for `examples/canon.musa` and one tie-heavy fixture.
- Package README section: the id contract, the two callbacks, `highlight`, a styling example.

## Check

```sh
pnpm --filter @musa/web test
pnpm --filter @musa/web build
```

Commit as `Add provenance interaction to @musa/web`.

## Stop

- No `EventId → source span` API, no wasm-shell change (named above as later work, with a caller).
- No selection model, caret, keyboard navigation, or editing: this is a callback surface, not an editor.
- No playback-cursor or playhead highlighting (arrives with prompt 149, if it arrives).
- No tooltip/popover built-ins: embedders compose their own from the callbacks.
