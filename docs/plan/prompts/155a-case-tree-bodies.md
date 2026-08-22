---
id: 155a
slug: case-tree-bodies
status: in-progress
depends_on: [155, 155aa]
phase: 3
---

# Make a Case Tree a Definition Body

## Task

Prompt 155 reifies the case tree and makes elimination dependent, but the tree it builds is still only an
*intermediate*: §6.2 compiles it straight to nested applications of the generated eliminators and then throws it away.
`docs/rules/language/02-core-calculus.md` §1 says a name's reduction behaviour may be **a compiled case tree**, and
`kernel/term.rs`'s `Definition::Defined` carries a note naming a prompt that will replace it with one. Do that: a tree
becomes a **definition's** body, δ becomes tree reduction, and a body that calls itself becomes something the checker
owes a real termination argument for. §2.4 is the other half and names it outright — descent "used to fall out of the
generated eliminator", and with `match` compiled to a tree "the checker walks the tree instead".

**Corrected against the code before implementing.** ι and the generated eliminator are *not* touched, and a tree body
carries the binders it stands under. Both corrections are argued in the Design, and both are measurements rather than
preferences.

**And it waits on 155aa.** A definition that names itself is a name in the globals table, and a term-position `rec` —
`prelude.rs`'s two folds — closes over binders rather than names, so it can never be one. Retiring the `#ih` rewrite
before those are lifted leaves them with no compilation at all. 155aa lifts them; by the time this prompt runs, every
recursion in the language is at a definition's top, which is the one place a tree body can be.

**Why this is not part of 155.** Three measurements, taken while 155 was being prepared.

- §6.2 governs 155's delivery and says the tree is compiled *to eliminator applications*. A tree that is only an
  intermediate has no reduction of its own, so `case_tree.rs`'s reduction and a termination check over it have nothing
  to act on inside 155's boundary.
- `crates/musa-calculus/src/elaboration/rec.rs` obtains structural descent for free by rewriting a recursive call into
  `<field>#ih`, an induction-hypothesis binder the eliminator's method supplies. 155's first repair established that the
  eliminator survives, so the hypotheses survive, so descent stays free for exactly as long as a tree is not a body.
  This prompt is where it stops being free.
- §2.4 states the termination rule over "the case tree", and until a tree is a body the only tree in the crate is one
  the builder discards between compiling a `match` and emitting it. There is nothing for a checker to walk that outlives
  the walk that built it.

*What is **not** a reason.* Prompt 157 was cited here at first and does not belong: it says "Prompt 155 made that
representable" and its `depends_on` names 146 and 156, not this prompt. A generated projection is a one-branch tree
*emitted*, which 155 delivers. The split stands on the two reasons above.

## Read

- `docs/rules/language/02-core-calculus.md` §1, the `Definition` list — "undeclared, **a compiled case tree**, a
  constructor, a type constructor, a registered base type, or a compiler builtin". Six arms, and the second is what this
  prompt puts there. Read §6.2 with it: the emission 155 delivers is what a body's tree replaces.
- `crates/musa-calculus/src/kernel/term.rs`, `Definition` — the five arms today, and `Defined`'s note.
- `crates/musa-calculus/src/kernel/program.rs`, `Defined` — a definition holds its body as an already-evaluated
  [`Value`] *and* as a [`Term`], for the universe-polymorphism reason its doc gives. A tree body has to answer both
  questions, and that is the design's one real constraint.
- `crates/musa-calculus/src/kernel/case_tree.rs` — what 155 built, and the `Impossible` node it left without a producer.
- `crates/musa-calculus/src/kernel/family/iota.rs` — the ι that fires at a recursor's arity today, and the **two
  measured optimizations it carries**: the tower-avoiding numeral decrement, and the unread-hypothesis rule whose module
  doc records that without it `match n { 12 -> … }` at `n = 12` spent the whole 200,000-step budget. Read it as evidence
  about what a generic tree reducer would have to re-derive before it could replace this one.
