---
id: 118
slug: elaboration-performance-closure
status: pending
depends_on: [93, 98, 104, 108, 111, 112, 114, 115, 116]
phase: 3
---

# Elaboration Performance Closure

## Task

Measure the complete elaboration language against prompt 93's frozen workloads and new worst-plausible musical
workloads, explain every material regression, and optimize only demonstrated bottlenecks. Close the language block with
budgets that cover compiler latency, memory and allocation growth, editor queries, analysis, template expansion, and
kernel quotation without weakening determinism, totality, provenance, diagnostics, or module boundaries.

## Read

- Prompt 38 and prompt 93's benchmark protocol, artifacts, compatibility corpus, and frozen baseline.
- `docs/interface/06-performance.md` and all existing B-budget definitions.
- `docs/language/02-core.md`, `03-music.md`, `04-templates-and-modules.md`, `06-kernel-escape.md`, and `07-analysis.md`.
- Cache, semantic-hash, last-valid-artifact, realization, and provenance invariants from prompts 43, 50, 67, and 77.

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
- Updated `docs/interface/06-performance.md` and language resource-budget documentation with measured thresholds and
  scaling variables.
- A public-surface and dependency audit confirming performance work did not leak compiler internals or add a new crate.

## Check

```sh
cargo bench -p musa-compiler
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
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
