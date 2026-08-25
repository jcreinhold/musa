---
id: 177
slug: performance-gestures
status: done
depends_on: [119, 174, 176, 176a, 176b, 176c]
phase: 3
---

# Performance Profiles Produce Source-Declared Gestures

> **Repaired twice before implementation.** Commit `b5405b89` moved temporal support out of payloads. Note 79 then
> caught the uncommitted fixed Rust gesture/control ontology. Both findings stand: occurrence spans own time, and
> ordinary Musa declarations own musical payload vocabulary and interpretation policy.

## Task

Introduce the instrument-independent performance object between `EventTrack(WrittenTime, ScoreFact)` and a scheduled
event-source machine. Declare gestures, indexed controls, profiles, and standard interpretation policy in
`std::performance`; use a compiler-owned provenance/track bridge only for work source cannot perform. Tempo, groove, and
tuning then schedule the exact checked gesture track. No gesture names a primitive, processor, MIDI controller, or
render-plan parameter index.

## Read

- Constitution §§8–9; `docs/rules/language/{00-semantics,02-core-calculus,08-performance-and-sound}.md`; payload
  admission rule A1–A6 from prompt 176a; note 79.
- `musa-events` term/timeline/occurrence laws and the track builtins' private-representation/provenance ownership.
- `stdlib/src/notation`, current profile/performance lowering, and the provisional Rust `Gesture` used by prompts
  172–174. Treat it as a runtime oracle, not the new source schema.
- OMT 007 for dynamics/articulation and OMT 114 for why loudness change is not one DSP operation.
- Peyton Jones chapters 3–6 for source data/pattern translation; the existing Miller-pattern unifier and indexed-family
  tests for the only admissible omitted-index mechanism.

## Design

Add a real `std::performance` module. Declare storable source data for gesture identity, note gestures, control curves,
phrase/group relations, techniques, releases, and their exact canonical schema. Standard controls are ordinary values.
There is no closed Rust `Gesture`, `ControlKey`, connection, or technique enum as semantic authority.

Controls are dependently related rather than dynamically tagged by host code. A source `ControlKind` indexes both
`ControlKey(K)` and `ControlValue(K)`; a heterogeneous stored control uses an ordinary source family that binds the
index. A constructor or function may omit `K` only where the existing Miller-pattern unifier uniquely solves it.
Duplicate, escaping, flex-flex, or unresolved constraints are postponed/refused by the general rules—never guessed by a
sound-specific table. Add focused elaboration laws for inferred, postponed, ambiguous, and ill-scoped control indices.

A performance profile is an ordinary source record/function collection evaluated during finite compilation. It maps a
public source-declared performance view of notation facts into exact gesture data. Functions never cross a storable
payload or machine boundary. The host bridge may traverse the opaque written track, attach/reuse provenance, change
coordinates, enforce work budgets, and construct the performed track; it must not decide dynamic levels, articulation
meaning, grouping, technique, or standard-control policy. State and test the source/profile result presented to each
bridge call so there is no hidden host musical input.

The gesture object is `EventTrack(PerformedTime, Gesture)`, not another container. An occurrence span carries performed
onset and extent. Its payload carries stable identity, written pitch until tuning, separation/hold/emphasis intent,
symbolic techniques, grouping, per-note controls, and source-derived origin—but no absolute position in any coordinate.

MIDI score mode and provenance still need written support. Record it in a separate immutable
`GestureId -> written EventId/span` lineage projection. It is presentation/conversion data, excluded from gesture
canonical equality and scheduling. Physical attack seconds remain a separately keyed temporary legacy projection and are
removed at 178.

For a hairpin on `[s,e]`, the source neutral profile states and tests `E(b) = d0 + (d1-d0) * p((b-s)/(e-s))` with exact
endpoints and `Progress`. Sampling remains downstream. Preserve symbolic technique/group identity even when a source
profile also supplies a numeric fallback.

Rust consumers receive the minimum opaque/read-only projection needed by scheduling, MIDI, and DSP. Every projected
field derives from the checked source gesture and has exact/differential laws. No projection is a public construction
API or alternate canonical encoding.

## Target

- `stdlib/src/performance/`: source-declared gesture/control/profile vocabulary, neutral profile, docs, and laws.
- Pattern-unification tests for indexed controls, including postponement and refusal cases.
- One provenance-preserving, budgeted host bridge from checked profile results to `EventTrack(PerformedTime, Gesture)`
  plus separate lineage/temporary compatibility projections.
- Profile interpretation of all existing marks with parity where the old model was expressive and explicit retained
  information where it was not.
- Exact payload encoding/admission row; curve, grouping, scheduling, MIDI-loss, and source↔projection laws.
- No authoritative Rust musical enums and no studio dependency in the interpretation pass.

## Check

```sh
cargo nextest run -p musa-calculus -p musa-compiler -p musa-notation -p musa-dsp -p musa-project
cargo clippy --all-targets -p musa-calculus -p musa-compiler -p musa-notation -p musa-dsp -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-compiler
rg -n "data Gesture|data ControlKey|neutral" stdlib/src/performance
```

Commit as `Interpret notation with source performance profiles`.

## Stop

- No instrument implementation, sample selection, part routing, or DSP parameter resolution.
- No event-track operation, new core term, physical-time payload, or second temporal structure.
- No universal ontology of expression; standard definitions are edition-pinned library policy and custom declarations
  remain explicit.
- No source closure stored in an occurrence/machine and no Rust enum mirroring a source family.
- No special unifier, coercion, default control kind, or host-side type guess.
