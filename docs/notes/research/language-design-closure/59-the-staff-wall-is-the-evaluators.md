# 59. The staff adapter's two walls are the evaluator's, measured before the rewrite

**Status: governs nothing.** `../../../rules/language/02-core-calculus.md` §4 holds the budget and
`../../../plan/prompts/` holds the ordering; this page holds the measurement that repaired prompt 166 and cut prompt
165b out of prompt 165. Taken at prompt 166, on a clean tree at `104d6bf9`, before a line of the rewrite was written.

Prompt 166 was written on a claim: the thirty staff budget failures are the adapter's algorithm, and rewriting
`stdlib/src/adapters/staff.musa` on the new language is what turns them green. Its Check states it twice — "this prompt
owes *both*: an adapter whose expansion is shallow enough not to stand 257 evaluator frames deep, and one whose step
count is inside 200,000" — and `budget.rs`'s own doc says the same thing from the other side: "The residual is the
adapter's algorithm and prompt 166's to remove."

The claim is false, and one measurement is enough to see it: **a staff region with nothing in it costs 455,942 reduction
steps against a budget of 200,000.** There is no algorithm there to remove.

## 1. The method

Four probes on top of a clean tree, all reverted afterwards and none of them a change this note proposes keeping:

- an `eprintln!` on `Unrun::Stopped`'s `ElabError::Exhausted`, which is where the expansion's `ResourceError` is
  otherwise discarded in favour of "crossed a compilation limit";
- the two `Spend`s in `phase/transform.rs` — the adapter module's declaration and the `expand` run — printed rather than
  only summed;
- a tally in `kernel/eval.rs`'s `unfold` and `matched`, keyed by the definition's name, dumped at the end of the run;
- a peak-level tally in `Meter::nested`, keyed by the operation string the charge is made under.

The workloads are four `.musa` pieces holding a `syntax staff { … }` region of 0, 1, 3, and 4 items, and
`examples/staff-page.musa`, which is 77 lines and the corpus's own staff workload.

## 2. What it costs today

`Budget::LANGUAGE` is 200,000 steps and `Budget::NESTING` is 320.

