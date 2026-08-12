# The desktop interface

The desktop app has one job:

> Make the engraved score the primary surface for reading, hearing, and editing a piece — while never hiding that the
> score was produced from text you control.

Everything in the design follows from that sentence. The score must be genuinely well engraved, because it is what the
user looks at for hours. The provenance must be visible, because that is the one thing musa knows that no other program
does. The text must be one keystroke away and never in the way.

## The leaf and the margin

The design inverts both competing conventions — the DAW's wall of lit panels and the notation editor's gray desktop with
a floating page:

- The score is a **leaf**: a real sheet with real paper luminance, a hairline edge, and the single soft shadow in the
  application.
- Everything else lives in **margins** that share one flat surround color. No panel backgrounds, no borders, no panel
  headers; separation is made by space and ink weight, the way it is made on a page.
- **Nothing covers the leaf.** Drawers and dialogs push the leaf or sit beside it; the score never disappears behind a
  modal.
- Chrome is **typographic, not boxed**: a transport is a row of set text and notation glyphs, not a strip of beveled
  buttons.

The rule is easy to check: if a new element needs a box, a border, or a second shadow to be legible, it is being placed
wrong.

## The signature: Origin view

Every note on the page is either authored or generated. Hold the Origin key and the page separates — authored music
stays full ink, generated music falls back — and hovering a generated note traces it to the occurrence that produced it,
on the page and in the source. Origin view gets the application's boldness budget; everything around it stays quiet. See
[Source and provenance](provenance.md) for the machinery underneath.

## What the design rejects

Icon toolbars; drag-to-edit as a primary interaction; modal dialogs for musical operations; a second visual authority in
the frontend; skeuomorphic studio hardware; animation as ambience. Entry is keyboard-first, and every action is
reachable from the keyboard.

The full specification — visual language, engraving quality bar, interaction model, states, performance budgets — lives
in `docs/rules/desktop/` in the repository, and it is governing: code that drifts from it is wrong until the document is
deliberately repaired.
