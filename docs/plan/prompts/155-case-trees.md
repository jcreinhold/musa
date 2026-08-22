---
id: 155
slug: case-trees
status: in-progress
depends_on: [153]
phase: 3
---

# Replace Generated Recursors with Case Trees

## Task

`family/assemble.rs` generates a **non-dependent** recursor per declaration and `case.rs` (1,203 lines) compiles
`match` straight into an application of it, with the goal itself as the motive. Reify the intermediate: a `CaseTree` the
kernel holds, a `Definition::Compiled(CaseTree)` as a function's body, ι as case-tree reduction, and coverage decided on
the tree. **This is where the eliminator stops being non-dependent**, which is the defect prompt 143's amendment named.

**Corrected against the code and against §1.1.** The eliminator is not deleted — see the Design's first paragraph. What
is deleted is its *non-dependence*, and the primitive ι rule that made a recursor a kernel constant rather than a
generated definition.

## Read

- `crates/musa-calculus/src/kernel/family/assemble.rs:154` — `motive_type`, and the doc comment on `motives` above it:
  *"A *type*, not a family of them. §1.1's eliminator is non-dependent."* That is the sentence this prompt deletes.
  `method_type` and `hypothesis` are the two places the sentence is *implemented*, and both change with it.
- `docs/rules/language/02-core-calculus.md` §1 (the `Definition` list), §1.1 (*Elimination is dependent*), and §6.2 — the
  three sentences that decide what survives, and they only agree under one design. §1 lists a name's reduction behaviour
  as "undeclared, **a compiled case tree**, a constructor, a type constructor, a registered base type, or a compiler
  builtin", with **no recursor in it**; §1.1 keeps "**the generated eliminator**" and makes its motive a family; §6.2
  compiles `match` "to a case tree and then to nested applications of the generated eliminators". Prompt 156 says the
  same from the other side: "a family's fold *is* its eliminator".
- `/Users/jcreinhold/Code/Idris2/src/Core/Case/CaseBuilder.idr` and `Core/Case/CaseTree.idr` — the reference.
- `crates/musa-calculus/src/elaboration/rec.rs` — where structural descent is currently obtained *for free*, and the
  file that says why it stops being free. `rec` rewrites a recursive call into a reference to `<field>#ih`, an
  induction-hypothesis binder that `case.rs` gets from the recursor's method for nothing. The rewrite is on **raw
  syntax** — deliberately, because the core has no substitution on terms — so it can only certify a definition an author
  wrote. A tree the kernel generates and then reduces is not source, which is what this prompt owes a real check for.

## Design

**The eliminator is still generated; what goes is the *primitive* recursor.** §1.1 keeps `N.elim` and makes its motive a
family, §6.2 still compiles a `match` to applications of it, `Found::named` puts the spelling in the namespace, and
`family_laws.rs` states fifteen laws over `Nat.elim`. Deleting the name would delete a language feature this prompt has
no business touching, and §1.1 is post-amendment text that says the opposite. What §1's `Definition` list deletes is the
recursor as a **kernel constant with its own ι rule** — the arm of `family/iota.rs` that fires at a recursor's arity,
and the `Constant::recursor_type` that is non-dependent by construction. `N.elim` is generated instead with a dependent
motive, and its reduction is the tree's. That is the single reading in which §1's list, §1.1's sentence, and §6.2's
sentence are all true at once, and `docs/rules/language/README.md` is explicit that a contradiction between them would
be a prompt defect to repair rather than a licence to pick one.

**The tree, three nodes.** `Split { on, alternatives }`, `Answer(Term)`, `Impossible`. `Impossible` is the branch index
unification ruled out — it is not a runtime error, because there is no runtime and totality means coverage cannot fail.
Idris2 has a fourth node, `Unmatched`, for exactly the case musa forbids.

**ι becomes tree reduction.** A `Named` whose `Definition` is a `Compiled` tree reduces when the scrutinee of its
outermost `Split` is canonical data. `family/iota.rs` shrinks to the lookup; the tower-avoiding numeral decrement that
`Constant` exists for stays, because a `Nat` split still must not unfold a tower.

**The motive becomes dependent, and that is the point.** A branch is checked at the return type *instantiated at that
branch's pattern*. This is what makes `Vect`, `Equal`, and refinement by matching work at all, and it is the single line
of the current design that no amount of index machinery could substitute for.

**Termination becomes real work, and it belongs here.** Today `rec.rs` gets structural descent for free: an argument of
a generated recursor is structurally smaller by construction. Case trees give that up, so this prompt owes a termination
check *over the tree* — an argument is smaller when it is a field of the pattern the split matched, and a recursive call
is admitted when some argument is smaller and none is larger. **Structural descent is sufficient and is much smaller
than size-change termination**; do not build the latter. What it costs is that a function whose recursion is not
structural is refused, which is the same set of functions the current recursor refuses.

**Coverage.** Every constructor of the split family has a branch, or the branch is `Impossible` and index unification
proves it. A missing branch is a refusal at the `match`, naming the constructor.

**The pattern matrix stays.** Musa compiles patterns through a matrix already and does not elaborate a clause's
left-hand side *as a term* — which is why `PVar`/`PLet`/`PVTy` are absent from the term language. The cost is that
pattern-implicit solving lives in the case builder rather than falling out of elaboration; pay it here explicitly.

## Target

- `crates/musa-calculus/src/kernel/case_tree.rs`: the three nodes and their reduction.
- `crates/musa-calculus/src/elaboration/case.rs`: the builder — matrix to tree, with index unification per split.
- `crates/musa-calculus/src/kernel/family/assemble.rs`: `motive_type` **deleted**, and `motives` rebuilt as the family
  `(t : N p⃗) → Type ℓ` that §1.1 states. `method_type`'s result and `hypothesis` become the motive *applied* — to the
  constructor form for the result, to the recursive field for the hypothesis — which is the whole of what dependence
  means here.
- `crates/musa-calculus/src/kernel/family/constant.rs` and `family/iota.rs`: `recursor_type` follows the dependent
  motive, and the primitive ι arm gives way to tree reduction. The tower-avoiding numeral decrement stays, as the
  Design says.
- `crates/musa-calculus/src/kernel/terminate.rs`: structural descent over the tree, with the smaller-argument rule
  stated as a doc comment before it is implemented.
- `crates/musa-calculus/tests/suite/`: the ported coverage corpus, the dependent-motive law, and a termination law with
  a negative control.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! grep -rn 'motive_type\|Unmatched' crates/musa-calculus/src
git diff --stat crates/musa-calculus
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

`git diff --stat` is recorded rather than gated, and it is a *record* rather than a signal: with the eliminator
surviving, most of `family/`'s 1,823 lines survive with it, and the number this prompt moves is the tree, the dependent
motive, and the termination check. It is written down so 156 and 157 can be read against it.

**The re-checker's obligation for this prompt, and it is the second important one.** It must verify that each branch
checks at the motive instantiated at that branch's pattern, that coverage is complete as re-derived from the family
rather than from the builder's bookkeeping, and that structural descent holds over the finished tree. Negative controls
for all three. Nothing else in the crate re-states the dependent-elimination invariant.

Commit as `Replace generated recursors with case trees`.

## Stop

- No indexed *families* yet — constructors still may not choose an index. 156.
- No size-change termination, no `assert_total`, no partiality.
- No new surface syntax for `match`.
