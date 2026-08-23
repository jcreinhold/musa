---
id: 165b
slug: graph-update-and-data-descent
status: in-progress
depends_on: [162a]
phase: 3
---

> **Cut out of [`165`](165-diagnostics-and-performance.md) by the measurement in
> [note 59](../../notes/research/language-design-closure/59-the-staff-wall-is-the-evaluators.md), taken at prompt 166
> before a line of the rewrite was written.** Both halves below were 165's, and neither can wait for it: 165 follows
> 164, 164 follows 166, and 166 cannot be developed — let alone measured — while every compile of the file under rewrite
> dies at a resource limit. What is left in 165 is its diagnostics, its P1/P2 gate, and the four smalltt items.

# Memoize the Unfold, and Take Data Descent Off the Nesting Metric

## Task

Two defects in `musa-calculus`'s evaluator, unrelated to each other and to any library, together make the standard
library's staff adapter impossible to run. Note 59 measures both, on the adapter exactly as it stands:

- **δ has no graph update.** `kernel/eval.rs`'s `unfold` replays a definition's spine and records the result nowhere, so
  every consumer of an `Arc`-shared folded value pays the whole replay again — and where the replayed body ends in more
  folded applications the cost multiplies down the chain. A staff region **with nothing in it** costs 455,942 reduction
  steps against a budget of 200,000. One word costs more than 20,000,000. With a memo on the neutral and nothing else
  changed, those become 2,796 and 10,168.
- **The nesting metric counts data.** `eval.rs`'s `canonical` and `family/datum.rs`'s `realize` are structural walks
  over data — reading a δ-builtin's argument, and building its answer — and each charges one nesting level per level of
  the data. A `List` of six hundred is six hundred deep, and `examples/staff-page.musa` peaks at 679 of 320 levels with
  a data charge standing at 672 of them. That is the size of one argument charged against a stack-safety guard.
  *(Measured by removal during this prompt: the data walk is worth 208 of those levels and the adapter's own recursion
  the other 471. Note 59 §5 carries the corrected reading; the second half below is still the fix for the 208, and 165a
  is the fix for the 471.)*

Fix both, and say in `02-core-calculus.md` §4 what each does to acceptance.

## Read

- [note 59](../../notes/research/language-design-closure/59-the-staff-wall-is-the-evaluators.md) in full — the method,
  the two tables, the per-definition tally, and the peak-nesting table. It is the evidence this prompt exists on, and
  its §6 is the four-item verdict this prompt takes two of.
- [`44-audit-against-smalltt-and-peyton-jones.md`](../../notes/research/language-design-closure/44-audit-against-smalltt-and-peyton-jones.md)
  §6's closing — the mechanism, recorded before the workload existed: "glued evaluation without graph update is laziness
  without memoization … a pure `Arc`-shared value has no thunk to overwrite."
- Peyton Jones ch. 12 §12.4 — the update of a shared redex's root with its result, which is what makes shared work
  happen once, and which is a separate idea from laziness. And `~/Code/Idris2/src/Core/Normalise/Eval.idr`'s `Glued`,
  whose normal-form side is a suspended host computation the runtime runs at most once. These are the two texts 165's
  Design already named for this verdict; what has changed is that the number is now in hand.
- Peyton Jones ch. 11 §11.6 and ch. 18 §18.8 — a reducer's control state as data it owns rather than as host frames, and
  the saved-frame mechanism, for the second half. Take the analysis and not the machine: musa's core is strict, finite,
  and total, and there is no graph to collect.
- `crates/musa-calculus/src/kernel/eval.rs` — `unfold`, `unfold_spine`, and `eliminate_replayed`, and the accounting
  paragraph above `unfold` that says why the replay is uncharged; `canonical` and `constructed`, the two halves of the
  data walk; and `Neutral` in `value.rs`, with its two constructors `head` and `eliminated`.
- `crates/musa-calculus/src/kernel/family/datum.rs` — `realize`, `realize_case`, `realize_count`, the inverse walk.
- `crates/musa-calculus/src/kernel/budget.rs` — `Metric::Nesting`'s own words ("how far inside itself an evaluation
  currently is"), `Budget::NESTING`'s doc and its derivation, and `Budget::LANGUAGE`'s doc, whose closing paragraph
  assigns the residual to prompt 166 and is what note 59 corrects.
- `docs/rules/language/02-core-calculus.md` §4 and §4.1 — the cost table, the three outcomes, "a value is charged once,
  where it is constructed", the nesting metric's stated motivation, the room obligation, and the amendment paragraph
  155a added. Both halves of this prompt are measured against those sentences.
