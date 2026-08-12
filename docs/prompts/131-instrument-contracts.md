---
id: 131
slug: instrument-contracts
status: pending
depends_on: [128, 129, 130]
phase: 3
---

# Instruments Expose Contracts and Hide Implementations

> **Governed by `docs/core-boundary.md`.** Prompt 126 decided that the core is a calculus of occurrences of any
> canonical payload, that signals stay outside it, and what that forbids. Read it before this prompt's Design.

## Task

Make `instrument` the deep score-to-sound abstraction. An instrument exposes a typed gesture/control signature and hides
whether it is implemented by oscillators, samples, or later adapters. Replace the shallow public patch topology boundary
with one audio preparation operation that binds scheduled performance lanes, instrument declarations, and mix intent
into an opaque prepared plan.

## Read

- `docs/core-boundary.md` §5 — the signal question, and why the prepared plan is the object that crosses. `R1` in
  `docs/kernel/07-backend-contract.md`, which this prompt's preparation operation must satisfy.
- `docs/spec/03-process-calculus.md`, `04-identity-and-realization.md`, and `docs/architecture/process-runtime.md`;
  these fix the private IR, whole-node scheduling, exact preparation signature, and factorization/cache premises.
- `docs/language/08-performance-and-sound.md`; roadmap §§2, 6.5, 10.6, 13, 15.
- Current `StudioSpec`, `StudioGraphSpec`, `RenderPlan`, project/CLI/offline/engine callers, and prompts 29–31 repairs.
- Module-design audit of `musa-compiler`, `musa-audio`, and `musa-engine`; compare recent history for their facades.

## Design

Implement a source/compiler `InstrumentSpec` with stable identity, `ControlSignature`, defaults, documented technique
support/fallbacks, output channel shape, and a private implementation body. Native graph bodies may name and modulate
their own stages. Outside the body, only exposed controls are addressable. An exposed control has stable key, type/unit,
default/range, documentation, and an explicit mapping to one or more private parameters.

Existing `patch` declarations remain source-compatible by desugaring to native instruments. Record a deprecation or
expert-surface policy from the candidate spec; do not make old graph paths the new contract. A library may export an
instrument and its signature but not its private nodes.

Compare two real module boundaries in completion notes:

1. project/compiler separately hand a `PerformancePlan` and graph-shaped `StudioSpec` through callers; or
2. `musa-audio` exposes one preparation operation over caller-oriented performance/studio intent and returns an opaque
   prepared audio plan consumed by offline rendering and the engine.

Choose the second unless caller inspection proves otherwise. Graph compiler, node addresses, resolved parameter indices,
buffers, sample voices, and DSP processor instances remain private to `musa-audio`. The engine receives only a prepared,
RT-safe plan and transport commands.

**The prepared execution is the object that crosses the signal boundary.** `docs/core-boundary.md` §5 settled that
signals stay outside the core: a signal is coinductive where a timeline is inductive and finite, and a signal graph has
no extent. The consequence for this prompt is precise, and it is the reason preparation is one operation rather than
several:

- Implement the conceptual signature
  `prepare_execution(Sem_Gesture, Bindings, Seed, Options) -> Result<PreparedExecution, PrepareError>`. `Options`
  includes sample rate, channel contract, fixed semantic tick/block policy, render bounds, and every deterministic
  quality/acceptance choice. No option remains ambient. This is **the one place a rational becomes a float**; nothing
  upstream holds seconds/frames/samples and nothing downstream holds a `Beat`.
- `Sem_Gesture` is the structured admitted semantic projection, not a full presentation and not merely a finite digest.
  Presentation-only origin fields feed a separate `prepare_lineage(Presentation_Gesture, PreparedExecution)` operation
  and cannot modify the execution result.
- The successful result owns a private finite process definition implementing `docs/spec/03-process-calculus.md`:
  first-order total node transitions, a whole-node dependency DAG, and feedback only through explicit initialized
  registers. A port-level DAG is not sufficient. The semantic tick is fixed in options and independent of caller render
  partition.
- **R1** is equality of the complete preparation `Result` under equal complete arguments. Frame equality is conditional
  on equal allocation/initial state, external inputs, and conforming deterministic processors. Do not strengthen R1 to
  lineage equality or unconditional cross-device bit equality.
- A later cache uses a digest only to find candidates and confirms the exact complete versioned argument bytes. A key of
  `semantic_hash(M) ⊕ bindings ⊕ seed` is incomplete because it omits options and trusts finite hashes.

## Target

- Instrument/signature declarations in language/compiler and migration of patches as specified.
- Native graph implementation hidden behind `musa-audio` preparation; curated facade and documented invariants.
- Private typed process IR, formation checker, canonical whole-node schedule, explicit register state, and reference
  tick evaluator in `musa-audio`; current caller-block-sensitive feedback is migrated to the fixed semantic tick.
- Static checking for duplicate/missing controls, incompatible mappings, private-node access, technique support, and
  channel shape.
- Instrument replacement law: two implementations of one signature accept the same gesture/control lanes without
  changing their schedule.
- One preparation operation pure in all complete arguments, plus separate presentation lineage. Test equal semantic
  timelines with unequal presentation-only data, every execution-affecting option independently, the whole-node
  scheduling counterexample, registered cycles, and caller-block partitions.
- Module-design audit and caller comparison; delete pass-through surface made obsolete by the deep boundary.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-audio -p musa-engine -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-audio
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-engine
```

Commit as `Give instruments typed sound contracts`.

## Stop

- No trait or plug-in registry for hypothetical implementations; use the concrete closed implementation family with
  native graph as the one current case and add sample bodies at prompt 137.
- No part routing yet, no sample decoding, and no GUI graph canvas.
- No score, context, measure, or notation type crosses into `musa-audio`.
- No signal, stream, or other coinductive value in a kernel payload, and no `Beat` past the preparation boundary. The
  execution boundary is one function in one direction; the separate lineage query cannot mutate it.
