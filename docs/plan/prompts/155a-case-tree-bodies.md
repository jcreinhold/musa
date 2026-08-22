---
id: 155a
slug: case-tree-bodies
status: pending
depends_on: [155]
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
over the finished tree takes its place: an argument is **smaller** when it is a field of the pattern the split bound,
and a recursive call is admitted when some argument is smaller and none is larger. **Structural descent is sufficient
and is much smaller than size-change termination**; do not build the latter. What it costs is that a function whose
recursion is not structural is refused, which is the same set `rec.rs` refuses today — and `rec.rs`'s own module doc
enumerates that set, so it is the negative-control corpus this prompt inherits rather than invents.

**`Impossible` gets its producer if 156 landed first, and stays unreachable otherwise.** This prompt does not depend on
156 and must not assume it: a tree it builds has an `Impossible` alternative only where index unification put one.

## Target

- `crates/musa-calculus/src/kernel/term.rs`: `Definition::Compiled`, carrying the tree **and the binders it stands
  under**, in place of `Defined` for a definition whose body splits, with the note that named this prompt removed
  because it has happened.
- `crates/musa-calculus/src/kernel/case_tree.rs`: reduction — a tree, an environment of arguments, a forced scrutinee,
  and the alternative it selects; a scrutinee that is not canonical leaves the name neutral.
- `crates/musa-calculus/src/kernel/program.rs`: a definition's body as a tree, answering the [`Value`]-and-[`Term`]
  question its `Defined` doc poses.
- `crates/musa-calculus/src/kernel/family/iota.rs`: **unchanged**, for the reason the Design's second correction gives.
- `crates/musa-calculus/src/kernel/terminate.rs`: structural descent over the tree, with the smaller-argument rule
  stated as a doc comment before it is implemented.
- `crates/musa-calculus/src/elaboration/rec.rs`: the `#ih` rewrite retired in favour of the check, with
  `Refusal::UncheckedRecursion` still naming the call it refused.
- `crates/musa-calculus/tests/suite/`: a reduction law per node, a law that a split on a counting family costs bounded
  steps, a termination law with a negative control, and `rec.rs`'s enumerated refusals ported so that none of them
  quietly becomes accepted.

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