- [`165`](165-diagnostics-and-performance.md)'s Design — the paragraph beginning "*Repaired before the prompt opens*",
  which states the charge question and the metavariable-safety question for the memo, and step 2, which is where the
  flat-region wall was first written down. This prompt is those, with the measurement attached.
- [`165a`](165a-explicit-control-stack.md) — *recursion* leaving the nesting metric. A different quantity in a different
  recursion; this prompt must not absorb it, and its Stop says so.
- [`54-the-nesting-limit.md`](../../notes/research/language-design-closure/54-the-nesting-limit.md) — why the limit is
  320 and what bounds it above. This prompt does not move the limit; it removes a quantity from what the limit counts.
- `crates/musa-calculus/src/kernel/room.rs` and `crates/musa-compiler/src/phase_budget.rs` — the two places that size a
  stack as `NESTING × FRAME_CEILING`. A walk that stops charging nesting and keeps using host frames is an abort §4.1
  forbids, so the second half owes an explicit stack and not only a removed charge.

## Design

**The memo is a graph update, and its unit is the neutral.** A folded neutral's unfolded value is a function of the
definition's value and the spine, both of which the neutral owns and neither of which changes. So the memo belongs on
the `Neutral` — one shared cell, filled by the first consumer that forces it, read by every later one — and it is shared
exactly where the value is shared, which is what makes it a graph update rather than a cache. `Neutral::head` starts a
fresh cell and `Neutral::eliminated` starts another, because an extended spine is a different value.

**The charge is the decision, and §4 already contains both readings.** A memoized unfold does the machine work once
where today every consumer pays it, so the meter would record less for the same program unless something is done. Two
honest options:

- *first consumer pays* — the meter records what the machine did, and a program that used to be refused at 200,000 and
  now fits is an acceptance change, argued in §4 as a cost-table version bump;
- *every consumer charged the recorded spend* — the meter prices the language's computation rather than the host's, the
  machine does the work once, and acceptance does not move at all. §4's "a value is charged once, where it is
  constructed" is the sentence this reading is checked against.

Measure both against the corpus before choosing, and record which programs flip under the first. The second is the one
that keeps §4 unamended, and a prompt that can deliver a five-orders-of-magnitude speedup without an acceptance change
should have to argue for taking the other.

**The memo is unsound over an unsolved metavariable.** A spine or a result mentioning one may unfold to a different
answer once the solution arrives, so those must not be filled. The expansion run this exists for is post-elaboration and
meta-free, so a conservative meta-freedom test is sound and cheap; a test that is *wrong* in the safe direction only
loses the memo. Say what the test is, and pin it with a law: solving a metavariable after a folded value has been forced
must not change what the value unfolds to.

**Data descent is not evaluation depth, and the metric's own words say so.** `Metric::Nesting` reads "how far inside
itself an evaluation currently is" and its derivation in §4.1 is about a term a composer wrote and a value NbE walks
back. Reading a six-hundred-element list into a `Datum` so a δ-rule can look at it is neither: the depth is the length
of one argument. Stop charging nesting there. What replaces it is the step budget, which is what already prices "how
much work" and which a long argument genuinely costs.

**Removing the charge without removing the frames is an abort, and §4.1 forbids aborts.** `canonical`/`constructed` and
`realize`/`realize_case` recurse on the host stack, and today the nesting limit is the only thing that stops a long list
from overflowing it. So the pair get an explicit work stack — the analysis of Peyton Jones ch. 11 §11.6, on two
functions rather than on the evaluator — and the charge moves to steps in the same change. One without the other is not
half of this prompt; it is a regression.

**What must not move.** `quote`'s descent over a value and the traversal's descent through a term are structural, are
what §4.1 derives the metric from, and keep charging nesting on host frames. Recursion is 165a's. The step budget and
the nesting limit are the numbers they are; this prompt changes what is counted, not what the count is compared to.

**The staff class is the workload, and it is the honest way to state the result.** After both halves,
`examples/staff-page.musa` expands in 381,055 steps against 200,000 — still red, and *correctly* red, because closing
the remaining factor of 1.9 is what prompt 166's rewrite is for. This prompt does not promise a green staff class.

