---
id: 14
slug: lilypond-export
status: done
depends_on: [07]
phase: 1
---

# LilyPond Export

## Task

Add the LilyPond print backend: a typed LilyPond document model with a deterministic pretty-printer, driven from the
shared `NotationPlan`, exposed as `musa render --to lilypond`.

## Read

- Roadmap §12.3 (three-stage pipeline, `LyNode` sketch, determinism requirements, why LilyPond is export-only and GPL
  considerations), §15.4 (facade).
- Prompt 07's `NotationPlan`, prompt 13's `render_notation` facade.

## Design

- Three stages exactly as §12.3 prescribes: `NotationPlan` → typed `LyDocument` (internal `LyNode` tree: `Sequential`,
  `Simultaneous`, `Note`, `Rest`, `Command`, `Context`) → deterministic pretty-printer → `.ly` text. `LyNode` stays
  **private**; the public surface is `NotationTarget::LilyPond` on the existing facade.
- Generated files contain: `\version` with an explicit LilyPond version, deterministic declaration order (parts sorted
  by `PartId`), stable variable names (`part-violin-lead`), escaped strings, and `% event:<hex>` source-map comments on
  notes where the comment doesn't disturb the layout semantics.
- Duration spelling: map `NotatedDuration`'s *spelling* (dotted quarter stays `4.`) rather than re-deriving from the
  rational value; ties from the plan become `~`.
- Multi-voice parts emit LilyPond voice contexts (`\\` simultaneous voices or `\voiceOne`/`\voiceTwo`) per the plan's
  voice allocation.
- Unsupported constructs produce `RenderError` diagnostics naming the event (§7.2: no raw escape hatches, no silent
  dropping).
- Do **not** invoke a `lilypond` binary from the CLI; export only (§12.3). If `lilypond` is installed locally, compile
  one fixture manually as a smoke check and note the result; CI compilation is optional and must be skippable.

## Target

- `musa-render`: private `ly` module (document model + printer), `NotationTarget::LilyPond`.
- `musa`: `render --to lilypond`.
- Tests: insta snapshots for all three examples; determinism test (render twice, byte equality); unit tests for duration
  spelling (dotted vs tied vs triplet-free rational values) and escaping.

## Check

```sh
cargo nextest run -p musa-render -p musa
cargo clippy --all-targets -p musa-render -p musa -- -D warnings
cargo fmt --check
cargo run -p musa -- render examples/counterpoint.musa --to lilypond -o /tmp/cp.ly
# manual smoke (only if lilypond is installed):
command -v lilypond >/dev/null && lilypond -o /tmp/cp /tmp/cp.ly
```

Commit as `Add LilyPond export`.

## Stop

- No LilyPond import, ever (§12.3).
- No invocation or bundling of the LilyPond binary in the application.
- No page-layout options, paper size, or typography knobs in `NotationOptions` — the plan is semantic; engraving belongs
  to LilyPond itself.
