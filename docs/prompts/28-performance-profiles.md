---
id: 28
slug: performance-profiles
status: done
depends_on: [27]
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
- Prompt 15's `lower_performance`, prompt 27's annotations.

## Design

- Language: parse the `performance { profile <name> { ... } }` block (lexer/parser additions: `profile`, `articulation`,
  `dynamic`, unit-bearing values like `8 ms`, gate floats). Profiles declare: articulation realizations (gate ratio,
  attack time), dynamic levels (mapping `pp..ff` to abstract amplitude values — still not decibels; the mapping to gain
  happens at the instrument boundary in prompt 30's parameter system), and default profile per part.
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
cargo run -p musa -- render examples/profile-fixture.musa --to midi --mode performance -o /tmp/p.mid
cargo run -p musa -- render examples/glass-mountain.musa --to wav -o /tmp/gm4.wav
cmp /tmp/gm4.wav /tmp/gm.wav   # no profile ⇒ unchanged audio
```

Commit as `Add performance profiles and MIDI export`.

## Repairs made while implementing

- **Profiles live on `ScoreSnapshot`, not `PerformanceOptions`.** The Design said the options gain the resolved
  profiles, but `PerformanceOptions` is `Copy` and shared by every caller (CLI, project, engine); threading a
  `ProfileSet` through all of them invites rendering one piece against another's profiles. A profile is a *source
  declaration* — it belongs beside `motifs`, inert until read. No call site changed as a result.
- **Realization values are exact rationals, not floats.** `ScoreSnapshot` derives `Eq`, which floats forbid, and the
  repo's exact-time rule already says numbers become floats only at the DSP edge. `gate = 0.55` is stored as `11/20`, so
  it neither drifts nor prints as `0.55000000000000004` in a snapshot. `parse_decimal` does the conversion.
- **Grammar as built:** `performance { profile <name> { articulation <mark> { gate = <ratio>; attack = <n> ms; } dynamic
  <mark> { amplitude = <ratio>; } } }`, with `profile <name>;` inside a part choosing one. The unit belongs to the
  value, not the setting, so `8 ms` and `0.008 s` both parse.
- **A profile is a *partial* reading.** A mark the profile says nothing about is neutral, not an error — requiring every
  profile to name all ten dynamics and every articulation would make profiles unusable. Gates from several marks on one
  note multiply; `attack` takes the last written.
- **`part_metadata` extracted.** Clef reading was duplicated between `lower.rs` and `elaborate.rs`; both paths now call
  one function, which is also how the profile binding reached both without a parity risk.
- **Score vs. performance MIDI from one plan** needed `PerformedNote::notated_off`: the plan now carries the written
  extent alongside the performed one, because §2 says they are two facts, not one fact and a setting.
- **`an_unprofiled_piece_is_timed_the_same_in_both_modes`, not byte-identical.** The two modes necessarily differ in
  velocity — score MIDI states its neutral 80, performance MIDI states the amplitude it was handed, which with nothing
  realized is full. The invariant worth asserting is that *no gate shortened anything*, so the test compares spans.
- **A permanent golden-audio guard was added** (`an_unprofiled_piece_renders_the_golden_audio` in `musa-project`): the
  Check's `cmp` against a WAV rendered by hand cannot survive the session. The pinned digest was verified against a
  build of `f76f1cf` rather than merely recorded from this one, and byte-identity was confirmed for all six existing
  examples the same way.
- **MIDI details:** channel 10 is skipped (percussion by convention), `NEUTRAL_VELOCITY = 80` rather than 127 because a
  file full of 127 is an opinion, and `midly` is taken with `default-features = false` (its `parallel` feature is for
  parsing huge files; we only write).
- **`attack` is carried, not realized.** `PerformedNote::attack` reaches the audio layer, but the placeholder synth
  applies only amplitude — the Stop section limits realization to that, and prompt 30's parameter system is where an
  instrument decides what an attack request means.

## Stop

- No tempo/expression **curves** (constant tempo per piece; curves are prompt 36).
- No rubato/humanization, no per-note timing jitter.
- No live MIDI output; MIDI is files only until a later measured need (§12.5).
- No studio/synth realization of dynamics beyond per-note amplitude on the existing sine synth (full parameter routing
  arrives with prompts 29–25).
