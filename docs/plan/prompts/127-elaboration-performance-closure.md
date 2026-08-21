---
id: 127
slug: elaboration-performance-closure
status: done
depends_on: [93, 98, 104, 115, 118, 119, 121, 122, 124]
phase: 3
---

# Score-Elaboration Performance Closure

## Task

Measure the score/elaboration implementation through prompt 125 against prompt 93's frozen workloads and new
worst-plausible musical workloads, explain every material regression, and optimize only demonstrated bottlenecks. Close
the language block with budgets that cover compiler latency, memory and allocation growth, editor queries, analysis,
template expansion, and kernel quotation without weakening determinism, totality, provenance, diagnostics, or module
boundaries. Preserve the audio-bridge baseline unchanged except for explicitly completed expected-change entries; prompt
170 performs the audio preparation/render/asset closure after those features exist.

## Read

- Prompt 38 and prompt 93's benchmark protocol, artifacts, compatibility corpus, and frozen baseline.
- `docs/rules/desktop/06-performance.md` and all existing B-budget definitions.
- `docs/rules/language/02-core.md`, `03-music.md`, `04-templates-and-modules.md`, `06-kernel-escape.md`, and
  `07-analysis.md`.
- Cache, semantic-hash, last-valid-artifact, realization, and provenance invariants from prompts 43, 50, 67, and 77.
- `crates/musa-compiler/src/elaborate.rs` — `Share`, `music_key`, and `scale_in_force`; and
  `crates/musa-compiler/tests/suite/scale_context_laws.rs`, which fixes what a call site's pitch context means.
- `docs/rules/kernel/10-term-calculus.md` §"Provenance of the sharing discipline", and Peyton Jones (1987) Chapters
  14.7.2, 15, and 23. Chapter 15 defines the technique this prompt must measure; Chapter 23 is why it must be measured
  rather than assumed.

## Design

First extend the benchmark corpus with checked-in deterministic workloads that isolate and combine:

- deep but valid total-function calls, folds over large finite lists, partial application, and resource-budget failure;
- repeated and nested `Music` templates, modules/functors, context changes, source imports, and dense provenance;
- scales/chords, schema harmonization, post-tonal transformations, tonal analysis, and counterpoint/voice-leading
  checks;
- large `.musa.kernel` inclusion, many typed quote holes, formatter/parser recovery, hover/completion, and one source
  edit near the beginning and end of a project.

Collect wall time, CPU time where stable, peak resident memory, allocation count/bytes where the harness supports it,
cache hit/miss counts, expanded occurrence count, diagnostics/findings count, and artifact size. Compare median and p95
under the same pinned environment as prompt 93; retain raw machine-readable results and profiler artifacts.

Any cache introduced or changed here is correct only if its key includes semantic definition revision, typed arguments,
the relevant elaboration/context environment, source/instance identity required by provenance, realization parameters,
and compiler/stdlib format version. Prove cached and uncached results equal in kernel normal form, ordered diagnostics,
provenance, analyses, and resource failures. Eviction may change time and memory, never semantics or diagnostic order.

**Measure the sharing gaps explicitly, and decide them with the measurement.** The `Share` type in
`crates/musa-compiler/src/elaborate.rs` keys an elaborated body on `music_key`, and that key includes the **call span**.
Two calls therefore share a body only when they are written in the same place, which is why `examples/changes.musa`
binds one twenty-two-occurrence body twice under two names. There are two gaps, and they are not the same size:

- **The call-site gap.** Two identical calls at different sites duplicate the body outright. This is *below*
  common-subexpression elimination rather than above it: the key separates calls that denote the same thing.
- **The full-laziness gap.** A body's argument-independent subexpressions are re-elaborated once per distinct argument.
  `examples/tuplet-fixture.musa`'s `motif turn(root)` is the shape, with its second note independent of `root`.

Add a workload that scales each deliberately — many identical calls at distinct sites, and a large motif body whose
majority is argument-independent called with many distinct arguments — and report elaboration time, allocations, and
occurrence count against a hand-written equivalent. That difference is the whole prize; a gap that is immaterial at
realistic sizes is closed by recording its number.

