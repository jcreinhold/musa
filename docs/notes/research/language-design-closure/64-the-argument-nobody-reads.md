# 64. The argument nobody reads

**Status: governs nothing.** `../../../plan/prompts/165-diagnostics-and-performance.md` is the prompt. This page is the
attribution that found the defect, the one-line reason it is a defect and not a tuning knob, the numbers before and
after, and the re-derivation of the step limit that follows from them. Debug builds, `cargo nextest`, step counts read
off a temporary probe in `Meter::step` that was removed before the commit; the counts are reproducible without it by
bisecting `Budget::LANGUAGE.steps` against a workload, which is how the headline number below was confirmed.

## 1. The defect

`Elaborator::supplied_spine` walks an application's arguments against the function's Π type. At each argument it did

```rust
let term = self.check(scope, argument, &domain)?;
let value = scope.eval(&mut self.meter, &term)?;
ty = apply_closure(&mut self.meter, &codomain, value)?;
```

and the `eval` is there for one reason: `apply_closure` needs something to open the codomain at. But `check` has already
walked the whole of `argument`; evaluating the term it produced walks it again. For a nested application the inner
argument is walked once by its own `check`, then again by its parent's `eval`, then again by *its* parent's, so a tree
of size `n` is charged the sum of its subtree sizes rather than `n`.

Attribution on a voice of 400 plain notes, before: **281,264 of 367,097 steps — 76.6% of the run — inside that one
`eval`.** That alone would only say the line is hot. What makes it a defect is the second measurement: of those 281,264
steps, **every one stood at a codomain that never reads its binder.**

A Π whose codomain does not mention its binder *is the ordinary non-dependent arrow*. `A → B` says nothing about the `A`
it was handed, so the argument's value is unobservable at that codomain and computing it is work with no reader. The
change is therefore to ask the closure first:

```rust
ty = apply_closure_read(&mut self.meter, &codomain, |meter| value.read(scope, meter))?;
```

where `apply_closure_read` opens the closure at `Value::unread()` when `Closure::reads_its_binder()` is false, and calls
the thunk otherwise. The predicate is a de Bruijn scan of the closure body for index 0, and it answers `true` at a
metavariable: §2.1 writes an unknown `?m[σ]` with its spine read out of the *environment* rather than carried in the
term, so a body containing an unsolved unknown may read every binder in scope. Conservative there is the whole of the
soundness argument.

This is one-directional. A program accepted before is accepted after with the same value — the skipped evaluations
produced values nothing looked at — and a program refused for a reason other than the budget is refused the same way.

## 2. The bound the removal took away, and putting it back

Three suite laws went red on the change, and one of them was the finding: `numeral_laws`'s tower of 322 nested
applications elaborated inside 320 nesting levels, when the law exists to say it does not. The nesting counter had been
riding on the argument evaluation. Take the evaluation away and the counter stops seeing the nesting the *elaborator*
does.

§4.1 charges nesting "wherever an evaluation can stand inside another one". `check` and `infer` stand inside one another
at every written node — 516 `infer` frames deep on a workload whose nesting reading was 2 — and were charged nothing.
Charging them one level each restores the bound, and it is the honest charge rather than a patch: the elaborator's two
judgments are exactly the recursion §4.1 describes.

What it costs is measurable and small, and it is a cost-table version bump because the recorded number moves. The
structural minimum for `a_definition_recursing_far_past_the_nesting_limit_is_accepted` was 3 levels and is **11**, and
it is the same 11 at one call, ten, a hundred and three thousand — a constant of the term's *type*, not of how far the
recursion went, which is what that law says and what did not change.

## 3. The numbers

| workload | before | after |  |
| --- | ---: | ---: | ---: |
| a voice of 400 plain notes | 367,097 | 85,833 | 4.3× |
| `examples/diatonic-sequences.musa` | 206,041 | 24,661 | 8.4× |
| `tests/fixtures/analysis-pressure.musa` | 444,113 | 107,952 | 4.1× |
| `tests/fixtures/large-score.musa` | 1,674,615 | 355,992 | 4.7× |
| `examples/staff-page.musa` | 180,873 | 180,873 | 1.0× |

`staff-page` is the row worth reading. The adapter's cost is a syntax walk — note 59 §3 decomposes it — and a walk is
not an application spine, so nothing here touches it. A change that improved every row would have been suspicious.

`diatonic-sequences` is the row that closes a class. It ran past the step budget with no adapter involved at all, which
is why `document/laws.rs`'s `BUDGET_WALL` had a tonal entry in it; the list is now empty, and `every_example_elaborates`
asserts the corpus reads, checks, and fits.

Where the remaining 355,992 go, on two axes. By elaborator frame, self-exclusive — a frame is charged only what it spent
outside its children:

| frame | steps |
| --- | ---: |
| `infer` at a `Let` | 92,039 |
| `check` at an application | 63,837 |
| `check` at a hosted value | 59,339 |
| `infer` at a variable | 44,034 |
| `infer` at an application | 38,756 |
| `check` at a literal | 25,522 |

and by charged operation, which partitions the same total differently: evaluation 222,463, unification 66,626, the rest
spread across quotation and the data walk. No second site of the first kind — one place doing work with no reader —
turned up behind them. What is left looks like a dependent checker doing its job.

## 4. The step limit is not re-derived, and the reason is a second defect

§4 said the defaults were open: "the course correction re-derives them against the simplified checker and records the
derivation in its final report." That derivation was attempted here and **must not land yet**. It is recorded because
the reason is a defect, not a preference.

