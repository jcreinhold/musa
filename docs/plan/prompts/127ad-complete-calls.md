---
id: 127ad
slug: complete-calls
status: pending
depends_on: [127ac]
phase: 3
---

# Make Every Call Complete

> **Governed by the event-track and machine core installed by prompts 127a–127i.** Fourth of the five prompts that
> replace the source checker and evaluator; the chain is 127aa, 127ab, 127ac, 127ad, 127b.

## Task

Delete every mechanism that lets a call's meaning depend on a missing argument — partial built-in values, named hole
filling, default parameters, and the closure states that represent missing arguments — and migrate the whole corpus to
explicit functions and complete calls.

## Read

- `docs/rules/language/02-core-calculus.md` §1, "A call must be complete", and its reason: partial application makes an
  argument list a place where a function silently becomes a function-valued result, which is the value that may not be
  stored.
- `docs/rules/constitution.md` §9, "Complete", which names all three — partial application, default parameters, and
  named hole filling — as ambiguity about what a call means rather than features. That is the authority for deleting a
  default rather than keeping it: an inserted default is a complete call, so §1 alone would not reach it.
- `docs/rules/language/02-core-calculus.md` §5, which had described defaults as a surface elaboration and closure
  environments as carrying preceding defaults, and `docs/rules/language/06-performance.md`'s `core-pressure` row, which
  had named partial application as the pressure it applies. Both were candidate-spec restatements the constitution had
  already overruled, and both were repaired in the commit that repaired this prompt, before any code moved.
- `docs/rules/language/00-semantics.md` §3, "Higher-order construction and the pitch traversal" — `transpose(i)` is
  written as a function that takes its track argument, not as a closure produced by an under-applied call.
- `crates/musa-compiler/src/core.rs` — `BuiltinValue`, its `bound: Vec<Option<Value>>` field, `Builtin::parameters`, and
  every construction site of a partially bound builtin.
- `docs/plan/clean-break-ledger.md` §1, which lists these forms as deleted rather than aliased.
- The stdlib, `examples/`, the compiler test suite, and the generated documentation that spell an under-applied call.

## Design

An application supplies every declared parameter. An under-applied call is a located type error naming the parameters
that were not supplied. There is no compatibility mode, no warning-and-accept, and no edition flag.

A multi-argument function receives one product argument, evaluated left to right. Delete `BuiltinValue::bound` and the
partial-application path it exists for; a builtin is applied once, completely.

Where a genuinely function-valued result is wanted, it is written as one: a function of the leading argument whose
result type is an arrow. `transpose(P5)` remains legal exactly when `transpose` is declared as a one-argument function
returning a function — which is a complete call — and is a type error when it is declared as a two-argument function.
Decide each such stdlib entry explicitly and record the decision; do not leave the reader to infer which reading applies
from whether the call happens to check.

For a *source* declaration that reading is not available, and the reason is §5: there is no anonymous-function surface,
so every source function value is a named `fn`, and a named `fn` cannot close over the argument a caller just supplied.
A source function may therefore be passed by name and may not be manufactured. The decision for the standard library is
accordingly uniform — **no stdlib entry returns a function** — and the one place that had needed to manufacture one,
`std::post_tonal::pcset`'s `set_transposed` and `set_inverted`, is served by two primitives, `pcset12_transposed` and
`pcset12_inverted`, so that T_n and I_n on a set are the set operations they already were rather than a map over a
closure. `transposed_by` and `inverted_about` stay two-argument functions on a single member; their index-first order
stays, and the comment claiming `transposed_by(3)` is itself a function goes.

Rewrite the corpus. `examples/` are regression fixtures and must keep compiling and rendering; the stdlib, the compiler
test suite, and every generated documentation page move in this commit. A fixture whose meaning changes is a finding to
report, not a silent edit.

## Target

- Deletion of partial built-in values, named hole filling, default parameters, and the closure states representing
  missing arguments, with no compatibility path.
- A located under-application diagnostic naming the missing parameters.
- The recorded decision that no stdlib entry returns a function, and the `pcset12_transposed`/`pcset12_inverted`
  primitives that let `std::post_tonal::pcset` keep its spelling without one.
- Migrated stdlib, `examples/`, compiler tests, generated fixtures, and generated docs.
- Compile-fail tests for an incomplete call and for an under-applied builtin.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
find examples stdlib -name '*.musa' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Commit as `Make every source call complete`.

## Stop

- No compatibility mode, deprecation shim, or edition flag for partial calls or default parameters.
- No new type, no `EventTrack` rename, no machine type, no scheduler, no DSP change.
- No change to the resource semantics; that is 127b.
- No anonymous-function or local-`fn` surface. The absence is what makes the stdlib decision above forced, and adding
  one would be a language change, not a way to keep a partial call working under another spelling.
