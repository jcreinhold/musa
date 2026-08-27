---
id: 204a
slug: rhythm-candidate-search
status: pending
depends_on: [204]
phase: 2
---

# Search Exact Rhythm Candidates under Checked Policy Bounds

## Task

Implement prompt 203's admitted bounded transcription model as a private `musa-project` module: turn one calibrated take
plus a named 204 policy into a small ranked set of exact metrical candidates, with the trial's proven structural costs
ported under policy ownership and every published bound enforced by law. This is the optimizer prompt 204 deferred; it
exposes no public API.

## Read

- Prompt 203's report (`docs/notes/research/90-midi-transcription-trial.md`): the admitted candidate-DAG representation,
  the "Decisions fixed for prompts 204–205" thresholds and orderings, the "What the errors look like" lessons, and the
  "Bounds and latency" measurements.
- Prompt 204's repaired policy module, decoder, and derivability law; the take/clock facts from prompt 202 in
  `crates/musa-project/src/midi.rs` and prompt 200's exact-time conventions in `crates/musa-project/src/barlines.rs`.
- Prompt 61, 64, 72–75 for notation-plan/meter/tempo APIs assumed by the meter scope input.
- Open Music Theory chapters `009`–`012` (meter, subdivision, ties, rests), `098` (twentieth-century rhythm), and `118`
  (metrical dissonance) for what a metrical scope must be able to say.

## Design

- Private `transcription_search` module in `musa-project`. The candidate search is an immutable DAG arena with shared
  back-pointers (indices into a flat arena), replacing the trial's cloned per-state vectors, so storage stays under the
  128-KiB law at 128 notes.

- Positions and durations are exact. Known-clock input inverts the captured transport time map into performed beat and
  keeps the calibrated score-tick-zero origin; it is never rebased to the first onset. Free-clock input returns ranked
  tempo, pulse, phase, and meter hypotheses under explicit bounds. Physical-time residuals may be floats only at the
  device edge and are normalized to integers before weights apply; musical positions and every tie-break are float-free.

- Port the trial's structural DP cost structure rather than redesigning it: squared residual, the declared complexity
  ladder, transition length, and adaptive group split — each now multiplied by its 204 policy weight. The complete cost
  record is the published ordered field list (onset displacement, key-release/duration displacement, tempo smoothness,
  notation complexity, rests, ties, tuplets, syncopation preservation, user constraints) with a `cost_version`.

- Hard evidence and musician constraints filter first. A user tap anchor or downbeat is an exact local constraint over
  the same take; reproducing the same take and constraints must reproduce the identical result. Optimization is over the
  joint phrase path, never per event independently.

- Bounds are enforced by construction and by law: at most five materialized candidates, at most 96 states retained per
  search layer, at most 128 completed notes, and candidate/back-pointer storage below 128 KiB at the 128-note bound. An
  oversized take splits only at a retained complete phrase boundary; otherwise it refuses with its exact retained
  length. An exhausted search bound refuses rather than returning a partial rank.

- The candidate carries exact onset/duration groups, rests/ties/tuplets required for notation, a complete cost
  breakdown, alternative boundaries, and a derivation from raw event ids. Onset-group alternatives and at most four
  voice slots are preserved structurally for 205, but no voice choice is made here.

- "Needs review" is computed structurally over retained candidates (disagreement on phase, grouping, written end, rest,
  tie, or tuplet), never as a probability. Unmeasured scope or the `unmeasured` policy yields `write source`; there is
  no invented grid.

- The trial's structural-DP corpus numbers are reproduced through the private optimizer, not a new design: an internal
  test `include_str!`s `crates/musa-project/tests/fixtures/transcription/corpus.json` and asserts at least 54/58 exact
  onsets, 8/9 intended top-five recall, 47/58 key-duration matches, and no more than 23 source-token edits. This makes
  the 204b facade plumbing, not a numerical surprise.

## Target

- Private `musa-project` module implementing the search; no public items.
- In-module laws: determinism, exact-boundary (float-free), top-K ordering (weighted total → cost vector in published
  field order → canonical structural bytes), constraint locality, 96-state/128-note/128-KiB bounds, adversarial-size
  refusal with retained length.
- The internal corpus regression test above, through the private optimizer.
- Extend `crates/musa-project/benches/transcription_trial.rs` with production-optimizer cases measuring the 128-note
  reference ≤ 50 ms law.

## Check

```sh
cargo nextest run -p musa-project
cargo clippy --all-targets -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-project --bench transcription_trial
```

Commit as `Search exact rhythm candidates under checked policy bounds`.

## Stop

- No public facade, report type, or ProjectSession request (204b).
- No pitch spelling, chord grouping, voice separation, dynamics, pedal, articulation, or source insertion (205–208).
- No HMM/probability facade, learned/network model, unbounded or random search, hidden host policy, or style universal.
- No candidate becomes canonical or editable.
