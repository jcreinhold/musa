---
id: 07
slug: notation-plan
status: done
depends_on: [06]
phase: 1
---

# Notation Plan

## Task

Implement `musa-notation`'s foundation: the backend-neutral `NotationPlan` that derives measures, beaming groups, and
tied-duration decomposition from a `ScoreSnapshot`, so MEI, LilyPond, and MusicXML writers (prompts 08, 09, 22) share
one source of truth and no backend assumption enters the compiler.

## Read

- Roadmap §12.1 (what the plan contains and why it exists), §6.3 (`NotatedDuration` keeps spelling), §15.4 (crate
  ownership).
- Prompt 05's `ScoreSnapshot`, `MeterMap`, `KeyMap`.

## Design

- Create `musa-notation` with dependencies: `musa-compiler`, `serde`, `thiserror` (quick-xml and midly arrive at prompts
  08 and 18).
- Public surface:

  ```rust
  pub struct NotationPlan { /* measures, staves, voice allocation, beams, ties,
      clefs, key/time signatures — private layout */ }

  pub fn plan_notation(
      score: &ScoreSnapshot,
      options: &NotationOptions,
  ) -> Result<NotationPlan, NotationError>;
  ```

  Accessors for what backends need: iterate staves → measures → voice lanes →
  notated events (note/rest/chord with spelled durations, tie flags, beam groups).
- Scope for this prompt: single voice per staff lane plus the multi-voice allocation the examples need (strings
  upper/bass on one part), measure splitting at meter boundaries, beaming of eighth-and-shorter groups per meter, and
  decomposition of cross-measure durations into tied notes.
- Spelling decisions: use the event's `WrittenPitch` verbatim; no respelling. Key signature affects only the
  key-signature element, not pitch spelling.
- Tie decomposition must preserve provenance: one `ScoreEvent` spanning a barline becomes several notated notes all
  pointing at the same `EventId`. The plan carries `EventId` on every notated item (MEI `xml:id` in prompt 08 depends on
  this).
- Notation errors are explicit (§7.2: unsupported constructs are diagnostics, never raw escapes): e.g. a duration that
  cannot be spelled within one measure with standard ties produces a `NotationError` naming the event.

## Target

- `musa-notation`: `NotationPlan`, `NotationOptions`, `plan_notation`.
- Tests: insta snapshots of a debug rendering of the plan for all three examples; unit tests for measure splitting,
  beaming groups (4/4 vs 6/8), tie decomposition with shared `EventId`; proptest: the sum of tied pieces equals the
  original duration and every piece is inside one measure.
- A `musa render --to plan` debug subcommand (hidden or clearly marked debug) that dumps the plan as text — this is the
  snapshot surface reviewers use until MEI exists.

## Check

```sh
cargo nextest run -p musa-notation
cargo clippy --all-targets -p musa-notation -- -D warnings
cargo fmt --check
cargo run -p musa -- render examples/counterpoint.musa --to plan
```

Commit as `Add backend-neutral notation plan`.

## Stop

- No MEI/LilyPond/MusicXML writers yet.
- No tuplets/slurs/dynamics/articulations (prompt 17); plan for their *presence* in the data model only if trivially
  cheap, otherwise extend in 17.
- No layout/engraving concepts (line breaks, spacing) — the plan is semantic, not typographic (§12.1).