**The call span is load-bearing until something replaces it.** `music_key` records `cx.scale` but not `cx.pitch_scale`,
and a body reads the scale in force *at the call* (`scale_in_force`): the innermost `in scale`, or else the key latest
at the cursor. Distinct call spans are what keep two readings of one saved phrase apart today, and
`crates/musa-compiler/tests/suite/scale_context_laws.rs::one_bound_phrase_elaborates_differently_under_two_scales` fails
the moment the span is dropped on its own. Closing the call-site gap therefore means keying on the *effective* pitch
context, and showing that what a shared body would otherwise stop doing once per call — the diagnostics reported from
inside it, the output meter's charge, and the realization decision sequence — is either unchanged or re-charged at the
reference.

If a gap is material, close it — the call-site gap by a sound key, the full-laziness gap by hoisting
argument-independent subexpressions to piece-level bindings — under three conditions and no others:

- **Semantics unchanged.** Evaluated kernel normal form, ordered diagnostics, provenance, and semantic hash identical
  before and after, proved by differential test on the compatibility corpus. The shared binding is a `let`, so this is
  T2, and the *printed* term is allowed to shrink: a `let` bound once and used twice is the same term as the same body
  written twice, which is what T2 says. The kernel-corpus goldens record the printed term and are refreshed with the
  measurement that authorizes them.
- **Visible, not magic.** The hoist appears as an Origin step. Peyton Jones (1987) §23.2.1's point transfers even though
  its laziness caveats do not: whether the sharing is found depends on how the source was written, and a performance
  property that turns on syntactic accident must be inspectable rather than silent.
- **Not a cache.** It is a transformation of the term, not a memo table, so the cache-key obligations above do not apply
  and no new invalidation surface is created.

Chapter 23's warnings about space leaks and the delicacy of full laziness are about *lazy* evaluation and do not
transfer to a strict, total, finite calculus. Say so in the report rather than inheriting the caution unexamined.

Optimize only profiles that identify a material regression or an existing budget miss. Prefer compact internal
representations, sharing, interning with explicit ownership, and avoiding repeated resolution/evaluation. Do not expose
HIR/evaluator types, fuse semantic layers, replace exact rationals with floats, erase pitch spelling, skip validation,
or add concurrency whose scheduling changes results. Add incremental elaboration only if an interactive workload misses
its budget and measured invalidation boundaries are both sound and narrower than a full compile.

Set or repair budgets from evidence, including explicit scale variables (source bytes, definitions, call sites,
occurrences, analysis requests). A resource-limit diagnostic must remain deterministic and identify the responsible
source construct; “faster” is not permission to make accepted programs machine-dependent.

## Target

- Extended Criterion/UI benchmark suites and checked-in workloads, results, machine metadata, and comparison report.
- Profiles for each failed or materially regressed workload, with the chosen fix linked to the observed hotspot.
- Focused internal optimizations and cached/uncached differential law tests where measurement justifies them.
- Both sharing gaps decided by their measured numbers, each either closed or recorded as immaterial.
- Updated `docs/rules/desktop/06-performance.md` and language resource-budget documentation with measured thresholds and
  scaling variables.
- A public-surface and dependency audit confirming performance work did not leak compiler internals or add a new crate.

## Check

```sh
cargo bench -p musa-compiler
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
cd apps/musa-desktop/ui && npx playwright test --project=budgets
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-compiler
```

Attach baseline/current comparison tables and profiler evidence to the prompt's completion notes. Record every retained
regression with a musical workload, an owner, and a reason; an unexplained regression is a failed check. Commit as
`Close elaboration performance against measured workloads`.

## Stop

- No intuition-driven optimization, benchmark-only special case, relaxed semantic check, or lowered diagnostic quality.
- No float musical time, erased pitch spelling, nondeterministic parallel result order, or unbounded cache.
- No incremental compiler unless a measured interactive miss and sound invalidation design require it.
- No declaration that the block is fast because a microbenchmark passed; combined real-piece workloads are mandatory.
- No audio optimization or revised audio budget here; prompt 170 owns measurement-driven audio closure.