`tests/fixtures/large-score.musa` is the fixture the desktop's frame budgets are measured against, and its generator law
is called `large_score_is_the_size_the_budgets_assume`. It is 1,500 plain notated events across four parts and 100 bars,
plus a 56-event coda that exercises ties, tuplets, chords and dynamics — 1,556 events, **355,992 steps, 229 steps an
event**, confirmed exactly by bisection: it compiles at a limit of 355,992 and exhausts at 355,991. 200,000 admits 873
events, and the specification it serves is written against 1,500. So the arithmetic says raise the limit.

**Raising it aborts the process on a deep region, and that is the finding.** Setting `Budget::LANGUAGE.steps` to
1,000,000 and expanding a region 2,000 groups deep ends in `fatal runtime error: stack overflow`, with memory climbing
until the host is unusable. §4.1 forbids exactly that outcome, and `expand::tests`'s
`a_region_deeper_than_the_budget_allows_is_refused_rather_than_fatal` exists to prevent it. It did not, because **the
nesting metric does not see a region's depth at all** — measured directly, with `Budget::NESTING` temporarily narrowed
to 40, a region 100 groups deep still expands. What bounds region depth today is the *step* budget and nothing else:

| region depth | expansion steps |
| ---: | ---: |
| 50 | 17,270 |
| 100 | 31,820 |

which is about 290 steps a level. At 200,000 that caps a region at roughly 680 levels, which the 32 MiB
`NESTING × FRAME_CEILING` thread survives. At 1,000,000 it caps it at roughly 3,440, and the stack goes at about 2,000.
The step limit was the depth guard, accidentally, and raising it removes the guard.

`phase_budget::NESTING`'s doc claim — "the limit that turns a stack overflow into a refusal … 256 admits regions nested
some sixty deep" — is therefore false today, and so is `with_room`'s "the room is `NESTING × FRAME_CEILING`, derived
rather than picked, so raising the published limit cannot quietly outrun the stack that honours it." Two constants have
also drifted apart: `Budget::NESTING` is 320 and `phase_budget::NESTING` is 256, so the room is sized for a limit 25%
below the one the meter enforces. `Budget::NESTING`'s own doc comment records the symptom without naming it — the
adapter's peak is "the same 62 for a region of nothing, of one item, of three, of four … a number that is the depth of
the adapter's own source and does not move with what it reads."

**So the step limit stays at 200,000 until region depth is bounded by something other than it.** That is prompt 165's
Design step 4 — "`Budget::NESTING`'s doc claim ends this prompt re-earned or corrected" — and correcting it is what
unblocks the re-derivation above. The two large-score generator laws stay red meanwhile, and they are red for the right
reason: they are the measurement saying the published table and the desktop specification disagree.

## 5. Three host recursions over a region's depth, two of them now gone

The abort above was chased to its sites, on a deliberately small (256 KiB) thread so that each walk could be run alone
and cheaply. Building a region is a loop and is fine; dropping one recurses at about 64 bytes a level and is fine in
practice. Three walks were not.

| Walk | Cost a level | Died on 256 KiB at | Status |
| --- | ---: | ---: | --- |
| `quote::gate`'s `walk`, under `check_expression` | ~500 B | ~500 | **loop now** |
| `Syntax::clone`, derived | ~850 B | ~300 | **loop now** |
| the traversal's own descent, in evaluation | ~16 KiB | — | open |

`check_expression` carried a second defect and it is the one that froze a host rather than aborting it: `built` and
`binders` were `Vec`s scanned with `contains` for every node, and each comparison walks a path, so the gate cost the
square of a region's node count times its depth. Both are `HashSet`s now. With those two changed, a region 4,000 groups
deep passes the gate and clones instantly on a 256 KiB thread, and expansion is linear again — 0.43 s at depth 400, 0.65
s at 600, against a run that took minutes and climbed in memory before.

The third is the one §4.1 is actually about, and it is still open. Isolated by swapping the transformer's group branch
for one that reads nothing (`NONE_AT_ALL`): a region 2,000 deep then costs 1,394 steps and recurses not at all, where
the reading branch (`EACH_ONCE`) aborts. So the frames are the traversal's descent, at about 16 KiB a level — and the
counter does not see them. Narrowing `Budget::NESTING` to 40 proves both halves at once: a region 100 groups deep still
*expands* (so the metric is not charged per level), and a region 300 deep *overflows* (so the room, which is derived
from that same constant, is being spent per level at ~16 KiB). `NESTING × FRAME_CEILING` is therefore not a derivation
for this workload; it multiplies a constant nobody charges by a ceiling measured against a different descent.

§4.1 says what the two ways out are and prices them: charge the descent, or remove the frames. Charging it bounds region
depth at 320 and lets the step limit rise freely, at the cost of refusing regions between 320 and the ~680 that 200,000
steps admits today — an acceptance narrowing, so a cost-table version bump with a stated reason. Removing the frames is
prompt 165a's control-stack treatment pointed at the traversal, and narrows nothing. Either is a change to the evaluator
with its own measurement, and neither belongs in the same commit as the two walks above.

## 6. One more repair that did land: the depth fixture was malformed

`expand::tests`'s `nested_region` built every level at the region's *root* path, so the transformer wrote every one of
its answers to the same path and a deep region met `NotAnExpression(DuplicatePath)` rather than the limit the law exists
to reach. It went unnoticed for as long as the step budget tripped first. The builder now derives each level's path the
way reading a real region would, and lives in `quote::build` where deriving a child path belongs.
