---
id: 165a
slug: explicit-control-stack
status: done
depends_on: [165b]
phase: 3
---

# Give the Evaluator an Explicit Control Stack

> **Moved ahead of 165 — and ahead of 166 — by the measurement in
> [note 59](../../notes/research/language-design-closure/59-the-staff-wall-is-the-evaluators.md) §5.** Nothing below
> changed; only `depends_on` did. This prompt used to follow 165 because it was written as the second of the evaluator's
> two performance repairs, and 165 follows 164, which follows 166. Prompt 165b was cut out of 165 for that deadlock;
> implementing 165b's second half showed the deadlock also holds this prompt. With data descent charged in steps,
> `examples/staff-page.musa` still peaks at 471 nesting levels of 320, and every one of them is the adapter's own
> recursion at about seven levels a call — so the staff class stays red on `nested evaluation levels` until this prompt
> runs, and prompt 166 cannot be measured, or usefully developed, before it. This prompt never needed 165: its Read
> cites 165 once, to say the elaborator's own missing charge is a different recursion this prompt must not absorb, and
> its Stop says the same. The one thing it borrowed forward is the `FRAME_CEILING` re-measurement method, which is
> written down in 165's Design and can be read there without 165 having run.

## Task

`Budget::NESTING` bounds two unrelated quantities. The metric was derived to bound how deeply a *term* is written —
§5.9's traversal descends through a transformer's branches, NbE's `quote` descends over a value — and recursion never
entered that derivation. It is charged anyway: a recursive call is evaluated inside the enclosing evaluation and holds
its level until the steps beneath it finish, so one number decides both how deeply a composer may write a term and how
many times a definition may call itself.

The conflation is older than case trees — `resource_validation.rs` has recorded it since prompt 142 — and prompt 155a
made it more expensive, from about 2.5 levels per recursive call to about 3.0, which pushed the standard library's staff
adapter past the limit it had been sitting sixteen levels under. 155a's answer was to raise the limit to 320, which is
bounded above by reachability at 362 and below by the corpus at 272; there is no third raise available.
[Note 54](../../notes/research/language-design-closure/54-the-nesting-limit.md) derives all of that and names this
prompt as what actually repairs it.

Retire the conflation. Make the evaluator's pending work explicit data in its own state rather than a chain of Rust
frames, so that a recursion's depth is bounded by the step budget — the counter that measures work done, which is what a
recursion spends — and `nesting` goes back to bounding structural descent, which is what §4.1 derives it for.

## Read

- `docs/rules/language/02-core-calculus.md` §4 and §4.1 — the cost table, the three outcomes, the nesting metric's two
  stated motivations, the room obligation, and the amendment paragraph 155a added. §4.1's closing is the thing this
  prompt is written to make true again.
- [note 54](../../notes/research/language-design-closure/54-the-nesting-limit.md) — the measurement, the two mitigations
  155a landed inside its own boundary, and what this prompt is expected to change.
- `docs/rules/across-stages/05-metatheory.md` §1a — which obligations musa argues and which it tests. The evaluator
  changing shape does not change what it computes, and this prompt must be able to say why.
- [155a](155a-case-tree-bodies.md) — what a compiled case tree is, how `Compiled::reduce` walks it, and the tail-call
  trampoline already in it. The trampoline is the special case this prompt generalizes.
- [165](165-diagnostics-and-performance.md) — the *elaborator*'s missing nesting charge and the spine walk. That is a
  different hole in a different recursion; this prompt must not absorb it.
- Peyton Jones ch. 11 §11.6 and ch. 12 — the spine stack: the pending applications a reducer is in the middle of are
  kept as a stack of pointers in the machine's own store, and the arguments are then readable by depth rather than by
  unwinding a host frame. Ch. 11's observation that a nested evaluation "needs a brand new stack" and that the existing
  one does not change until it finishes is the invariant the design below turns into a saved frame.