- `crates/musa-calculus/src/kernel/family/group.rs`, `Role::Recursor(Sort)` — the recursor's level "rides on the *use
  site* rather than on the declaration". That is why an eliminator is not one stored definition.
- `crates/musa-calculus/src/elaboration/rec.rs` — the `#ih` rewrite this prompt retires, and its module doc, which
  argues at length for why the rewrite is on raw syntax. That argument is what a termination check over a tree has to
  answer.
- `docs/plan/prompts/155aa-lift-local-recursion.md` — why every `rec` is at a definition's top by the time this runs,
  and the measurement that made the lift a prerequisite rather than a nicety.
- `crates/musa-calculus/src/elaboration/case.rs`, `leaf` — an arm's body is wrapped in a `let` per pattern binding. That
  is what a descent walk actually reads at a recursive call, and it is why "smaller" has to see through an alias.
- `crates/musa-compiler/src/prelude.rs`, `list_from_start` — `walk rest (step built first)`, the corpus evidence for the
  fixed descending position. Every forward fold in the language passes an accumulator it computed, and a termination
  rule that constrained the arguments other than the descending one would refuse all of them.
- `/Users/jcreinhold/Code/Idris2/src/Core/Case/CaseTree.idr` and `Core/Normalise.idr`'s case-tree evaluation — the
  reference for reducing a tree rather than an eliminator spine. Read `Core/Context.idr`'s `PMDef` with them: a
  tree-bodied definition there is `PMDef args tree`, the **arguments beside the tree**, which is the shape this prompt's
  first correction adopts and for the same reason.

## Design

**Corrected against the code, and each correction is measured.**

*A tree body carries its arguments.* A definition whose body splits is written `λx⃗. match x_i { … }`, so the tree stands
under a λ prefix and the arm cannot be a bare `CaseTree`. It cannot be a closure around one either: `Form::Lam` holds a
[`Closure`] over a **`Term`**, and a tree is not a term — §1's seven shapes have no case node, which is the whole reason
a tree lives in a `Definition` at all. So the arm carries the binders beside the tree, which is exactly Idris2's
`PMDef args tree` and is what makes reduction statable: given as many arguments as there are binders, build the
environment from them, force the scrutinee, take the alternative, evaluate that `Answer` in that environment.

*A body that names itself must be a name.* See the Task: this is what 155aa is for, and it is why the `#ih` rewrite can
be retired here rather than merely narrowed. After the lift, `rec::define`'s one caller is a definition's own body, so
removing the rewrite removes a mechanism rather than half of one.

*The generated eliminator keeps ι, and `family/iota.rs` stays.* Three measurements say so. **One:** an eliminator is not
a definition. It is a `family::Constant` at `Role::Recursor(Sort)`, reached through `Definition::Declared`, and its
level "rides on the *use site* rather than on the declaration" — so there is no one stored body for a tree to be, and
synthesizing one per family per level is a definition table indexed by a universe. **Two:** ι carries two optimizations
this repo measured and wrote down, the numeral tower decrement and the unread-hypothesis rule; `iota.rs`'s own doc
records that without the second, `match n { 12 -> … }` at `n = 12` exhausted the entire 200,000-step budget. A generic
tree reducer must re-derive both before it is even neutral, and re-deriving a measured optimization is not what this
prompt is for. **Three:** §1 says "a **name's** reduction behaviour", and the names this crate defines are `Program`
members. So what gets a tree body is a *definition*, and an inline `match` keeps 155's emission, which is what §6.2
requires of a `match` standing in a term.

That is not a narrowing of §1's list — it is what the list says. `Definition` is what the *context* answers about a
name, `Definition::Declared` is the arm an eliminator already has, and the arm this prompt adds is the one a top-level
definition needs.

**`Definition::Compiled`.** A definition with no `match` at its top is `Answer(body)` at zero binders and reduces
exactly as `Defined` did. A definition whose body splits holds the tree, and δ becomes: look up the tree, force the
scrutinee, and take the alternative its constructor names. A scrutinee that is not canonical leaves the name neutral,
which is what makes a case tree behave like the eliminator it replaces rather than like a runtime `switch`.

