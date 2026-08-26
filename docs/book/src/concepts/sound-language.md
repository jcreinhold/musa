# How the sound language crosses the host boundary

Musa's sound language is ordinary dependently typed Musa source. `std::performance` declares indexed control keys,
values, gestures, and profiles; `std::sound` declares exact quantities, instrument signatures and mappings, studio data,
sample maps, standard instruments, and processor wrappers. A new declarable instrument or control does not require a
Rust enum variant.

The central chain is:

```text
EventTrack[WrittenTime, ScoreFact]
  → profile interpretation
EventTrack[PerformedTime, Gesture]
  → exact scheduling
Schedule[Gesture]
  → instrument implementation
Machine[AudioFrameStep, State, Frame]
  → buses, sends, routes, main
Machine[AudioFrameStep, State, StereoFrame]
```

The first two stages remain exact. `prepare_audio` is the single late boundary that rounds event positions to frames,
converts exact quantities to DSP numbers, resolves source-declared controls to private parameter indices, allocates
bounded state, and returns an immutable plan. The callback only steps that plan: it does not allocate, lock, perform
I/O, log, fetch, decode, or inspect source names.

## One source language, one inference discipline

A `ControlKey(K)` and its `ControlValue(K)` share the same ordinary source index. Omitted indices are solved by the
core's one Miller-pattern unifier: a metavariable may be assigned only at a distinct local-variable spine, subject to
occurs and scope checks; blocked constraints postpone, and unresolved constraints are refused. Sound elaboration adds no
special inference table, coercion, delimiter convention, or host fallback.

Checked Rust projections exist for callers that need compact immutable facts. They have no public construction path,
retain the complete checked source identity, and are tested against the source artifact. Tooling reads declaration
indexes and these projections; it never reconstructs a declaration from spelling.

## Deep boundaries

The host owns what source cannot: provenance-preserving opaque event-track construction, verified asset bytes, foreign
format parsing, registered primitive state and resource contracts, exact-to-frame scheduling, compact resolved indices,
and real-time execution. The compiler does not expose environments, unification variables, HIR, private graph state, or
decoded media to an editor.

SFZ and SoundFont therefore enter as strict versioned adapters to the same checked `SampleMapArtifact` a Musa package
can declare. Their parsers are bounded and off-thread. Unsupported sound-changing opcodes, generators, and modulators
are errors that name the rejected input. The generated support reference cites the SFZ catalogue and SoundFont 2.04
specification without copying either specification.

## Adding one extension

To add a source processor wrapper, declare its musician-facing name, signature, units, ranges, defaults, documentation,
and example in `std::sound::catalogue`. If it composes existing operations, no host change is needed. If it requires new
private state, register one bounded RT-safe primitive and prove that its port/resource contract agrees with the checked
source wrapper.

To add a foreign format, write a bounded adapter that validates the foreign input and emits an existing checked source
contract before preparation. Give the adapter a versioned identity, publish its exact support/refusal matrix, preserve
asset provenance, and test equality with an equivalent native map. Do not add the foreign format's opcodes or controller
numbers to Musa's gesture vocabulary.
