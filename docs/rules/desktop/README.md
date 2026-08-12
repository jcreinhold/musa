# docs/rules/desktop — the desktop interface specification

Status: **governing**, graduated at prompt 26 on the same pattern as `docs/rules/kernel/`.

This is now a document the repo is held to: where the built interface and this specification disagree, either the code
is wrong or the specification needs a deliberate repair, and neither may drift silently.

`docs/plan/roadmap.md` §14 and §15.9 fix the desktop app's architecture: Rust owns the semantics, the frontend owns
ephemeral state only, the score is the main interface, entry is keyboard-first, invalid source keeps the last valid
score, the Tauri shell is thin. That remains law.

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
| [`06-performance.md`](06-performance.md) | Eleven named budgets, how they are measured, the structure they imply |
| [`07-the-volume.md`](07-the-volume.md) | The project as a bound volume: the contents page, the running order, and what a project of one shows |
| [`08-elaboration.md`](08-elaboration.md) | Terms, library documents, the Origin steps the elaboration language adds, advisory findings, raw kernel |

[`prototype.html`](prototype.html) is a static reference mockup of the Compose workspace: the real token system, real
Verovio/Bravura engraving of `examples/glass-mountain.musa`, both themes, selection, Origin view, and the stale-revision
state. Open it in a browser. It is a **design reference, not code** — prompt 20 rebuilds it properly in
Svelte/TypeScript with bundled fonts and a worker-based engraver. Its SVG was produced by a throwaway script outside the
repository; nothing generates it in CI, and when the spec and the mockup disagree, the spec wins.

Implemented by prompts 20–26. Where this document and roadmap §14 disagree on *architecture*, §14 wins; where §14 is
silent, this document is the authority, and code that drifts from it is either wrong or needs a deliberate repair here
first.
