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
becomes a body, ι becomes tree reduction, and a body that calls itself becomes something the checker owes a real
termination argument for.

**Why this is not part of 155.** Three measurements, taken while 155 was being prepared.

- §6.2 governs 155's delivery and says the tree is compiled *to eliminator applications*. A tree that is only an
  intermediate has no reduction of its own, so `case_tree.rs`'s reduction and a termination check over it have nothing
  to act on inside 155's boundary.
- `crates/musa-calculus/src/elaboration/rec.rs` obtains structural descent for free by rewriting a recursive call into
  `<field>#ih`, an induction-hypothesis binder the eliminator's method supplies. 155's first repair established that the
  eliminator survives, so the hypotheses survive, so descent stays free for exactly as long as a tree is not a body.
  This prompt is where it stops being free.
- Prompt 157 says "a generated projection is an ordinary one-branch case tree", which needs a tree that is a **body**
  and not a step in a compilation. 157 is what this prompt is a prerequisite for.

## Read

- `docs/rules/language/02-core-calculus.md` §1, the `Definition` list — "undeclared, **a compiled case tree**, a
  constructor, a type constructor, a registered base type, or a compiler builtin". Six arms, and the second is what this
  prompt puts there. Read §6.2 with it: the emission 155 delivers is what a body's tree replaces.
- `crates/musa-calculus/src/kernel/term.rs`, `Definition` — the five arms today, and `Defined`'s note.
- `crates/musa-calculus/src/kernel/program.rs`, `Defined` — a definition holds its body as an already-evaluated
  [`Value`] *and* as a [`Term`], for the universe-polymorphism reason its doc gives. A tree body has to answer both
  questions, and that is the design's one real constraint.
- `crates/musa-calculus/src/kernel/case_tree.rs` — what 155 built, and the `Impossible` node it left without a producer.
- `crates/musa-calculus/src/kernel/family/iota.rs` — the ι that fires at a recursor's arity today, and the
  tower-avoiding numeral decrement that must survive whatever replaces it.
- `crates/musa-calculus/src/elaboration/rec.rs` — the `#ih` rewrite this prompt retires, and its module doc, which
  argues at length for why the rewrite is on raw syntax. That argument is what a termination check over a tree has to
  answer.
- `/Users/jcreinhold/Code/Idris2/src/Core/Case/CaseTree.idr` and `Core/Normalise.idr`'s case-tree evaluation — the
  reference for reducing a tree rather than an eliminator spine.

## Design

**`Definition::Compiled(CaseTree)`.** A definition with no `match` at its top is `Answer(body)` and reduces exactly as
`Defined` did. A definition whose body splits holds the tree, and δ becomes: look up the tree, force the scrutinee, and
take the alternative its constructor names. A scrutinee that is not canonical leaves the name neutral, which is what
makes a case tree behave like the eliminator it replaces rather than like a runtime `switch`.

**ι becomes tree reduction, and `family/iota.rs` shrinks to the lookup.** A generated eliminator becomes a definition
whose body is a tree: one `Split` on the target, one `Answer` per constructor applying that constructor's method to its
fields and to the eliminator at each recursive field. The tower-avoiding numeral decrement stays, because a `Nat` split
still must not unfold a tower to decide which alternative it takes.

**Termination becomes real work, and it belongs here.** A definition whose body is a tree may name itself, so the `#ih`
rewrite is retired and a check over the finished tree takes its place: an argument is **smaller** when it is a field of
the pattern the split bound, and a recursive call is admitted when some argument is smaller and none is larger.
**Structural descent is sufficient and is much smaller than size-change termination**; do not build the latter. What it
costs is that a function whose recursion is not structural is refused, which is the same set `rec.rs` refuses today —
and `rec.rs`'s own module doc enumerates that set, so it is the negative-control corpus this prompt inherits rather than
invents.

**`Impossible` gets its producer if 156 landed first, and stays unreachable otherwise.** This prompt does not depend on
156 and must not assume it: a tree it builds has an `Impossible` alternative only where index unification put one.

## Target

- `crates/musa-calculus/src/kernel/term.rs`: `Definition::Compiled(CaseTree)` in place of `Defined`, with the note that
  named this prompt removed because it has happened.
- `crates/musa-calculus/src/kernel/case_tree.rs`: reduction — a tree, a scrutinee value, and the alternative it selects.
- `crates/musa-calculus/src/kernel/family/iota.rs`: a generated eliminator's body as a tree, and the arm that fired at
  the recursor's arity gone.
- `crates/musa-calculus/src/kernel/terminate.rs`: structural descent over the tree, with the smaller-argument rule
  stated as a doc comment before it is implemented.
- `crates/musa-calculus/src/elaboration/rec.rs`: the `#ih` rewrite retired in favour of the check, with
  `Refusal::UncheckedRecursion` still naming the call it refused.
- `crates/musa-calculus/tests/suite/`: a reduction law per node, a termination law with a negative control, and
  `rec.rs`'s enumerated refusals ported so that none of them quietly becomes accepted.

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
- No new surface syntax, and no change to what an author writes for a recursive definition.
- No change to `docs/rules/`. §1's `Definition` list is what this prompt implements.
