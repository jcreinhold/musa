# 54. The nesting limit, and the two things it counts

**Status: governs nothing.** `../../../rules/language/02-core-calculus.md` §4 and §4.1 hold the decision; this page
holds the argument behind the one time the limit moved. Written after prompt 155a measured it.

## 1. What the metric was derived to bound

§4.1 states the metric's motivation in two clauses, and both are about how deeply a *term* is written:

- §5.9's traversal descends through a transformer's own branches, so one level of a region's nesting costs a whole chain
  of evaluator frames rather than one.
- NbE's `quote` adds a second such descent, over a value rather than a region.

Neither clause mentions recursion, and the counter charges it anyway. That is the fact this page is about. A structural
recursion holds one level per step for the duration of the steps beneath it, because the recursive call is evaluated
inside the enclosing evaluation and its level is only released on the way back out. So a single number decides two
unrelated questions: how deeply a composer may write a term, and how many times a definition may call itself.

**This is not new, and 155a did not introduce it.** `crates/musa-compiler/tests/suite/resource_validation.rs`'s module
doc has recorded it since prompt 142, in the vocabulary of the evaluator that existed then: `range` is
`stdlib/src/list.musa`'s `counting_from`, one recursive call per element, and a recursive call cost about 2.5 nesting
levels through the generated recursor, so the 256-level limit refused every list past about a hundred elements. Nesting
was already the first limit a large value met, and §4's node and byte limits were already unreachable behind it.

## 2. What 155a changed, measured

**Derived**, by bisecting `Budget::NESTING` and re-running the corpus. Two measurements, on two different shapes:

| Shape | Body form | Cost |
| --- | --- | --- |
| `range(n)` — `counting_from`, one recursive call per element | generated recursor (prompt 142) | ≈ 2.5 levels per call |
| `range(n)` — the same source, unchanged | compiled case tree (155a) | ≈ 3.0 levels per call |
| The staff adapter, as `staff_package_laws::engrave_*` exercises it | generated recursor | peak of at most 240 levels |
| The same, unchanged | compiled case tree | peak of at most 272 levels |

The `range` figure is bisected directly: at a 320-level limit the largest list that compiles is between 105 and 109
elements. The staff figure is bisected on the limit itself, and 272 is the number that matters, because 256 is what the
limit was.

So 155a's case tree costs about twenty per cent more nesting per recursive step than the recursor it replaced, and that
was enough to push the deepest workload in the corpus over a limit it had been sitting just under. The pre-155a peak of
240 out of 256 was a margin of sixteen levels; the corpus had been running at ninety-four per cent of the limit and
nobody had said so.

Two mitigations landed inside 155a's own boundary and are worth naming because they bound the damage:

- A split whose scrutinee is a bare variable reads the environment instead of calling `eval`, which costs no level.
  Worth about eight levels on the staff path.
- A saturated self-call in tail position is trampolined in `Compiled::reduce` rather than evaluated, charging by hand
  the exact steps the whole-term evaluation would have charged, in the same order. Tail recursion is consequently free
  in nesting at any depth.

Neither helps a non-tail recursion, which is the shape a fold is written in.

## 3. The decision

**Judged.** The limit moves from 256 to 320.

The number is bounded on both sides, and the interval is narrow enough that the choice is nearly forced:

- **Above 272**, because that is what the corpus measures. Anything at or below it refuses the standard library's staff
  adapter, which would make the corpus the thing that moves rather than the constant.
- **At or below 362**, because the nesting limit has to be *reachable*. Bisected: at a limit of 362 a tower of 363
  constructors is refused for nesting, and at 363 the same tower is refused for steps instead — the language's 200,000
  step budget cannot afford to build a term any deeper than that. A nesting limit above 362 would be a limit no term can
  reach through pure depth, and `budget_laws::elaborating_a_term_nested_past_the_limit_is_refused` and
  `numeral_laws::a_tower_past_the_nesting_limit_exhausts_where_the_numeral_answers` — the two laws that hold §4.1's room
  obligation to account — could no longer be stated at the language budget at all.

320 sits in the middle of that interval: forty-eight levels above what the corpus needs, which is three times the margin
256 gave it, and forty-two below the point where the metric stops being reachable. It costs 10 MiB of scoped-thread
stack under §4.1's room obligation rather than 8, at the 32 KiB frame ceiling.

Chosen against: **512**, which was the first proposal and is wrong for a reason worth recording — it is past the
reachability ceiling, so the two room laws fail there, and the failure is not a stale assertion but the metric going
quiet on the term-depth path it was derived for. And chosen against **leaving it at 256**, which refuses a standard
library file that compiles today.

**This does not repair the conflation.** At 320 the deepest recursion a definition may perform is about a hundred steps,
and the limit still answers two questions with one number. A composer who writes a genuinely deep term and a composer
who writes a hundred-element list are refused by the same counter for unrelated reasons, and neither diagnostic can say
which.

## 4. What actually retires it

**Judged.** An evaluator with an explicit control stack — a work list of pending continuations in the evaluator's own
state rather than a chain of Rust frames. Peyton Jones ch. 11 §11.6 and ch. 12 argue the direction: the pending
applications a reducer is in the middle of are a stack of pointers in the machine's own store, and ch. 18 §18.8's *dump*
is where a nested evaluation's saved state goes so that it can return without a host call. Musa needs that analysis and
not the machine, because it has neither laziness nor unbounded computation to manage. With the control stack explicit:

