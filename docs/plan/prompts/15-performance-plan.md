---
id: 15
slug: performance-plan
status: done
depends_on: [06]
phase: 1
---

# Performance Plan

## Task

Implement the performance layer's neutral core: integrate the tempo map and lower a `ScoreSnapshot` into a
frame-scheduled `PerformancePlan` of note-on/note-off events. Interpretation (articulation, dynamics, rubato) is
deliberately minimal — this prompt proves the scheduling pipeline that audio (11–13) and MIDI export (18) consume.

## Read

- `docs/rules/kernel/06-surface-elaboration.md` (tempo is a monotone map `Beat → Second` supplied by the performance
  layer; symbolic kernel positions stay in beats; "stretch the material" and "perform it more slowly" are different
  operations — this prompt implements the second, never the first), §23 (audio is a separate semantic layer; this prompt
  ends at physical musical events).
- Roadmap §6.4 (`PerformancePlan`/`PerformanceEvent` shapes and what the layer is responsible for), §8.1 (`Tuning` trait
  — concrete 12-TET default now, service boundary later), §5.5 (lowering laws).
- Roadmap §2: a written A4 is not MIDI note 69; `p` is not a velocity. This prompt's neutrality is the reason those
  separations hold.

## Design

- Lives in `musa-compiler` (§15.3 owns "performance lowering"); public additions:

  ```rust
  pub struct PerformancePlan { pub tempo: IntegratedTempoMap,
      pub lanes: Vec<PerformanceLane> /* private layout */ }

  pub enum PerformanceEvent {
      NoteOn  { frame: u64, note: PerformedNote },
      NoteOff { frame: u64, voice: VoiceInstanceId },
      Parameter { frame: u64, target: ParameterId, value: f32 },
  }

  pub fn lower_performance(
      score: &ScoreSnapshot,
      options: &PerformanceOptions,   // sample_rate, tuning (default 12-TET A440)
  ) -> Result<PerformancePlan, PerformanceError>;
  ```

- `IntegratedTempoMap` converts musical onsets to absolute frames through the declared tempo curve (constant tempo only
  in the current grammar; design the map as a piecewise structure so prompt 36's curves extend data, not code).
- `PerformedNote` carries sounding pitch (post-transposition-instrument — none yet), frequency (via the tuning service),
  symbolic dynamic (none yet → default), and the event's `EventId` + `Origin` for provenance. Frequency is derived at
  this boundary, never stored in the score.
- Gates: note-off frame = onset + notated duration × tempo factor. No articulation shortening yet (§6.4: that is a
  profile decision, prompt 28).
- Sorting/stability: events sorted by frame; simultaneous events ordered deterministically by lane then `EventId`.
- Laws to test (§5.5, §17.2): `lower(a then b)` schedules b after a's span; `lower(a together_with b)` merges lanes
  without frame drift; total event count is preserved (2 per note at this stage).
- `musa render --to performance` debug dump (text), mirroring prompt 07's `--to plan` for snapshotting.

## Target

- `musa-compiler`: `lower_performance`, `PerformancePlan`, `IntegratedTempoMap`, tuning (12-TET with configurable
  concert A).
- Tests: frame-exact unit tests (72 bpm quarter = known frame count at 48 kHz); proptest laws above; insta snapshot of
  the debug dump for the counterpoint example.

## Check

```sh
cargo nextest run -p musa-compiler -p musa
cargo clippy --all-targets -p musa-compiler -p musa -- -D warnings
cargo fmt --check
cargo run -p musa -- render examples/counterpoint.musa --to performance
```

Commit as `Add performance lowering with frame scheduling`.

## Stop

- No articulation/dynamic interpretation or profiles (prompt 28).
- No `Parameter` events from the grammar (nothing produces them until prompt 29/30); the variant exists now so the enum
  is stable.
- No audio rendering, no MIDI file export.
- Do not put `lower_performance` in `musa-render` or `musa-audio`; it is part of the compiler's lowering pipeline
  (§10.6, §15.3).
