# docs/rules/desktop — the desktop interface specification

Status: **governing**, graduated at prompt 26 on the same pattern as `docs/rules/events/`.

This is now a document the repo is held to: where the built interface and this specification disagree, either the code
is wrong or the specification needs a deliberate repair, and neither may drift silently.

`docs/plan/roadmap.md` §14 and §15.9 fix the desktop app's architecture: checked source owns declarable musical
semantics, Rust owns the calculus and runtime boundaries, and the frontend owns ephemeral state only. The score is the
main interface, performance and commands are keyboard-first, invalid source keeps the last valid score, and the Tauri
shell is thin. Exact notation is written in source; playing may become source only through the reviewed capture path in
[`10-keyboard-composition.md`](10-keyboard-composition.md).

This directory fixes everything §14 leaves open — the design. It exists because §14.3's wireframe, read as a visual
specification, produces a four-panel toolbar application indistinguishable from the notation editors this project exists
to improve on. The desktop app is the product for most users; its quality is not a finishing pass.

| File | Fixes |
| --- | --- |
| [`00-thesis.md`](00-thesis.md) | Who it is for, the one job, the leaf-and-margin thesis, the signature, what is rejected |
| [`01-visual-language.md`](01-visual-language.md) | Color, typography, space, elevation, motion, the Compose layout |
| [`02-engraving.md`](02-engraving.md) | The Verovio quality bar: worker, theming, zoom, pagination, non-disruptive re-render, goldens |
| [`03-interaction.md`](03-interaction.md) | Selection model, pointer, keyboard map, transport and playhead, accessibility floor |
| [`04-provenance.md`](04-provenance.md) | Origin view — the signature — and the editing choice it makes legible |
| [`05-states.md`](05-states.md) | Empty, loading, stale-revision, diagnostics, failure; the interface's voice |
| [`06-frame-budgets.md`](06-frame-budgets.md) | Named budgets, how they are measured, the structure they imply |
| [`07-the-volume.md`](07-the-volume.md) | The project as a bound volume: the contents page, the running order, and what a project of one shows |
| [`08-elaboration.md`](08-elaboration.md) | Terms, library documents, the Origin steps the elaboration language adds, advisory findings, raw events |
| [`09-sound-and-mix.md`](09-sound-and-mix.md) | Instruments, exposed controls, part outputs, recorded media, assets, and deliberate machine disclosure |
| [`10-keyboard-composition.md`](10-keyboard-composition.md) | Audition, finite MIDI capture, transcription review, acceptance, privacy, and revision |

[`prototype.html`](prototype.html) is a static reference mockup of the Compose workspace: the token system,
Verovio/Bravura engraving of `examples/glass-mountain.musa`, both themes, selection, Origin view, and the stale-revision
state. Open it in a browser. It is a **design reference, not code**. The implemented Svelte interface uses bundled fonts
and a worker-based engraver; the mockup is not generated in CI. When it and this specification disagree, the
specification wins.

Introduced by prompts 20–26 and amended by prompt 201's keyboard-composition contract. Where this document and roadmap
§14 disagree on *architecture*, §14 wins; where §14 is silent, this document is the authority, and code that drifts from
it is either wrong or needs a deliberate repair here first.
