# 54. The nesting limit, and what a case-tree body did to it

**Status: governs nothing.** `../../../rules/language/02-core-calculus.md` §4 and §4.1 hold the decision; this page
holds the argument behind the one time the limit moved. Written after prompt 155a measured it.

## 1. What the metric was derived to bound

§4.1 states the metric's motivation in two clauses, and both are about how deeply a *term* is written:

- §5.9's traversal descends through a transformer's own branches, so one level of a region's nesting costs a whole chain
  of evaluator frames rather than one.
- NbE's `quote` adds a second such descent, over a value rather than a region.

Neither clause mentions recursion, and that is not an omission. **Derived.** Before prompt 155a, the only recursion a
checked program could perform was the one the generated eliminator performed, and ι computes a method's induction
hypothesis *before* entering the method, through an application (`kernel::eval::apply`) that charges no nesting level. A
recursion of any depth therefore stood at a constant nesting depth. 256 bounded how deeply a composer had *written* a
term, and a program that recursed ten thousand times was refused, if at all, by the step budget — which is the counter
that measures work done and never returns, and is the right counter for the job.

## 2. What 155a changed

**Derived.** A definition whose body is a compiled case tree evaluates its recursive call *inside* the enclosing `eval`,
because the call is a subterm of the alternative's body rather than a value handed to a method from outside. Each
non-tail recursive step therefore holds one nesting level for the duration of the steps beneath it, and the metric now
bounds recursion depth as well as term depth — two quantities with nothing in common except that both consume host
stack.

Two mitigations landed inside 155a's boundary and are worth naming because they bound the damage:

- A split whose scrutinee is a bare variable reads the environment instead of calling `eval`, which costs no level.
  Worth about eight levels on the staff path.
- A saturated self-call in tail position is trampolined in `Compiled::reduce` rather than evaluated, charging by hand
  the exact steps the whole-term evaluation would have charged, in the same order. Tail recursion is consequently free
  in nesting at any depth.

Neither helps a non-tail recursion, which is the shape the staff adapter's `pare`-style folds are written in.

## 3. The measurement

**Derived**, by bisecting `Budget::NESTING` and re-running the corpus. The deepest workload is the standard library's
staff adapter as exercised by `staff_package_laws::engrave_*`:

| Body form | Peak nesting on the staff path |
| --- | --- |
| Generated eliminator (before 155a) | more than 236, at most 240 |
| Compiled case tree (155a) | more than 264, at most 272 |

The 32-level increase is the adapter's own non-tail recursion depth, previously charged nothing.

## 4. The decision, and what it is not

**Judged.** The limit moves from 256 to 512.

- It is past the measured requirement of 272 with the same kind of margin 256 carried over the pre-155a 240: the corpus
  reaches a little over half the limit either way.
- It costs stack rather than acceptance. `kernel::room` reserves `NESTING × FRAME_CEILING` bytes on a scoped thread, so
  at a 32 KiB ceiling the reservation goes from 8 MiB to 16 MiB. That is a reservation, not a commitment, and it is
  affordable on every host musa builds for.
- It changes which programs are accepted, which is why it is recorded here rather than done quietly. A program refused
  at 256 levels and accepted at 512 is a program two compilers disagree about; there is exactly one such disagreement
  and it is this one.

**This does not repair the conflation.** At 512 the deepest non-tail recursion a definition may perform is about 511
steps, which is one cliff further out rather than no cliff. It also leaves the metric measuring two different things, so
a future composer who writes a genuinely deep term and a future composer who writes a genuinely deep recursion are
refused by the same number for unrelated reasons, and neither diagnostic can say which.

Chosen against: leaving the limit at 256 and refusing the staff adapter, which would make the corpus the thing that
moves rather than the constant; and raising it to 1024 or beyond, which buys more of a headroom that is not the problem
and reserves stack for a case nothing measures.

## 5. What actually retires it

**Judged.** An evaluator with an explicit control stack — a work list of pending continuations in the evaluator's own
heap rather than a chain of Rust frames. Peyton Jones ch. 11 and ch. 12 argue the direction (a reduction engine's state
is data, not the host's call stack), and ch. 18–21's G-machine is the fully worked form; musa needs the analysis and not
the machine, because it has neither laziness nor unbounded computation to manage. With the control stack explicit:

- Recursion depth is bounded by the step budget, which is the counter that already measures work.
- The nesting metric goes back to bounding term structure — `quote`'s descent and the traversal's — which is what §4.1
  derived it for.
- The room obligation shrinks to whatever the deepest *structural* descent costs, and stops scaling with how a composer
  wrote a fold.

Prompt 165a carries that work. Until it lands, 512 is the number.
