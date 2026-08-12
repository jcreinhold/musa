# 03 — Denotational Semantics

The denotation of every finite kernel composition, and the definition of every operation. Laws these definitions satisfy
are in `04-algebraic-laws.md`; how they are normalized and compared is in `05-normalization.md`.

## D0 — The denotation

For a payload type `A`, the denotation of a finite kernel composition is:

```text
⟦ composition ⟧ = (d, E)
```

where `d ∈ ℚ≥0` is the extent of the ambient musical-time interval `[0, d]`, and `E` is a **finite multiset** of
occurrences `(s, e, a)` with `0 ≤ s ≤ e ≤ d` and `a : A`.

Two consequences, both load-bearing:

- **Time is ambient**. `E` may be empty; `[0, d]` exists regardless. There is no `Silence(d)` or `Rest(d)` object: an
  uncovered region *is* silence with respect to that payload type, by absence.
- **Duration is temporal support, not a payload field**. A `Note` payload describes *what* is sounding; the occurrence's
  `[s, e]` describes *when* it sounds. Payloads that smuggle their own duration field invite inconsistency and are
  rejected at the payload-schema layer (`02-static-semantics.md`, K5, by convention of the score adapter).

## D1 — `timeline`

```text
⟦ timeline d { occurrence a₁ from s₁ to e₁; … occurrence aₙ from sₙ to eₙ; } ⟧
    = (d, { (sᵢ, eᵢ, aᵢ) | 1 ≤ i ≤ n })      provided ∀i. 0 ≤ sᵢ ≤ eᵢ ≤ d
```

Multiple equal occurrences are allowed (multiset, §6). The extent may exceed every `eᵢ`: the tail is simply uncovered —
ambient time, not padding.

## D2 — `sequence` (temporal succession)

For `M = (d, E)` and `N = (e, F)`:

```text
M ; N = (d + e, E ⊎ τ_d(F))    where τ_d(s, t, a) = (d + s, d + t, a)
```

The second timeline is translated by the duration of the first. `;` generalizes pointwise to `n` arguments by left- or
right-fold — associativity (L1) makes the choice immaterial.

The empty timeline `0 = (0, ∅)` is the two-sided identity (L2). Note the difference from `(d, ∅)` for `d > 0`:
sequencing after `(d, ∅)` *does* shift what follows — empty timelines have extent, and extent is real.

## D3 — `overlay` (simultaneous presence)

```text
M ⊕ N = (max(d, e), E ⊎ F)
```

Both occurrence multisets live in the union's ambient region. **Nothing is inserted into the uncovered portion of the
shorter timeline** — it simply lives inside a larger ambient region.

- Commutative (L5) and associative (L4).
- **Not idempotent**: `M ⊕ M ≠ M` whenever `E ≠ ∅` — multiplicity doubles. `overlay` is a union of multisets, never a
  union of sets.
- At any fixed extent `d`, overlay forms a commutative monoid with identity `(d, ∅)` (L6).

## D4 — Ambient extension *(struck: prompt 37)*

`extend_{d,e} (d, E) = (e, E)` was a standalone operation. It is removed: nothing called it. `sequence` and `overlay`
compute their extents themselves, and no surface construct asks a timeline to grow without adding material, so under §34
the basis shrinks. Re-adding it needs new evidence, not taste.

Ambient extension as a *concept* stays, and is exactly what it always was: `(d, ∅)` is silence by absence, and `overlay`
of unequal extents takes the maximum without padding the shorter argument. What went is the operation that only ever
restated that.

## D5 — Time scaling (an external action)

Positive rational scaling `r ∈ ℚ>0` acts on timelines:

```text
scale_r (d, E) = (r·d, { (r·s, r·e, a) | (s, e, a) ∈ E })
```

Scaling is exact and preserves both `sequence` and `overlay` (L13–L15). Augmentation and diminution in the surface
language **evaluate into** ordinary kernel timelines through this action; no permanent `Stretch` node exists in the
normalized representation.

## D6 — Restriction (observation, not mutation)

Restricting `M = (d, E)` to a window `I = [i, j)` yields an **observation**: for each occurrence `(s, e, a)` visible
through `I`, report

```text
whole_span    = [s, e)
visible_span  = [max(s, i), min(e, j))
payload       = a
```

An occurrence is visible through `[i, j)` when:

- it has positive duration and `[s, e) ∩ [i, j) ≠ ∅`; or
- it is a **point** (`s = e`) and `s ∈ [i, j)`; a point occurrence is otherwise unobservable through every window, which
  the interval-intersection phrasing does not intend; or
