# 53. One theory: the three partial mechanisms, measured

A decision record. Governs nothing. It carries the evidence behind prompt 143's amendment, which commits musa's core to
a single dependently typed theory and reverses the stratified index note 51 proposed. Notes 42, 50 and 51 stand unedited
beside it: the admission, the deletion, and the audit that found the deletion had gone one step too far. This note is
what happens when that audit is run once more, on what replaced it.

## 1. The finding

The core is **three partial mechanisms where one would do**. Each was admitted in order to avoid a dependent core, and
each now approximates what a dependent core does properly — at the cost of a separate implementation, a separate
specification section, and, in one case, a stated rule the code cannot obey.

They are not three independent problems. Given inductive families, all three disappear: the index stratum is families,
the non-dependent eliminator is a family-valued motive, and first-order instantiation is pattern unification. That is
the whole argument for the amendment. The rest of this note is the measurement.

## 2. The index stratum is a second sort of type

`02-core-calculus.md` §1.5 admits a type-level index language: a separate grammar over ℕ, exact ℚ, and finite enums,
erased before evaluation, and decided by a solver the conversion checker *consults* rather than implements.

**Its admission was a count, and a family discharges that count without it.** Prompt 142c's evidence is seventeen of the
compiler's `Builtin` variants hardcoded to modulus 12 — `Pc12Of` through `Row12Missing`, still seventeen of 122 today.
None of the seventeen needs a separate index sort. `Base` already carries a `kind: Term`, so a base type may be applied
to arguments, and `Syntax<Cat>` is already a type applied to one. What the stratum uniquely buys is index *arithmetic* —
`Bar(p + q)`, a bar whose contents are checked to sum to its meter — and **no committed `.musa` file uses it**. 142c's
own Design conceded the arithmetic is checkable only where the durations are static, which is the case a family covers.

**It made the conversion rule false.** §3 states the rule the whole calculus rests on: `A ≡ B` exactly when
`quote(A) = quote(B)`. §1.5 erases indices at `quote`. Taken together they say `Pc(12) ≡ Pc(24)`, which is absurd, so
the implementation does something else — `convert.rs` compares `Form::Indexed` structurally on *values*, before quoting.
The rule as written is not the rule the code runs, and the gap is exactly the shape the amendment now forbids: an
acceptance decision read-back cannot see. This is the same defect class as subtyping, which is why prompt 143 refuses
that at the same time and on the same argument.

## 3. The eliminator is non-dependent

`family/assemble.rs`'s `motive_type` answers a **type**, not a family. A method's result and its induction hypothesis
are both the plain `R_j`, and nothing is applied to the value being eliminated. So a `match` refines nothing: after
matching a `Row(n)` against its cons case, the checker knows no more about `n` than it did before.

That leaves the core with **dependent Π formation over simply-typed elimination** — not a weaker theory, but two halves
of two different ones. Every construct that wants elimination to refine a type has to be built beside it instead, which
is the second mechanism paying for the first.

## 4. Instantiation is first-order

§2.1 takes Idris2's `checkRtoL` without the fallback that makes it a unifier: a type parameter a call omits is solved
once, at that call, by first-order matching against the written arguments, with no constraint queue and nothing that
waits. `elab/spine.rs` is that matcher.

The consequence is not that solving is weak; it is that whole constructs are unavailable, and unavailable by *omission*
rather than by decision. An implicit argument that cannot be read off one call, index unification in a `match`, a
metavariable that outlives the call that created it — each is impossible because the mechanism underneath it was never
built. Pattern unification with a constraint queue and postponement is the mechanism, and it is the step to port from
`Unify.idr` rather than reinvent.

## 5. What the amendment adds, and what it costs

**Subtyping is refused in every form**, including cumulativity and the one coercive rule musa has today: `Accepts` in
`crates/musa-calculus/src/base.rs:143`, fired from four sites in `elab/check.rs`, deliberately kept out of conversion —
which is precisely the confession that it is an acceptance rule conversion cannot see. Its one real use is `Syntax<Cat>`
forgetting, which becomes an ordinary total function once `Syntax : Cat -> Type` is a family. The measured cost of going
explicit is four internal call sites and two stdlib projections.

**The universe ceiling is lifted**, non-cumulatively. Two fixed levels stop being enough the moment a record holds a
type, which is what the trait deletion and the declaration-form collapse are about to make ordinary. Non-cumulative is
the conventional choice and the safer half: Agda and Lean 4 are both non-cumulative, and a non-cumulative theory accepts
strictly fewer terms than its cumulative counterpart, so it cannot be unsound where that one is sound. Coq is the
outlier, and its cumulativity is where its hardest metatheory lives.

**Traits are removed rather than narrowed.** Six traits, 82 call sites, and **zero** trait-constrained signatures — not
one function in the corpus is polymorphic over a trait, verified by the absence of any `where` clause in `stdlib/` or
`examples/`. `Eq`'s five instance bodies are literally the five δ-builtins, `text_equal` through `interval_equal`, each
written as the builtin itself rather than a λ around it, as `prelude.rs` says in its own doc comment. A record holding
functions says all of this in the core the language already has.

## 6. What this note is answerable to

It inherits the falsifier its two predecessors carry and adds nothing softer. `stdlib/src/adapters/staff.musa` was 2,404
lines when prompt 128's amendment was granted on it and 2,515 lines when note 51 was written; it is rewritten on the
surviving language, and if the number does not move, the diagnosis was wrong whatever the compiler's own line counts
say. `stdlib/src/post_tonal/` is rewritten for arbitrary n and must lose the seventeen builtins without getting longer.

One measurement is new, and it is the one this note would most like to be wrong about. Dependent elimination means
**large elimination during conversion**: a user's own definition can run while a type is being checked. The resource
meter is already the backstop, and the current corpus already presses on it — the staff adapter's expansion crosses the
nesting limit at 257 of 256 levels before it crosses the step limit at all. Those numbers will move, and the prompt that
re-measures them says so rather than assuming the theory is free.