| region | `expand` run, steps |
| --- | --- |
| nothing at all | 455,942 |
| `clef treble` | > 20,000,000 |
| three header words | > 200,000,000 |
| `examples/staff-page.musa` | > 4×10⁹ (`budget.rs`'s own recorded probe) |

Declaring the adapter module costs a further 84,937, which is charged separately and is not the wall.

The tally says where it goes. For the one-item region, against a region of some eight nodes:

| definition | unfoldings |
| --- | --- |
| `nothing_pending` | 327,850 |
| `nothing_read` | 200,289 |
| `no_head` | 109,254 |
| `pending_of` | 9,225 |
| `token_read` | 9,118 |
| `run_syntax_step` | 9,118 |

`run_syntax_step` runs once per node of the region. It ran 9,118 times for eight nodes. The traversal is not doing too
much work per node; it is being re-run about a thousand times per node, and each re-run rebuilds the empty reader from
scratch — `nothing_pending` is written at four sites in the whole file and `no_head` is a nullary `let`.

## 3. Why: δ has no graph update

This is note [`44`](44-audit-against-smalltt-and-peyton-jones.md) §6's closing, seen from the workload it predicted.
`kernel/eval.rs`'s `unfold` replays the spine over a definition's value and returns the result without recording it
anywhere; the neutral it was asked about is `Arc`-shared, so every consumer that forces it pays the whole replay again.
Where the replayed body itself ends in folded applications — a record whose fields are `nothing_read(here)`, whose
fields are `no_head` — the cost multiplies down the chain, and the exponent is the depth of the adapter's data, not the
size of the composer's region. That is why the empty region already costs twice the budget and one word costs forty
times more than that.

Peyton Jones ch. 12 §12.4 is the reference point, and it is not about laziness: what makes shared work happen once is
that the root of a shared redex is *physically overwritten* with its result. smalltt gets that from GHC's thunks and
Idris2's `Glued` from a suspended host computation; a strict pure evaluator over `Arc` has to build it or decline it.

## 4. The same tree, with the unfold memoized

One probe: an `Arc<OnceLock<Value>>` on `Neutral`, fresh at `Neutral::head` and at `Neutral::eliminated`, read and
filled in `unfold`. Nothing else changed — **the same adapter, unrewritten, with all twenty `callN` call sites and all
fifty-five `syntax_built` calls still in it — and the same 200,000-step budget.**

| region | before | after |
| --- | --- | --- |
| nothing at all | 455,942 | 2,796 |
| `clef treble` | > 20,000,000 | 10,168 |
| three header words | > 200,000,000 | 28,406 |
| three header words and a bar of one note | — | 49,010 |
| `examples/staff-page.musa` | > 4×10⁹ | 381,055 |

Roughly 2,800 steps of fixed cost and 9,000 an item, straight-line across the range.

With that memo in place and `Budget::NESTING` raised, **23 of the 24 `staff_expansion_laws` pass at the unchanged
200,000-step budget against the unrewritten adapter.** The twenty-fourth,
`a_tuplet_that_plays_nothing_in_the_time_of_some_is_refused_at_the_tuplet`, reports `NotATransformer` rather than a
resource limit, which is a separate question and is not a budget failure.

## 5. The second wall is data descent, not evaluator descent

With the memo in place, `examples/staff-page.musa`'s peak nesting, by the operation each charge is made under:

| operation | peak level |
| --- | --- |
| `evaluation` | 679 |
| `data realization` | 672 |
| `canonical data` | 672 |
| `quotation` | 54 |
| `neutral typing` | 54 |
| `re-checking` | 48 |
| `unification` | 4 |

Six hundred and seventy-two of the six hundred and seventy-nine levels are `canonical data`: `eval.rs` charges one
nesting level per level of *data* structure, and the emitted expression is a right-nested list some six hundred long.
Nothing an adapter does changes that number except emitting less music. This is exactly the reading prompt 165's own
note proposed — "`Metric::Nesting` is documented as 'how far inside itself an evaluation currently is', a *stack-safety*
guard, and `eval.rs`'s `canonical` charges it once per level of **data** structure, so a long right-nested value costs
nesting the way deep recursion does. Whether those are the same quantity is this prompt's measurement to make" — and it
is the answer to the question that note asked prompt 166 to settle: the wall was not the adapter's shape.

## 6. What follows for the plan

Three things, and the third is the one prompt 166 keeps.

1. **The unfold memo is not an optimization to be weighed against its complexity; it is what makes the adapter path run
   at all.** Prompt 165's Design assigns the verdict to itself and prices it as a judgement call between a memo and the
   numbers. The numbers are above, and they are five orders of magnitude on the workload that the pass's own acceptance
   gate is written against.
2. **The nesting metric must distinguish evaluator descent from data descent**, or the staff class cannot go green at
   any adapter length. 165 names this too, as the repair to make "if the staff rewrite does not make `core-pressure`
   green". It does not, and could not: the charge is not about the adapter.
3. **The rewrite is still load-bearing, and now it has a real target.** After the memo, `examples/staff-page.musa`'s
   expansion costs 381,055 steps against 200,000 — a factor of 1.9, not 20,000. That is a number a shorter adapter can
   close, and closing it is prompt 166's, measured against the budget that does not move.

Both of (1) and (2) are prompt 165's by ownership and neither can wait for it, because 165 follows 164, 164 follows 166,
and 166 cannot be *developed* — let alone measured — while every compile of the file under rewrite dies at a resource
limit. They are cut out into prompt [`165b`](../../../plan/prompts/165b-graph-update-and-data-descent.md), which runs
before 166 and leaves 165 its diagnostics, its P1/P2 gate, and the four smalltt items.
