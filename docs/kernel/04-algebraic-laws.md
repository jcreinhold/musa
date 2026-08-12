# 04 — Algebraic Laws

The laws of the temporal kernel, stated formally against the definitions of `03-denotational-semantics.md`. Every law
names the property test that must implement it in `crates/musa-kernel/tests/laws.rs` (prompt 10); every non-law names
its counterexample test. Equality (`=`) throughout is **semantic equality**: equality of canonical normal forms
(`05-normalization.md`, N4), never of internal representation.

`M, N, P, Q` range over finite timelines; `d, e, f` over `ℚ≥0`; `r, s` over `ℚ>0`; `I ⊇ J ⊇ K` over restriction windows;
`f : A → B`, `g : B → C` over payload functions.

## Sequence laws

- **L1 — associativity.** `(M ; N) ; P = M ; (N ; P)`. Test: `seq_associativity`.
- **L2 — zero identity.** `0 ; M = M = M ; 0`, where `0 = (0, ∅)`. Test: `seq_zero_identity`.
- **L3 — duration additivity.** `duration(M ; N) = duration(M) + duration(N)`. Test: `seq_duration_additivity`.

## Overlay laws

- **L4 — associativity.** `(M ⊕ N) ⊕ P = M ⊕ (N ⊕ P)`. Test: `overlay_associativity`.
- **L5 — commutativity.** `M ⊕ N = N ⊕ M`. Test: `overlay_commutativity`.
- **L6 — fixed-duration identity.** For `M` with extent `d`: `M ⊕ (d, ∅) = M = (d, ∅) ⊕ M`. Overlay at fixed duration is
  a commutative monoid with identity `(d, ∅)`. Test: `overlay_fixed_duration_identity`.

## Ambient-extension laws *(struck: prompt 37)*

**L7** (extension identity and composition) and **L8** (overlay respects extension) were laws about `extend`, which was
removed at prompt 37 for want of a caller (docs/kernel/03 D4). Their tests went with them. No behaviour changed: the
laws described an operation nothing used.

## Payload-map laws

- **L9 — identity.** `Timeline(id) = id`. Test: `map_identity`.
- **L10 — composition.** `Timeline(g ∘ f) = Timeline(g) ∘ Timeline(f)`. Test: `map_composition`.
- **L11 — preserves sequence.** `Timeline(f)(M ; N) = Timeline(f)(M) ; Timeline(f)(N)`. Test: `map_preserves_sequence`.
- **L12 — preserves overlay.** `Timeline(f)(M ⊕ N) = Timeline(f)(M) ⊕ Timeline(f)(N)`. Test: `map_preserves_overlay`.

## Time-scaling laws

- **L13 — identity and composition.** `scale_1 = id`; `scale_r ∘ scale_s = scale_{r·s}`. Test: `scale_identity`,
  `scale_composition`.
- **L14 — preserves sequence.** `scale_r(M ; N) = scale_r(M) ; scale_r(N)`. Test: `scale_preserves_sequence`.
- **L15 — preserves overlay.** `scale_r(M ⊕ N) = scale_r(M) ⊕ scale_r(N)`. Test: `scale_preserves_overlay`.

## Restriction laws

- **L16 — identity.** `restrict_I = id` when `I` is the whole extent: every occurrence's visible span equals its whole
  span, and no occurrence is dropped. Test: `restrict_identity`.
- **L17 — composition.** For **any** windows `J` and `K` that meet: `restrict_K(restrict_J(M)) = restrict_{J ∩ K}(M)`;
  windows that do not meet observe nothing. Narrowing an observation intersects the windows, so the law holds without a
  nesting precondition (it specializes to `restrict_K(restrict_J(M)) = restrict_K(M)` when `K ⊆ J`). Whole spans are
  **preserved**: restricting twice never moves an occurrence's origin claim. Test: `restrict_composition`,
  `restrict_composition_strictly_nested`.

## Query laws

- **L20 — coverage agrees with observation.** For every window `I` containing `t`, the occurrences of `restrict_I(M)`
  whose whole support contains `t` are exactly `covering(M, t)`, in the same canonical order. The two ways of asking
  what is in force cannot disagree. Test: `coverage_agrees_with_observation`.
- **L21 — coverage is stable under time transformation.** `covering(scale_r(M), r·t)` corresponds to `covering(M, t)`,
  and `covering(M ; N, d + t)` corresponds to `covering(N, t)` for `d = extent(M)` and `t` strictly past the seam. The
  queries commute with the algebra; the seam itself is excluded because `[s, e)` gives that instant to `N` alone, which
  is the first convention of D11. Test: `coverage_is_stable_under_time_transformation`.
- **L22 — prevailing is the last selected start.** For every `t`, `prevailing(M, t, σ)` equals `σ` applied to the
  canonically last occurrence with `start ≤ t` that `σ` accepts, and is `⊥` when there is none. Test:
  `prevailing_is_the_last_selected_start`.
- **L23 — prevailing is monotone in information.** Overlaying a timeline whose `σ`-selected occurrences all start
  strictly after `t` does not change `prevailing(M, t, σ)`. This is the law that lets a projection build a piece
  incrementally and still answer correctly about its beginning. Test: `prevailing_ignores_facts_that_start_later`.

## Payload-shape laws

- **L24 — a curve-bearing occurrence transforms by its span alone.** For every operation, an occurrence carrying a
  `Progress` has a byte-identical payload afterwards, and `p(u)` at corresponding absolute instants agrees before and
  after `scale`, `sequence`, `overlay`, and `restrict`. Continuous shape is therefore a payload *value* and costs the
  kernel no operation (`08-open-questions.md` Q4). Test: `a_curve_bearing_occurrence_transforms_by_its_span_alone`.

## The synchronized interchange law

- **L18 — synchronized interchange.** If `duration(M) = duration(N)` and `duration(P) = duration(Q)`, then

  ```text
  (M ⊕ N) ; (P ⊕ Q) = (M ; P) ⊕ (N ; Q)
  ```

 . Musically: two voices across two synchronized sections can be built section-wise then sequenced, or
  voice-wise then overlaid; the temporal facts are identical.
  Test: `synchronized_interchange`.

  **The synchronization conditions matter.** When `duration(M) ≠ duration(N)`, the equation fails in general.
  Counterexample test: `interchange_fails_without_synchronization` — exhibit `M, N, P, Q` with unequal section
  durations where the two sides differ (the shorter section's voice B material starts under voice A's still-sounding
  section on one side, and after it on the other).

## Non-laws (tested as counterexamples)

- **X1 — overlay is not idempotent.** `M ⊕ M ≠ M` whenever `M` has a non-empty occurrence multiset: multiplicity doubles
  (D3). Two performers playing the same note must not collapse. Test: `overlay_not_idempotent`.
- **X2 — no distributivity.** `M ; (N ⊕ P) ≠ (M ; N) ⊕ (M ; P)` in general: the left side contains one copy of `M`, the
  right side two. Test: `sequence_does_not_distribute_over_overlay`.
- **X3 — no monadic join.** There is no operation in the kernel that flattens `Timeline[Timeline[A]]`; any function
  claiming to be `join` must pick one of several musically distinct meanings. Not a runtime test — a design assertion
  recorded here so no one adds the operation casually. Enforced by code review against the public surface.

## Meta-law

- **L19 — semantic equality is quotient-correct.** For all constructors and operations above, semantic equality
  (`05-normalization.md`, N4) is a congruence: replacing an argument by a semantically equal one preserves semantic
  equality of the result. Test: `semantic_equality_is_congruence`.