- Peyton Jones ch. 18 §18.8 — the *dump*: the G-machine saves the old stack pointer and return address on a second stack
  so that a nested evaluation returns without a host call. That is the mechanism, minus the graph.
- Peyton Jones ch. 21 — generalized tail calls, and why the tail case is worth separating from the general one.
- Peyton Jones ch. 17 §17.2 — reclaiming a structure without a stack proportional to its depth. Musa has no collector
  and needs none: `Arc` already reclaims. What ch. 17 supplies is the observation that *reclamation is a traversal*, and
  a traversal of a deep structure written as host calls is the same defect as an evaluation written as host calls.

**Take the analysis, not the machine.** Musa's core is strict, finite, and total: there is no laziness to manage, no
thunk to update, no graph to garbage-collect. Ch. 12–17's machinery answers questions musa does not have. What transfers
is one idea — that a reducer's control state is data it owns rather than the host's call stack — and the two places the
book shows it being saved and restored.

## Design

**The frames that grow with recursion are the ones to move.** Profile first and name them, and do not trust this
paragraph's guess over the measurement — the first version of it guessed wrong. Reducing a compiled case tree looked
like the answer, because that is where 155a's level goes, and the mitigations already there say which parts do not: a
split on a bare variable reads the environment and calls nothing, and a saturated self-call in tail position is already
trampolined in `Compiled::reduce`, charging by hand the steps the whole-term evaluation would have charged, in the same
order. Measured on `examples/staff-page.musa` at the wall, the cycle is not the case tree at all. It is ι firing on a
generated recursor and the δ-unfold replaying its spine — `eval → application → apply → applying → iota → ready → opened
→ unfold → unfold_spine → eliminate_replayed → applying → apply_closure → eval`, about seven nesting levels and thirty
host frames per recursive call. Those are the frames to move; the case tree rides along because `Compiled::reduce` sits
inside the same cycle.

**What stays on Rust frames.** `quote`'s descent over a value and the traversal's descent through a term are structural:
their depth is the depth of the thing being walked, they are what §4.1 derives the metric from, and they keep charging
nesting and keep using host frames. Converting two closures likewise. This is not a rewrite of NbE. The boundary to hold
is that after this prompt, the nesting counter's maximum over a run is a function of the *terms and values* the run
touches, and not of how many times any definition called itself.

**Destruction is a traversal too, and this prompt is what makes it reachable.** A `Value` is a tree of `Arc`s and its
destructor is Rust's, which is one host frame per level. That was invisible while nesting refused the programs that
build deep values; with recursion off the metric they are accepted, and a list of a few thousand elements is a value a
few thousand deep whose *drop* aborts the process — measured, `range(4000)` compiles and `range(4500)` overflows the
stack in `drop_glue<Value>` with no musa frame on the stack at all. Trading a refusal for an abort is the one outcome
§4.1 forbids, so the destructor becomes explicit data by the same move as the evaluator: take the children out into a
worklist and dismantle them in a loop. This is not a collector and not a change to how anything is owned — `Arc` still
reclaims, at the same moment, in the same order — it is ch. 17's observation that a deep traversal must not be written
as host calls. It also closes a shortfall `resource_validation.rs` has recorded since prompt 142, where a fold over
50,000 elements aborted "past about 1,256 elements" and the descent was recorded as unidentified; this is the descent.

**Acceptance moves, in both directions, and both are one argued amendment.** A recursion that is no longer charged
nesting is a program that was refused and is now accepted, and §4's sentences forbid that happening quietly. In the
other direction, 320 looks unearned once recursion is off the metric — the measurement that justified it was the staff
adapter's recursion depth — and this paragraph used to say the limit should therefore come down to whatever `quote`'s
descent alone argues for. It must not, and the evidence is three laws: while nesting is the *only* charge standing
between the elaborator's own uncharged `check`/`infer` recursion and the host's stack, lowering the limit lowers the
room with it, and `elaborating_a_term_nested_past_the_limit_is_refused` stops being a refusal and becomes an abort.
Charging that recursion is prompt 165's and this prompt's Stop forbids it, so the limit stays where 155a put it and what
gets re-derived is the *room* — the frame ceiling, measured on the chain as it is now shaped. Land the widening as one
paragraph in §4.1 replacing 155a's, saying which programs it accepts that were refused, and close note 54 with a line
saying which of its §3 and §4 predictions held. A version bump recorded as one argued change is honest; landing silently
is not.