**A split must not unfold a tower either.** ι's numeral decrement is the rule that a `Nat` scrutinee decides which
alternative it takes from its count rather than by being unfolded into a spine, and a tree that splits on a counting
family owes the same rule. It is not inherited from `iota.rs` by sharing code — the two reducers ask the question of
different things — so it is stated again here, and a law pins it: splitting on a large numeral costs a bounded number of
steps.

**Termination becomes real work, and it belongs here.** This is the half of the prompt a governing document names
outright: §2.4's second paragraph says descent "used to fall out of the generated eliminator" and that with `match`
compiled to a tree "the checker walks the tree instead", and §1.3's refusal list says "Structural descent over the case
tree is the whole rule". A definition whose body is a tree may name itself, so the `#ih` rewrite is retired and a check
over the finished tree takes its place: an argument is **smaller** when it is a variable a split bound as a field of the
pattern it matched, transitively along the path — which is §2.4's own sentence. **Structural descent is sufficient and
is much smaller than size-change termination**; do not build the latter. What it costs is that a function whose
recursion is not structural is refused, and `rec.rs`'s own module doc enumerates the calls it refuses today, so the
negative-control corpus is inherited rather than invented.

*Inherited, and smaller than it was — which is a correction.* This prompt first said the refused set "is the same set
`rec.rs` refuses today". It is a strict subset, and the two definitions that leave it leave it because the `#ih`
rewrite's limits were the rewrite's and not termination's. **One:** `λa. λb. match a, b { Zero, y => Zero; x, Zero =>
Zero; Succ x, Succ y => both x y }`, which the rewrite refused because a call varying both columns had no single
hypothesis to name. A walk over the finished tree names nothing: it observes that position 0 is handed a field of the
split on `a` at every call, and that is descent. **Two:** `λa. λb. match b { Zero => a; Succ k => skew (Succ a) k }`,
refused because the hypothesis stood at *this* branch's `a` and the call wanted a different one. An arm of a tree is an
ordinary term and a recursive call in it is an ordinary call, so only the descending position is constrained — which is
the same fact `list_from_start` establishes below, arriving from the other side. Both become positive laws, stated as
computations; a definition admitted and stuck would be worse than one refused.

*The descending position is fixed for the definition, and that is a correction.* This prompt first said "some argument
is smaller and none is larger", and two measurements say the rule has to be the narrower one.

**One:** without a fixed position the rule accepts a definition that does not terminate:

```text
f a b = match a, b { Zero, y => Zero; x, Zero => Zero; Succ x, Succ y => f y b }
```

Every argument of that call is a name in scope, and `y` is genuinely smaller than `b` — a field of the split on `b`. But
it is handed to the *first* position, whose binder it did not come from, and the second position is handed `b`
unchanged, so `f 1 2` reduces to `f 1 2`. A checker that asks only "is one of them smaller somewhere" admits a loop.
Requiring one position `p` such that **every** recursive call passes, at `p`, a variable descending from binder `p`
refuses it, and that is Coq's guarded-fixpoint condition rather than anything larger.

**Two:** the "none is larger" clause has to go with it, because read strictly it refuses the corpus.
`musa-compiler/src/prelude.rs`'s `list_from_start` calls `walk rest (step built first)` — the second argument is neither
a binder nor a smaller binding, it is an accumulator the author computed, and every forward fold in the language is
shaped that way. Under a fixed position the clause is unnecessary: what the other arguments are cannot matter when the
measure is the size of the argument at `p` alone.

*What "smaller" has to see through.* `case.rs` binds an arm's pattern variables with a `let` around that arm's body, so
the argument of a recursive call is very often a `let`-bound name aliasing the field rather than the field's own
variable. A `let` that names a smaller variable is that smaller variable; the walk carries the binding forward rather
than losing descent at the alias.

**A tree body is binders and a tree, with nothing in between.** `case.rs` names a subject that is not already a variable
with a `let` around the emitted term, and a [`Compiled`] has nowhere to put one. So `match f(x) { … }` at a definition's
top is not a tree body: a non-recursive definition of that shape keeps the evaluated body it has today, and a
*recursive* one is [`Refusal::UncheckedRecursion`], which is the same answer `rec.rs` gives it now under "a body whose
top-level form is not a `match`".

**A definition is in scope during its own elaboration, and that is what replaces the `#ih` binder.** A recursive call
has to resolve to *something* while the body is being checked, and after this prompt that something is the definition
itself — at its declared type, with no reduction behaviour yet, so a use of it is a blocked spine and nothing can
compute with it. That third state belongs beside the two bodies rather than in a separate table. It also restructures
prompt 155aa's `lift`: a term-position `rec` must be installed *before* its body is elaborated rather than after, since
the body now names the lifted definition where it used to name a hypothesis.

