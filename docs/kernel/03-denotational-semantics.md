# 03 — Denotational Semantics

> **Status: candidate** — provisional until prompts 10–11 pass; see `00-purpose.md`.

The denotation of every finite kernel composition, and the definition of every operation. Laws these definitions
satisfy are in `04-algebraic-laws.md`; how they are normalized and compared is in `05-normalization.md`.

## D0 — The denotation

For a payload type `A`, the denotation of a finite kernel composition is:

```text
⟦ composition ⟧ = (d, E)
```

where `d ∈ ℚ≥0` is the extent of the ambient musical-time interval `[0, d]`, and `E` is a **finite multiset** of
occurrences `(s, e, a)` with `0 ≤ s ≤ e ≤ d` and `a : A` (course correction §3).

Two consequences, both load-bearing:

- **Time is ambient** (§2). `E` may be empty; `[0, d]` exists regardless. There is no `Silence(d)` or `Rest(d)`
  object: an uncovered region *is* silence with respect to that payload type, by absence.
- **Duration is temporal support, not a payload field** (§3). A `Note` payload describes *what* is sounding; the
  occurrence's `[s, e]` describes *when* it sounds. Payloads that smuggle their own duration field invite
  inconsistency and are rejected at the payload-schema layer (`02-static-semantics.md`, K5, by convention of the
  score adapter).

## D1 — `timeline`

```text
⟦ timeline d { occurrence a₁ from s₁ to e₁; … occurrence aₙ from sₙ to eₙ; } ⟧
    = (d, { (sᵢ, eᵢ, aᵢ) | 1 ≤ i ≤ n })      provided ∀i. 0 ≤ sᵢ ≤ eᵢ ≤ d
```

Multiple equal occurrences are allowed (multiset, §6). The extent may exceed every `eᵢ`: the tail is simply
uncovered — ambient time, not padding (§9).

## D2 — `sequence` (temporal succession)

For `M = (d, E)` and `N = (e, F)`:

```text
M ; N = (d + e, E ⊎ τ_d(F))    where τ_d(s, t, a) = (d + s, d + t, a)
```

The second timeline is translated by the duration of the first (§7). `;` generalizes pointwise to `n` arguments by
left- or right-fold — associativity (L1) makes the choice immaterial.

The empty timeline `0 = (0, ∅)` is the two-sided identity (L2). Note the difference from `(d, ∅)` for `d > 0`:
sequencing after `(d, ∅)` *does* shift what follows — empty timelines have extent, and extent is real.

## D3 — `overlay` (simultaneous presence)

```text
M ⊕ N = (max(d, e), E ⊎ F)
```

Both occurrence multisets live in the union's ambient region (§8). **Nothing is inserted into the uncovered portion
of the shorter timeline** — it simply lives inside a larger ambient region.

- Commutative (L5) and associative (L4).
- **Not idempotent** (§8): `M ⊕ M ≠ M` whenever `E ≠ ∅` — multiplicity doubles. `overlay` is a union of multisets,
  never a union of sets.
- At any fixed extent `d`, overlay forms a commutative monoid with identity `(d, ∅)` (L6).

## D4 — Ambient extension

For `d ≤ e`:

```text
extend_{d,e} (d, E) = (e, E)
```

No occurrence is introduced, moved, or removed (§9). This is the correct reading of "making a temporal region
longer" — categorically different from "padding with silence," which would require a silence object the kernel does
not have (§2).

## D5 — Time scaling (an external action)

Positive rational scaling `r ∈ ℚ>0` acts on timelines (§14):

```text
scale_r (d, E) = (r·d, { (r·s, r·e, a) | (s, e, a) ∈ E })
```

Scaling is exact and preserves both `sequence` and `overlay` (L13–L15). Augmentation and diminution in the surface
language **evaluate into** ordinary kernel timelines through this action; no permanent `Stretch` node exists in the
normalized representation (§14).

## D6 — Restriction (observation, not mutation)

Restricting `M = (d, E)` to a window `I = [i, j]` with `0 ≤ i ≤ j ≤ d` yields an **observation**: for each occurrence
`(s, e, a)` with `[s, e] ∩ [i, j] ≠ ∅`, report

```text
whole_span    = [s, e]
visible_span  = [max(s, i), min(e, j)]
payload       = a
```

The observation knows both spans (§17): an occurrence over `[3, 6)` observed through `[5, 8)` has whole support
`[3, 6)` and visible support `[5, 6)` — cropping never claims the occurrence began at 5. The normalized serialized
kernel stores whole spans only; the whole/visible pair is the observation-time representation.

## D7 — Payload mapping (functorial, not a temporal primitive)

Given `f : A → B`:

```text
Timeline(f) (d, E) = (d, { (s, e, f(a)) | (s, e, a) ∈ E })
```

Payload mapping preserves temporal support exactly, is functorial (L9–L10), and preserves `sequence` and `overlay`
(L11–L12) (§13). Transformations such as transposition — which touch payload information only — are payload maps
evaluated before normalization. **No generic opaque `Transform(...)` kernel node exists**; source provenance preserves
how material was produced, the normalized kernel preserves what it means (§13, §20).

## D8 — Delay (derived)

A delayed timeline is ambient extent before its occurrences (§15):

```text
delay_b(M) = (b, ∅) ; M
```

No silence object is involved — `(b, ∅)` is an empty ambient region, sequenced before `M`. Delay therefore needs no
primitive constructor.

## D9 — Explicitly not defined

- **No `join`.** `Timeline[Timeline[A]]` has no canonical flattening: begin-at-onset, stretch-to-fit, crop, repeat,
  and preserve-inner-duration are genuinely different musical operations (§16). Specific higher-level abstractions may
  define their own; the universal kernel does not.
- **No distributivity.** `M ; (N ⊕ P) ≠ (M ; N) ⊕ (M ; P)` in general — the left side has one copy of `M`, the right
  side two (§10). The kernel is not a semiring, and no law is claimed that would make it one.
- **No infinity.** Every denotation is finite in extent and in occurrence count. Patterns and loops produce coherent
  finite observations (§18); they are not kernel values.
