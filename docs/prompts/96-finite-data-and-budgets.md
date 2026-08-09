---
id: 96
slug: finite-data-and-budgets
status: pending
depends_on: [95]
phase: 3
---

# Finite Data, Structural Folds, and Honest Resource Limits

## Task

Complete the total core with naturals, options, finite lists, products, case analysis, and structural eliminators. Make
computed finite repetition and collection algorithms expressible without general recursion, and add deterministic work,
value-size, and output-size limits whose diagnostics distinguish resource rejection from type or semantic failure.

## Read

- `docs/language/02-core-calculus.md`, especially the strictly positive data and monomorphic-after-elaboration rules.
- Prompt 95's evaluator and proof; prompt 93's measurement protocol.
- Current repeat count limits and all diagnostics that reject recursion or expansion blow-up.

## Design

The production core remains monomorphic. `list τ`, `option τ`, and their eliminators are typed families instantiated
at concrete types; prelude-facing rank-1 schemes are elaborated by monomorphizing a finite copy per use. Do not smuggle
impredicative System F or unrestricted inference into a feature described as STLC.

Natural and list folds recurse only over a structurally smaller constructor. Extend the normalization proof by the
standard logical-relations/strictly-positive-data argument and state the induction measures explicitly. Add property
tests against small reference folds, including empty, singleton, large-but-accepted, and nested data.

Mathematical termination is not permission to allocate `10^12` events. Instrument the evaluator with one private work
meter covering reduction steps, constructed nodes/bytes, monomorphized instances, and eventual music-output estimates.
Derive defaults from measured real/synthetic curves and document them in `docs/language/06-performance.md`; CLI/project
options may expose one coarse compilation budget only if a current caller needs it. Exhaustion is deterministic for a
given source/options pair, carries the operation and limit, and leaves last-valid artifacts intact.

## Target

- Private core types/values/eliminators for `nat`, `option`, `list`, and products; `map`, `fold`, and computed finite
  `repeat` in the prelude.
- Work/value/output accounting with boundary tests and structured diagnostics.
- `docs/language/02-core-calculus.md`: data rules and strong-normalization extension.
- `docs/language/06-performance.md`: size curves, chosen defaults, command, uncertainty, and failure behavior.
- `crates/musa-compiler/tests/{finite_data_laws,resource_validation}.rs` and focused evaluator benchmarks.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-project -- -D warnings
cargo fmt --check
cargo bench -p musa-compiler
for f in examples/*.musa; do cargo run -q -p musa-cli -- check "$f"; done
```

Commit as `Add finite folds and elaboration budgets`.

## Stop

- No recursive function declarations, lazy/infinite lists, streams, exceptions, or evaluator threads.
- No polymorphic user definitions; prelude schemes monomorphize privately.
- Do not pick a limit without the recorded size curve, and do not call a rejected finite program nonterminating.
- No optimization beyond work required to measure and enforce the limits.
