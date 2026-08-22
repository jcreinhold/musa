---
id: 165a
slug: explicit-control-stack
status: pending
depends_on: [165]
phase: 3
---

# Give the Evaluator an Explicit Control Stack

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

**Take the analysis, not the machine.** Musa's core is strict, finite, and total: there is no laziness to manage, no
thunk to update, no graph to garbage-collect. Ch. 12–17's machinery answers questions musa does not have. What transfers
is one idea — that a reducer's control state is data it owns rather than the host's call stack — and the two places the
book shows it being saved and restored.

## Design

**The frames that grow with recursion are the ones to move.** Profile first and name them: reducing a compiled case tree
is the expected answer, because that is where 155a's level goes, and the mitigations already there say which parts do
not. A split on a bare variable reads the environment and calls nothing. A saturated self-call in tail position is
already trampolined in `Compiled::reduce`, charging by hand the steps the whole-term evaluation would have charged, in
the same order. The remaining growth is the non-tail recursive call — the shape a fold is written in — and that is what
this prompt makes flat.

**What stays on Rust frames.** `quote`'s descent over a value and the traversal's descent through a term are structural:
their depth is the depth of the thing being walked, they are what §4.1 derives the metric from, and they keep charging
nesting and keep using host frames. Converting two closures likewise. This is not a rewrite of NbE. The boundary to hold
is that after this prompt, the nesting counter's maximum over a run is a function of the *terms and values* the run
touches, and not of how many times any definition called itself.

**Acceptance moves, in both directions, and both are one argued amendment.** A recursion that is no longer charged
nesting is a program that was refused and is now accepted, and §4's sentences forbid that happening quietly. In the
other direction, with recursion off the metric, 320 is no longer earned — the measurement that justified it was the
staff adapter's recursion depth — and the limit should be re-derived from `quote`'s descent alone, which will argue for
a smaller one. Land both as one paragraph in §4.1 replacing 155a's, with the re-measured numbers, and close note 54 with
a line saying which of its §3 and §4 predictions held. Two version bumps recorded as one change is honest; either one
landing silently is not.

**The step budget has to actually bound it.** Moving recursion off the nesting metric is only sound if the step budget
already charges every recursive step, so that an unbounded recursion is refused rather than run forever in a flat loop.
`Compiled::reduce`'s trampoline is the precedent and the trap: it charges by hand, and a hand-written charge that drifts
from what the evaluator would have charged is a silent acceptance change. Whatever the explicit stack charges, a law has
to pin it against the frame-based reading, on a program small enough to count by hand.

**Room follows the limit, not the other way round.** `kernel::room` reserves `NESTING × FRAME_CEILING`. If the limit
comes down the reservation comes down with it, and `FRAME_CEILING` should be re-measured rather than inherited, because
the chain it was measured on has changed shape. Prompt 165's re-measurement is the method to reuse.

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
  importing a mechanism because the book has it is the opposite of a designed step.
- No rewrite of `quote`, conversion, or the traversal into the same style. Their depth is the depth of what they walk,
  which is the metric working as derived.
- No `RUST_MIN_STACK`, in a test command, a `.cargo/config.toml`, a CI file, or a doc.
- No raised step budget to absorb a recursion that now costs steps. If a real program no longer fits, that is a
  measurement and an argument in §4, not a threshold moved to make a suite pass.
- No adapter rewrite. [166](166-staff-rewrite.md) and [167](167-studio-rewrite.md).
