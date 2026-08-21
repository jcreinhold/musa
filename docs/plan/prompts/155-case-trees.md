---
id: 155
slug: case-trees
status: pending
depends_on: [153]
phase: 3
---

# Replace Generated Recursors with Case Trees

## Task

`family/assemble.rs` generates a recursor per declaration and `case.rs` (1,191 lines) compiles `match` into an
application of it. Replace both with case trees: a `Definition::Compiled(CaseTree)` as a function's body, ι as case-tree
reduction, and coverage checking in place of the constructor-coverage rule. **This is where the eliminator stops being
non-dependent**, which is the defect prompt 143's amendment named.

## Read

- `crates/musa-calculus/src/kernel/family/assemble.rs:140` — `motive_type`, and its own doc comment: *"A type, not a
  family of them. §1.1's eliminator is non-dependent."* That is the sentence this prompt deletes.
- `/Users/jcreinhold/Code/Idris2/src/Core/Case/CaseBuilder.idr` and `Core/Case/CaseTree.idr` — the reference.
- `crates/musa-calculus/src/kernel/rec.rs` — where structural descent is currently obtained *for free*.

## Design

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
- `crates/musa-calculus/src/kernel/family/assemble.rs`: the recursor generator **deleted**; `motive_type` with it.
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

`git diff --stat` is recorded rather than gated: this prompt adds roughly 1,000–1,400 lines and deletes most of
`family/`'s 1,800 and much of `case.rs`'s 1,191. A large positive number means the recursor machinery did not go.

**The re-checker's obligation for this prompt, and it is the second important one.** It must verify that each branch
checks at the motive instantiated at that branch's pattern, that coverage is complete as re-derived from the family
rather than from the builder's bookkeeping, and that structural descent holds over the finished tree. Negative controls
for all three. Nothing else in the crate re-states the dependent-elimination invariant.

Commit as `Replace generated recursors with case trees`.

## Stop

- No indexed *families* yet — constructors still may not choose an index. 155.
- No size-change termination, no `assert_total`, no partiality.
- No new surface syntax for `match`.
