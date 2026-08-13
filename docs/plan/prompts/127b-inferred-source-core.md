---
id: 127b
slug: inferred-source-core
status: pending
depends_on: [127a]
phase: 3
---

# Replace the Source Evaluator with the Small Inferred Core

## Task

Implement the reviewed pure total source calculus as the one checker and evaluator in `musa-compiler`. Remove partial
calls, default arguments, and compatibility paths that make a call's meaning depend on missing parameters. Infer types
wherever the program determines them.

## Read

- The governing core-calculus specification produced by prompt 127a and research `05-selected-calculus.md` §2.
- Current `crates/musa-compiler/src/{core,typecheck,eval}.rs`, prompts 94–96 and 108, and all higher-order call tests.
- Peyton Jones, *The Implementation of Functional Programming Languages*, chapters 3 and 6, as cited by the research.

## Design

The language has `Unit`, `Bool`, `Nat`, `Ratio`, `Text`, products, sums, `Option`, `List`, structural `Result`,
functions, and finite strictly positive nominal data. It has non-recursive `let`, exhaustive matching, generated
structural folds, and rank-1 Hindley–Milner inference. Top-level and local annotations remain optional except where
separate checking or an abstract public signature needs one.

Keep file modules, qualified imports, records, signatures, abstract data members, and private constructors in the rich
source form. Resolve and seal them before lowering. Keep `let`, constructors, exhaustive `match`, and folds in the
private evaluation core because they preserve sharing and have direct total rules. Do not invent join points or a second
decision-tree evaluator merely to remove a form the small core already handles.

Every type is a value type. A transitively function-free type with a versioned injective finite encoding is also
storable data. Type variables retain whether they range over any value or only data. Reject a source function hidden in
any list, constructor, abstract value, track payload, primitive argument, or stored configuration.

Calls are complete. A multi-argument function receives one product argument and evaluates it left to right. Delete
partial built-in values, named hole filling, default parameters, and old closure states that represent missing
arguments. Rewrite the repository corpus to explicit functions and complete calls; do not accept the old forms with a
warning.

Use typed evaluator configurations `run`, `done`, and `failed(ResourceError)`. Charge a fixed versioned integer cost for
the unique next reduction; never read wall time or allocator behavior. Foreign source primitives are first-order,
data-only, total trusted operations with ordinary `Result` failures.

Keep core terms, environments, inferred schemes, nominal ids, and evaluator values private. Expose only compiler facts
required by actual callers.

## Target

- One kinded-HM checker and strict total evaluator with the stated data boundary and resource semantics.
- Clean deletion of partial/default-call machinery and migration of stdlib, examples, tests, and generated docs.
- Compile-fail tests for hidden functions, incomplete calls, recursive terms, non-exhaustive matches, and bad data
  instantiations; principal-type and termination law tests for the accepted fragment.
- Updated language facts, hover text, and diagnostics with inferred types shown in plain form.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
find examples stdlib -name '*.musa' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Commit as `Replace the source evaluator with the inferred core`.

## Stop

- No `EventTrack` rename, machine type, scheduler, DSP change, type-directed macro, general recursion, overloading,
  subtyping, higher-rank type, or dependent type.
- No compatibility mode for partial calls or default parameters.
- No public Rust mirror of the private core AST or evaluator value.
