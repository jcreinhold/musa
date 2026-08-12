---
id: 95
slug: total-functional-core
status: done
depends_on: [94]
phase: 3
---

# A Private Total Functional Core

## Task

Implement the private, call-by-value typed evaluator for scalar values, products, named functions, application, and
lexical `let`. Replace the stage diagnostic from prompt 94 with type checking and evaluation, prove the core's safety
and normalization claims at the level stated by the specification, and keep every evaluator/HIR type behind
`musa_compiler::compile`.

## Read

- `docs/rules/language/{00-semantics,02-core-calculus,05-verification}.md`.
- `crates/musa-compiler/src/{compile,resolve,elaborate}.rs` and its external callers in project, render, audio, CLI,
  LSP, and desktop.
- The module audit recorded by prompt 92: the compiler already has a broad public snapshot surface; this feature does
  not justify exporting `Type`, `Value`, `Closure`, environments, or pass errors.

## Design

Use one private subsystem whose interface to the existing compiler is conceptually
`check_and_evaluate(parsed declarations, compile context) -> CheckedDefinitions`. Its modules may separate syntax
lowering, types, checking, and evaluation when those decisions change independently, but callers receive one opaque
result and never orchestrate passes.

The core is monomorphic STLC after elaboration. Function types are genuine; closures capture lexical values; named
functions form an acyclic dependency graph; recursive self/mutual references are rejected with a cycle diagnostic.
Arithmetic is exact for integers/rationals and typed musical quantities already present. Partial domain operations use
refined input, `option` later, or a source diagnostic before a value exists—never panic/exception semantics visible to
Musa.

Complete `docs/rules/language/02-core-calculus.md` with small-step and big-step rules, canonical forms,
substitution/weakening lemmas, preservation, progress, determinism, and a reducibility-candidates proof of strong
normalization for this fragment. The paper proof and executable reference evaluator are separate oracles: tests compare
production evaluation against a deliberately small definition on generated well-typed terms, plus compile-fail programs
for each type error.

## Target

- Private `musa-compiler` elaboration subsystem; no new crate and no new compiler re-export.
- Type-checked/evaluated scalar `let` and `fn`; definition/reference index and Origin spans preserved.
- Stable diagnostics for mismatch, unknown type/name, wrong arity, duplicate binding, escaping stage, and dependency
  cycle, with secondary labels and certain fixes where possible.
- `docs/rules/language/02-core-calculus.md`: completed judgments and proofs for this fragment.
- `crates/musa-compiler/tests/{core_laws,core_validation}.rs`: reference agreement, substitution/preservation samples,
  closure capture, higher-order scalar functions, and negative programs.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
for f in examples/*.musa; do cargo run -q -p musa -- check "$f"; done
cargo bench -p musa-compiler
```

Append prompt 93's performance row; justify any large-workload P1/P2 regression above 10%. Commit as
`Add the total functional elaboration core`.

## Stop

- No `music` values or music-producing functions — prompt 97.
- No natural/list recursion, fold, map, or option elimination — prompt 96.
- No general recursion, `fix`, mutation, I/O, exception effect, concurrency, syntax reflection, or user macros.
- No public evaluator hook for tests or benchmarks; keep reference/testing seams crate-private.
