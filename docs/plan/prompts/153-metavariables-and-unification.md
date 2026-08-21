---
id: 153
slug: metavariables-and-unification
status: pending
depends_on: [152]
phase: 3
---

# Give the Core Real Metavariables and Pattern Unification

## Task

`elab/spine.rs` does first-order matching: §2.1 took Idris2's `checkRtoL` and dropped its fallback. Every construct that
wants a real solution — an implicit argument, index unification in `match`, a metavariable that outlives one call — is
unavailable because the mechanism underneath it is not there. Put it there: `Meta` in the term language with a context
and a solution slot, a `UnifyState` with a constraint queue, and pattern-fragment unification.

**This is the step where dependent-type implementations go wrong, and it is the one to port rather than reinvent.**

## Read

- `/Users/jcreinhold/Code/Idris2/src/Core/Unify.idr` in full — `unifyD`, `postpone`, `retryGuess`, `patternEnv`, and the
  invertibility test. This prompt is that file, in Rust, minus quantities and laziness.
- `/Users/jcreinhold/Code/Idris2/tests/` — the unification corpus. Port it here, not at the end.
- `crates/musa-calculus/src/elaboration/elab/spine.rs` — what is being replaced.
- `crates/musa-calculus/src/kernel/meta.rs` — `MetaSource` already exists with two arms; it gains the solution.

## Design

**Miller pattern fragment, and the fallback is postponement.** A constraint `?m x₁ … xₙ ≟ t` is solved directly when the
`xᵢ` are distinct bound variables — the pattern fragment, where the solution is unique. Everything else is *postponed*,
not guessed and not refused: a queue entry retried when a metavariable it mentions is solved. Refusing outside the
fragment is what the current first-order matcher effectively does, and it is why implicits cannot work.

**A metavariable carries its context.** `Meta { source, ty, scope }` — the spine of variables it may depend on — so a
solution can be checked to mention nothing outside it. The occurs check and the scope check are the same walk.

**Solutions live in a table, not in the term.** A solved metavariable is not substituted through the program; `eval`
looks it up. This keeps the invariant the crate already has — **no substitution function anywhere** — and it is why
`zonk` exists as a separate pass rather than as a side effect of solving.

**Three outcomes, not two.** *Solved*, *definitely not unifiable*, and *blocked*. The third is a real answer and the
queue is where it lives; collapsing it into the second is the classic bug that makes elaboration order-dependent. Prompt
150 already assigned these variants to `CoreError`.

**What is deliberately not implemented.** No higher-order pattern unification beyond the Miller fragment. No constraint
solving across declaration boundaries. No `retryGuess` for `Delay`, which musa has no constructor for.

**The budget meets a new client.** Unification charges the same meter, and a postponed constraint retried *n* times
costs *n* times. `budget.rs` gains a metric or reuses `Steps` with an argument for which; the specification says which
and the law suite proves a runaway queue exhausts rather than hangs.

## Target

- `crates/musa-calculus/src/kernel/meta.rs`: `Meta` with scope and solution; the table; occurs and scope checks.
- `crates/musa-calculus/src/kernel/unify.rs`: `unify`, the pattern test, `postpone`, `retry`.
- `crates/musa-calculus/src/elaboration/elab/spine.rs`: deleted, its callers routed through `unify`.
- `crates/musa-calculus/src/elaboration/zonk.rs`: the pass that reads solutions back before a term is stored.
- `crates/musa-calculus/tests/suite/unify_laws.rs`: the ported corpus, plus the three-outcome law and the budget law.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! grep -rn 'fn subst' crates/musa-calculus/src
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

The `grep` is not decoration: a substitution function appearing here means solutions were pushed into terms, which
breaks the NbE presentation prompt 144 specified and makes `quote` no longer the only reader.

**The re-checker's obligation for this prompt, and it is the important one.** `Checked::try_from` gains its rejection of
unsolved metavariables — prompt 149 built the newtype for this line — and the re-checker must verify that a solution
mentions nothing outside its metavariable's scope, independently of the scope check made at solving time. A negative
control that solves a metavariable with an out-of-scope variable is part of the deliverable. **Without this, this
prompt's failure mode is silent**, which is why 149 comes first.

Commit as `Give the core real metavariables and pattern unification`.

## Stop

- No implicit *insertion* — that is 154, and separating them is what makes this prompt reviewable against `Unify.idr`.
- No case trees, no index unification in patterns. 155 and 156.
- No elaborator reflection, no `Elab` monad, no reification. Musa has one macro system and it is 160's.
