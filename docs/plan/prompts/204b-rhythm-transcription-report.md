---
id: 204b
slug: rhythm-transcription-report
status: pending
depends_on: [204a]
phase: 2
---

# Report Ranked Rhythm Candidates for a Captured Take

## Task

Expose the 204a search through one narrow, versioned, immutable report behind a single `ProjectSession` request over a
captured MIDI take, and prove the corpus admission thresholds, determinism, exact-boundaries, and reference-host
latency/memory laws through that facade.

## Read

- Prompt 203's report (`docs/notes/research/90-midi-transcription-trial.md`) thresholds and the "Decisions fixed for
  prompts 204–205"; prompt 204a's search module and its cost/candidate structure; prompt 204's policy facade.
- Prompt 202's complete take evidence in `crates/musa-project/src/midi.rs` and the immutable revision-scoped report
  conventions of prompt 200 in `crates/musa-project/src/barlines.rs`.
- Prompt 200's checked-score survivor list for how a report names the source/revision it describes.
- Open Music Theory chapters `009`–`012`, `098`, `118` for what a reported ambiguity must let a musician distinguish.

## Design

- One facade: `ProjectSession::transcribe_take_rhythm(take, policy, constraints) -> RhythmTranscriptionReport`, or a
  narrow request type of that shape. The report is immutable and carries a `report_version`.

- Each candidate in the report carries its exact onset/duration groups, rests/ties/tuplets required for notation, the
  complete versioned cost breakdown, alternative boundaries, the source take/revision/policy identities, a derivation
  from raw event ids, and local review needs. Onset-group alternatives and voice slots ride through untouched for 205.

- No optimizer state and no standard-library projection is public. Refusals are typed report outcomes rather than
  errors: `write source` for unmeasured scope, unsplittable oversized take (with exact retained length), exhausted bound,
  or a proposal beyond four voices. Free capture remains a review result until the retained structural candidates agree
  under the supplied constraints; there is no inferred-probability threshold.

- "Needs review" is local and structural: it names a region whenever retained candidates disagree on phase, grouping,
  written end, rest, tie, or tuplet structure there. It is never a percentage.

- Laws, run through the facade over prompt 203's corpus: at least 54/58 exact onsets, 8/9 intended top-five recall,
  47/58 key-duration matches, and no more than 23 source-token edits, with the tie/tuplet regressions asserted visible
  rather than averaged away. Determinism law: the same take, policy, and constraints produce byte-identical reports.
  Exact-boundary law: no reported musical position or tie-break contains a float. Top-K ordering law. Constraint
  locality law: a tap anchor changes only the region it constrains. Latency/memory law: the 128-note stress take
  completes through the facade with candidate/back-pointer storage below 128 KiB and within the reference-host 50 ms
  budget (the exact number is the bench's job; the test asserts completion, storage, and a generous CI-safe ceiling).

## Target

- Public versioned `RhythmTranscriptionReport` (with its candidate type) and one `ProjectSession` request, documented
  with invariants before implementation.
- Suite laws named above in `crates/musa-project/tests/suite/`.
- The existing `transcription_trial` bench remains green on the facade path.

## Check

```sh
cargo nextest run -p musa-project
cargo clippy --all-targets -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-project --bench transcription_trial
```

Commit as `Report ranked rhythm candidates from captured takes`.

## Stop

- No candidate becomes canonical or editable; it stays a derived proposal until 208 accepts it.
- No pitch spelling, chord grouping, voice separation, dynamics, pedal, or articulation (205).
- No review/rating UI or audition (207), no source insertion (208), no audio DSP or learned model.
- No probability language, hidden threshold, or style universal in the report.
