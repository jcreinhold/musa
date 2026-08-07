---
id: 39
slug: score-facts
status: done
depends_on: [36, 38]
phase: 3
---

# One Timeline, Many Kinds of Fact

## Task

Make the kernel timeline the *only* place a temporal fact lives. Today notes are occurrences while slurs, phrases,
tuplets, dynamics, hairpins, and ties are tags copied onto every note payload they cover
(`elaborate.rs`'s `Marks`), reconstructed afterwards by the adapter into `AnnotationStore` spans keyed by event ids the
adapter itself assigns. Replace that with a heterogeneous payload — `ScoreFact` — so each of those is an occurrence with
its own span, its own provenance, and no copy of itself on anything else. `ScoreSnapshot` keeps its exact public shape
and is computed from the timeline by projection.

This prompt adds **no kernel constructor**. That it needs none is the evidence course correction §34 asks for.

## Read

- Course correction §12 (payloads are typed and musically opaque to the kernel), §21 (typed interval payloads are the
  agreed shape for facts with temporal extent), §27 (if a score concept will not fit, the *snapshot* grows, not the
  kernel), §34.
- `docs/kernel/06-surface-elaboration.md` — the elaboration table this prompt rewrites, and the adapter contract it must
  keep satisfying.
- `crates/musa-compiler/src/elaborate.rs`: `Marks`, `VoicePayload`, `PayloadKind`, `adapt_voice`, `reverse`, `retie`.
  `retie` exists only because a tie is encoded as a payload flag; it should not survive this prompt.
- `crates/musa-compiler/src/score.rs`: `AnnotationStore`, `SlurSpan`, `TupletSpan`, `PhraseSpan`, `DynamicMarking`,
  `ArticulationMarking`, `HairpinSpan` — the shapes the projection must reproduce byte-identically.
- Rich Hickey, *Simple Made Easy*: the current design **complects** three independent things — what a fact is, where it
  is in time, and which notes happen to be under it — into one payload field. The test applied throughout below is
  whether two things change independently.
- PoSD ch. 9 (better together or apart) and the red flag *information leakage*: a slur's extent is known during
  elaboration, discarded into per-note tags, and rebuilt downstream.

## Design

### The payload: three orthogonal axes, nothing complected

```rust
/// One elaborated fact of a score: what is stated, where in the score's
/// structure it belongs, and why it exists. Where it is in *time* is the
/// occurrence's span — never a field here (docs/kernel/03 D0).
pub(crate) struct ScoreFact {
    scope: Scope,      // Piece | Part(PartId) | Voice(PartId, VoiceId)
    kind: FactKind,    // what is stated
    origin: Origin,    // provenance, above the kernel (§20)
}
```

`Scope` is structural position; the span is temporal position; `FactKind` is content. Each varies without the others: a
slur moves in time without changing voice, a voice is renamed without moving anything, a dynamic changes from `mf` to
`f` in place. Keeping them in one flat struct with a `groups: Vec<u32>` field, as `Marks` does, is precisely the
complecting this prompt removes.

`FactKind` for this prompt (score-level kinds arrive at prompt 40):

| Variant | Span | Notes |
| --- | --- | --- |
| `Note { pitch, duration, articulations }` | the sounding extent | `duration: NotatedDuration` is *notation intent* — how many noteheads spell the span — and stays in the payload (roadmap §2: notated ≠ performed) |
| `Rest { duration, articulations }` | the notated extent | notation intent, not silence; the kernel gains no silence object (§2) |
| `Slur` / `Phrase { name }` | first onset → last end | |
| `Tuplet { num, den }` | the bracketed region | unreduced, as the backends need |
| `Dynamic { mark }` | a point at the onset it applies from | |
| `Hairpin { grows, target }` | the region it spans | prompt 36 introduced it |

### What stays inside a note, and why

Articulations stay a field of `Note`/`Rest`; they are not occurrences. The rule applied is the one from the reading:
**does it have an extent and an identity of its own?** A staccato dot has neither — it has no span but the note's, it
cannot be moved without moving the note, and making it a separate occurrence would force the projection to re-join it to
its note by span-and-voice, which is the information loss this prompt exists to delete, merely inverted. A slur has both.

### Ties disappear entirely

A tie is not a fact. It is a statement that two written noteheads spell **one** occurrence. So elaboration merges a tied
statement with its continuation at the point of elaboration: one occurrence, span the sum, `NotatedDuration` the
compound spelling (`NotatedDuration::pieces` already exists for exactly this). Nothing carries a tie flag afterwards.

Three consequences to verify, not assume:

- `retie` is deleted. `retrograde` reverses spans; a merged occurrence mirrors like any other, so the relation that
  needed repairing no longer exists to be broken. Prompt 34's double-reversal law must still pass unchanged — it is now
  true for a simpler reason.
- A tie onto a different pitch, or with nothing after it, is still a diagnostic, emitted at merge time.
- The adapter's merge loop (`Adapted`, the articulation-extending pass) goes away with it.

### The timeline is per piece, not per voice

Elaboration produces exactly **one** `Timeline<ScoreFact>` for the whole piece: voices overlay into parts, parts overlay
into the piece. One compilation, one temporal object. Part and voice identity are in `Scope`, which is the working
stance Q3 already adopted and which this prompt is further evidence for — record that in `08-open-questions.md`.

### The projection, and its one performance rule

`ScoreSnapshot` and `AnnotationStore` keep their current public shape and are **computed** from the timeline. The
projection:

- visits the canonically ordered occurrences **once**, bucketing by `Scope` — it must not filter the whole timeline once
  per part or per annotation kind, which would be O(facts × parts). Benchmark P3 from prompt 38 is the check on this,
  not inspection;
- assigns `EventId`s to `Note`/`Rest` occurrences in the order it visits them, preserving today's ids exactly;
- resolves span-anchored facts to the event ids the annotation types use: a `Slur` over `[s, e)` becomes
  `SlurSpan { from, to }` naming the first event at or after `s` and the last ending at or before `e` in that voice.
  **Invariant:** elaboration only ever emits a region fact whose boundaries coincide with event boundaries in its scope,
  because the region is built from the extent of the items it encloses. State it as a doc comment on the projection and
  assert it in a debug assertion; if it can be violated, the elaboration that violated it is the bug.

### Parity is the safety net, and it still exists

The prompt-05/06 oracle is still compiled in (it dies at prompt 41). Every existing snapshot test, `insta` golden,
notation-details law, and the differential suite must pass **unchanged**. That is the whole acceptance criterion: the
observable output of `compile` is byte-identical, and the only thing that changed is where the facts live.

## Target

- `crates/musa-compiler/src/elaborate.rs`: `ScoreFact`/`Scope`/`FactKind`; `Marks`, `PayloadKind`, `VoicePayload`,
  `retie`, and the adapter's merge loop deleted; tie merging at elaboration; one timeline per piece.
- `crates/musa-compiler/src/project.rs` (new): the timeline → `ScoreSnapshot` projection, private to the crate. It is a
  module, not a method on the timeline: the kernel must not learn what a score is.
- `crates/musa-compiler/src/lower.rs`: unchanged (the oracle is frozen).
- `docs/kernel/06-surface-elaboration.md`: elaboration table and adapter contract rewritten for `ScoreFact`.
- `docs/kernel/08-open-questions.md`: Q3 and Q7 evidence; the §34 note that heterogeneity needed no constructor.
- `docs/kernel/09-performance.md`: this prompt's P1–P4 row.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-render -p musa-project
cargo clippy --all-targets -p musa-compiler -- -D warnings
cargo fmt --check
cargo insta test -p musa-compiler -p musa-render --unreferenced=reject   # no golden may change
for f in examples/*.musa; do cargo run -p musa-cli -- check "$f"; done
cargo bench -p musa-compiler   # append the row to docs/kernel/09-performance.md
grep -rn "struct Marks\|fn retie" crates/ | wc -l   # 0
```

Commit as `Elaborate every notated fact as a kernel occurrence`.

## Stop

- No kernel constructor, no kernel change of any kind. A perceived need is a spec repair committed first (§29, §34).
- `ScoreSnapshot`'s public shape does not change here — not one field, not one accessor. That is prompt 42.
- Key, meter, tempo, section, and harmony are **not** in scope; they are prompt 40.
- Do not delete the oracle or the differential suite; they are this prompt's safety net.
- No render, project, or desktop changes. If one is needed, the projection is not faithful — fix the projection.

## Repairs made while implementing

- **`Scope` has one variant, not three.** `Piece` and `Part` have no producer until prompt 40, and the workspace denies
  warnings, so a variant nobody constructs is a build failure — and AGENTS.md's "no public item without a caller" says
  the same thing more usefully. `Scope::Voice { part, voice }` is what this prompt needs; 40 adds the other two with
  their producers.
- **The tie flag is a field of `ScoreFact`, cleared before the fact leaves elaboration.** The prompt says nothing
  carries a tie flag afterwards, which is true — but merging needs lookahead, so the flag has to exist somewhere while
  the statement and its continuation are both in hand. It is documented as elaboration-only and the projection asserts
  it is `false` in debug builds.
- **Merging happens in `elaborate_items`, at every nesting level**, not once per voice. That is what makes the
  `retrograde` consequence true: an inner block's ties are already merged when the block is reversed. A tie that
  crosses a block boundary merges at the level containing both sides. The one thing that must stay at voice level is
  the *dangling* tie diagnostic — a tie at the end of a `slur` block continues into what follows the block, so
  complaining per level would reject valid music.
- **`grep -rn "struct Marks\|fn retie" crates/` returns 1, not 0**: `musa-render/src/plan.rs` has an unrelated
  `struct Marks` — the notation planner's per-event annotation index, which predates this prompt and has nothing to do
  with payload tags. Scoped to `crates/musa-compiler`, the check returns 0.
- **`cargo insta test --unreferenced=reject` is not runnable here** (`cargo-insta` is not installed). The stronger
  statement was checked instead and holds: `git status` shows **no snapshot file changed at all**, and all 430
  workspace tests pass. The canonical key was deliberately shaped so a note or rest with nothing written on it keys
  exactly as it did before, which is why the kernel normal-form goldens are untouched.
- **Motif note overrides count events, not facts.** `with { note 2 = g5; }` numbers positions by groups of occurrences
  sharing a span; once a `slur` inside a motif body is an occurrence of its own, that numbering had to skip non-event
  facts, or bracketing a motif would have silently renumbered its notes.
- **P2 on the large workload is +13% against prompt 38's row**, over the block's 10% gate and declared here as the rule
  requires, with the reasoning in `docs/kernel/09-performance.md`: the timeline holds more occurrences because regions
  are occurrences now, and elaboration groups statements to merge ties. The trade is that a slur is stored once instead
  of once per note it covers, and the `Vec<u32>` every note used to carry is gone.