**`Impossible` gets its producer if 156 landed first, and stays unreachable otherwise.** This prompt does not depend on
156 and must not assume it: a tree it builds has an `Impossible` alternative only where index unification put one.

## Target

- `crates/musa-calculus/src/kernel/term.rs`: `Definition::Compiled`, in place of `Defined` for a definition whose body
  splits, with the note that named this prompt removed because it has happened. It carries the definition and not a
  second copy of the tree: `Definition` is what the context answers *about a name*, the tree and the binders it stands
  under live in the body that `program.rs` gets below, and a copy beside the definition would be the same fact twice —
  free to disagree, and free to be the one that was never instantiated at the use site's levels.
- `crates/musa-calculus/src/kernel/case_tree.rs`: reduction — a tree, an environment of arguments, a forced scrutinee,
  and the alternative it selects; a scrutinee that is not canonical leaves the name neutral.
- `crates/musa-calculus/src/kernel/program.rs`: a definition's body as one of three things — evaluated at its
  declaration as today, a compiled tree, or in scope during its own elaboration with neither — answering the
  [`Value`]-and-[`Term`] question its `Defined` doc poses.
- `crates/musa-calculus/src/elaboration/case.rs`: the builder hands back the tree as well as the term it emits, since a
  tree body needs the first and a `match` in a term needs the second.
- `crates/musa-calculus/src/kernel/family/iota.rs`: **unchanged**, for the reason the Design's second correction gives.
- `crates/musa-calculus/src/kernel/terminate.rs`: structural descent over the tree, with the smaller-argument rule and
  the fixed descending position stated as a doc comment before either is implemented.
- `crates/musa-calculus/src/elaboration/rec.rs`: the `#ih` rewrite retired in favour of the check, with
  `Refusal::UncheckedRecursion` still naming the call it refused.
- `crates/musa-calculus/tests/suite/`: a reduction law per node, a law that a split on a counting family costs bounded
  steps, a termination law with a negative control, and `rec.rs`'s enumerated refusals ported. Two of them are admitted
  now, for the reason the Design's inherited-corpus correction gives; each of those becomes a positive law that
  *computes*, and the corpus gains the looping definition above so that the position stays fixed.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! grep -rn 'hypothesis_name' crates/musa-calculus/src
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

**The re-checker's obligation for this prompt.** A tree that is a body is a *new acceptance surface*: nothing the
re-checker walked before could reduce by matching. It must re-derive that every alternative answers the motive
instantiated at that alternative's pattern, that the alternatives are exactly the family's constructors, and that
structural descent holds over the finished tree. Negative controls for all three.

Commit as `Make a case tree a definition body`.

## Stop

- No size-change termination, no `assert_total`, no partiality.
- No change to ι or to the generated eliminator. `family/iota.rs` is read as evidence here, not edited.
- No tree body for an inline `match`. A `match` standing in a term is 155's emission, which is what §6.2 requires.
- No new surface syntax, and no change to what an author writes for a recursive definition.
- No change to `docs/rules/`. §1's `Definition` list is what this prompt implements.
