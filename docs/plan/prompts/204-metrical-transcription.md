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

- Prompt 203's final report (`docs/notes/research/90-midi-transcription-trial.md`), checked-in corpus/results, and
  repaired Design/thresholds in this file; prompt 202's take and clock facts; prompts 61, 64, 72–75, and 200; exact
  meter/tempo/barline and notation-plan APIs.
- Governing exact-time/event-track rules and Open Music Theory chapters 009–012, 098, and 118 cited by prompt 201.
- The current bundled standard-library module tree and checked-value bridge before adding any transcription policy.

## Design

Separate the high-information known-clock case from free capture:

- **Known clock:** invert the exact captured transport time map into performed beat, retaining calibration residuals and
  the calibrated score-tick-zero origin. Never rebase a known-clock take to its first onset: the trial proved that this
  destroys pickup/bar phase.
- **Free capture:** return ranked tempo, pulse, phase, and meter hypotheses under explicit bounds. User-supplied tap
  anchors/downbeat constraints override inference locally and must be sufficient to reproduce the same result exactly.
  Free capture remains a Review result until the retained structural candidates agree under those constraints; there is
  no inferred-probability threshold.

Construct a finite hierarchical candidate DAG with shared back-pointers, not cloned phrase vectors. Legal exact rational
onsets and ends come from the meter context and checked policy: binary/ternary subdivision, allowed tuplets, pickup,
ties across beat/bar boundaries, and changing/asymmetrical or unmeasured scope. Optimize a joint phrase path, not each
event independently. The exact ordered cost record includes onset displacement, key-release/duration displacement,
tempo-map smoothness, notation complexity, rests, ties, tuplets, syncopation preservation, and user constraints. Hard
constraints filter first. Remaining candidates order by exact weighted total, then the complete cost vector in the
published field order, then canonical structural bytes. Every term, normalization, weight projection, and ordering
version is explicit. Physical-time residuals may be measured as floats at the device edge, but ranking converts them to
normalized integers before exact weights are applied; musical positions and tie-breaks never use floats.

Notation complexity is a preference, never a proof that simple rhythm was intended. Preserve systematic displacement,
swing, tresillo, and tuplets as competing candidates when evidence supports them. Do not turn velocity accents into
metrical truth. Ametric/unmeasured capture and a phrase whose top candidates disagree beyond the trial threshold return
an explicit “tap pulse / choose reading / write source” review need rather than a fabricated grid.

Declare public `TranscriptionPolicy` data and named structural policies in ordinary `stdlib` Musa source. The host reads
one checked value through the existing projection law and owns the bounded optimizer and captured timestamps. Rust has
no independently constructible meter/tuplet/style catalogue and no hidden default. Policy values use exact quantities;
floating point is confined to measured-time likelihood/cost arithmetic and never enters candidate musical positions.

Each immutable candidate carries exact onset/duration groups, rests/ties/tuplets required for notation, complete cost
breakdown, alternative boundaries, source take/revision/policy identities, and a derivation from raw event ids. Preserve
onset-group alternatives and voice slots for prompt 205; do not choose voices in a rhythm-only pass that prompt 205 must
undo. “Needs review” is local whenever retained candidates disagree on phase, grouping, written end, rest, tie, or
tuplet structure; it is not a percentage. Return at most five materialized candidates, retain at most 96 states at a
search layer, accept at most 128 completed notes, and keep candidate/back-pointer storage below 128 KiB on that bound.
The reference-host 128-note benchmark must remain at or below 50 ms. Split only at a retained complete phrase boundary;
otherwise refuse an oversized/exhausted take with its exact retained length.

The trial's 24-tick beat is a corpus measurement grid, not a host catalogue. The checked source policy declares its
allowed binary/ternary subdivisions and tuplets. The host projects and differential-tests those finite exact values; it
does not independently admit every trial tick. Unmeasured input returns `write source` unless a musician supplies a
metrical scope.

## Target

- Standard-library transcription policy declarations and exact differential readback into a private project optimizer.
- Versioned immutable rhythm candidate/report facade behind one project request; no optimizer state or standard-library
  projection exposed publicly.
- Known-clock, free-clock, tap-anchor, simple/compound/asymmetric/changing/unmeasured, pickup, tie, rest, tuplet,
  syncopation, swing, drift, and ambiguity fixtures from prompt 203's corpus.
- Determinism, exact-boundary, top-K ordering, constraint locality, bounded-search, adversarial-size, and measured
  latency/memory laws meeting prompt 203's exact five-candidate/96-state/128-note/128-KiB/50-ms thresholds. Corpus laws
  preserve at least 54/58 exact onsets, 8/9 intended top-five recall, 47/58 key-duration matches, and no more than 23
  source-token edits; tie/tuplet regressions remain visible rather than being averaged away.

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
- No HMM/probability facade: the measured HMM-shaped weighting produced the same ranking and errors as structural DP.
- No candidate becomes canonical or editable; it remains a derived proposal until prompt 208 accepts it.
