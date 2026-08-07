---
id: 44
slug: kernel-queries
status: in-progress
depends_on: [43]
phase: 3
---

# Give the Kernel an Interface, Not Just a Representation

## Task

The kernel can *state* every temporal fact and *answer* nothing about them. Its public surface is constructors
(`timeline`, `sequence`, `overlay`, `scale`, `map_payload`), one observation (`restrict`), and canonicalization — so
every consumer that needs "what is in force here" or "what covers this" writes the scan itself, and they do not agree.
Add the two queries the codebase already contains by hand, define them denotationally, prove them, and delete the
hand-rolled versions.

This is the prompt that answers "is the kernel actually useful, or is it ceremony?" A representation nobody can
interrogate is ceremony. The fix is an interface, not an ontology: **no new constructor, no new stored state.**

## Read

The four rebuilds, read them before designing anything:

- `crates/musa-compiler/src/project.rs` `project_piece` — finds the key and meter by sorting occurrences on
  `origin.definition_span.start` and taking last-wins. That is a prevailing-value scan keyed on **source position**,
  which is correct only while every such fact spans the whole piece, and silently wrong the day `modulate` exists.
- `crates/musa-compiler/src/project.rs` `project_regions` — converts each region's span to an `(from, to)` event-id pair
  by linear search over the event extents (`.iter().find`, `.iter().rev().find`), once per region.
- `crates/musa-render/src/plan.rs` `Marks::collect` — converts back, calling `score.events_in(from, to)` to rebuild
  which events a phrase or hairpin contains. Span → ids → membership: the projection destroys the containment the
  timeline had, and the plan reconstructs it.
- `crates/musa-compiler/src/performance.rs` `lower_performance` — carries `dynamic = Some(mark)` forward per voice.
  A prevailing-value scan, written a third time, with its own answer for what happens at a coincident onset.

Then:

- `docs/kernel/03-denotational-semantics.md` D6 and prompt 37's `Observation` — the existing observation operation, and
  the shape a new one should follow.
- `docs/kernel/04-algebraic-laws.md` — the law style the new operations must be stated in.
- Prompt 42 (in progress) — its Set B accessors *regions covering an event* and *marks attached to an event* are the
  snapshot-level consumers of this prompt's `covering`. Do not duplicate prompt 42; re-implement its accessor on top of
  the kernel query and delete the hand-rolled scan underneath it.
- The module-design pressures this clears: *information loss* (callers rebuild what the module had), *repetition*
  (the same scan four times), and PoSD ch. 8 (pull complexity downward).

## Design

### Design it twice

- **Design 1 — two concrete queries on `Timeline`.** `covering` and `prevailing`, each with a denotational definition
  and laws, each with existing callers.
- **Design 2 — an FRP-shaped `Behavior<V>` abstraction.** `timeline.behavior(rule)` returning a sampled function of
  time, with `Step`, `Ramp`, and `Coverage` rules. Conceptually tidier — it names the fact that a finite occurrence set
  induces total functions of time — and it is what a functional-reactive treatment of this domain would reach for.

**Take Design 1.** Design 2 buys a vocabulary and costs a trait with one implementor per rule, closures at every call
site, and a sampling protocol (`sample_over(window, step)`) that no current caller wants. The module-design rules are
explicit: generalize the interface, not unused functionality; a one-implementor trait is a concrete type. Record Design 2
in `docs/kernel/08-open-questions.md` as the shape to revisit **if** a third rule appears with two callers — that is the
evidence that would justify it, and until then it is speculative generality.

### The two operations

```rust
impl<A> Timeline<A> {
    /// Occurrences whose support contains `at`, in canonical order (D8).
    ///
    /// Containment is the support's own convention: `[s, e)` contains `at`
    /// when `s ≤ at < e`; a point occurrence at `s` is contained at `s`.
    pub fn covering(&self, at: Beat) -> impl Iterator<Item = &Occurrence<A>>;

    /// The value in force at `at`: the last occurrence starting at or before
    /// `at` for which `select` yields a value, in canonical order (D9).
    ///
    /// `select` is how the caller says which facts participate; the kernel
    /// does not know which payloads are context-bearing (§12).
    pub fn prevailing<'a, V>(&'a self, at: Beat, select: impl Fn(&'a A) -> Option<V>) -> Option<V>;
}
```

Two things this design is careful about:

- **`prevailing` takes a selector rather than a payload trait.** A trait would make the kernel ask payloads "are you
  context?", which is musical knowledge (§12). A closure lets the *caller* say "the `Key` facts" without the kernel
  learning what a key is. It also means one timeline supports many independent prevailing values — key, meter, clef,
  dynamic — without a new type per kind.
