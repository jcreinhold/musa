---
id: 205
slug: note-pairing-onset-groups
status: pending
depends_on: [202, 204b]
phase: 2
---

# Pair Note Lifecycles and Cluster Onset Groups

## Task

Turn the raw note-ons/offs and controller transitions a capture keeps into exact completed-note intervals and onset
groups, before any rhythm, voice, or spelling work. A completed note separates the physical key interval from the
pedal-extended sounding interval, keeps attack and release velocities, and names the raw event ids it came from. Near
onsets are clustered into candidate groups by the measured adaptive window, never a fixed global one.

## Read

- Prompt 202's complete event evidence and pairing laws in `crates/musa-project/src/midi.rs`.
- Prompt 203's measured grouping/voice decision in `docs/notes/research/90-midi-transcription-trial.md` (the adaptive
  window, the 40 ms attack at 120 BPM, the fixed 35 ms rejection).
- Prompt 204b's `Take`/`RhythmEvent` facade in `crates/musa-project/src/rhythm.rs` and `transcription_search.rs`, which
  this extends with pitch and evidence rather than reimplementing.

## Design

- Pair note-ons to note-offs **per channel and pitch**, in arrival order, so retriggers of a held key become distinct
  notes and an overlapping retrigger never merges into one. A key release ends the **physical** key interval (the
  "written end could be" fact); a sustain pedal release ends the **sounding** interval. The two stay two facts.

- Each completed note carries: note number, attack and release velocity, onset micros (the note-on), key-release micros
  (the paired note-off, or the last observed release time), sounding-end micros (pedal-extended when CC64 held the note
  into a pedal span, else the key release), and the ordered raw event ids that produced it. A note-on with no paired
  note-off and a note-off with no paired note-on are declared losses, not invented intervals.

- Pedal and controller spans are retained beside notes as their own fact list (channel, controller, value, span), never
  folded into written ends. Sostenuto and sustain are distinct controller records.

- Cluster near onsets with the measured starting proposal: **one twelfth of the local beat, clamped to 18–70 ms**. The
  40 ms corpus attack at 120 BPM must remain one available group; a fixed 35 ms baseline must not reappear. A compact
  spread is one group candidate, an ordered spread is an arpeggio candidate, a fast scale is neither, and competing
  groupings survive as alternatives when their cost separation is small. The window is a candidate cost, not a hard
  split.

- No probability, no voice/spelling, no rhythmic grid (204b owns grid search), no source preview.

## Target

- One private completed-note construction from captured events: exact physical-versus-sounding intervals, attack and
  release velocities, derivation event ids, and typed unpaired-on/unpaired-off losses. Retain pedal/controller spans as
  separate facts.
- One adaptive onset-group clustering with the beat-relative window computed from a quarter-micros value, and the
  block/rolled/arpeggio/separate alternative enumeration as candidate-group structure (no source text).
- Laws: retrigger-is-distinct, pedal-extended sounding end never becomes a key release, a 40 ms spread at 120 BPM is one
  group while the same spread at a much faster tempo is not, unpaired evidence is a loss not an interval, and the window
  formula clamps at 18 ms and 70 ms.

## Check

```sh
cargo nextest run -p musa-project
cargo clippy --all-targets -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-project --bench transcription_trial
```

Commit as `Pair note lifecycles and cluster onset groups`.

## Stop

- No rhythmic quantization, voice assignment, pitch spelling, or written-end inference (205a–205c).
- No source text, NotationProposal, Review UI, or audition.
- No fixed millisecond group window; no DSP/audio; no learned model.
