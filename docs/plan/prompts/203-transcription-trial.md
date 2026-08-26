---
id: 203
slug: transcription-trial
status: in-progress
depends_on: [202]
phase: 2
---

# Measure Transcription Models Before Choosing One

## Task

Build a production-neutral trial over repository-owned generated expressive MIDI takes and an exact-pinned external
real-performance corpus, compare candidate rhythm/voice models, and repair prompts 204–205 to the evidence. Decide what
Musa can transcribe reliably, where it must ask the musician, and which complexity and latency budgets make Review feel
immediate before a production API exists.

## Read

- Prompt 201's literature and contract, prompt 202's complete take/calibration facts, prompt 93's benchmark method, and
  prompt 191's measurement discipline.
- Cemgil et al.'s Bayesian onset/notation-complexity model; Nakamura et al.'s merged-output HMM for polyphonic rhythm;
  Madsen and Widmer's proximity/cost voice separation; Wachter, Murgul, and Heizmann's
  [beat-annotated transformer quantizer](https://arxiv.org/abs/2604.22290); qparse's
  [weighted-tree-automata/dynamic-programming account](https://qparse.gitlabpages.inria.fr/docs/scientific/); Beyer and
  Dai's [MIDI-to-score transformer](https://arxiv.org/abs/2410.00210); and the current
  [ASAP dataset](https://github.com/fosfrancesco/asap-dataset) documentation/license.
- Open Music Theory chapters 009–012, 098, and 118. Include syncopation, tuplets, asymmetrical/changing meter, ametric
  material, and metrical displacement as falsifiers for a “nearest grid is clean notation” assumption.

## Design

This is symbolic event-stream processing. MIDI already supplies discrete note numbers and key-action transitions—not
written pitch—so do not add audio decoding, spectrograms, onset detection, source separation, or pitch tracking. Trial
the actual unresolved stages:

1. timestamp calibration and robust conversion into physical time;
2. known-clock mapping versus free-performance tempo/beat/downbeat inference;
3. joint onset/offset quantization on a hierarchical metric grid;
4. readable rest, tie, dot, and tuplet spelling under bar boundaries;
5. onset grouping versus rolled chords/arpeggios;
6. polyphonic voice assignment with pitch/temporal continuity and crossings as costs, not universal prohibitions; and
7. pedal-aware distinction among key duration, sounding duration, and proposed written duration.

Compare at least: independent nearest-grid rounding; a bounded dynamic-programming candidate lattice with explicit
timing/complexity costs; a Bayesian/HMM-shaped baseline derived from the cited papers; and, as an offline research
reference only, published modern learned-model results or an exact-pinned local model when licensing and hardware
permit. No model download, Python environment, or noncommercial dataset becomes a production/build dependency.

Build a repository-owned corpus from short Musa fixtures and deterministic performed traces varying tempo drift, swing,
rubato, articulation, chord spread, pedal, tuplets, syncopation, pickup, mistakes, repeated notes, crossing voices, and
silence. Store a small exact event format and intended score and license it with the repository. Unit, property, and
benchmark tests inject this event format immediately below the device boundary; they never require MIDI hardware, an OS
virtual port, callback timing, or a particular computer keyboard layout. A small private performance driver may feed the
same seam from scripted events or QWERTY key actions for functional testing, but QWERTY input is an audition/test
controller, not a second notation-entry mode and not a production surface in this prompt.

Evaluate real human performance through an adapter for an exact-pinned user-provided ASAP v1.1 checkout
(`fad8d1e8078d0ae47ad2f280b5d022bd2de24784`). Record only aggregate results and fixture identities; do not copy its
[CC BY-NC-SA data](https://github.com/fosfrancesco/asap-dataset/blob/v1.1/LICENSE.md) into Musa, retain performer
identity, make the Check hardware/network-dependent, or call generated timing a human recording. The local trial keeps
no identity or raw take in the repository unless the performer explicitly consents to that exact artifact under Musa's
license; aggregate measurements and anonymous error classes are sufficient. Provide the local functional harness here,
but do not block this model trial when no port is currently connected: prompt 209 requires representative physical-
keyboard workflow evidence before deleting step entry. Prompt 202's machine record remains an accurate record of that
earlier run, when no physical input was connected, rather than a claim about later availability.

Evaluate notation, not only note matches: onset and duration accuracy, bar/beat phase, voice assignment, chord grouping,
tie/rest/tuplet structure, edit distance to intended Musa source, number and locality of review corrections, ranked
candidate recall, runtime, peak memory, and determinism. A score that matches sounding intervals but needs twenty manual
repairs is worse than one whose one ambiguous beat is honestly exposed.

The intended production direction is a deterministic top-K bounded search with an exact cost breakdown and musician
constraints, not a hidden “AI confidence” number. The trial may overturn that direction only with reproducible evidence
and an offline, versioned, inspectable, license-compatible model whose failure modes are still reviewable. Record why
tap-pulse/downbeat input resolves more ambiguity than additional inference where that is what the measurements show.

Before marking this prompt done, update prompts 204–205 with the winning representation, bounds, corpus thresholds, and
rejected alternatives. No later prompt may simply say “quantize the MIDI.”

## Target

- Private trial implementations and benchmark harness isolated from production callers, with corpus schema/generator,
  repository-owned fixtures, semantic-seam scripted/QWERTY driver, optional external-dataset adapter, raw results, and
  machine/toolchain/device record.
- A research report comparing models and error classes, including example score/source diffs and review-operation
  counts.
- Fixed production decisions: candidate representation, cost terms/order, top-K/search bounds, known/free-clock split,
  voice/chord/pedal handling, ambiguity threshold, latency/memory budgets, and unsupported cases.
- Repaired prompts 204–205 and planned code-map entries grounded line by line in the report.

## Check

```sh
cargo nextest run -p musa-project
cargo bench -p musa-project --bench transcription_trial
cargo clippy --all-targets -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
make docs-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Measure MIDI-to-notation transcription models`.

## Stop

- No production transcription facade, UI candidate, source edit, step-entry deletion, audio transcription, or model
  service.
- No network in tests/builds, vendored noncommercial corpus, unpinned model, hidden random seed, or opaque aggregate
  score.
- Do not optimize the benchmark before the report identifies a measured bottleneck.
