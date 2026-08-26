---
id: 204
slug: metrical-transcription
status: pending
depends_on: [203]
phase: 2
---

# Recover Readable Rhythm as Ranked Exact Candidates

## Task

Implement prompt 203's admitted bounded transcription model for timing and rhythm. Turn one calibrated MIDI take into a
small ranked set of exact metrical candidates with local alternatives and explanations, before pitch spelling, chords,
voices, source insertion, or UI review is added.

## Read

- Prompt 203's final report and repaired Design/thresholds in this file; prompt 202's take and clock facts; prompts 61,
  64, 72–75, and 200; exact meter/tempo/barline and notation-plan APIs.
- Governing exact-time/event-track rules and Open Music Theory chapters 009–012, 098, and 118 cited by prompt 201.
- The current bundled standard-library module tree and checked-value bridge before adding any transcription policy.

## Design

Separate the high-information known-clock case from free capture:

- **Known clock:** invert the exact captured transport time map into performed beat, retaining calibration residuals.
- **Free capture:** return ranked tempo, pulse, phase, and meter hypotheses under explicit bounds. User-supplied tap
  anchors/downbeat constraints override inference locally and must be sufficient to reproduce the same result exactly.

Construct a finite hierarchical lattice of legal exact rational onsets and ends from the meter context and checked
policy: binary/ternary subdivision, allowed tuplets, pickup, ties across beat/bar boundaries, and changing/asymmetrical
or unmeasured scope. Optimize a joint phrase path, not each event independently. The exact ordered cost record includes
onset displacement, key-release/duration displacement, tempo-map smoothness, notation complexity, rests, ties, tuplets,
syncopation preservation, and user constraints. Every term is normalized and versioned; deterministic lexical tie-breaks
follow equal total cost.

Notation complexity is a preference, never a proof that simple rhythm was intended. Preserve systematic displacement,
swing, tresillo, and tuplets as competing candidates when evidence supports them. Do not turn velocity accents into
metrical truth. Ametric/unmeasured capture and a phrase whose top candidates disagree beyond the trial threshold return
an explicit “tap pulse / choose reading / write source” review need rather than a fabricated grid.

Declare public `TranscriptionPolicy` data and named structural policies in ordinary `stdlib` Musa source. The host reads
one checked value through the existing projection law and owns the bounded optimizer and captured timestamps. Rust has
no independently constructible meter/tuplet/style catalogue and no hidden default. Policy values use exact quantities;
floating point is confined to measured-time likelihood/cost arithmetic and never enters candidate musical positions.

Each immutable candidate carries exact onset/duration groups, rests/ties/tuplets required for notation, complete cost
breakdown, alternative boundaries, source take/revision/policy identities, and a derivation from raw event ids. “Needs
review” names close alternatives and the events affected; it is not a percentage unless prompt 203 actually calibrated
one. Top-K, state count, phrase length, subdivisions, and wall time are bounded with an actionable refusal.

## Target

- Standard-library transcription policy declarations and exact differential readback into a private project optimizer.
- Versioned immutable rhythm candidate/report facade behind one project request; no optimizer state or standard-library
  projection exposed publicly.
- Known-clock, free-clock, tap-anchor, simple/compound/asymmetric/changing/unmeasured, pickup, tie, rest, tuplet,
  syncopation, swing, drift, and ambiguity fixtures from prompt 203's corpus.
- Determinism, exact-boundary, top-K ordering, constraint locality, bounded-search, adversarial-size, and measured
  latency/memory laws meeting prompt 203's thresholds.

## Check

```sh
cargo nextest run -p musa-calculus -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-calculus -p musa-compiler -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-project --bench transcription_trial
```

Commit as `Transcribe MIDI timing into exact rhythm candidates`.

## Stop

- No pitch spelling, chord grouping, voice separation, dynamics, pedal notation, UI, audition, or source insertion.
- No audio DSP, learned/network model, unbounded beam/search, random tie-break, hidden host policy, or style universal.
- No candidate becomes canonical or editable; it remains a derived proposal until prompt 208 accepts it.
