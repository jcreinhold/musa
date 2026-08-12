# 00 — Interface Thesis

Status: **governing** (graduated at prompt 26, alongside the source workspace).

## 1. What this document is

`docs/roadmap.md` §14 fixes the desktop app's *information architecture*: what Rust owns, what the
frontend owns, which workspaces exist, that the score is the main interface, that entry is keyboard-first, that invalid
source keeps the last valid score. That is law and this document does not contradict it.

What §14 does **not** fix is the design: the visual language, the engraving quality bar, the interaction feel, the
performance budgets, or the vocabulary the interface speaks. §14.3's ASCII box is a wireframe of *presence*, not a
specification of *appearance*. Read literally as a visual spec it produces a four-panel toolbar application
indistinguishable from every mediocre notation editor already on the market. This document is the design that sits
between the roadmap's architecture and the code.

`docs/interface/` is the third governing document of this repository, after the roadmap and the course correction. Where
it and §14 disagree on *architecture*, §14 wins. Where §14 is silent — everything below — this document is the
authority, and code that drifts from it is either wrong or requires a deliberate repair here first.

## 2. Who this is for and what the app is for

Musa's user is a composer who thinks structurally: motifs and their transformations, exact rational time, voices that
are real voices and not MIDI channels. Often, though not always, someone comfortable with a text language. They are not
looking for a DAW, and they are not looking for a page-layout program. They are looking for a place where a piece can be
*written* — where the structure they hear is the structure they type, and the printed page is a faithful projection of
it.

The app has one job:

> **Make the engraved score the primary surface for reading, hearing, and editing a piece — while never hiding that the
> score was produced from text you control.**

Everything below follows from that sentence. The score must be genuinely well engraved, because it is what the user
looks at for hours. The provenance must be visible, because that is the one thing musa knows that no other program does.
The text must be one keystroke away and never in the way.

## 3. The design thesis: the leaf and the margin

There are two competing conventions for this kind of application, and both are wrong for musa.

- **The DAW convention**: a dark chrome full of lit rectangular panels, each with a header bar, each competing for
  attention, accent colors everywhere. It is designed for surfaces that are all equally important. Musa's surfaces are
  not equally important.
- **The notation-editor convention**: a light gray desktop, a ribbon or palette of icons, a white page floating in the
  middle, dialogs for everything. It treats the score as a document to be laid out rather than a piece to be written.

Musa's design inverts both:

> **The paper is the only lit surface in the application. Everything else is unlit margin.**

Concretely:

1. The score is a **leaf** — a real sheet with real paper luminance, a hairline edge, and the single soft shadow in the
   entire application. It is centered and dominant.
2. Everything else lives in **margins** that share one flat, matte surround color. Margins have **no panel backgrounds,
   no borders, no dividers, and no panel headers**. Separation is made by space and by ink weight, the way it is made on
   a page.
3. **Nothing ever covers the leaf.** Drawers, inspectors, and dialogs push the leaf or sit beside it; they never overlay
   it. The score never disappears behind a modal.
4. Chrome is **typographic, not boxed**. A transport is a row of set text and notation glyphs, not a strip of beveled
   buttons. A field is a value with a hairline underline, not a bordered input — the underline is there at rest and
   darkens through hover to focus, because an affordance that appears only on hover can be found only by someone who
   already suspected it.

This is a restraint contract. It is easy to keep and easy to check: if a new UI element needs a box, a border, or a
second shadow to be legible, it is being placed wrong.

## 4. The signature: Origin view

One element carries the identity of this application, and it is the one thing musa can do that nothing else can.

Every note on the page is either **authored** — you typed it — or **generated** — the compiler produced it by expanding
a motif, a transform, or a pattern. The compiler already tracks this exactly (`Origin`, expansion paths, prompt 06). The
interface makes it visible:

> **Hold the Origin key and the page separates: authored music stays full ink, generated music falls back to a lighter
> verdigris. Hover any generated note and a hairline traces back to the occurrence that produced it — on the page, in
> the parts list, and in the source.**

This is not decoration. It is the answer to the question a composer working with transformed material asks constantly —
*where did this come from, and what happens if I change it?* — and it is the visual precondition for prompt 25's edit
story, where changing a generated note must offer a real choice between editing the definition and specializing the
occurrence.

Origin view gets the application's boldness budget. Everything around it stays quiet.

## 5. What this design explicitly rejects

- **Icon toolbars.** Musa has a 400-year-old symbol set for musical actions and a keyboard for the rest. Where a control
  needs a mark, use the notation glyph (a quarter note for the quarter duration; 𝄆 for loop). Where no glyph exists, use
  a word. Do not commission a pictogram language.
- **Drag-to-edit as a primary interaction** (roadmap §14.5 already rejects it; the design does not smuggle it back in as
  "just for pitch").
- **Modal dialogs for musical operations.** Naming a motif is an inline field, not a dialog.
- **A second visual authority.** The frontend renders state; it never keeps a musical model. If a highlight, a duration,
  or a bar number can only be produced by frontend arithmetic, the snapshot is missing a field — fix the snapshot.
- **Skeuomorphic studio hardware** in the later Sound and Mix workspaces: no wood grain, no brushed metal, no knobs with
  bevels. The same typographic discipline applies there.
- **Animation as ambience.** Motion exists for three reasons only (see `01-visual-language.md` §6): to keep the page
  from flashing, to move the playhead, and to cross-fade Origin view.

## 6. Quality floor

These are not aspirations; they are the definition of done for prompts 20–26, and each appears as a checkable item in
the prompt that introduces it.

| Floor | Where it is specified |
| --- | --- |
| The score is genuinely well engraved at every zoom, in both themes, on high-DPI displays | `02-engraving.md` |
| An edit never blanks, flashes, or scroll-jumps the score | `02-engraving.md` §6, `06-performance.md` |
| Every action is reachable from the keyboard; every note is focusable and announced | `03-interaction.md` §5 |
| Contrast ≥ 4.5:1 for all text and ≥ 3:1 for all meaningful marks, in both themes | `01-visual-language.md` §2 |
| `prefers-reduced-motion` is honored everywhere | `01-visual-language.md` §6 |
| The app is fully usable offline with zero setup (all fonts, Verovio, and DSP bundled) | `01-visual-language.md` §3 |
| Named performance budgets, measured, not estimated | `06-performance.md` |
