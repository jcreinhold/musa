---
id: 22
slug: score-engraving
status: done
depends_on: [21]
phase: 1.5
---

# Score Engraving: quality under edit, zoom, and scale

## Task

Take the engraver from "renders a page" to "is the surface a composer works in all day": non-disruptive re-render with
scroll and selection anchoring, true re-layout zoom, page and continuous modes, on-demand page virtualization, and a
raster golden suite in both themes. This is the prompt that decides whether musa's score looks engraved or looks
generated.

## Read

- `docs/rules/desktop/02-engraving.md` in full — it is this prompt's specification, section by section.
- `06-performance.md` B2, B7, B8 and the structural rules of §3.
- Prompt 20's `Engraver` interface and sanitizer; prompt 13's `xml:id` contract.

## Design

- **Re-render without disruption** (`02-engraving.md` §6) — the core of the prompt:
  - old SVG stays interactive while the worker lays out the new revision;
  - scroll is anchored to the `xml:id` of the first event visible at the top of the viewport, restored to the same
    viewport offset after the swap (**not** a pixel offset);
  - selection is restored by `EventId`; if the event is gone, move to the nearest surviving neighbor in the same voice
    and say so in the inspector;
  - 90 ms cross-fade, no white frame, no scrollbar jump, instant swap under `prefers-reduced-motion`.
- **Zoom as re-layout** (§5): discrete steps 50–203 %, `⌘−`/`⌘=`/`⌘0`, transient CSS transform during a pinch gesture
  replaced by a true re-layout 120 ms after it settles. Staff size (rastral) is a separate remembered preference. A
  persistent CSS-transform zoom is explicitly rejected — it is the reason score apps look like PDF viewers. Rastral
  itself gets no control here — it stays at Verovio's `unit: 11` — because the Target of this prompt is the zoom
  *mechanism*; a staff-size preference is an engraving preference and belongs with the others.
- **Page and continuous modes** (§4): page is default; continuous sets `breaks: "none"` with horizontal scroll and is
  what follow mode prefers. The mode is remembered per piece. Continuous keeps the page's *height*, and so the staff
  size, and gives away only the width — a mode toggle that re-scaled the music would be a second zoom control. That
  means the page element's scale is stated rather than derived from its container, which is what `pixelsPerUnit` is for.
- **Virtualization** (§7): current page ± 1, `IntersectionObserver` over placeholders sized from the layout result so
  the scrollbar is honest from the first layout. Off-window SVG is dropped.
- **Overlay layer** (§8) generalized: one SVG overlay in the page's coordinate system, all marks in staff spaces
  (`--sp`) exposed by the layout result. Selection and hover move here from prompt 20; playhead and Origin traces attach
  in prompts 23 and 24 without touching Verovio's output.
- **Fixture**: add `tests/fixtures/large-score.musa` — a generated 100-bar, 4-part piece — as the large-case workload
  for `06-performance.md`. It is a fixture, not an example; it does not go in `examples/`. Both the generator and its
  output are committed: the generator (`crates/musa-project/tests/large_score_generators.rs`) because it is the only
  readable description of what the fixture is, the output because the UI imports it and the suites run offline. The
  generator fails when the committed copy is stale, so the two cannot drift.
- **Goldens** (§9): sanitized-SVG structure snapshots plus raster goldens at 2× for `glass-mountain` / `counterpoint` /
  `twinkle`, **in both themes**, with a documented tolerance and an update command. The dark golden is what proves
  `currentColor` plumbing rather than a filter.
  - The rasters are **Playwright screenshots**, as established in prompt 20, not a `resvg` rasterization: the claim
    worth defending is what a browser draws from the sanitized SVG plus the tokens, and a separate rasterizer would
    prove something the application never runs. Tolerance and scale live in `playwright.config.ts`.
  - The structure snapshot holds only what two independent loads agree on. Verovio mints a fresh random token per
    unnamed element on every load, so a digest taken once differs from itself.
- **Stability test** (§9): edit a fixture, re-render, assert the anchor event moved < 2 px in the viewport and that no
  intermediate frame was empty.

## Target

- `apps/musa-desktop/ui/src/lib/engrave/`: anchored re-render, zoom re-layout, page/continuous modes, virtualization,
  generalized overlay layer.
- `tests/fixtures/large-score.musa` and the generator or the committed file (whichever is more honest — document which).
- Tests: structure snapshots; raster goldens (both themes, three fixtures); the stability test; perf assertions for B2,
  B8 on the large fixture and B7 on the small one.

## Check

```sh
cargo nextest run -p musa-project         # writes the large fixture; fails when it is stale
cargo nextest run -p musa-render          # MEI for the new fixture stays green
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm run check && npm run test   # includes goldens + perf budgets
cd apps/musa-desktop && cargo tauri dev   # manual: type in a 100-bar score; the page must never blank or jump
```

Commit as `Engrave the score at production quality`.

## Stop

- No playhead, no follow modes (prompt 23).
- No Origin view (prompt 24) — only the overlay layer it will draw into.
- No editing (prompt 25).
- No printing or PDF export (not in this sequence; LilyPond is the print path).
- No custom engraving rules beyond Verovio's options — musa does not write a notation renderer (roadmap §19).
