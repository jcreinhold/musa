---
id: 141b
slug: base-types-and-builtins
status: done
depends_on: [136b, 137, 141]
phase: 3
---

# Give the Core Its Base Types and Builtins

## Task

`musa-calculus` cannot express a single real Musa program: there is no way to write `3`, `"c"`, or `1/4`, and no way to
name `Nat`, `Text`, `Duration`, `Pitch`, `EventTrack`, `Machine`, or `Syntax`. `02-core-calculus.md` §5.8 already
obliges base types with no eliminator and four families of compiler-owned builtin over them; nothing implements that
obligation. Implement it: base types, literals, and a builtin registry the **host supplies and the core enforces**, so
that prompt 142 has something to wire the compiler to.

## Read

- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.8 in full — the four
  families, D1–D4, the theorem to re-derive, the corollary that a later musical domain needs no core amendment, and the
  law suite the registry is obliged to carry. Then §1's grammar, which does not mention base types or literals at all
  and calls itself "the whole language"; and §5.9, which fixes the phase registry as a *second registry, not a fifth
  family*. §2's sentence that only compiler-owned builtins may construct an `EventTrack` or a `Machine` says what a
  track builtin is for.
- [`127ca`](127ca-builtin-ownership-registry.md), which named the registry and merged the compiler's two ownership
  tables into `BUILTIN_OWNERSHIP`, and `crates/musa-compiler/src/phase/mod.rs`'s `BuiltinOwnership`, `Family`,
  `Builtin`, and its ownership law suite. That table is what prompt 142 hands to the core; this prompt does not move it.
- `crates/musa-calculus/src/{term.rs, raw.rs, value.rs, eval.rs, unify.rs, quote.rs, case.rs, context.rs, elab.rs,
  recheck.rs, storable.rs}` — the eleven files the extension touches, and in particular prompt 136b's
  `Neutral { head, spine }`, which is where a builtin stuck on a non-literal argument belongs.
- The `module-design` skill's audit questions, on one boundary specifically: what `musa-calculus` must know about a
  pitch. The answer this prompt commits to is *nothing*, and the interface is what makes that true or false.
- Peyton Jones ch. 3 §3.1 and ch. 6 — a core is enriched with constants and their δ-rules rather than made to enumerate
  them; the built-in functions are a parameter of the reduction machine, not part of its syntax.
- Peyton Jones ch. 4 §4.1 and ch. 5 §5.1 on what a pattern *is*: a constructor pattern is the elimination form of an
  algebraic type, so a type with no constructors and no eliminator has no pattern but a variable. That is why the
  literal pattern in `crates/musa-compiler/src/phase/mod.rs`'s `Pattern::Literal` desugars here rather than crossing
  into the core, and it is the derivation behind this prompt's Design clause on decidable equality.

## Design

**Base types are §5.8's conservative extension, not a hole in §1.** §1's grammar is the pure calculus and says so; §5.8
adds base types to it and re-derives §5's matrix over the extension. The two read as a contradiction today because §1
claims to be the whole language. Repair §1 with one line admitting the extension and a pointer to §5.8 — that is a
repair of a claim the implementation proved incomplete, which is the kind `docs/rules/language/` is candidate for, and
it changes no obligation.

**The core owns the mechanism; the host owns the table.** `musa-calculus` is a leaf and must not learn what a pitch is.
§5.8's corollary is explicit that a later musical domain needs no new proof — only a base type, arrow-free signatures,
and a D1–D4 discharge — and a core that enumerated the base types would make every domain a core amendment and falsify
the corollary. So the registry is caller-supplied and immutable, carried on `Cx` beside `classes` and `declared`, which
are already exactly this shape.

**Three things, each with one job.**

