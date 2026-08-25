---
id: 178
slug: instrument-contracts
status: pending
depends_on: [174, 175, 176, 177]
phase: 3
---

# Source Instruments Expose Contracts and Hide Implementations

## Task

Make `instrument` the deep gesture-to-audio abstraction as an ordinary Musa declaration. Its source-defined signature
states gestures, indexed controls, techniques, defaults, and output shape; its private source implementation constructs
a `Machine(AudioFrameStep, EventBatch(Gesture), AudioFrame)` from registered primitives and fixed wiring. Rust owns
primitive contracts and preparation, not an `InstrumentSpec` language beside source.

## Read

- `docs/rules/language/08-performance-and-sound.md` §§0, 3–5; repaired 175–177 and notes 79 and 82.
- Prompts 171–174 and `docs/plan/code-map/process-runtime.md`: private state, exact preparation, one-frame semantics,
  batching, and the build-local primitive registry.
- Source modules/privacy, parameterized records, indexed families, pattern unification, `Storable`, and machine
  builtins.
- Current studio/render/project callers and a module-design audit of `musa-compiler`, `musa-dsp`, and `musa-playback`.
- Peyton Jones chapters 3–6 and Ousterhout chapters 4, 7–8 for source translation and the deep preparation boundary.

## Design

Declare `InstrumentSignature`, technique support/fallbacks, channel shape, mappings, and `Instrument` in `std::sound`
using ordinary records, data, functions, and values. Do not add an `instrument` keyword or a parallel parser/evaluator
path: the musician-facing declaration syntax belongs to prompt 181 and must elaborate to these library constructs. A
signature is storable source data. A private implementation may contain functions while finite source evaluation
constructs the rechecked storable machine; no function crosses into the running machine.

An exposed control is indexed by the same `ControlKind` used by prompt 177. Its key, accepted value type/unit,
default/range, rate, docs, and mapping agree by ordinary dependent typing. Omitted kind arguments use the one
Miller-pattern unifier, including postponement; no instrument-specific compatibility table substitutes for conversion.
Custom controls are ordinary namespaced declarations.

This prompt supplies the executable source wrappers deliberately deferred by prompt 176. Native implementation bodies
name those wrappers over registered primitives and may address their own private graph paths. Outside the body only
signature keys are addressable. Finite checking rejects duplicate/missing controls, incompatible mappings, private-node
access, unsupported techniques, and channel mismatch before preparation.

Do not delete or extend the old `patch` path here. Note 80 and prompt 180a retain it unchanged as the differential
migration oracle until checked source reaches complete studio parity; prompt 181 removes the compatibility spelling
with its hard source fix. This prompt adds no new semantic authority to that Rust path. A library exports its
instrument/signature and may keep its implementation declarations private through ordinary module privacy. Standard
instruments and presets remain readable source.

`musa-dsp` exposes one deep preparation operation over exact checked projections of gestures, machine values, bindings,
seed, and complete options, returning opaque `PreparedMachine`/`PreparedAudio`. The conceptual signature is
`prepare_execution(Gestures, Bindings, Seed, Options) -> Result(PreparedMachine, PrepareError)`, but the Rust facade may
use opaque exact artifacts rather than mirror the source schema. Primitive state, resolved indices, buffers, voices, and
DSP instances remain private.

R1 is equality of the complete preparation result under equal complete arguments. Presentation lineage is attached by a
separate non-executing operation. Candidate cache hashes are followed by exact complete argument comparison.

## Target

- Source instrument/signature/control-mapping declarations and standard-library examples built from ordinary Musa
  constructs; no parser keyword, special elaborator, or public Rust `InstrumentSpec` mirror.
- Private source machine bodies over registered primitive wrappers, with complete static conformance diagnostics.
- Deletion of the prompt-177 physical-attack compatibility projection. The unchanged `patch` oracle remains temporary
  until prompt 180a, and its surface spelling is removed by prompt 181.
- One opaque preparation facade and separate lineage attachment, with replacement, privacy, R1, feedback, option, and
  block-partition laws.
- Module-design audit and removal of pass-through surfaces.

## Check

```sh
cargo nextest run -p musa-calculus -p musa-syntax -p musa-compiler -p musa-dsp -p musa-playback -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-dsp
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-playback
rg -n "record Instrument|record InstrumentSignature" stdlib/src/sound
```

Commit as `Give source instruments typed sound contracts`.

## Stop

- No dynamic native plug-in registry, arbitrary callback closure, part routing, sample decoding, or GUI node canvas.
- No score/notation type in `musa-dsp`, source evaluator in runtime, or source closure in a machine value.
- No signal/audio history as finite source data and no written-time coordinate past scheduling.
- No Rust schema independently constructible as an instrument declaration.
- No new surface `instrument` grammar and no deletion or semantic expansion of the temporary `patch` oracle.
