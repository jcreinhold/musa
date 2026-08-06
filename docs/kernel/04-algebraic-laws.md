# 04 — Algebraic Laws

> **Status: candidate** — provisional until prompts 10–11 pass; see `00-purpose.md`.

The laws of the temporal kernel, stated formally against the definitions of `03-denotational-semantics.md`. Every law
names the property test that must implement it in `crates/musa-kernel/tests/laws.rs` (prompt 10); every non-law names
its counterexample test. Equality (`=`) throughout is **semantic equality**: equality of canonical normal forms
(`05-normalization.md`, N4), never of internal representation.

`M, N, P, Q` range over finite timelines; `d, e, f` over `ℚ≥0`; `r, s` over `ℚ>0`; `I ⊇ J ⊇ K` over restriction
windows; `f : A → B`, `g : B → C` over payload functions.

## Sequence laws

- **L1 — associativity.** `(M ; N) ; P = M ; (N ; P)`.
  Test: `seq_associativity`.
- **L2 — zero identity.** `0 ; M = M = M ; 0`, where `0 = (0, ∅)`.
  Test: `seq_zero_identity`.
- **L3 — duration additivity.** `duration(M ; N) = duration(M) + duration(N)`.
  Test: `seq_duration_additivity`.

## Overlay laws

- **L4 — associativity.** `(M ⊕ N) ⊕ P = M ⊕ (N ⊕ P)`.
  Test: `overlay_associativity`.
- **L5 — commutativity.** `M ⊕ N = N ⊕ M`.
  Test: `overlay_commutativity`.
- **L6 — fixed-duration identity.** For `M` with extent `d`: `M ⊕ (d, ∅) = M = (d, ∅) ⊕ M`. Overlay at fixed duration
  is a commutative monoid with identity `(d, ∅)` (§8).
  Test: `overlay_fixed_duration_identity`.

## Ambient-extension laws

- **L7 — identity and composition.** `extend_{d,d} = id`, and for `d ≤ e ≤ f`:
  `extend_{e,f} ∘ extend_{d,e} = extend_{d,f}` (§9).
  Test: `extend_identity`, `extend_composition`.
- **L8 — overlay respects extension.** For `d ≤ e`: `(extend_{d,e} M) ⊕ N = M ⊕ N` whenever `extent(N) = e` —
  extending the shorter argument before overlaying changes nothing (§9).
  Test: `overlay_respects_extension`.

## Payload-map laws

- **L9 — identity.** `Timeline(id) = id`.
  Test: `map_identity`.
- **L10 — composition.** `Timeline(g ∘ f) = Timeline(g) ∘ Timeline(f)` (§13).
  Test: `map_composition`.
- **L11 — preserves sequence.** `Timeline(f)(M ; N) = Timeline(f)(M) ; Timeline(f)(N)`.
  Test: `map_preserves_sequence`.
- **L12 — preserves overlay.** `Timeline(f)(M ⊕ N) = Timeline(f)(M) ⊕ Timeline(f)(N)`.
  Test: `map_preserves_overlay`.

## Time-scaling laws

- **L13 — identity and composition.** `scale_1 = id`; `scale_r ∘ scale_s = scale_{r·s}` (§14).
  Test: `scale_identity`, `scale_composition`.
- **L14 — preserves sequence.** `scale_r(M ; N) = scale_r(M) ; scale_r(N)`.
  Test: `scale_preserves_sequence`.
- **L15 — preserves overlay.** `scale_r(M ⊕ N) = scale_r(M) ⊕ scale_r(N)`.
  Test: `scale_preserves_overlay`.

## Restriction laws

- **L16 — identity.** `restrict_I = id` when `I` is the whole extent: every occurrence's visible span equals its
  whole span, and no occurrence is dropped.
  Test: `restrict_identity`.
- **L17 — nested composition.** For `K ⊆ J ⊆ I`: `restrict_K(restrict_J(M)) = restrict_K(M)` — the observations
  coincide, with **whole spans preserved**: restricting twice never moves an occurrence's origin claim (§17).
  Test: `restrict_composition`.

## The synchronized interchange law

- **L18 — synchronized interchange.** If `duration(M) = duration(N)` and `duration(P) = duration(Q)`, then

  ```text
  (M ⊕ N) ; (P ⊕ Q) = (M ; P) ⊕ (N ; Q)
  ```

  (§11). Musically: two voices across two synchronized sections can be built section-wise then sequenced, or
  voice-wise then overlaid; the temporal facts are identical.
  Test: `synchronized_interchange`.

  **The synchronization conditions matter.** When `duration(M) ≠ duration(N)`, the equation fails in general.
  Counterexample test: `interchange_fails_without_synchronization` — exhibit `M, N, P, Q` with unequal section
  durations where the two sides differ (the shorter section's voice B material starts under voice A's still-sounding
  section on one side, and after it on the other).

## Non-laws (tested as counterexamples)

- **X1 — overlay is not idempotent.** `M ⊕ M ≠ M` whenever `M` has a non-empty occurrence multiset: multiplicity
  doubles (§8, D3). Two performers playing the same note must not collapse.
  Test: `overlay_not_idempotent`.
- **X2 — no distributivity.** `M ; (N ⊕ P) ≠ (M ; N) ⊕ (M ; P)` in general: the left side contains one copy of `M`,
  the right side two (§10).
  Test: `sequence_does_not_distribute_over_overlay`.
- **X3 — no monadic join.** There is no operation in the kernel that flattens `Timeline[Timeline[A]]`; any function
  claiming to be `join` must pick one of several musically distinct meanings (§16). Not a runtime test — a design
  assertion recorded here so no one adds the operation casually. Enforced by code review against the public surface.

## Meta-law

- **L19 — semantic equality is quotient-correct.** For all constructors and operations above, semantic equality
  (`05-normalization.md`, N4) is a congruence: replacing an argument by a semantically equal one preserves semantic
  equality of the result.
  Test: `semantic_equality_is_congruence`.
