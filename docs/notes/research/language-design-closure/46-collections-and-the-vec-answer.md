# Collections, and the answer on `Vec A n`

## Purpose

Prompt 141 asked one question it was not allowed to answer by taste: does the length-indexed `Vec A n` ship? Its Design
set the test — "if no program needed a length in a type, `Vec` is a mechanism nobody asked for" — and forbade the usual
reason for shipping it: "Do not ship it on the strength of it being the standard example of a dependent type."

This note records the answer, the evidence it rests on, the condition that re-opens it, and three defects that writing
the collection library found in the core it was written against. Nothing here governs.

## 1. The answer: `Vec A n` does not ship

The trial in [43-dependent-language-trial.md](43-dependent-language-trial.md) checked ten programs against every
mechanism prompts 129–131 specified. Its §7 table has one row per mechanism and one column for the programs that use it,
and `Vec A n`'s row reads **nothing** — one of six rows that do.

§10 says what happened to the two candidates prompt 141's own Read section nominated:

> `Vec A n` appears in no program. The two fixed-arity things the staff adapter has — up to two numbers, up to two words
> — are `Numbers` and `Words`, enums whose cases are named, and both read better than `Vec Syntax<Expr> 2` would.

And §5.6 records that the five corpus programs are entirely non-dependent, which note 28 §7 had already found.

So the ledger is empty on both sides of prompt 141's test. `List` ships with three programs asking for it
(`voiced_inside`, `voice`'s `OpenBass` arm, `transcribe`); `Vec` ships with none.

## 2. What is *not* deleted, and why the distinction matters

The **index mechanism** stays, and this note is not evidence against it. Indexed families have a user — `Syntax<Cat>` —
and prompt 135 built them for it. `Vec` also stays as a *fixture*: `family_laws.rs` and `coverage_laws.rs` declare it to
state the laws of indexed families, index refinement, and coverage, and `termination_laws.rs` recurses over it in the
test that decided §2.4's measure. A fixture is a program that exercises a mechanism, not a type the language hands an
author.

The distinction is the whole of the answer: the mechanism earned its keep and the library type did not.

## 3. What re-opens it

A program that needs a length *in a type* — not one that happens to have a fixed arity, which an enum with named cases
says better and more legibly, but one where the arity is computed and a mismatch has to be a type error rather than a
runtime one.

Two measurements ahead could produce one: prompt 166's staff rewrite and prompt 167's studio rewrite. If either writes a
`Vec`, that is the program, and adding the type is an ordinary prompt rather than an amendment — nothing in
`docs/rules/` promises `Vec` and nothing forbids it. `02-core-calculus.md` §1.4 and note 43 §10 already record the
adjacent condition for the K axiom, which the same program would have to be measured against: `Nat` has decidable
equality, so a `Vec` indexed by it is probably not the program that needs K either.

## 4. Where the collection library lives, and why it is not in `src/`

`musa-calculus` is a leaf calculus with no base types. There is no `Bool`, `Nat`, `Option`, or `Text` in its `src/`; the
one pre-declared thing is `Storable`, and it is pre-declared because the *check* is the evidence rather than a
declaration an author could write. Shipping `List`, `Buildable`, `Iterable`, and `Index` as crate items would make the
calculus a calculus of one particular library.

So the collection library is written where every other library in this crate is written — as ordinary `RawData`,
`RawTrait`, and `RawImpl` fixtures, in `collection_laws.rs`, exactly as `Nat`, `Box`, `Vec`, `Eq`, `Ord`, and `Add`
already are. Prompt 142 writes the same declarations in `.musa` and hands them to authors; what this prompt owes is that
they *are writable*, and eight laws that say what they mean.

## 5. Three defects the library found in the core

Writing a collection library is the first program that asks the core for a traversal that carries an accumulator, and
each of these was found by a law failing rather than by review.

**An accumulating recursion was silently miscompiled.**
`λxs. λacc. match xs { Nil => acc; Cons h t => walk t (step acc h) }` is how every author writes a fold that runs
forwards. A `match` splits at the goal it is checked against, so the hypothesis it binds was the answer for the tail *at
this branch's own `acc`* — and `rec.rs` dropped the new accumulator the call passed, on a rule that said the other
arguments were "decided by conversion rather than here". Nothing decided them: the result type-checked, re-checked, and
computed the seed. The repair is in `rec.rs` — the binders after the recursive argument are moved inside the arms before
the match is elaborated, so the motive generalizes them and the hypothesis is a function of exactly them.
`termination_laws.rs`'s `a_recursion_that_accumulates_carries_the_argument_it_changed` is stated as a sum for that
reason: a wrong hypothesis here returns the seed rather than failing.

The same repair made a second rule statable. An argument *before* the recursive one is genuinely held fixed, and a call
that changes one is now refused by name rather than silently answered with the argument there is.

**The re-checker had no rule for a `let` in checking position.** It inferred through one, which forced every
definition's body to be inferable — and a case tree's leaf is a `let` per pattern binder around a body that, after the
repair above, may be a λ. The rule is the ordinary bidirectional one and it was simply missing.

**A `match` whose goal was still a metavariable was an internal error.** §1.3 has no universe polymorphism, so a split
reads its motive's universe off the goal per use site; a goal handed over by an argument position is a metavariable,
which is not written syntax and has nothing to read. The level was never unknown, only recorded elsewhere — a
metavariable carries the type it stands at, and a goal's type is `Type ℓ`. `coverage_laws.rs`'s
`a_match_whose_goal_is_a_metavariable_elaborates` holds it. Every `filter` written against a trait whose element type is
postponed needs this, which is how it was found.

## 6. What the eight laws say

`collection_laws.rs` states them, and two are worth naming here because they are the prompt's own gates.

`a_nested_forward_traversal_joins_what_a_map_could_not` is note 41 §4's `voiced_inside`, written the way that trial
recorded it could not be. A chord's pitches must come out as one list; a nested group may hold several; the phase
language had no operation that built a list, so a group written inside brackets contributed no pitch. What closes it is
a fold inside a fold with `push` at the bottom — the outer traversal carries the accumulator across children, the inner
one adds each child's own contribution — and both run from the start, so the pitches come out in written order. That is
the property the reversed reading algorithm existed to recover, and prompt 166 measures what recovering it is worth.

`collect_in_an_inferring_position_is_refused` states the price. `collect`'s target is fixed by *checking*; choosing an
instance from a result type nobody wrote down is return-type-directed overloading, which makes elaboration depend on the
order constraints are reached. The refusal is the design, and its diagnostic is an ordinary unsolved metavariable rather
than a special case invented for `collect`.
