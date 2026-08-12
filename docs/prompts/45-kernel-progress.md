---
id: 45
slug: kernel-progress
status: done
depends_on: [44]
phase: 3
---

# Continuous Shape in the Denotation — Q4 Answered

## Task

A hairpin is the one musical fact the kernel cannot say. The timeline records *that* a crescendo spans this region;
**how** it grows exists only inside `performance.rs`, as `struct Curve { target, step, last }`, invented during lowering
and thrown away. Two consequences, and they are the reason this is a kernel prompt and not a performance one:

- The `.kernel` interchange artifact (prompt 48) cannot carry the shape, so two implementations reading the same file
  produce different sound. That is not an implementation difference; it is a specification hole.
- Every future continuous control — `gliss`, *rit.*, a filter sweep, a fader move — arrives with nowhere to live, and
  the pressure will be to invent a private curve for each. Four private curves is `docs/kernel/08-open-questions.md`'s Q4
  going unanswered four times.

Answer Q4. Add one payload value type, `Progress`, and prove that it needs **no new kernel operation** and **no change
to any existing law**.

## Read

- `docs/kernel/08-open-questions.md` Q4 (continuous controls — the open question this prompt closes), §12 (payload
  opacity), §13 (payload mapping is functorial), §14 (time scaling is external to the payload), §4 (exact rationals),
  §22 (tempo is `Beat → Second` and never kernel material), §34 (the smallest complete semantic basis).
- `crates/musa-compiler/src/performance.rs` — `hairpin_curves` and its `Curve`. Note precisely what it interpolates
  over: `step / last`, an **event index**, not a time. That choice is deliberate (interpolating over notes makes the
  arrival independent of the rhythm) and this prompt must preserve it, not silently replace it.
- `docs/kernel/03-denotational-semantics.md` D5 (`scale`) and D7 (`map_payload`) — the two operations a
  time-parameterized payload could plausibly break, and the reason the design below does not.
- `docs/kernel/04-algebraic-laws.md` L11–L15 (the functor laws).
- Prompt 48 (`kernel-interop`) — the payload text form that must carry this.
- PoSD ch. 6 (define errors out of existence) — an ill-formed curve should be unrepresentable, not diagnosed.

## Design

### The construct

```rust
/// A monotone reparameterization of an occurrence's own span: a piecewise-linear
/// map from normalized local time `u ∈ [0, 1]` to an exact fraction `v ∈ [0, 1]`.
///
/// `Progress` says *how far along* — never *how loud*, *how fast*, or *how high*.
/// What the fraction means is the consuming layer's business (§12): the profile
/// maps endpoints to amplitudes, the tempo map maps them to seconds. A dynamic
/// marking is still not a decibel.
pub struct Progress { /* breakpoints: (u, v), strictly increasing in u, u₀ = 0, uₙ = 1 */ }

impl Progress {
    pub fn linear() -> Self;
    /// `None` if the breakpoints are not strictly increasing in `u` or do not
    /// span `[0, 1]`. Construction is the only place this can fail.
    pub fn piecewise(points: impl IntoIterator<Item = (Ratio, Ratio)>) -> Option<Self>;
    pub fn at(&self, u: Ratio) -> Ratio;
}
```

### Why normalized local time is the whole design

Index the curve by `u ∈ [0, 1]` over the occurrence's own span rather than by absolute beats. Then **every kernel
operation acts on the span and leaves the payload bytes identical**:

- `scale r` multiplies the span; `u` is unchanged, so the curve is carried along automatically and correctly.
- `sequence` translates; same.
- `overlay` does not touch spans at all.
- `map_payload` is still an arbitrary `A → B` and still functorial — the kernel never inspects a `Progress`.

That is the property that makes this safe. An absolute-time curve would have to be rewritten by `scale` and `sequence`,
which means the kernel would have to *look inside payloads* to transform them — precisely the §12 violation the design
exists to avoid, and it would break L11–L15. State this as a theorem in `03-denotational-semantics.md` and test it: **a
curve-bearing occurrence transforms by its span alone.**

So Q4's answer is that the kernel needed a *value*, not an *operation*. Record that in §32 and in
`docs/kernel/08-open-questions.md`: it is the third piece of §34 evidence, alongside prompts 39 and 44, that the
operation set is complete.

### What `Progress` deliberately cannot express

