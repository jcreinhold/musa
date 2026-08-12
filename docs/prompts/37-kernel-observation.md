---
id: 37
slug: kernel-observation
status: done
depends_on: [12]
phase: 3
---

# Make the Kernel Surface Tell the Truth About Its Own Algebra

## Task

Repair three places where `musa-kernel`'s public surface and `docs/kernel/` disagree with each other or with the code:
`restrict` returns a type that cannot be restricted again although L17 says restriction composes; `Timeline::extend` is
public with no caller; and two specification rows record behaviour the implementation does not have. Nothing outside
`crates/musa-kernel/` and `docs/kernel/` changes. This prompt opens the 37–48 consolidation block and adds its rows to
the prompt README.

## Read

- `docs/kernel/03-denotational-semantics.md` D6 (restriction reports whole and visible spans), D4 (ambient extension).
- `docs/kernel/04-algebraic-laws.md` L7–L8 (extension laws), L16–L17 (restriction identity and *composition*).
- `docs/kernel/08-open-questions.md` prompt-09 and prompt-10 log entries — they record a D6 refinement "updated when the
  candidate banner comes off (prompt 12)". The banner came off; D6 was not updated.
- `crates/musa-kernel/src/timeline.rs` (`RestrictedView`, `Timeline::extend`, `Timeline::restrict`),
  `crates/musa-kernel/tests/laws.rs` (`restrict_view` — the helper that exists *because* the API does not compose).
- `docs/kernel/03-denotational-semantics.md` (observation never moves an occurrence's origin claim), §34 (smallest complete basis — this
  prompt removes a constructor rather than adding one).
- PoSD ch. 10 "define errors out of existence" and the red flag *information leakage*: the visible span is currently
  stored next to the whole span, so two facts that must agree are kept in two places.

## Design

### Observation composes, and stores no derived state

`RestrictedView` and `ObservedOccurrence` are replaced by one type:

```rust
/// A timeline observed through a window (D6). Borrowed, cheap, composable.
pub struct Observation<'a, A> { /* window, source extent, selected occurrences */ }

impl<A> Timeline<A> {
    /// Observe through `window`; occurrences keep their whole spans (§17).
    pub fn restrict(&self, window: Span) -> Observation<'_, A>;
}

impl<'a, A> Observation<'a, A> {
    pub fn window(&self) -> Span;
    /// Narrow to `window ∩ self.window()`. L17 holds by construction.
    pub fn restrict(&self, window: Span) -> Observation<'a, A>;
    /// Each occurrence with its visible span `whole ∩ window`, in canonical order.
    pub fn observed(&self) -> impl Iterator<Item = (Span, &'a Occurrence<A>)>;
    pub fn is_empty(&self) -> bool;
}
```

Three decisions, each with a reason:

- **The visible span is computed, never stored.** It is a function of the whole span and the window; storing it lets the
  two disagree and forced the test helper to recompute it anyway. `observed()` yields the pair; `Occurrence::span()`
  remains the whole span. This deletes `ObservedOccurrence` entirely.
- **Narrowing intersects rather than requiring containment.** L17 is stated for `K ⊆ J ⊆ I`; making `restrict` intersect
  makes the operation total, so there is no precondition for a caller to violate and no error to return. L17 then holds
  for *all* windows, and its property test states the stronger law.
- **The observation carries the source extent**, because the point-at-the-end rule below is a property of the timeline
  being observed, not of the window alone; without it, narrowing a full-extent observation would silently drop a point
  occurrence that the unnarrowed one reported.

### Delete `Timeline::extend`

`extend` has no caller: `sequence` and `overlay` compute extents themselves, and no surface construct asks a timeline to
grow without adding material. Deleting it removes a public constructor, an error variant
(`KernelError::ShrinkingExtension`), and laws L7–L8 from the maintained set. Under §34 this is the correct direction:
the basis shrinks when the evidence says a construct is not needed. Record the deletion in `08-open-questions.md`'s
implementation log with the argument, so re-adding it requires new evidence rather than taste.

Ambient extension as a *concept* stays — it is what `overlay` does to the shorter argument and what `(d, ∅)` means. It
is the standalone operation that goes.

### Specification repairs, committed with the code

- **D6** gains the two refinements the log already records: a point occurrence at `s` is visible through `[i, j)` when
  `s ∈ [i, j)`, and a window whose end equals the observed timeline's extent is closed at its right end. State the
  second as the rule it is — "the final instant of a timeline is observable" — rather than as an exception, and note
  that L16 follows from it.
- **D4 and L7–L8** are struck, with a one-line note that the operation was removed at this prompt and why.
- **`06-surface-elaboration.md`'s `rest` row** still says "No occurrence"; since prompt 11 a `rest` elaborates to a
  `Rest` payload occurrence. Repair the row to match the code and the log entry. AGENTS.md forbids leaving this
  disagreement standing.

## Target

- `crates/musa-kernel/src/timeline.rs`, `occurrence.rs`, `error.rs`: `Observation` replaces `RestrictedView` and
  `ObservedOccurrence`; `extend` and `ShrinkingExtension` deleted; `lib.rs` facade list updated.
- `crates/musa-kernel/tests/laws.rs`: `restrict_view` helper deleted; L16/L17 stated through the public API; L17
  generalized to arbitrary windows; `extend_identity`, `extend_composition`, `overlay_respects_extension` deleted.
- `docs/kernel/03`, `04`, `06`, `08`: the repairs above.

## Check

```sh
cargo nextest run -p musa-kernel
cargo clippy --all-targets -p musa-kernel -- -D warnings
cargo fmt --check
cargo build --workspace   # nothing outside the kernel crate referenced extend or RestrictedView
grep -rn "RestrictedView\|ObservedOccurrence\|ShrinkingExtension" crates/ | wc -l   # 0
```

Commit as `Make kernel observation composable and drop ambient extension`.

## Stop

- No change to `sequence`, `overlay`, `timeline`, `scale`, `map_payload`, or normalization.
- No new kernel constructors. This prompt only removes and repairs.
- No compiler, render, project, or desktop changes — if one is forced, the prompt is mis-scoped: stop and repair it.
- Do not "fix" `Canonical`'s `String` keys here; that is prompt 43, and it needs prompt 38's measurement first.

## Repairs made while implementing

- Narrowing to a window that does not meet the current one is an explicit empty case, not a clipped span. `Span::clip`
  is only correct where the two overlap, and a *fabricated* empty span can land on the extent — where the
  point-at-the-end rule would then read an occurrence that neither window shows. The proptest found this immediately
  once L17 was generalized to arbitrary windows, which is the argument for generalizing it.
- `docs/kernel/02-static-semantics.md` K3 also stated the extension rule; the prompt named 03, 04, and 06 but not 02. It
  is struck with the same note rather than left contradicting the code.
- `restrict_composition` is now stated as `restrict_K ∘ restrict_J = restrict_{J ∩ K}` over two independently generated
  windows, with "observes nothing" as the law for windows that do not meet; the strictly-nested case is kept as a worked
  example (`restrict_composition_strictly_nested`) since it is the case the law was originally written for.
- Added `the_final_instant_survives_narrowing`: the point-at-the-extent rule is the reason `Observation` carries the
  source extent, and nothing else pinned that narrowing preserves it.
