---
id: 142b
slug: finish-the-excision
status: pending
depends_on: [142]
phase: 3
---

# Finish Note 50's Excision, and Make the Prose Stop Teaching the Deleted Design

## Task

Note 50's phase 2 was executed piecemeal across six unlabelled refactor commits and never closed out. `musa-calculus`
went 20,869 → 17,841 lines against a stated target of 11–12K, several audited mechanisms are still standing, and — the
part that costs a reader most — the module documentation now teaches the superseded design and contradicts itself inside
single files. Finish the excision, and repair every doc comment the excision falsified.

This prompt adds no feature. It is the closing audit phase 2 should have had.

## Read

- [`50-the-course-correction-audit.md`](../../notes/research/language-design-closure/50-the-course-correction-audit.md)'s
  audit table, row by row — the verdict column is this prompt's checklist.
- [`51-the-terseness-audit.md`](../../notes/research/language-design-closure/51-the-terseness-audit.md) §7, which
  records what stays deleted, and §4, which is why `family/`'s *index* machinery goes even though prompt 142d re-admits
  a different index mechanism. They are not the same thing and must not be confused: an inductive family still has
  parameters only.
- The audit's own mis-scoring, corrected in note 51 §1's preamble: `unify.rs` is not 791 lines of unification.
  `assignment` is 68; the rest is the conversion checker and the glued-definition path, both **keep** rows.

## Design

**What is still standing, with its verdict:**

| Site | Now | Verdict |
| --- | --- | --- |
| `family/` (5 files, 1,782) | dependent recursors with motives; `mod.rs` teaches indices with a `Vec A n` example | indices and dependent motives out; a plain non-dependent eliminator that still gives `rec.rs` its hypotheses |
| `dictionary.rs` 1,162 + `class.rs` 408 | telescoped `where` clauses, super-constraint fields | the flat `(trait, head)` table note 50 specified |
| `Plicity` (17 files, 81 sites) | three-valued, with `Implicit` documented as "inserted at every use, as a metavariable" | two-valued: a type parameter and a constraint. The word *implicit* goes |
| `Refusal::IdType` | a variant with no constructor | delete |
| `case.rs` motive inference | general | `match` checks against the expected type |

**The prose repairs are not cosmetic and are listed because they are the defect a reader hits first:**

- `unify.rs`'s module header teaches pattern unification, postponement, and a retry loop, and is contradicted by the
  `Unifier` struct doc twelve lines below it. The header is deleted; the struct doc is already correct.
- `meta.rs` opens "the calculus creates no metavariables, so there is nothing here to solve" and then defines `Hole`
  with `solve()`. Say what a hole is.
- `eval.rs` documents ι as `jay` on a `Form::Refl`. Neither exists; three intra-doc links point at a deleted function.
- `rec.rs` cites "§2.4's measure" where §2.4 is now *Termination is structural*.
- `raw.rs:19` and `elab/mod.rs:32` describe implicit binders producing a metavariable at each use.
- `family/mod.rs`'s opening example declares `data Vec (A : Type l) : (n : Nat) → Type l`, which §1.1 does not admit.
- `docs/rules/language/citations.md` still lists Miller (1991) as a mechanism in force. It is the fragment the surviving
  rule is the base case of; the row says that or goes.

**Rename `unify.rs` to `convert.rs`, and `Unifier` to `Conversion`.** One file answers three questions — is-equal,
solve-this, which-definition-unfolds-first — under a name that says only the middle one, which is the defect the crate
renames just removed at workspace scale. `Conversion::deciding()` and `Conversion::solving()` name the two modes the
`deciding: bool` already distinguishes.

**A doc comment falsified by a commit is that commit's defect.** After this prompt, a prose repair travels with the
change that caused it; there is no second sweep.

## Target

- `crates/musa-calculus/`: the five verdicts above, `unify.rs` → `convert.rs`, and every doc comment listed.
- `docs/rules/language/citations.md`: the Miller row.
- No public API change outside the crate. `musa-compiler`'s `lower/laws.rs:731` asserts a function's type parameter is
  `Plicity::Implicit` and moves with the enum.
- A recorded line count, before and after, in the commit message — the number note 50 owes and never took.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
cargo doc -p musa-calculus --no-deps          # intra-doc links resolve; `jay` does not
```

The red ledger of prompt 142 — 27 staff, 2 tonal, 3 pressure — moves through this prompt byte-identically. New red or
missing red is a finding.

## Stop

- No new mechanism. The index stratum is 142d and must not be anticipated here.
- Do not touch `eval.rs`'s glued-definition path, `budget.rs`, `base.rs`, or the quotation builtins.
- Do not delete `unify.rs`'s conversion code while renaming it. The audit's 935 → 200 row was mis-scoped; the conversion
  checker is a **keep**.
