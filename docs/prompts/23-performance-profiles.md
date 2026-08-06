---
id: 23
slug: performance-profiles
status: pending
depends_on: [22]
phase: 2
---

# Performance Profiles and MIDI Export

## Task

Give the performance layer its interpretive voice: `performance` blocks with named profiles that map articulations and
dynamics to realization parameters (gate, attack, velocity), applied per part. Then export both score MIDI (neutral) and
performance MIDI (interpreted) via `midly`.

## Read

- Roadmap §6.4 (what interpretation means: a staccato mark is not 0.5×; `p` is not a velocity — profiles make those
  choices explicit and per-instrument), §7.1 (the
  `performance { profile violin { articulation staccato { gate = 0.55; attack = 8 ms; } } }` example), §12.5 (score MIDI
  vs performance MIDI; MIDI is an edge format, never canonical), §17.6 (dynamic/articulation interpretation fixture).
- Prompt 15's `lower_performance`, prompt 22's annotations.

## Design

- Language: parse the `performance { profile <name> { ... } }` block (lexer/parser additions: `profile`, `articulation`,
  `dynamic`, unit-bearing values like `8 ms`, gate floats). Profiles declare: articulation realizations (gate ratio,
  attack time), dynamic levels (mapping `pp..ff` to abstract amplitude values — still not decibels; the mapping to gain
  happens at the instrument boundary in prompt 25's parameter system), and default profile per part.
- Compiler: `PerformanceOptions` gains the resolved profiles; `lower_performance` applies them — gate shortens note-off
  per articulation, dynamics scale per-note amplitude, attack shapes the envelope request carried on `PerformedNote`.
  The score remains untouched: interpretation lives only in the plan (§6.4).
- The default profile (no `performance` block) reproduces prompt 15–17 behavior exactly: full gate, neutral amplitude.
  Golden WAV from prompt 17 must remain byte-identical for pieces without profiles.
- MIDI export in `musa-render` (add `midly`, §15.4):

  ```rust
  pub fn render_midi(
      performance: &PerformancePlan,
      options: &MidiOptions,   // mode: Score | Performance
  ) -> Result<Vec<u8>, RenderError>;
  ```

  Score mode: neutral — full durations, fixed velocity, tempo from the map.
  Performance mode: profiled gates, velocity from dynamics, tempo events, control
  curves. Both derive from `PerformancePlan`/the score; **never** store MIDI numbers
  in the score (§12.5). Written pitch → MIDI note number mapping happens at this
  edge via the tuning service (12-TET).
- `musa render --to midi [--mode score|performance]`; project `ExportRequest` + desktop export menu updated.
- Fixture §17.6: a piece with a profile exercising staccato gate and two dynamic levels; snapshot both MIDI modes (parse
  back with midly and snapshot the event list — §17.4 requires reading generated MIDI back).

## Target

- `musa-language`/`musa-compiler`: `performance` blocks, profile model, interpreted `lower_performance`.
- `musa-render`: `render_midi` both modes; CLI + project + desktop export wiring.
- Tests: profile gate/velocity unit tests; default-profile WAV byte-identity regression; midly round-trip snapshots;
  determinism.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-render -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-render -p musa-project -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- render examples/profile-fixture.musa --to midi --mode performance -o /tmp/p.mid
cargo run -p musa-cli -- render examples/glass-mountain.musa --to wav -o /tmp/gm4.wav
cmp /tmp/gm4.wav /tmp/gm.wav   # no profile ⇒ unchanged audio
```

Commit as `Add performance profiles and MIDI export`.

## Stop

- No tempo/expression **curves** (constant tempo per piece; curves are prompt 31).
- No rubato/humanization, no per-note timing jitter.
- No live MIDI output; MIDI is files only until a later measured need (§12.5).
- No studio/synth realization of dynamics beyond per-note amplitude on the existing sine synth (full parameter routing
  arrives with prompts 24–25).