- **`covering` returns occurrences, not spans or ids.** Event identity is the score layer's invention (prompt 39
  assigns `EventId` during projection); the kernel must not learn it.

### The boundary conventions, decided once

These are the decisions currently made four times and not identically. State each in `03-denotational-semantics.md`
with its reason, and test it:

| Question | Ruling | Why |
| --- | --- | --- |
| Is a fact covering at its end instant? | No — support is `[s, e)` | Consistent with D6 and with `sequence`: the next fact's start is the previous one's end, and one instant must not belong to both |
| Is a point occurrence covered at its own instant? | Yes | Otherwise a point fact is unobservable, the same defect prompt 37 repaired in D6 |
| Two prevailing candidates at the same instant? | The later in canonical order wins | Canonical order is total (N2) and already includes the payload key, so the answer is deterministic and does not depend on construction |
| Does a fact starting exactly at `at` prevail at `at`? | Yes | `dynamic mf;` on a note applies to that note; anything else surprises a composer |

### The laws

In `04-algebraic-laws.md`, with property tests in `tests/laws.rs`:

- **L20 — coverage agrees with observation.** `m.covering(t)` yields exactly the occurrences observed by
  `m.restrict(w)` for every window `w` containing `t`. The two ways of asking cannot disagree.
- **L21 — coverage is stable under time transformation.** `scale_r(m).covering(r·t)` corresponds to `m.covering(t)`;
  `(m ; n).covering(d + t)` corresponds to `n.covering(t)` for `d = extent(m)`. The queries commute with the algebra.
- **L22 — prevailing is the last selected start.** For every `t`, `prevailing` equals the `select`-image of the
  canonically last occurrence with `start ≤ t` that `select` accepts, and `None` when there is none.
- **L23 — prevailing is monotone in information.** Overlaying a timeline whose selected facts all start after `t` does
  not change `prevailing(t)`. This is the law that makes it safe for a projection to build the piece incrementally.

### The performance rule, stated because a point query invites O(n²)

Both queries are linear scans. That is correct for a one-off ask (the interface's "what covers the selection") and
wrong for bulk derivation: calling `covering` once per event is O(events × facts), and benchmark P3 will say so.

The rule, in the projection's module docs and in this prompt's Check: **the kernel defines what the answer is; bulk
derivation does one ordered pass.** The projection keeps its single sweep and uses the queries' *definitions* — the
conventions above — rather than calling them per event. If P3 regresses, the migration turned a sweep into n queries and
must be reverted, not tuned.

Do not add a bulk API to the kernel for this. No caller wants one, and the sweep belongs where the score's ordering
lives.

## Target

- `crates/musa-kernel/src/timeline.rs`: `covering`, `prevailing`.
- `crates/musa-kernel/tests/laws.rs`: L20–L23.
- `docs/kernel/03-denotational-semantics.md`: D8, D9, and the convention table.
- `docs/kernel/04-algebraic-laws.md`: L20–L23 with test names.
- `docs/kernel/07-backend-contract.md`: a section stating that consumers ask the kernel these questions rather than
  answering them privately — the guarantee is that two consumers get the same answer.
- `docs/kernel/00-purpose.md`: the "what the kernel is" statement gains its second half — it is a representation *and*
  the interface for interrogating it. This is the framing repair; the document currently reads as if stating facts were
  the whole job.
- `docs/kernel/08-open-questions.md`: Design 2 recorded as the revisit-with-evidence shape.
- `crates/musa-compiler/src/project.rs`: `project_piece` uses time order and the prevailing rule, not
  `origin.definition_span`; `project_regions` uses the containment convention.
- `crates/musa-compiler/src/performance.rs`: the per-voice dynamic scan replaced.
- `crates/musa-render/src/plan.rs`: `Marks::collect`'s membership rebuild replaced by prompt 42's accessor.
- `docs/kernel/09-performance.md`: this prompt's row.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject   # no golden may change
for f in examples/*.musa; do cargo run -p musa-cli -- check "$f"; done
cargo bench -p musa-compiler                          # P3 must not regress; see the rule above
grep -n "definition_span" crates/musa-compiler/src/project.rs | wc -l   # 0 in project_piece
```

Commit as `Add coverage and prevailing-value queries to the kernel`.

## Stop

- No new constructor, no new stored state, no new payload requirement. Queries only.
- No `Behavior` type, no sampling protocol, no rule trait. That is Design 2, and it is recorded, not built.
- No bulk/indexed query API — no interval tree, no acceleration structure. If a measurement demands one it is its own
  prompt, and the first fix is the caller's sweep, not the kernel's storage.
- No behaviour change anywhere. Every golden holds; this prompt replaces four answers with one.