- **the final instant of a timeline is observable**: it is a point at `s = j = d`. A window that ends at the observed
  timeline's extent is closed at its right end. L16 follows from this rule — without it, observing at the full extent
  `[0, d)` would drop an occurrence at `d` and so would not be the identity.

The third rule is a fact about the timeline being observed, not about the window: an observation therefore carries the
extent it was taken from, and narrowing it cannot silently drop an occurrence the wider observation reported (L17).

The observation knows both spans: an occurrence over `[3, 6)` observed through `[5, 8)` has whole support `[3, 6)` and
visible support `[5, 6)` — cropping never claims the occurrence began at 5. The visible span is a *function* of the
whole span and the window, so it is computed on demand rather than stored beside it; the normalized serialized kernel
stores whole spans only.

Restriction is total and composes: narrowing an observation to `K` intersects the windows, so there is no containment
precondition and no error (L17). Windows that do not meet observe nothing — which is *not* the same as a degenerate
window sitting at the extent, where the rule above applies.

## D7 — Payload mapping (functorial, not a temporal primitive)

Given `f : A → B`:

```text
Timeline(f) (d, E) = (d, { (s, e, f(a)) | (s, e, a) ∈ E })
```

Payload mapping preserves temporal support exactly, is functorial (L9–L10), and preserves `sequence` and `overlay`
(L11–L12). Transformations such as transposition — which touch payload information only — are payload maps evaluated
before normalization. **No generic opaque `Transform(...)` kernel node exists**; source provenance preserves how
material was produced, the normalized kernel preserves what it means.

## D8 — Delay (derived)

A delayed timeline is ambient extent before its occurrences:

```text
delay_b(M) = (b, ∅) ; M
```

No silence object is involved — `(b, ∅)` is an empty ambient region, sequenced before `M`. Delay therefore needs no
primitive constructor.

## Progress — continuous shape as a payload value (`08-open-questions.md` Q4)

A hairpin is the one musical fact the kernel could not say. The timeline recorded *that* a crescendo spans a region;
**how** it grows existed only inside the performance lowerer, invented there and thrown away — so an interchange file
could not carry it, and every future continuous control (`gliss`, *rit.*, a filter sweep, a fader move) would have
arrived with nowhere to live.

The answer is a **value**, not an operation:

```text
Progress = a piecewise-linear, monotone map  p : [0, 1] → [0, 1]
           given by breakpoints (u₀, v₀) … (uₙ, vₙ),
           u strictly increasing, u₀ = 0, uₙ = 1, v non-decreasing in [0, 1]
```

`u` is **normalized local time**: the fraction of the *occurrence's own span* that has elapsed. `v` is a unit-free
fraction of the distance covered. `Progress` says how far along — never how loud, how fast, or how high. What the
fraction means belongs to the consuming layer: a performance profile maps a hairpin's endpoints to amplitudes, a tempo
map maps them to seconds. A dynamic marking is still not a decibel.

### The span-alone theorem

> **A curve-bearing occurrence transforms by its span alone.** For every kernel operation `op ∈ {sequence, overlay,
> scale, restrict, map_payload}`, the `Progress` in an occurrence's payload is byte-identical before and after, and
> `p(u)` evaluated at corresponding absolute instants agrees before and after.

This is why the design is safe, and it is a consequence of normalizing `u` rather than a property that had to be
arranged. `scale r` multiplies the span; `u` is a fraction of the span, so it is unchanged. `sequence` translates; same.
`overlay` does not touch spans. `map_payload` never inspects a payload at all.

An **absolute-time** curve would have to be rewritten by `scale` and by `sequence` — which means the kernel would have
to look inside payloads to transform them. That is precisely the §12 violation the kernel exists to prevent, and it
would break the functor laws L11–L15. The theorem is tested as **L24**.

So Q4's answer is that the kernel needed a value, not an operation. That is the third piece of §34 evidence — alongside
prompt 39's total timeline and prompt 44's queries — that the operation set is complete.

### Shape is normative; sampling policy is not

- The **shape** — "linear from 0 to 1 across this hairpin" — is a fact about the piece. It lives in the timeline, it
  serializes, it contributes to the semantic hash, and two implementations must agree on it.
- The **sampling policy** — evaluate once per notated event at `u = index / (count − 1)`, or once per onset at
  `u = (onset − start) / width` — is the consuming layer's interpretation, and consumers may differ. See
  `07-backend-contract.md`.

### What `Progress` deliberately cannot express

