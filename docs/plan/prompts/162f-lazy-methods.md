---
id: 162f
slug: lazy-methods
status: pending
depends_on: [155a, 161]
phase: 3
---

# A Recursor's Methods Are Evaluated When ι Chooses One, Not Before

## Task

`Bool`'s constructors carry nothing, so the method a case tree holds for each of them *is a value*, and the machine
evaluates every argument of an application before applying it. An `if`/`else if` chain therefore evaluates every
condition in the chain **and both branch values**, whichever branch is chosen. Note 60 §3 measured what that costs and
what an author does about it: `stdlib/src/adapters/staff.musa` carries a two-stage `Sighted`/`Kinded` classifier that
exists for no reason in the domain, because shortening a chain is the only optimization the evaluator rewards and
reordering one is worthless. Make a recursor strict in its target and lazy in its methods, and add the fifth rule to
`02-core-calculus.md` §3's evaluation strategy that says so.

## Read

- `docs/notes/research/language-design-closure/60-the-staff-rewrite-measured.md` §3 — the measurement, in full. The cost
  table, the reorder that made the file *slower* (15,981 → 16,716 steps), and the closing sentence: "An arm for a
  nullary constructor that is not evaluated when it is not chosen would delete all three." This prompt exists because
  that note names the change and does not make it.
- `docs/rules/language/02-core-calculus.md` §3, its four-bullet **"The strategy is part of the specification"** list —
  the paragraph this prompt amends by adding a fifth bullet — and §3's opening `≡` definition, which the change must
  leave exactly as it stands.
- `docs/rules/language/02-core-calculus.md` §4 and §4.1 — budget and nesting. A step the machine no longer takes is a
  step the meter no longer charges, so this prompt moves numbers that §4's limits are stated against, and the Check has
  to show which way.
- `docs/rules/language/02-core-calculus.md` §6.2 — patterns are compiled to case trees, which is why this is a rule
  about recursor methods rather than about a `match` form the core does not have.
- `crates/musa-calculus/src/kernel/eval.rs` — the machine: `Frame::Argument`/`Frame::Applied`, which is where
  left-to-right call-by-value is decided; `eliminating` and `opens_last`, which already single out the target as the one
  argument opened by a transition rather than from inside a rule; and `Frame::Hypotheses`/`Frame::Hypothesis`, which is
  where a chosen method already meets its fields.
- `crates/musa-calculus/src/kernel/family/iota.rs` — `opens_last`, `Role::Recursor`, and `Constant::arity`. Which
  argument is the target and how many methods precede it are already known there; nothing new has to be computed to know
  which arguments this prompt delays.
- `crates/musa-calculus/TRUST.md` — the trusted half. This changes the evaluator, so the file's account of what is
  trusted and why has to still be true afterwards.
- Peyton Jones ch. 11 §11.3 and ch. 22 — evaluation order as a property of the *implementation* under a fixed
  denotation, and strictness as the analysis that recovers eager evaluation where it is safe. Musa is total, so the
  direction is the easy one: laziness here can only remove work, never change an answer.

## Design

**Totality is what makes this a cost change and not a semantic one.** In a language with divergence, making an argument
lazy changes which programs terminate, and the specification would have to say which evaluation order it means. Musa's
core is total (§2.4), every δ-rule is a function of its arguments, and no rule emits a diagnostic. So an unevaluated
method and an evaluated one are the same value, and the only observable difference is the meter — which is also the one
place it must be observable, because `Budget::LANGUAGE` is what an author runs out of. §3's `≡` is unchanged, `quote`
answers the same normal forms, and the conformance oracle is expected byte-identical. **The Check states that as a
prediction, not a hope**: if a single snapshot moves, the change is wrong.

**Strict in the target, lazy in the methods.** For a `Role::Recursor` constant the target is the last argument and the
methods are the arguments between the motive and it. Those arguments are collected unevaluated — environment and term —
and the machine evaluates the target, fires ι, and evaluates the one method the ι-rule selects, then its induction
hypotheses as it does today. The motive is a type and is not a method; it is delayed on the same terms, because nothing
reads it unless the spine gets stuck.

**A delayed argument is forced at exactly two places, and they are named.** (1) ι selects it. (2) The spine *stays*
stuck — the target opened to a neutral — and the elimination must be read back or compared, at which point every delayed
argument on the spine is forced before the neutral is handed on. Nothing else may look at one, and the type should make
that hard to get wrong rather than a rule a reader has to audit: a delayed argument is not a `Value` and does not
implement what a `Value` implements.

**It is a memo, not a re-evaluation.** A method forced once is forced once. A recursor under a fold meets its methods
per turn, and a delayed argument that re-evaluated its term each time would turn note 60's saving into a loss on exactly
the programs it is for.

**The rule reaches every recursor, not `Bool`.** `if` is a case tree over `Bool` and this fixes `if`, but the rule is
stated over recursors because that is what the core has, and an eight-arm `match` over a token kind has the same seven
unevaluated arms.

**Projections are not touched.** ι at a field accessor takes the parameters and the value and has no method (`iota.rs`
says so), so there is nothing to delay and the rule does not mention them.

## Target

- A recursor's methods and motive are evaluated only when ι selects one, or when the spine stays stuck; forcing is
  memoized.
- `docs/rules/language/02-core-calculus.md` §3 gains its fifth strategy bullet, stated as the four above it are — a
  rule, with the reason it is cheap and what a checker that lost it would still be.
- `crates/musa-calculus/TRUST.md` updated where the evaluator's account changed.
- Laws in `musa-calculus`: an unchosen arm whose body would exhaust the budget on its own does not exhaust it; a chosen
  arm's cost is unchanged; forcing happens once under a fold; a stuck recursor reads back exactly as it does today; ι at
  a projection is unaffected.
- A measured before/after in the commit message, with the note 60 probe restated: `examples/staff-page.musa`'s expansion
  step count before and after, and the twenty-item differential's.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Every snapshot and `tests/fixtures/elaboration-compatibility.txt` must come out byte-identical — that is the check that
this was a cost change. One moved snapshot is a failure of the prompt, not a snapshot to accept.

```sh
cargo nextest run --run-ignored all
```

Carries whatever reds prompt [164](164-builtin-collapse.md)'s Check enumerates; a *new* red is this prompt's, and a red
that goes *green* here is the point and belongs in the report.

Commit as `Evaluate a recursor's method when the target chooses it`.

## Stop

- No general laziness. Function arguments stay strict, `let` values stay strict, δ stays as it is. Recursor methods and
  the motive, and nothing else.
- No strictness analysis, no cost annotations, no author-visible `lazy` or `force`.
- No rewrite of `stdlib/src/adapters/staff.musa` to spend the saving. Note 60's `Sighted`/`Kinded` split can come out
  once this lands; that is prompt [166](166-staff-rewrite.md)'s to do and this prompt's only to make possible.
- No change to `Budget::LANGUAGE` or `Budget::NESTING`. The numbers move under the limits; the limits do not move to
  meet them.
- No change to §3's `≡`, to `quote`, or to conversion. If the implementation needs one, the design is wrong and the
  prompt needs repair.