- Recursion depth is bounded by the step budget, which is the counter that already measures work done and never returns
  — and which a recursion genuinely spends.
- The nesting metric goes back to bounding term structure, which is what §4.1 derived it for, and its limit can be
  argued from `quote`'s descent alone.
- The room obligation stops scaling with how a composer wrote a fold.

That is a change in what the language accepts, in both directions, and it is prompt 165a's to argue. Until it lands, 320
is the number.

## 5. Closed: what held, at prompt 165a

The control stack landed. Measured on the five workloads note [`59`](59-the-staff-wall-is-the-evaluators.md) recorded —
a `syntax staff` region of 0, 1, 3 and 4 items, and `examples/staff-page.musa` — and on `rec add : Nat → Nat → Nat`,
which is the smallest definition that recurses once per unit of its argument.

### The profile the change was made on

§3's prediction was that the growth is in reducing a compiled case tree. It is not, and this is the one place the note
was wrong. Dumped at the wall, the cycle is ι firing on a *generated recursor* together with the δ-unfold replaying its
spine: `eval → application → apply → applying → iota → ready → opened → unfold → unfold_spine → eliminate_replayed →
applying → apply_closure → eval`, about seven nesting levels and thirty host frames per recursive call.
`Compiled::reduce` sits inside that cycle rather than being it. What the profile separates:

| frames | grow with | after 165a |
| --- | --- | --- |
| the evaluator's own descent into a term | recursion depth *and* term depth, indistinguishably | explicit data; term depth only, and only within one term |
| ι, the δ-unfold replay, the case tree's reduction | recursion depth | explicit data; charged steps |
| §5.9's traversal | region depth | host frames, charged nesting — unchanged, and now the deepest thing the room has to hold |
| `quote` | value depth | host frames, charged nesting — unchanged |
| the elaborator's `check`/`infer` | raw-term depth | host frames, charged *nothing* — prompt 165's, unchanged |
| a value's destructor | value depth | explicit data — see below |

### The predictions

§3's, in order:

- **"Above 272, because that is what the corpus measures."** Held while recursion was charged, and is now the wrong
  measurement of the corpus. The staff adapter peaks at **62** levels, and at the same 62 for every one of the five
  workloads — the number is the depth of the adapter's own source and does not move with what it reads. 272 was almost
  entirely the recursion.
- **"At or below 362, because the limit has to be reachable."** Held, and still holds: the reachability argument is
  about term depth through `quote` and the elaborator, which this prompt did not touch.
- **"This does not repair the conflation."** Held, and 165a repairs it. `add 105 0` answered and `add 106 0` was refused
  at 321 of 320; now `add 3000 0` answers *inside eight nesting levels* — three, measured, and what refuses at two is
  unification rather than the recursion — and the last count that answers is 16,665, refused at 200,001 of 200,000
  steps.

§4's, in order:

- **"Recursion depth is bounded by the step budget."** Held. `add n 0` costs `9 + 12n` steps, and the constant is
  measured against the frame-based evaluator at `3846ec72` rather than derived from the new one: 21, 33, 129 and 1,209
  steps for 1, 2, 10 and 100 calls, byte for byte the same before and after. The explicit stack charges what the frames
  charged.
- **"The nesting metric goes back to bounding term structure."** Held.
- **"Its limit can be argued from `quote`'s descent alone."** **Did not hold, and the limit stays at 320.** The corpus
  argues for something five times smaller, but the limit is not only a cost-table entry: §4.1 derives the *room* from
  it, and the elaborator's `check`/`infer` recursion is still charged nothing, so this counter is the only thing keeping
  a deeply written term a refusal instead of an abort. Lowering the limit lowers that backstop's stack in step. Three
  laws say so by failing —`budget_laws::elaborating_a_term_nested_past_the_limit_is_refused` first among them. Charging
  the elaborator is prompt 165's, and it is what makes a smaller limit arguable.
- **"The room obligation stops scaling with how a composer wrote a fold."** Held, and the ceiling went *up* rather than
  down. With the evaluator's cheap frames gone, a level is bought by the traversal, by `quote`, and by the uncharged
  elaborator descent, so it costs between 60 and 64 KiB in a debug build where it used to cost about 10, and between 8
  and 16 KiB in a release build. The frame ceiling moved from 32 KiB to 128 KiB and the reserved room from 10 MiB to 40
  MiB. Room is non-normative, so this is free; it is recorded because the number reads backwards otherwise.

### What the profile found that no prediction named

**A value's destructor.** `resource_validation.rs` has recorded since prompt 142 that a fold past about 1,256 elements
aborts the process and that "some descent proportional to the count is uncharged". That descent is Rust's derived
`Drop`: a `Value` is a tree of `Arc`s and freeing one is one host frame per level, with no musa frame on the stack for
any charge to see. It was invisible while nesting refused every list past about sixty; taking recursion off the metric
made those lists compile, and `range(4000)` compiled while `range(4500)` aborted in `drop_glue<Value>`. A widening paid
for with a crash is not a widening, so reclamation became explicit data in the same change — Peyton Jones ch. 17 §17.2's
observation that reclamation is a traversal, with no collector and no change to what owns what.