- **Steps.** Piecewise-*linear* only; no step segments, no jump discontinuities. A sudden change is a `Dynamic` fact at
  a point — the timeline already has one, prompt 44 already answers "what is in force here" with `prevailing`, and
  expressing the same change two ways is the complecting this whole block exists to remove.
- **Units.** Values are unit-free fractions in `[0, 1]`. If a caller wants a curve in decibels, that is the profile's
  mapping of the endpoints, applied to this fraction.
- **Curves that are not monotone in `u`.** A vibrato or an LFO is periodic, not progress; it is a different construct
  and it is not this one. If one is ever wanted, it arrives with its own name and its own evidence.

Each of these is a boundary that stays cheap only if it is stated. Put them in the type's doc comment, not just here.

### Shape is in the kernel; sampling policy is not

This is the distinction that keeps `performance.rs` honest and keeps every golden byte-identical.

- The **shape** — "linear from 0 to 1 across this hairpin" — is a fact about the piece. It goes in the timeline, it is
  serialized, and two implementations must agree on it.
- The **sampling policy** — "evaluate once per notated event, at `u = index / (count − 1)`" versus "evaluate at each
  note's onset, at `u = (onset − start) / span`" — is the performance layer's interpretation.

`hairpin_curves` keeps its current index-based sampling; it stops inventing the shape and reads it from the payload
instead. **No WAV, MIDI, or notation golden changes in this prompt.** Document the sampling choice in `performance.rs`'s
module docs with the rhythm-independence reason, so the next reader knows it is a decision rather than an accident, and
note in `07-backend-contract.md` that a conforming consumer must honour the shape and may choose its own sampling.

### Where it lives, and the argument for the kernel over the compiler

`Progress` could be a compiler-side type — prompt 48's payload grammar is compiler-supplied, so it would serialize
either way. It belongs in `musa-kernel` for one reason: **the invariance theorem above is a statement about the kernel's
operations**, and it must be stated and property-tested where those operations are. A payload type whose correctness
argument is "the kernel's operations leave it alone" cannot have its proof live in a crate that does not contain the
kernel's operations.

Its exactness and its canonical form follow from that placement: `Ratio` breakpoints (§4), a `Canonical` implementation,
and therefore a stable contribution to prompt 43's semantic hash.

### What this unlocks, and builds none of

With `Progress` present and prompt 44's `prevailing` present, each of these is a new `FactKind` and zero kernel change:
`gliss` (a note carrying a pitch progress), non-linear crescendo, `rit.`/`accel.` as a tempo-map input, studio parameter
automation, `modulate` mid-piece. **Build none of them.** The grammar freeze (§35.1) holds; this prompt adds a value
type and one use of it. Listing the payoff is how the prompt justifies the construct; implementing the payoff is a
different prompt with a different check.

`TempoSegment` (prompt 36) *may* be re-expressed in terms of `Progress` as a shared value type — tempo would still be
computed above the kernel and §22 would be untouched. Only do it if it deletes code; if it merely relocates it, leave it
and say so.

## Target

- `crates/musa-kernel/src/progress.rs` (new): `Progress`, `Canonical`, the stated non-goals.
- `crates/musa-kernel/tests/laws.rs`: **L24** — for every operation, a curve-bearing occurrence's payload is
  byte-identical after transformation, and `at(u)` evaluated at corresponding absolute times agrees before and after
  `scale`, `sequence`, `overlay`, and `restrict`.
- `docs/kernel/03-denotational-semantics.md`: `Progress` and the span-alone theorem.
- `docs/kernel/04-algebraic-laws.md`: L24, with its test name.
- `docs/kernel/05-normalization.md`: the canonical form of a `Progress`.
- `docs/kernel/08-open-questions.md` and `docs/kernel/08-open-questions.md`: **Q4 resolved**, with the answer and why it
  cost no operation.
- `crates/musa-compiler/src/elaborate.rs`: `FactKind::Hairpin` gains a `Progress` (`Progress::linear()` from the current
  grammar).
- `crates/musa-compiler/src/performance.rs`: `Curve` deleted; `hairpin_curves` reads the shape and keeps its sampling.
- `docs/kernel/07-backend-contract.md`: shape is normative, sampling is the consumer's.
- `docs/kernel/09-performance.md`: this prompt's row.

## Repairs made while implementing

