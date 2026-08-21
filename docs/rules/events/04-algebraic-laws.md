# 04 — Algebraic Laws

The laws of the event-track core, stated formally against the definitions of `03-denotational-semantics.md`. Every law
names the property test that must implement it in `crates/musa-events/tests/suite/laws.rs` (prompt 10); every non-law
names its counterexample test. Equality (`=`) throughout is **semantic equality**: equality of canonical normal forms
(`05-normalization.md`, N4), never of internal representation.

The test names below are the names after the prompt 127c rename; the ledger (`../../plan/clean-break-ledger.md`) lists
what each was called before.

`M, N, P, Q` range over finite event tracks in one coordinate; `d, e, f` over `ℚ≥0`; `r, s` over `ℚ>0`; `I ⊇ J ⊇ K` over
restriction windows; `f : A → B`, `g : B → C` over payload functions.

## `follow` laws

- **L1 — associativity.** `follow(follow(M, N), P) = follow(M, follow(N, P))`. Test: `follow_associativity`.
- **L2 — zero identity.** `follow((0, ∅), M) = M = follow(M, (0, ∅))`. Test: `follow_zero_identity`.
- **L3 — duration additivity.** `duration(follow(M, N)) = duration(M) + duration(N)`. Test:
  `follow_duration_additivity`.

## `together` laws

- **L4 — associativity.** `together(together(M, N), P) = together(M, together(N, P))`. Test: `together_associativity`.
- **L5 — commutativity.** `together(M, N) = together(N, M)`. Test: `together_commutativity`.
- **L6 — fixed-duration identity.** For `M` of duration `d`: `together(M, (d, ∅)) = M = together((d, ∅), M)`. `together`
  at fixed duration is a commutative monoid with identity `(d, ∅)`. Test: `together_fixed_duration_identity`.

## Ambient-extension laws *(struck: prompt 37)*

**L7** (extension identity and composition) and **L8** (simultaneity respects extension) were laws about `extend`, which
was removed at prompt 37 for want of a caller (`03-denotational-semantics.md` D4). Their tests went with them. No
behaviour changed: the laws described an operation nothing used.

## Payload-map laws

- **L9 — identity.** `map_payloads(id) = id`. Test: `map_identity`.
- **L10 — composition.** `map_payloads(g ∘ f) = map_payloads(g) ∘ map_payloads(f)`. Test: `map_composition`.
- **L11 — preserves `follow`.** `map_payloads(f)(follow(M, N)) = follow(map_payloads(f)(M), map_payloads(f)(N))`. Test:
  `map_preserves_follow`.
- **L12 — preserves `together`.** `map_payloads(f)(together(M, N)) = together(map_payloads(f)(M), map_payloads(f)(N))`.
  Test: `map_preserves_together`.

## Time-scaling laws

- **L13 — identity and composition.** `scale_1 = id`; `scale_r ∘ scale_s = scale_{r·s}`. Test: `scale_identity`,
  `scale_composition`.
- **L14 — preserves `follow`.** `scale_r(follow(M, N)) = follow(scale_r(M), scale_r(N))`. Test:
  `scale_preserves_follow`.
- **L15 — preserves `together`.** `scale_r(together(M, N)) = together(scale_r(M), scale_r(N))`. Test:
  `scale_preserves_together`.

## Restriction laws

- **L16 — identity.** `restrict_I = id` when `I` is the whole duration: every occurrence's visible span equals its whole
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
  and `covering(follow(M, N), d + t)` corresponds to `covering(N, t)` for `d = duration(M)` and `t` strictly past the
  seam. The queries commute with the algebra; the seam itself is excluded because `[s, e)` gives that instant to `N`
  alone, which is the first convention of D11. Test: `coverage_is_stable_under_time_transformation`.
- **L22 — prevailing is the last selected start.** For every `t`, `prevailing(M, t, σ)` equals `σ` applied to the
  canonically last occurrence with `start ≤ t` that `σ` accepts, and is `⊥` when there is none. Test:
  `prevailing_is_the_last_selected_start`.
- **L23 — prevailing is monotone in information.** Placing together a track whose `σ`-selected occurrences all start
  strictly after `t` does not change `prevailing(M, t, σ)`. This is the law that lets a projection build a piece
  incrementally and still answer correctly about its beginning. Test: `prevailing_ignores_facts_that_start_later`.

## Payload-shape laws

- **L24 — a curve-bearing occurrence transforms by its span alone.** For every operation, an occurrence carrying a
  `Progress` has a byte-identical payload afterwards, and `p(u)` at corresponding absolute instants agrees before and
  after `scale`, `follow`, `together`, and `restrict`. Continuous shape is therefore a payload *value* and costs the
  core no operation (`08-open-questions.md` Q4). Test: `a_curve_bearing_occurrence_transforms_by_its_span_alone`.

## The synchronized interchange law

- **L18 — synchronized interchange.** If `duration(M) = duration(N)` and `duration(P) = duration(Q)`, then

  ```text
  follow(together(M, N), together(P, Q)) = together(follow(M, P), follow(N, Q))
  ```

 . Musically: two voices across two synchronized sections can be built section-wise then followed, or voice-wise then
  placed together; the temporal facts are identical. Test: `synchronized_interchange`.

  **The synchronization conditions matter.** When `duration(M) ≠ duration(N)`, the equation fails in general. Counterexample
  test: `interchange_fails_without_synchronization` — exhibit `M, N, P, Q` with unequal section durations where the two
  sides differ (the shorter section's voice B material starts under voice A's still-sounding section on one side, and
  after it on the other).

## Non-laws (tested as counterexamples)

- **X1 — `together` is not idempotent.** `together(M, M) ≠ M` whenever `M` has a non-empty occurrence multiset:
  multiplicity doubles (D3). Two performers playing the same note must not collapse. Test: `together_not_idempotent`.
- **X2 — no distributivity.** `follow(M, together(N, P)) ≠ together(follow(M, N), follow(M, P))` in general: the left
  side contains one copy of `M`, the right side two. Test: `follow_does_not_distribute_over_together`.
- **X3 — no monadic join.** There is no operation in the core that flattens `EventTrack<C, EventTrack<C,A>>`; any
  function claiming to be `join` must pick one of several musically distinct meanings. Not a runtime test — a design
  assertion recorded here so no one adds the operation casually. Enforced by code review against the public surface.

## Meta-law

- **L19 — semantic equality is quotient-correct.** For all constructors and operations above, semantic equality
  (`05-normalization.md`, N4) is a congruence: replacing an argument by a semantically equal one preserves semantic
  equality of the result. Test: `semantic_equality_is_congruence`.

## What is not a law here

The machine laws M1–M8 are not stated in this file and do not follow from anything in it. A machine is the *other* core
value, its equality is a different relation, and `../across-stages/03-machine-calculus.md` §7 owns both. In particular
there is no law relating `follow` on tracks to any composition of machines: scheduling `follow(M, N)` and chaining two
scheduled machines are different operations with different results, and §7 says why.