**The step budget has to actually bound it.** Moving recursion off the nesting metric is only sound if the step budget
already charges every recursive step, so that an unbounded recursion is refused rather than run forever in a flat loop.
`Compiled::reduce`'s trampoline is the precedent and the trap: it charges by hand, and a hand-written charge that drifts
from what the evaluator would have charged is a silent acceptance change. Whatever the explicit stack charges, a law has
to pin it against the frame-based reading, on a program small enough to count by hand.

**Room follows the limit, not the other way round.** `kernel::room` reserves `NESTING × FRAME_CEILING`. If the limit
comes down the reservation comes down with it, and `FRAME_CEILING` should be re-measured rather than inherited, because
the chain it was measured on has changed shape. The method is written out in prompt 165's Design and is reused here
ahead of it, which is what the reordering above costs.

**No new refusal, and no new diagnostic vocabulary.** A recursion too deep for the step budget is exhausted at
`reduction steps`, which is a diagnostic that already exists and already reads correctly. If it does not read correctly
for this case, that is prompt 165's standard to meet, not a new outcome to invent.

## Target

- A profile naming, on the four recorded workloads, which evaluator frames grow with recursion depth and which grow with
  term or value depth, measured rather than assumed, recorded in note 54's closing.
- The recursion identified above evaluated against an explicit control stack in `musa-calculus`'s kernel, with the
  saved-frame representation doc-commented against the invariant it encodes.
- Nesting charged only for structural descent, and a law in `budget_laws.rs` pinning that: a definition recursing far
  past the old limit is accepted if it fits the step budget, and the nesting counter's peak over that run is bounded by
  the term's depth rather than by the recursion's.
- A law pinning the step charge against the frame-based reading, on a program whose steps can be counted by hand.
- A value's destructor iterative rather than recursive, and a law building a value far deeper than the host stack could
  hold and dropping it — so that the widening above cannot be paid for with a crash.
- The two acceptance changes landed as one argued paragraph in `docs/rules/language/02-core-calculus.md` §4.1, replacing
  155a's, carrying the re-measured structural depth; §4's cost-table sentence updated to whatever limit that argues for.
- `Budget::NESTING`'s doc comment re-earned on the new measurement, and `kernel::room`'s `FRAME_CEILING` re-measured
  with the command that produced the number beside it.
- Note 54 closed: a line per prediction in its §3 and §4, saying which held.
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

Commit as `Give the evaluator an explicit control stack`.

## Stop

- No change to what a case tree means, how one is compiled, or how termination is checked on it. That is
  [155a](155a-case-tree-bodies.md)'s, and it is `done`.
- No explicit stack for the *elaborator*. `check` and `infer` standing inside one another is a different missing charge
  in a different recursion, and [165](165-diagnostics-and-performance.md) owns it.
- No graph reduction, no thunk update, no laziness, no garbage collector. Musa's core is strict, finite, and total, and
  importing a mechanism because the book has it is the opposite of a designed step. The iterative destructor above is
  not an exception to this: nothing changes about what owns what or when it is freed, only about how many host frames
  the freeing stands on.
- No rewrite of `quote`, conversion, or the traversal into the same style. Their depth is the depth of what they walk,
  which is the metric working as derived.
- No `RUST_MIN_STACK`, in a test command, a `.cargo/config.toml`, a CI file, or a doc.
- No raised step budget to absorb a recursion that now costs steps. If a real program no longer fits, that is a
  measurement and an argument in §4, not a threshold moved to make a suite pass.
- No adapter rewrite. [166](166-staff-rewrite.md) and [167](167-studio-rewrite.md).
