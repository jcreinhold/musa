---
id: 05
slug: compiler-core
status: pending
depends_on: [04]
phase: 1
---

# Compiler Core: Semantic Model and Score Snapshot

## Task

Implement `musa-compiler`: name resolution, exact rational musical time, the
high-level compositional model, and lowering of plain voices (notes, rests, chords —
no motifs yet) into an immutable, sorted, provenance-carrying `ScoreSnapshot`.
`musa check` becomes a full semantic check.

## Read

- Roadmap §5.1–§5.3 (exact time, composition laws), §6.2–§6.3 (high-level model and
  `ScoreSnapshot` shapes), §8.1 (concrete written pitch), §9 (`Origin`), §10.6
  (pipeline + facade), §15.3 (crate ownership and dependencies).
- Roadmap §2's separation table — written pitch is not a MIDI number; notated duration
  is not performed duration.

## Design

- Add dependencies to `musa-compiler`: `musa-language`, `num-rational`, `slotmap`,
  `indexmap`, `serde` (derive), `thiserror`. Follow §15.3: slotmap keys are transient
  arena keys, never serialized as permanent identities.
- Core public types (fixed by the roadmap; internals private):

  ```rust
  pub struct MusicalTime(Ratio<i64>);      // whole note = 1
  pub struct MusicalDuration(Ratio<i64>);

  pub struct WrittenPitch { pub letter: Letter, pub accidental: Accidental, pub octave: i8 }
  pub struct NotatedDuration { /* exact value + notational spelling */ }

  pub struct ScoreSnapshot { pub parts: PartMap, pub tempo_map: TempoMap,
      pub meter_map: MeterMap, pub key_map: KeyMap, pub annotations: AnnotationStore }
  pub struct ScoreEvent { pub id: EventId, pub origin: Origin,
      pub onset: MusicalTime, pub notated_duration: NotatedDuration,
      pub kind: ScoreEventKind }

  pub fn compile(source: &SourceDocument, options: &CompileOptions) -> Compilation;
  ```

- `Compilation` is a deep facade: it exposes `snapshot()` (the `ScoreSnapshot`, only
  if compilation succeeded), `diagnostics()`, and nothing about passes. Pass modules
  (resolution, units, lowering) are private (§10.6).
- `Origin` carries `SourceSpan` + `DeclarationId` + `Vec<ExpansionStep>`; at this
  prompt the expansion path is always empty — prompt 06 fills it. Define the full type
  now.
- Voices are `BTreeMap<VoiceId, Fragment>`-style identified lanes (§5.3); a
  single voice containing two simultaneous monophonic note streams is a diagnostic,
  not a silent merge. Chords are the explicit polyphony mechanism.
- Unit checking (§7.2, "units are part of the syntax"): tempo `= 72` in a
  `tempo q = 72;` context is a bare integer by grammar design (bpm is implied by the
  tempo statement); reject anything else unitless where a unit literal is required
  (none exist in the core grammar yet — write the unit-checking pass skeleton against
  Hz/ms/s/dB so prompt 19 only extends tables).
- `musa check` now runs `compile` and reports semantic diagnostics (unknown clef,
  duplicate part names, overlapping same-voice events, meter/duration sanity) via
  `miette`. The CLI calls `musa_compiler::compile` through a thin
  `SourceDocument::open` — still no project layer.

## Target

- `musa-compiler`: the public types above + `compile`; private passes for resolution,
  units, lowering.
- `ScoreSnapshot` is immutable, sorted by onset, serializable (`serde`) for later
  debugging/export tests.
- Tests: snapshot equality for both examples (insta over a debug rendering);
  diagnostics for unknown names and same-voice overlaps; proptests for the §5.1/§5.2
  laws on `MusicalDuration` and on sequential span (`span(a then b) = span a + span
  b`) using the lowering of generated voice fragments.
- `examples/counterpoint.musa`: two-part counterpoint exercising chords + multiple
  voices.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-cli
cargo clippy --all-targets -p musa-compiler -p musa-cli -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/glass-mountain.musa examples/invention.musa examples/counterpoint.musa
```

Commit as `Add compiler core with score snapshot and provenance`.

## Stop

- No motifs, repeat, transpose (prompt 06).
- No performance lowering, tempo-to-frames (prompt 10). `TempoMap` stores the
  declared tempo; it is not integrated yet.
- No notation planning or export (prompts 07–09).
- No Salsa or incremental anything (roadmap §10.7).
