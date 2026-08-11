---
id: 144
slug: audio-performance-closure
status: pending
depends_on: [93, 127, 132, 133, 137, 138, 139, 141, 142, 143]
phase: 4
---

# Close Audio Preparation and Rendering Against Measurement

> **Governed by `docs/core-boundary.md`.** Prompt 126 decided that the core is a calculus of occurrences of any
> canonical payload, that signals stay outside it, and what that forbids. Read it before this prompt's Design.

## Task

Measure the complete performance→instrument→audio pipeline against prompt 93's audio baseline and realistic native,
sampled, media, routing, and UI workloads. Explain every material regression and optimize only demonstrated bottlenecks
while preserving exact intent, deterministic realization, deep module boundaries, source authority, and the callback's
no-allocation/no-lock/no-I/O contract.

## Read

- `docs/core-boundary.md` §5, whose deferral of the signal question names *this prompt's measurement* as one of the two
  events that reopens it. `R1` in `docs/kernel/07-backend-contract.md`.
- Prompt 93 baseline/expected-change ledger, prompt 127 score-elaboration report, `docs/interface/06-performance.md`,
  roadmap §§13.2/17.5, and all completion notes from prompts 130–143.
- Audio/compiler/project benchmarks, callback instrumentation, offline/live render code, prepared-plan queues and
  retirement, decoded-asset store, sampler/media voices, UI Sound/Mix performance tests.

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

**Measure `R1`, and report the result whether or not it is convenient.** `docs/core-boundary.md` §5 deferred the signal
question on the strength of one law: semantically equal gesture timelines prepare identically and render frame for frame
identically under the same instrument bindings and realization seed. The measurement is a differential one and belongs
with the workloads above — construct pairs of gesture timelines that differ only in what normalization forgets (N7:
declaration order where order does not matter, an inlined name, sharing structure) and compare prepared plans byte for
byte and rendered output frame for frame. A failure is not a bug to patch downstream: it means something the kernel
forgets is load-bearing for sound, and it reopens `docs/core-boundary.md` §5, whose §5 "what reopens this" clause says
the report belongs there. Record the finding in the comparison report either way.

Rendering must be independent of host block partition wherever the specification promises it. Caches key on exact
source/studio semantics, instrument signature/body, locked asset/package digests, realization seed, sample rate, and
render options. A cache keyed on the gesture timeline's semantic hash is correct exactly because R1 holds, so the R1
measurement is a precondition for the cache rather than a nicety alongside it. Eviction changes cost only. Streaming is
admitted only if measured preloading misses a stated workload; its control-side producer, bounded queue, underrun
semantics, and offline determinism must then be specified and tested.

## Target

- Checked-in generated/small workloads, benchmark harnesses, raw results, profiles, and comparison report.
- Measured budgets and scale variables added to language/interface performance documentation.
- Focused fixes tied to observed profiles, with cached/uncached and block-partition differential laws.
- RT instrumentation proving callback and destruction constraints across native/sample/media plans.
- The R1 differential result, in the comparison report, with the pairs used and the outcome stated plainly.
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
