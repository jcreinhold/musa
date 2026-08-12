---
id: 144
slug: audio-performance-closure
status: pending
depends_on: [93, 127, 132, 133, 137, 138, 139, 141, 142, 143]
phase: 4
---

# Close Audio Preparation and Rendering Against Measurement

> **Governed by `docs/rules/constitution.md` §7 and §4.** Prompt 126 decided that the core is a calculus of occurrences
> of any canonical payload, that signals stay outside it, and what that forbids. Read them before this prompt's Design.

## Task

Measure the complete performance→instrument→audio pipeline against prompt 93's audio baseline and realistic native,
sampled, media, routing, and UI workloads. Explain every material regression and optimize only demonstrated bottlenecks
while preserving exact intent, deterministic realization, deep module boundaries, source authority, and the callback's
no-allocation/no-lock/no-I/O contract.

## Read

- `docs/rules/constitution.md` §4, whose deferral of the signal question names *this prompt's measurement* as one of the
  two events that reopens it. `R1` in `docs/rules/kernel/07-backend-contract.md`.
- Prompt 93 baseline/expected-change ledger, prompt 127 score-elaboration report,
  `docs/rules/desktop/06-performance.md`, roadmap §§13.2/17.5, and all completion notes from prompts 130–143.
- Audio/compiler/project benchmarks, callback instrumentation, offline/live render code, prepared-plan queues and
  retirement, decoded-asset store, sampler/media voices, UI Sound/Mix performance tests.
- `docs/rules/across-stages/03-process-calculus.md` and `04-identity-and-realization.md`; prompt 131's process/identity
  completion notes.

## Design

Add deterministic workloads which isolate and combine:

- many parts sharing one instrument declaration but requiring isolated instances and sends;
- dense note gestures and simultaneous standard/custom control curves;
- large key/velocity sample maps, round-robin/release/pedal behavior, and high polyphony;
- bounded large SFZ and SoundFont imports, cold preparation versus warm verified-cache preparation;
- overlapping fixed-media cues and looping/rate clips at several sample rates/block sizes;
- plan install/retire, seek/loop, Sound/Mix source edit→recompile→prepare→audible update;
- malformed/adversarial assets and packages outside timed success paths but inside resource bounds.

Measure compiler/profile cost, audio preparation wall/CPU time, allocation count/bytes, peak resident/decoded asset
memory, prepared-plan size, callback max/p95 time and deadline misses, voices processed/stolen, control evaluation,
offline throughput, UI response, and cache behavior. Record machine/toolchain, sample rate, block size, corpus digests,
method, uncertainty, and raw results.

**Measure `R1`, and report the result whether or not it is convenient.** Construct presentation pairs with equal
`Sem_Gesture` and unequal presentation-only fields; under equal bindings, seed, and complete options they must return
the same complete preparation `Result`. Then vary each option, binding, and seed independently to prove it is in the
exact argument record rather than ambient. Lineage may differ and is measured separately. A preparation difference under
equal complete inputs means the boundary is wrong and is reported against `docs/rules/constitution.md` §4.

Frame comparison is a second conditional experiment: hold allocation/initial node and register state, external input
history, parameters, and processor conformance fixed, then compare output. Report whether the promise is exact bits or a
named numeric tolerance per processor/target. Do not attribute a failed runtime premise to temporal semantic equality.

Rendering must be independent of host block partition wherever the specification promises it. Test all partitions of the
same frame count, especially registered feedback, envelopes, modulation, and media. Caches key on one canonical complete
`ExecArgs` record covering the operation version, exact gesture semantic bytes/schema, instrument/studio bindings,
locked asset/package identities, seed, sample rate/channel/tick policy, bounds, and render options. A digest selects
candidates; exact complete argument bytes confirm a hit. Inject deliberate digest collisions. Eviction changes cost
only. Streaming is admitted only if measured preloading misses a stated workload; its control-side producer, bounded
queue, underrun semantics, and offline determinism must then be specified and tested.

## Target

- Checked-in generated/small workloads, benchmark harnesses, raw results, profiles, and comparison report.
- Measured budgets and scale variables added to language/interface performance documentation.
- Focused fixes tied to observed profiles, with cached/uncached and block-partition differential laws.
- RT instrumentation proving callback and destruction constraints across native/sample/media plans.
- The R1 preparation, conditional-frame, lineage-separation, digest-collision, and host-partition differential results,
  with pairs/premises/outcomes stated plainly.
- Public-surface/dependency audit after optimization.

## Check

```sh
cargo bench -p musa-compiler
cargo bench -p musa-audio
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo deny check
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
cd apps/musa-desktop/ui && npx playwright test --project=budgets
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-audio
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-engine
```

Commit as `Close audio performance against measured works`.

## Stop

- No intuition-driven optimization, unbounded cache, callback best effort, benchmark-only branch, or relaxed semantics.
- No graph/decoder/buffer type exposed to improve a benchmark harness.
- No plugin hosting or pitch-preserving time stretch smuggled in as an optimization.