- **Steps.** Piecewise-*linear* only; no jump discontinuities. A sudden change is a fact at a point, the timeline
  already has one, and D11 already answers what is in force there. Saying one thing two ways is the complecting this
  block exists to remove.
- **Units.** Unit-free fractions in `[0, 1]`. A curve "in decibels" is a consumer's mapping of the endpoints.
- **Non-monotone shapes.** Vibrato and an LFO are periodic, not progress. A different construct, with its own name and
  its own evidence, if one is ever wanted.
- **An easing catalogue.** No `ease_in`, no exponential, no Bézier: breakpoints approximate any of them, and a catalogue
  is a vocabulary two implementations would then have to agree on.

Ill-formed curves are unrepresentable rather than diagnosed (PoSD ch. 6): construction is the only place the breakpoints
are checked, so a `Progress` that exists is well-formed and no consumer validates one.

## D10 — Coverage (which occurrences are in force at an instant)

```text
covering(M, t) = [ (s, e, a) ∈ E | contains([s, e), t) ]   in canonical order (N2)
```

where containment is the support's own convention:

```text
contains([s, e), t)  ≡  s = e ? s = t : s ≤ t < e
```

Coverage is an **observation, not a constructor**: it stores nothing, changes nothing, and adds no denotation D0–D7 did
not already give. It exists because every consumer that wanted "what is sounding here" was computing it privately, and
they did not agree (prompt 44).

Coverage yields *occurrences*, not identities: which events a score gave names to is the score layer's invention, and
the kernel must not learn it.

Coverage and restriction are the same question asked two ways, which L20 states: `covering(M, t)` is exactly what
`restrict(M, I)` observes for every window `I` containing `t`, filtered to those occurrences containing `t`.

## D11 — Prevailing value (what is in force at an instant)

Given a selector `σ : A → V ∪ {⊥}` naming which payloads participate,

```text
prevailing(M, t, σ) = σ(a) for the canonically last (s, e, a) ∈ E with s ≤ t and σ(a) ≠ ⊥
                    = ⊥ when there is none
```

The selector, rather than a payload trait, is what keeps the musical knowledge out of the kernel: the *caller* says "the
key facts", and the kernel never learns what a key is. One timeline therefore supports as many independent prevailing
values as a consumer has questions — key, meter, clef, dynamic — without a type per kind.

Note what is *not* in the definition: the occurrence's end. A prevailing value is anchored by where it was stated, not
by how far its support happens to reach, which is why a whole-piece key fact and a key fact stated at bar 40 answer the
same way.

### The boundary conventions, decided once

These four decisions were previously made in four places and not identically. They are the definition, and each is
tested (L20–L23).

| Question | Ruling | Why |
| --- | --- | --- |
| Is a fact covering at its end instant? | No — support is `[s, e)` | Consistent with D6 and with `sequence`: the next fact's start is the previous one's end, and one instant must not belong to both |
| Is a point occurrence covered at its own instant? | Yes | Otherwise a point fact is unobservable — the same defect prompt 37 repaired in D6 |
| Two prevailing candidates at the same instant? | The canonically later wins | Canonical order is total and includes the payload key (N2), so the answer is deterministic and does not depend on how the timeline was built |
| Does a fact starting exactly at `t` prevail at `t`? | Yes | `dynamic mf;` on a note applies to *that* note; anything else surprises a composer |

### The performance rule

Both queries are linear scans, which is right for a one-off ask ("what covers the selection") and wrong for bulk
derivation: calling `covering` once per event is O(events × facts).

**The kernel defines what the answer is; bulk derivation does one ordered pass.** A projection that must answer for
every event keeps its single sweep and honours the conventions above rather than calling the queries per event — see
`musa-compiler`'s `project_regions` and `lower_performance`, both of which cite this rule. If benchmark P3 regresses, a
sweep was turned into n queries and must be reverted, not tuned. No bulk or indexed query API belongs in the kernel for
this: no caller wants one, and the sweep belongs where the score's ordering lives.

## D12 — Explicitly not defined

- **No `join`.** `Timeline[Timeline[A]]` has no canonical flattening: begin-at-onset, stretch-to-fit, crop, repeat, and
  preserve-inner-duration are genuinely different musical operations. Specific higher-level abstractions may define
  their own; the universal kernel does not.
- **No distributivity.** `M ; (N ⊕ P) ≠ (M ; N) ⊕ (M ; P)` in general — the left side has one copy of `M`, the right
  side two. The kernel is not a semiring, and no law is claimed that would make it one.
- **No infinity.** Every denotation is finite in extent and in occurrence count. Patterns and loops produce coherent
  finite observations; they are not kernel values.