*Repaired mid-implementation, by the second half's own measurement.* This paragraph used to promise more than that: that
every failure in the class would report a step count rather than a nesting level. It does not, and the reason is a
misreading of note 59 §5 that implementing the second half caught. `Meter::nested` charges one shared counter, so a peak
recorded under an operation is the combined depth at that moment and not that operation's own contribution; reading
`canonical data`'s 672 as 672 levels of data was reading a total as a part. Measured by removal, the data walk is worth
208 of the 679 levels and **471 are the adapter's own recursion**, at about seven nesting levels per call. So after this
prompt the class is still refused at `nested evaluation levels` — at 471 of 320 rather than 679 of 320 — and what closes
that is [165a](165a-explicit-control-stack.md), whose `depends_on` note 59 §6 moves in front of 166 for exactly this
reason. Note 59 §5 and §6 carry the corrected reading, taken with this prompt's code in the tree.

## Target

- The memo on `Neutral` in `crates/musa-calculus/src/kernel/value.rs`, filled and read in `eval.rs`'s `unfold`,
  doc-commented with the invariant it encodes and with the metavariable condition that keeps it sound.
- The charge decision, made on a measurement over the corpus, with the programs that flip under each reading listed —
  and, if the first reading is taken, the version bump argued in `docs/rules/language/02-core-calculus.md` §4.
- A law in `budget_laws.rs` pinning the memo's semantics: a value forced twice answers the same thing, and forcing a
  value that mentions an unsolved metavariable and then solving it answers the *new* thing.
- `canonical`/`constructed` and `realize`/`realize_case` walking data with an explicit work stack, charged steps rather
  than nesting, doc-commented against the sentence of `Metric::Nesting` they no longer answer to.
- A law pinning that: data of a depth far past `Budget::NESTING` is read and built without a nesting refusal and without
  an abort, on the default 2 MiB test thread; and the step charge for that walk is what the frame-based reading charged,
  on a case small enough to count by hand.
- Note 59 §5 and §6 repaired with the shared-counter reading and the measured decomposition, and 165a and 166 reordered
  to match. *(Landed ahead of the rest as its own repair commit, per the operating procedure.)*
- `Budget::LANGUAGE`'s doc comment repaired: its closing paragraph assigns the staff residual to prompt 166, which note
  59 measures as wrong, and the numbers in it are re-taken after this prompt.
- `Metric::Nesting`'s doc and `02-core-calculus.md` §4.1 saying which descents the metric counts, now that one has left.
- Note 59 closed with the after-measurement: the four regions and `examples/staff-page.musa`, re-run.
- `docs/plan/code-map/` rows for whatever moved.

## Check

```sh
cargo build --workspace
env -u RUST_MIN_STACK cargo nextest run --workspace   # the default 2 MiB thread is the point of this one
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-compiler -- p1_compile p2_elaborate
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

`--run-ignored all` still carries the staff budget class, and that is the expected result rather than a failure of this
prompt — see the Design's last paragraph, which was repaired mid-implementation and now states what is actually owed:
**the nesting peak on `examples/staff-page.musa` falls from 679 to 471, no charge in it is made by a data walk, and the
count of failures does not grow.** The class is still refused at `nested evaluation levels`, from the adapter's own
recursion, and 165a is what closes that. Record the peak, so 165a is measured against it.

P1 and P2 move here, and they should move downwards. Report them against `06-elaboration-baseline.md`'s baseline under
its 10% gate; a memo that makes the pipeline *slower* has been built wrong and the number says so.

The oracle stays fixed: memoizing a computation and re-charging a walk change no value, so no semantic hash, no
diagnostic code, and no rendered corpus file moves. If one does, stop and report it.

Commit as `Memoize the unfold and take data descent off the nesting metric`.

## Stop

- No change to `Budget::LANGUAGE`'s step count or to `Budget::NESTING`. This prompt changes what is counted; the limits
  are 165's and 165a's to re-derive.
- No explicit control stack for recursion, and no change to what a case tree means.
  [165a](165a-explicit-control-stack.md).
- No explicit stack for the *elaborator*'s `check`/`infer`. [165](165-diagnostics-and-performance.md).
- No laziness, no thunks in the source language, no garbage collector. The memo is one cell on a value that already
  exists, and nothing above the kernel may observe it.
- No adapter rewrite and no library change. [166](166-staff-rewrite.md) and [167](167-studio-rewrite.md), and this
  prompt's whole claim is that the adapter did not need one to get this far.
- No diagnostics work. 165 owns the vocabulary; this prompt may not invent a refusal or reword one.
- No `RUST_MIN_STACK`, in a test command, a `.cargo/config.toml`, a CI file, or a doc.
