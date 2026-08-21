---
id: 157
slug: instrument-contracts
status: pending
depends_on: [153, 154, 155, 156]
phase: 3
---

# Instruments Expose Contracts and Hide Implementations

> **Governed by the event-track and machine core installed by prompts 127a–127e and 150–153.** An instrument is a typed
> machine contract over private registered primitives, not a separate graph semantics.

## Task

Make `instrument` the deep gesture-to-audio abstraction. An instrument exposes a typed gesture/control signature and
hides whether it is implemented by oscillators, samples, or later adapters. Its implementation constructs a
`Machine<AudioFrameStep,EventBatch<Gesture>,AudioFrame>` from registered primitives and fixed wiring.

## Read

- The revised machine, scheduling, and audio rules; `R1` in the revised backend contract.
- Prompts 150–152 and `docs/plan/code-map/process-runtime.md`; these fix private runtime state, exact preparation,
  one-frame semantics, and batching premises.
- `docs/rules/language/08-performance-and-sound.md`; roadmap §§2, 6.5, 10.6, 13, 15.
- Current `StudioSpec`, `StudioGraphSpec`, `RenderPlan`, project/CLI/offline/engine callers, and prompts 29–31 repairs.
- Module-design audit of `musa-compiler`, `musa-dsp`, and `musa-playback`; compare recent history for their facades.

## Design

Implement a source/compiler `InstrumentSpec` with stable identity, `ControlSignature`, defaults, documented technique
support/fallbacks, output channel shape, and a private implementation body. Native machine bodies may name and modulate
their own primitives. Outside the body, only exposed controls are addressable. An exposed control has stable key,
type/unit, default/range, documentation, and an explicit mapping to one or more private parameters.

Delete the old `patch` declaration. Its former spelling is a hard error with a source fix to the new instrument form,
not an accepted desugaring. A library may export an instrument and its signature but not its private primitives or
state.

Compare two real module boundaries in completion notes:

1. project/compiler pass separate gesture, machine, and routing internals through every caller; or
2. `musa-dsp` exposes one preparation operation over caller-oriented event tracks, machine values, bindings, and options
   and returns an opaque prepared machine consumed by offline rendering and the engine.

Choose the second unless caller inspection proves otherwise. Primitive state, resolved parameter indices, buffers,
sample voices, and DSP instances remain private to `musa-dsp`. The engine receives only a prepared, RT-safe machine and
transport commands.

**The prepared machine is the runnable result.** An audio history is not a finite source value, but the finite machine
that produces it is part of the core language. Preparation is one operation rather than several:

- Implement the conceptual signature
  `prepare_execution(Gestures, Bindings, Seed, Options) -> Result<PreparedMachine, PrepareError>`. `Options` includes
  sample rate, channel contract, batching policy, render bounds, and every deterministic quality/acceptance choice. No
  option remains ambient. This is **the one place a rational becomes a float**; nothing upstream holds sample frames and
  nothing downstream holds a written-time coordinate.
- `Gestures` is the exact event-track projection, not a full presentation and not merely a finite digest.
  Presentation-only origin fields feed a separate `prepare_lineage(GesturePresentation, PreparedMachine)` operation and
  cannot modify the execution result.
- The successful result owns the machine built by `schedule`, instrument implementations, routing, and effects. Feedback
  comes only from the initialized core constructor. One sample frame is the semantic step, independent of caller render
  partition.
- **R1** is equality of the complete preparation `Result` under equal complete arguments. Frame equality is conditional
  on equal allocation/initial state, external inputs, and conforming deterministic processors. Do not strengthen R1 to
  lineage equality or unconditional cross-device bit equality.
- A later cache uses a digest only to find candidates and confirms the exact complete versioned argument bytes. A key of
  `semantic_hash(M) ⊕ bindings ⊕ seed` is incomplete because it omits options and trusts finite hashes.

## Target

- Instrument/signature declarations in language/compiler and hard-error migration fixes for removed patches.
- Native machine implementation hidden behind `musa-dsp` preparation; curated facade and documented invariants.
- Instrument-body checking against the machine constructors and registered primitive catalogue from prompts 150–128.
- Static checking for duplicate/missing controls, incompatible mappings, private-node access, technique support, and
  channel shape.
- Instrument replacement law: two implementations of one signature accept the same gesture/control lanes without
  changing their schedule.
- One preparation operation pure in all complete arguments, plus separate presentation lineage. Test equal semantic
  event tracks with unequal presentation-only data, every execution-affecting option independently, the whole-machine
  ordering counterexample, `feedback` cycles, and caller-block partitions.
- Module-design audit and caller comparison; delete pass-through surface made obsolete by the deep boundary.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-dsp -p musa-playback -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-dsp
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-playback
```

Commit as `Give instruments typed sound contracts`.

## Stop

- No trait or plug-in registry for hypothetical implementations; use the concrete closed implementation family with a
  native machine as the current case and add sample bodies at prompt 163.
- No part routing yet, no sample decoding, and no GUI node canvas.
- No score, context, measure, or notation type crosses into `musa-dsp`.
- No signal or audio history as a finite source value, and no written-time coordinate past scheduling. The separate
  lineage query cannot mutate execution.
