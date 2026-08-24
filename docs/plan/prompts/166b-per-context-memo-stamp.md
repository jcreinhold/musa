---
id: 166b
slug: per-context-memo-stamp
status: done
depends_on: [165b]
phase: 3
---

> **Cut out of [`166`](166-staff-rewrite.md) by the measurement in
> [note 60](../../notes/research/language-design-closure/60-the-staff-rewrite-measured.md) §6, taken when 166 un-ignored
> the twenty-five staff expansion laws and ran them in one process for the first time.** Nothing in 166 caused it and
> nothing in 166 can fix it: the defect is in the memo [`165b`](165b-graph-update-and-data-descent.md) added, and 166's
> Stop admits one line of `crates/`.

## Task

Give the δ-unfolding memo's invalidation stamp the scope its soundness argument actually needs. It is a process-global
`AtomicU64` today, so **one compilation's reduction-step count depends on what other compilations are doing on other
threads of the same process** — which makes the budget's acceptance non-deterministic and contradicts
`docs/rules/language/02-core-calculus.md` §4.

## Read

- `crates/musa-calculus/src/kernel/meta.rs`, `SOLUTIONS` and `solutions()` — the stamp, and the doc comment that states
  the soundness argument it is discharging: "equal stamps mean no solution arrived in between … a solution anywhere
  invalidates every memo rather than only the ones that mention it."
- `crates/musa-calculus/src/kernel/eval.rs` — `Frame::Unfolding { cell, stamp }`, `folded`, and `unfold`'s memo read and
  write. The unit law `a_solved_metavariable_makes_a_forced_value_unfold_again` is the property that must survive.
- `crates/musa-calculus/src/kernel/value.rs` — `memo_for` and `Neutral::unfolded`, which decide which heads carry a cell
  at all.
- [`165b`](165b-graph-update-and-data-descent.md), which added the memo, and
  [note 60](../../notes/research/language-design-closure/60-the-staff-rewrite-measured.md) §6, which measured the
  defect: the staff adapter's module read is a fixed 75,377 steps and a small region's run 55,241 more, comfortably
  inside 200,000 — but under sixteen concurrent tests in one process the same run reports `attempted 200001`.
- `docs/rules/language/02-core-calculus.md` §4 and `crates/musa-calculus/src/kernel/budget.rs` — the determinism the
  budget claims, stated as acceptance rather than as timing.
- `crates/musa-calculus/TRUST.md` — this is the trusted half, so the change is argued rather than merely made.

## Design

**The stamp belongs to whatever owns the metavariables it is a stamp about.** `Meta::new` takes `cx.globals()`, so a
metavariable already knows a context; a counter on that, bumped in `Meta::solve` and read in `solutions()`'s place, is
the same conservative test with the scope its own doc comment describes. Two elaborations that share no globals cannot
solve each other's unknowns, and today's stamp says they can.

**Do not narrow the test in the other direction at the same time.** Asking *which* metavariable a memo mentions means
walking the value, which is the walk the stamp exists to avoid, and it is a separate change with its own argument.
Per-context is the smallest scope that is obviously sound; keep the conservatism inside it.

**A thread-local is not the answer**, even though note 60 used one to confirm the diagnosis in a single experiment. It
happens to work because `with_room` runs each expansion on its own thread, which is an implementation detail of the
host, and it would silently stop being conservative the moment two contexts shared a thread or one context spanned two.

**The regression test is the one that found it.** A law that elaborates the same program twice — once alone, once while
a second context on another thread solves metavariables in a loop — and asserts the two spends are *equal*. Step counts
are the observable; asserting equality rather than "both under the limit" is what makes the law about determinism
instead of about headroom.

## Target

- The invalidation stamp scoped to the context that owns the metavariables, with `SOLUTIONS` gone from
  `crates/musa-calculus/src/kernel/meta.rs`.
- `a_solved_metavariable_makes_a_forced_value_unfold_again` passing unchanged.
- **The regression law is `cargo test -p musa-compiler --test suite`, and it is the only one that discriminates.**
  Design asked for a `musa-calculus` law that elaborates a program beside a thread solving metavariables and asserts the
  two spends equal. It was written and it does not work, and *why* it does not is worth recording rather than retrying:
  a memo cell exists only on a `Head::Def` carrying a `Folding::Value` (`memo_for`), and a *hit* needs the same neutral
  forced twice, which needs a value shared across two forcings. A micro-law can build a folded definition easily and
  cannot make one of its neutrals be forced twice without contriving the sharing — and a fixture bent to fit the
  mechanism rather than the domain is the shape `docs/plan/prompts/README.md` warns about. Real programs get the sharing
  for free: note 60 records 4,434 memo hits in one staff compile.

  Measured rather than assumed. With the stamp reverted to a process-global counter, `cargo test -p musa-compiler --test
  suite` fails **28 of 693** laws; with it scoped to the run, 0. That is a sharp regression signal, it is already in
  **Check**, and it is what this Target now asks for.
- `cargo test -p musa-compiler --test suite` green under libtest's default thread count, which is the failure note 60 §6
  records.
- `crates/musa-calculus/TRUST.md` updated if the change moves anything across the trusted boundary, and a sentence in
  `crates/musa-calculus/src/kernel/meta.rs` recording why the scope is what it is.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run -p musa-calculus -p musa-compiler --run-ignored all
cargo test -p musa-compiler --test suite
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
! grep -n 'static SOLUTIONS' crates/musa-calculus/src/kernel/meta.rs
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

`cargo test -p musa-compiler --test suite` is in the list on purpose and is not redundant with `nextest`: nextest runs
each test in its own process and cannot observe this defect at all. The one-process, many-threads run is the
observation, and it is the reason the bug lived through 165b's own Check.

Commit as `Scope the unfolding memo's invalidation stamp to its context`.

## Stop

- No narrowing of *which* memos a solution invalidates. Per-context conservatism only; the occurrence walk is a separate
  prompt with a separate argument.
- No change to the memo's shape, to `memo_for`'s choice of which heads carry a cell, or to what a step costs. The cost
  table is a version bump (`02-core-calculus.md` §4) and this is not one — the whole claim is that the *same* program
  spends the *same* number of steps.
- No budget change.
- No adapter, stdlib, or example change. If the staff adapter needs editing to make this pass, the diagnosis in note 60
  §6 is wrong and this prompt says so rather than editing it.
