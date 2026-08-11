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

**The prepared plan is the object that crosses the signal boundary.** `docs/core-boundary.md` §5 settled that signals
stay outside the core: a signal is coinductive where a timeline is inductive and finite, and a signal graph has no
extent. The consequence for this prompt is precise, and it is the reason preparation is one operation rather than
several:

- `prepare` takes an exact, finite, normalized `Timeline<Gesture>`, the instrument bindings, and the realization seed,
  and returns the opaque plan. It is **the one place a rational becomes a float**; nothing upstream of it holds seconds,
  frames, or samples, and nothing downstream of it holds a `Beat`.
- It must satisfy **`R1`** (`docs/kernel/07-backend-contract.md`, written at prompt 129a): semantically equal gesture
  timelines prepare identically and render frame-for-frame identically under the same bindings and seed. Preparation may
  therefore observe nothing that normalization forgets (N7). Prompt 144 measures this; a measured failure reopens
  `docs/core-boundary.md` §5 rather than being patched here.
- R1 is also what makes a preparation cache keyed on `semantic_hash(M) ⊕ bindings ⊕ seed` correct. Whether to build one
  is prompt 144's question, not this prompt's; keeping preparation a pure function of those three inputs is what leaves
  the option open.

## Target

- Instrument/signature declarations in language/compiler and migration of patches as specified.
- Native graph implementation hidden behind `musa-audio` preparation; curated facade and documented invariants.
- Static checking for duplicate/missing controls, incompatible mappings, private-node access, technique support, and
  channel shape.
- Instrument replacement law: two implementations of one signature accept the same gesture/control lanes without
  changing their schedule.
- One preparation operation, pure in `(Timeline<Gesture>, bindings, seed)`, with a test for R1's equal-in-meaning case:
  two gesture timelines that differ only in what normalization forgets prepare to the same plan.
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
  boundary is one function in one direction (`docs/core-boundary.md` §6 rule 4).
