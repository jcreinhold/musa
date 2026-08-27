---
id: 205b
slug: pitch-spelling-written-ends
status: pending
depends_on: [202, 205a]
phase: 2
---

# Spell Pitches and Infer Written Ends

## Task

Give every retained voice assignment exact spelled pitches and an inferred written end, without losing the facts those
choices are made from. Spelling preserves MIDI pitch identity and reads the key/collection at the insertion point; where
that evidence is ambiguous the ambiguity is an alternative or a declared loss, never a silent default. Written ends come
from key intervals, subsequent attacks, articulation, rhythm vocabulary, voice continuity, and bar structure — a short
key release is weak evidence for staccato, and pedal extension is never scored as a written end.

## Read

- Prompt 202's spelling table and round-trip law in `crates/musa-project/src/midi.rs` (`spell` and its tests).
- Prompt 203's key-duration and pedal facts in `docs/notes/research/90-midi-transcription-trial.md` (47/58 exact key
  durations, the 1.7 s pedal fixture).
- Prompt 205a's voice assignment and prompt 204b's candidate cost record.
- Open Music Theory `001`–`007` (spelling, accidentals, clefs), `009`–`012` (rhythm/rests/ties) and
  `118-metrical-dissonance.md`.

## Design

- Spell each completed note from the key/collection and scope at the proposed insertion point, preserving MIDI pitch
  identity. Chromatic, atonal, microtonal, pitch-bend, and ambiguous enharmonic evidence produce alternative spellings
  or an explicit declared loss; there is no "C major if unknown" default. Chord members keep individual spellings. A
  later selection respells without retranscribing timing.

- Infer the written end jointly from key interval, the next attack, articulation pattern, the rhythm vocabulary, voice
  continuity, and bar structure. A short key release is weak evidence for staccato and a high attack velocity is weak
  evidence for accent; neither writes a dynamic, articulation, slur, pedal mark, or performance control without a review
  choice. Grace/ornament candidates stay marked as such, not silently shrunk to ordinary notes.

- Key-release duration accuracy must not regress below 47/58, and pedal-extended sound is never a written end.

## Target

- Exact spelled-pitch and written-end completion over prompt 205a's voices, carrying alternatives/constraints and
  declared losses per ambiguous event.
- Locality law: respelling or re-ending one marked event leaves unaffected candidate bytes identical.
- Corpus thresholds (47/58 key durations, no pedal-as-marked-end regression) plus spelling round-trip laws for
  chromatic/atonal/microtonal/pitch-bend evidence.

## Check

```sh
cargo nextest run -p musa-project
cargo clippy --all-targets -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-project --bench transcription_trial
```

Commit as `Spell pitches and infer written ends`.

## Stop

- No NotationProposal, canonical source preview, or source insertion (205c, 208).
- No dynamics, articulation, pedal, slur, or ornament insertion by default; no learned model.
- No tonal/SATB/equal-temperament universal default.
