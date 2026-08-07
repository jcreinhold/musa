---
id: 22
slug: score-engraving
status: pending
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

- `docs/interface/02-engraving.md` in full — it is this prompt's specification, section by section.
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
- **Zoom as re-layout** (§5): discrete steps 50–200 %, `⌘−`/`⌘=`/`⌘0`, transient CSS transform during a pinch gesture
  replaced by a true re-layout 120 ms after it settles. Staff size (rastral) is a separate remembered preference. A
  persistent CSS-transform zoom is explicitly rejected — it is the reason score apps look like PDF viewers.
- **Page and continuous modes** (§4): page is default; continuous sets `breaks: "none"` with horizontal scroll and is
  what follow mode prefers. The mode is remembered per piece.
- **Virtualization** (§7): current page ± 1, `IntersectionObserver` over placeholders sized from the layout result so
  the scrollbar is honest from the first layout. Off-window SVG is dropped.
- **Overlay layer** (§8) generalized: one SVG overlay in the page's coordinate system, all marks in staff spaces
  (`--sp`) exposed by the layout result. Selection and hover move here from prompt 20; playhead and Origin traces attach
  in prompts 23 and 24 without touching Verovio's output.
- **Fixture**: add `tests/fixtures/large-score.musa` — a generated 100-bar, 4-part piece — as the large-case workload
  for `06-performance.md`. It is a fixture, not an example; it does not go in `examples/`.
- **Goldens** (§9): sanitized-SVG structure snapshots plus `resvg` raster goldens at 2× for
  `glass-mountain` / `counterpoint` / `twinkle`, **in both themes**, with a documented tolerance and an update command.
  The dark golden is what proves `currentColor` plumbing rather than a filter.
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