- **`Base`** — a base type: a name and a kind. `Shape::Base`, and `Head::Base` rather than a `Form`, because a base type
  may be applied — `Syntax Expr` is `Syntax` at a parameter — and a canonical form would need an arm saying what
  applying it means. It has no constructor, no eliminator, and contributes no ι-rule, which is D1 stated as a
  representation rather than as a rule to obey. Two base types are convertible iff they are the same name, for the same
  reason two constants are (`family.rs`'s `PartialEq` argues it already).
- **`Literal`** — a closed value of a base type, opaque to the core. `Shape::Lit`, `Form::Lit`, and `RawShape::Lit`;
  **no `RawPattern::Lit`**, for the reason the decidable-equality clause below argues. Conversion is §5.8's own
  sentence: *two closed values of an inert base type are convertible iff they are the same constant*.
- **`Builtin`** — a compiler-owned operation: a name, a declared core type, a family, and a δ-rule. `Head::Builtin`
  applied along a spine. Its *typing* needs no new rule — a builtin is a constant of a declared type and application is
  application — so the only new arm is reduction.

**A literal's payload is opaque, and that is a decision with an alternative.** The core needs three things from a
literal and no more: which base type it inhabits, whether it is the same literal as another, and how to show it in a
diagnostic. Two designs answer that:

- *A closed payload universe* — integer, rational, text, bytes — with every host base type encoded into one. Refused.
  `Syntax` is a tree, so the encoding is `encode_exactly` on every phase operation and the decode on every read, and
  prompt 165's rewrite runs the adapter per keystroke. It also puts a list of the host's data shapes inside a leaf
  calculus, which is the enumeration the corollary forbids wearing a different hat.
- *An opaque payload behind a trait* — `same`, `shown`, `as_any`, three methods with one reason each. Taken. Sharing
  survives, nothing is encoded, and the downcast is the host's own concern at the host's own δ-rule. The cost is `dyn`
  dispatch on the conversion path, which prompt 164 measures.

**D3 is enforced by the type, not by a promise.** A δ-rule is a `fn` pointer, not a closure: it cannot capture host
state, so "the result is a function of the argument values alone" is checked by the compiler rather than reviewed. A
rule that needs ambient context cannot be registered, which is the refusal we want at the moment we want it.

**Stuck is neutral, not an error.** A builtin applied to a variable, a metavariable, or a term that has not reduced to a
literal is a neutral spine — the same treatment a recursor already gets on a neutral subject. A δ-rule answering `None`
for well-typed literal arguments is a *bug in the host*, not a program error, and the refusal says so: D2 promises a
closed value for every closed argument tuple.

**Registration checks the shape half of D1–D4; the host's law suite checks the sampling half.** At registration the core
checks what it can see: every builtin classified into exactly one family, no arrow anywhere in a δ-builtin's signature,
every base type reachable from a δ signature declared inert, and no two registrations of one name. D2, D3's
order-independence, and D4's bound are properties of the host's functions over the host's domains, and 127ca's law suite
already samples them where the table lives. Say which half is checked where, in the module doc, so the next reader does
not look for D2 in the core.

**A literal pattern is decidable equality, and decidable equality is the host's.** D1's "the only pattern that may match
it is a literal or a catch-all" reads at first like a coverage rule for prompt 135's case-tree compiler — a base-typed
column splitting into the literals the arms name plus a required default. It cannot be: that split is a case analysis on
a base type, and D1's first clause says no reduction rule inspects a closed value of one. A base type has no eliminator,
so a base-typed column has *nothing to split on*, and a `Test::Literal` would be the eliminator D1 refuses, introduced
by the compiler instead of by the registry.

What a literal pattern actually means is `if x == "PitchLiteral" then … else …`: a δ-builtin deciding equality, and the
host's own `Bool` — an ordinary declared family with an ordinary recursor — doing the branching. Lean compiles `String`
patterns exactly this way, for exactly this reason. So the desugaring belongs to whoever owns both halves, and that is
`musa-compiler`: it knows which base types it registered, which δ-builtin decides each one, and which family it calls
`Bool`. A `RawPattern::Lit` in the core would be a third party to a conversation between two things the core does not
know, and it would have to learn `Bool` to compile it.

`musa-calculus` therefore has **no literal pattern**, and `case.rs` keeps the half of D1 it can enforce: a
`RawPattern::Constructor` or `RawPattern::Record` at a base-typed column is `Refusal::BaseNotMatchable`, naming the base
type. Coverage needs no rule at all — the only pattern the core admits there is `RawPattern::Bind`, which is the
catch-all D1 requires, and a column of catch-alls is never tested. Prompt 142 owns the desugaring, and this Design is
what tells it to write one.

**Charge before constructing.** A builtin application charges the `Budget` through the existing `Metric` before its
δ-rule runs, which is D4 where D4 can be enforced rather than where it is merely true.

**The registry is proved by a program, not by the compiler.** Like prompt 141's collections, this prompt's evidence is a
worked registry in the test suite — a handful of base types and δ-builtins written out, elaborated, normalized, and
matched — so the mechanism is exercised without `musa-compiler` changing. 142 is still the one cutover.

## Target

- `crates/musa-calculus/src/base.rs`: `Base`, `Literal`, the `Payload` trait, `Builtin`, `Family`, and `Registry` with
  its registration checks, all doc-commented with their invariants before the implementation.
- `Shape::Base`/`Shape::Lit`/`Shape::Builtin`, `RawShape::Lit`, `Form::Lit`, and `Head::Base`/`Head::Builtin`, with
  their arms in `eval.rs` (δ and stuck-is-neutral), `unify.rs` (base identity and literal identity), `quote.rs`,
  `recheck.rs`, and `storable.rs`.
- `case.rs`: the refusal for a destructuring pattern at a base-typed column, which is all of D1 the core can enforce.
- `Cx` carrying an optional `Arc<Registry>`, supplied by the caller the way `Classes` is; a context with no registry
  names no base type, which is what keeps every existing test unchanged.
- New `Refusal` variants and their `musa explain` codes in `crates/musa-score/src/diagnose.rs` — the compiler's
  diagnostic registry is the one file this prompt touches there, and its checker is untouched: unknown base type,
  destructuring pattern at a base type, a δ-builtin signature containing an arrow, and a builtin classified twice. A
  δ-rule that answers nothing at arguments it declared it accepts is **not** among them: it is a host defect rather than
  a program error, so it belongs to `error.rs`'s `Malformed`, which is the crate's existing word for a caller that
  handed over something that does not fit.
- `crates/musa-calculus/tests/suite/base_laws.rs`: the worked registry, and the laws — inertness (no reduction inspects
  a literal, and no destructuring pattern stands at a base type), literal conversion, δ agreement with the host function
  over a finite sample, stuck-is-neutral, budget charging, and the registration refusals.
- `docs/rules/language/02-core-calculus.md` §1: the one-line repair admitting §5.8's extension.
- `docs/plan/code-map/` rows for `musa-calculus`, replacing "a leaf calculus with no base types".
- No change to `musa-compiler`'s checker, no `stdlib/` or `examples/` change, and the compiler's `BUILTIN_OWNERSHIP`
  table left where it is. Prompt 142 hands it over; prompt 163 collapses it.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo clippy --all-targets -p musa-calculus -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Give the core its base types and builtins`.

## Stop

- No base type named after a musical domain inside `musa-calculus`. If the core mentions a pitch, a duration, or a
  `Syntax`, the boundary this prompt exists to draw has already been crossed.
- No move of `BUILTIN_OWNERSHIP`, no change to the compiler's checker, and no wiring. Prompt 142 owns the cutover, and a
  half-wired compiler is exactly the ambiguous middle 142's Design forbids.
- No collapse of any builtin behind a trait or a method. Prompt 163.
- No amendment to §5.8, its four families, or D1–D4. This prompt implements them; a disagreement is a finding to record.
- No `partial` δ-rule and no panicking one. D2 is a condition on registration, not a runtime hope.
- No interning table and no interior mutability in the registry. A registry that mutates during evaluation is an
  evaluation-order dependence, which is the D3 this prompt is enforcing.