**`Progress` carries no `serde`, and the compiler adapts it.** `HairpinSpan` is a serialized snapshot type, so adding a
`Progress` field to it demanded `Serialize`/`Deserialize`. The kernel's dependency list is `num-rational` and
`thiserror`; roadmap §15 sanctions `serde` for `musa-compiler`, not for the crate the kernel was carved into, and a
payload value type is not a reason to widen it. `musa-compiler/src/score.rs` therefore carries a ~30-line
`progress_serde` adapter that writes breakpoints as exact `(numer, denom, numer, denom)` quadruples through the
already-public `Progress::points`/`Progress::piecewise`. The adapter is arithmetic-free and cannot admit a curve the
constructor would reject, because deserialization goes through `piecewise` — an invalid file is a deserialization error,
not an ill-formed value.

**`Curve` became `Reached`, which is a smaller thing.** The prompt says delete `Curve` and it is deleted, but the
sampling policy still needs somewhere to put its answer per event. The replacement holds the target mark and the
*fraction the shape reached* — not `step`/`last`, which were the shape being invented. Where the old code computed
`from + (to − from) · step/last`, it now computes `from + (to − from) · shape.at(u)` with `u = step/last`, which is the
prompt's split made literal: the policy picks `u`, the kernel's value answers what fraction that is. With
`Progress::linear()` the two are identical rational expressions, which is why no golden moved.

**The single-event hairpin lost its special case.** `Curve` handled `last == 0` with an `if` that jumped straight to the
target. Now a lone event under a hairpin is sampled at `u = 1` and the shape answers `1`, so the branch disappears and
the arrival test becomes `fraction == 1` — one rule instead of two. This is the same behaviour, expressed once.

**`TempoSegment` was not re-expressed, and the prompt allowed for that.** It holds a *constant* seconds-per-quarter for
a segment plus its starting frame; there is no curve in it. Rewriting it in terms of `Progress` would relocate code
rather than delete any, which is the condition the prompt set for leaving it alone.

**`Progress::at` clamps rather than erroring.** Asking a curve about `u` outside `[0, 1]` is a question about its
endpoints — a consumer sampling at an onset slightly outside the region should get the endpoint value, not an `Option`.
The lookup loop also ends in a total fallback that the `uₙ = 1` invariant makes unreachable, because a total function is
cheaper than a `panic!` guarding an invariant construction already enforces.

**L24 tests both halves.** Byte-identity of the payload alone would pass for an absolute-time curve that happened to be
copied unchanged and therefore be *wrong*; the test also evaluates `at(u)` at corresponding absolute instants after
`scale`, `sequence`, and `overlay`, which is the half that would fail. `restrict` is checked for payload identity
through the observation, since it moves nothing.

**The measurement, and what it cannot see.** Every row is within noise of prompt 44's and every allocation count is
identical to the digit — because **neither benchmark workload contains a hairpin**. `glass-mountain.musa` has none and
`large-score.musa`'s coda is point dynamics, articulations, ties, slurs and tuplets. The table therefore confirms the
change costs nothing where there are no hairpins and says nothing else; the real cost is one two-element `Vec` per
hairpin at elaboration, one clone at projection, and one `at()` per event under a hairpin replacing a multiply. Growing
the fixture would invalidate forty existing rows, so `docs/kernel/09-performance.md` records the gap and leaves the
repair to the prompt that next needs the fixture to change.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject   # no golden may change
for f in examples/*.musa; do cargo run -p musa -- check "$f"; done
grep -rn "struct Curve" crates/musa-compiler/src/ | wc -l   # 0
grep -rn "Q4" docs/kernel/08-open-questions.md            # reads as resolved
cargo bench -p musa-compiler
```

Commit as `Put continuous shape in the kernel denotation`.

## Stop

- No new kernel operation. If the implementation seems to need one, the curve is not normalized correctly — fix that,
  not the operation set.
- No absolute-time curves, no step segments, no periodic shapes, no units, no easing catalogue (`ease_in`, `expo`,
  Bézier). Piecewise-linear over `[0, 1]` is the whole construct.
- No grammar change. `cresc.` still parses exactly as it does today and elaborates to `Progress::linear()`.
- No behaviour change. If a golden moves, the sampling policy changed and that was not this prompt's job.
- No `gliss`, no automation, no tempo curves. They are the argument, not the work.
