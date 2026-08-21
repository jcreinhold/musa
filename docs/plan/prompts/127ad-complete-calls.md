---
id: 127ad
slug: complete-calls
status: done
depends_on: [127ac]
phase: 3
---

# Make Every Call Complete

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** Fourth of the five
> prompts that replace the source checker and evaluator; the chain is 127aa, 127ab, 127ac, 127ad, 127b.

## Task

Delete every mechanism that lets a call's meaning depend on a missing argument — partial built-in values, named hole
filling, default parameters, and the closure states that represent missing arguments — give the surface the anonymous
function that takes over the one job partial application was doing, and migrate the whole corpus to explicit functions
and complete calls.

## Read

- `docs/rules/language/02-core-calculus.md` §1, "A call must be complete", and its reason: partial application makes an
  argument list a place where a function silently becomes a function-valued result, which is the value that may not be
  stored.
- `docs/rules/constitution.md` §9, "Complete", which names all three — partial application, default parameters, and
  named hole filling — as ambiguity about what a call means rather than features. That is the authority for deleting a
  default rather than keeping it: an inserted default is a complete call, so §1 alone would not reach it.
- `docs/rules/language/02-core-calculus.md` §5, which had described defaults as a surface elaboration and closure
  environments as carrying preceding defaults, and `docs/rules/language/06-elaboration-baseline.md`'s `core-pressure`
  row, which had named partial application as the pressure it applies. Both were candidate-spec restatements the
  constitution had already overruled, and both were repaired in the commit that repaired this prompt, before any code
  moved.
- `docs/rules/language/00-semantics.md` §3, "Higher-order construction and the pitch traversal" — `transpose(i)` is
  written as a function that takes its track argument, not as a closure produced by an under-applied call.
- `docs/rules/language/02-core-calculus.md` §5's term grammar, where `λ(x₁:τ₁,…,xₙ:τₙ).e` is already both a term and a
  value. The surface withheld a way to write one; the core never lacked the form. §5 was amended in the commit that
  repaired this prompt, before any code moved.
- Peyton Jones, *The Implementation of Functional Programming Languages*, chapters 2–3 (translating a high-level
  language into the lambda calculus) and 6 (the enriched lambda calculus): the surface form is erased by `⟦·⟧` onto the
  abstraction underneath, which is why it adds no case to any proof.
- `docs/plan/prompts/112-braced-function-bodies.md` and `docs/plan/prompts/94-expression-syntax.md`, whose **Stop**
  lists both refused an anonymous function. They refused it while partial application still supplied the need; this
  prompt removes that supply, which is what changes the answer.
- `stdlib/src/tonal/sequences.musa`, `stdlib/src/tonal/schemas.musa`, and `stdlib/src/post_tonal/serial.musa` — the nine
  sites that specialize a higher-order call with a value known only at run time, which is the whole of what partial
  application was carrying.
- `crates/musa-compiler/src/phase/mod.rs` — `BuiltinValue`, its `bound: Vec<Option<Value>>` field,
  `Builtin::parameters`, and every construction site of a partially bound builtin.
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

For a *source* declaration that reading needs somewhere to write the function down, and this is where deleting partial
application stops being a subtraction. A named `fn` is declared where the declarations are, so it cannot close over an
argument its caller just supplied; partial application was the only thing carrying that job, and the standard library
leans on it nine times — `map(schema_triad(collection), roots)`, `nat_fold(start, risen_by(steps), index)`, and seven
more of the same shape. So the surface gains the **anonymous function**:

```musa
fn schema_triads(collection: Scale, roots: List<Degree>) -> List<Option<ChordClass>> {
    map(fn (root: Degree) -> Option<ChordClass> { schema_triad(collection, root) }, roots)
}
```

It is spelled with the words a declaration already uses, minus the name: `fn`, a parameter list, an optional `-> τ`, and
a braced body holding one expression (prompt 112). Parameter and result types may be omitted exactly where a declaration
may omit them (prompt 127aa) and are inferred. It captures lexically, by value, into the closure the evaluator already
builds for a named `fn`. It is a value of arrow type, so it may be applied and passed and — by §1.1, unchanged — may not
be stored in a `data` field. It has no name, so it cannot apply itself, and the termination argument is untouched.

This is not a second way to leave an argument out. Every call inside a lambda and every call *of* a lambda supplies
every parameter; what the lambda supplies is a place to *write* the specialization that partial application used to
leave implicit in an argument list. That is the whole difference constitution §9 is drawing.

Two stdlib entries are decided the other way, because a lambda would be the wrong tool for them:
`std::post_tonal::pcset`'s `set_transposed` and `set_inverted` become the primitives `pcset12_transposed` and
`pcset12_inverted`, so that T_n and I_n on a set are the set operations they already were rather than a map over a
closure. `transposed_by` and `inverted_about` stay two-argument functions on a single member; their index-first order
stays, and the comment claiming `transposed_by(3)` is itself a function goes. **No stdlib entry returns a function.**

Rewrite the corpus. `examples/` are regression fixtures and must keep compiling and rendering; the stdlib, the compiler
test suite, and every generated documentation page move in this commit. A fixture whose meaning changes is a finding to
report, not a silent edit.

## Target

- Deletion of partial built-in values, named hole filling, default parameters, and the closure states representing
  missing arguments, with no compatibility path.
- A located under-application diagnostic naming the missing parameters.
- The anonymous function `fn (…) -> τ { e }` in the lexer, parser, lossless CST, formatter, and tree-sitter grammar,
  with the drift-law token fixtures and the tree-sitter corpus regenerated, and checking and evaluation reading it as
  the lexically capturing closure a named `fn` already becomes.
- The recorded decision that no stdlib entry returns a function, and the `pcset12_transposed`/`pcset12_inverted`
  primitives that let `std::post_tonal::pcset` keep its spelling without one.
- Migrated stdlib, `examples/`, compiler tests, generated fixtures, and generated docs, plus one `examples/` fixture
  that shows a lambda specializing a higher-order call.
- Compile-fail tests for an incomplete call, for an under-applied builtin, for an under-applied lambda, and for a lambda
  offered as a `data` field.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
find examples stdlib -name '*.musa' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Commit as `Make every source call complete`.

## Stop

- No compatibility mode, deprecation shim, or edition flag for partial calls or default parameters.
- No new type, no `EventTrack` rename, no machine type, no scheduler, no DSP change.
- No change to the resource semantics; that is 127b.
- No local `fn` *declaration* inside a block. A block holds one expression (prompt 112); the anonymous function is an
  expression, not a declaration, and nothing about it reopens that.
- No recursive or named lambda, and no `let rec` in any spelling. An anonymous function cannot reach itself, which is
  the only reason it costs the termination argument nothing.
- No storable function. §1.1 is untouched: a lambda may be applied and passed, and a `data` field that holds one is
  rejected exactly as prompt 127ac left it.
