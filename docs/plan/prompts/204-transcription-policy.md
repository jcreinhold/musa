---
id: 204
slug: transcription-policy
status: pending
depends_on: [203]
phase: 2
---

# Declare Transcription Policy as Checked Standard Source

## Task

Move every rhythm-transcription policy decision prompt 203 admitted out of host code before any optimizer exists.
Declare `TranscriptionPolicy` data, the named structural policies, exact cost weights, subdivision/tuplet allowances,
and the published bounded-search constants as ordinary standard-library Musa source; freeze them as one versioned
artifact through the checked-source boundary; and project them exactly into a private `musa-project` representation
held to the source by differential laws.

## Read

- Prompt 203's report (`docs/notes/research/90-midi-transcription-trial.md`), especially "Decisions fixed for prompts
  204–205": the exact five-candidate/96-state/128-note/128-KiB/50-ms thresholds, the published cost-field order, and the
  rule that the trial's 24-tick quarter is a corpus grid the host must derive rather than independently admit.
- The checked-source bridge pattern in `crates/musa-compiler/src/source_value.rs`; the artifact framing in
  `crates/musa-calculus/src/kernel/artifact.rs`; the source-owned-vocabulary decoder conventions in
  `crates/musa-dsp/src/source.rs` and its consumer cache in `crates/musa-project/src/vocabulary.rs`.
- The artifact and module-tree conventions in `stdlib/src/performance/mod.musa` and `stdlib/src/lib.musa`, and the
  source-declaration ownership test under "Source declarations versus host boundaries" in
  `docs/plan/prompts/README.md`.
- Open Music Theory chapters `009`–`012` for the subdivision and tuplet vocabulary the policy names; only the
  distinctions a presentation must make, not a hard-wired common-practice default.

## Design

- New module `std::transcription` declared from `stdlib/src/lib.musa` (`mod transcription;`), declared in one
  `stdlib/src/transcription/mod.musa`. It names, in ordinary source, the quantities prompt 203 fixed:

  - `Subdivision` — one admitted division of a beat with its exact beat fraction and a declared notation-complexity
    rank (binary eighth/sixteenth/thirty-second; ternary triplet-eighth).
  - `TupletAllowance` — one admitted irregular division (the standard policy admits simple triplets).
  - `CostWeights` — exact `Ratio` weights: onset residual, duration residual, tempo smoothness, notation complexity,
    transition, and group split. These are the trial's structural-DP terms as declared numbers, not host literals.
  - `SearchBounds` — `top_k = 5`, `layer_states = 96`, `max_notes = 128`, `max_storage_bytes = 131072`, `max_voices = 4`.
  - `GroupWindow` — the adaptive onset-group proposal: one twelfth of the local beat, clamped to `18/1000`–`70/1000`
    seconds.
  - `TranscriptionPolicy` — a named bundle of the above.
  - `TranscriptionPolicyArtifact` — `schema_version`, the published cost-field order as an exact list, a `cost_version`,
    and the list of named policies; frozen by one root binding `transcription_policies`.

  Named structural policies are at least `standard` (binary through thirty-second, ternary through triplet-eighth,
  the trial weights) and `unmeasured` (no admitted subdivision; its only honest outcome is `write source`). The
  unmeasured policy gives that refusal a declared identity instead of a host special case.

- Compiler bridge `checked_standard_transcription_policies()` beside the existing bridges in
  `crates/musa-compiler/src/source_value.rs`, requesting schema `std.transcription.TranscriptionPolicyArtifact`,
  version 1.

- `musa-project` decoder in a private `transcription_policy` module: a `pub(crate)` projection struct decoded from one
  `CheckedSource`. It has no constructor and no default; a caller must name a policy. The module records the checked
  `exact_bytes()` so any source/host drift is a decoding refusal, not silent reuse.

- The host search grid is the least common multiple of the policy's declared subdivision denominators. The corpus's
  24-tick quarter must be an assertion about derivability, never an independent host constant.

- Differential laws hold the projection to the source: hand-derived exact values (weights, window clamp, bounds,
  per-subdivision complexity ranks) match the decoded projection; a wrong-schema artifact is refused; the 24-tick grid
  equals the LCM of `standard`'s subdivisions; `unmeasured` admits no subdivision.

## Target

- `stdlib/src/transcription/mod.musa` and its `mod` line in `stdlib/src/lib.musa`.
- `musa-compiler`: `checked_standard_transcription_policies()`.
- `musa-project`: private policy decoder + the differential/derivability/refusal laws above.
- No optimizer, candidate, report, or benchmark code.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Declare transcription policy as checked standard source`.

## Stop

- No candidate search, optimizer state, transcription report, or ProjectSession request (204a/204b).
- No host-constructible policy value, hidden host default, or second policy catalogue.
- No pitch spelling, chords, voices, dynamics, or pedal policy (205 owns those inputs).
