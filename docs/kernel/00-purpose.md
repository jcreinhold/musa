# 00 — Purpose of the Temporal Kernel

Musa maintains two pictures at once:

1. **The surface language** — expressive, concise, musician-oriented, programmable. Composers write notes, rests,
   chords, motifs, repeats, transpositions, voices, parts, keys, meters, and (later) phrases, sections, dynamics, loops,
   and aleatory constructions.
2. **The temporal kernel** — a very small, exact, backend-independent semantics into which the surface language
   elaborates.

The kernel exists to give every downstream consumer one precise answer to a single question:

> **What musical facts exist, and where do they exist in musical time?**

Everything else — how those facts were produced (motif? repetition? transposition?), how they are displayed (beaming,
rest glyphs, line breaks), how they sound (tempo realization, instruments, DSP) — belongs to layers above or below the
kernel, never inside it.

Stating the facts is only half the job. A representation nobody can interrogate is ceremony: if every consumer that
needs "what is sounding at bar 12" or "which key is in force here" writes the scan itself, the kernel has centralized
the *storage* of temporal truth while leaving its *interpretation* scattered — and scattered interpretations disagree,
which is exactly what prompt 44 found. So the kernel is a representation **and** the interface for interrogating it
(`03-denotational-semantics.md` D10–D11): the second answer it gives is

> **What is in force at this instant, and what does it cover?**

The interface stays small in the same way the representation does. A query earns its place by being something more than
one consumer already computes by hand, and it adds no stored state, no constructor, and no denotation the operations
above do not already give.

## Why a kernel at all

Without a small semantic basis, every surface-language construct becomes something each backend must independently
understand, each transformation must traverse, equality must account for, serialization must preserve, and tests must
special-case. That accretion is the failure mode this specification exists to prevent.

The anti-pattern is **not** domain-specific syntax; musician-friendly syntax is desirable. The anti-pattern is
**domain-specific semantic accretion without a stable lower algebra**.

## The foundational model

Musical time is ambient: it exists independently of what occurs within it (`docs/governance/01-constitution.md` §3). A finite kernel object
is:

1. an ambient region of exact musical time `[0, d]`, `d ∈ ℚ≥0`; and
2. zero or more typed occurrences `(s, e, a)` supported within that region, `0 ≤ s ≤ e ≤ d`, payload `a : A`.

If a region contains no note occurrence, that region is silent with respect to notes. **Nothing representing silence
needs to exist.** A rest glyph is a notation decision a backend makes about an uncovered region of a notated voice — it
is not kernel ontology (`07-backend-contract.md`).

Time is exact: positions form the abelian group `(ℚ, +, 0)` and durations the ordered commutative monoid `(ℚ≥0, +, 0)`. Floats never represent symbolic musical time; physical seconds are a separate domain introduced by performance
realization.

## The governing design rule

Every proposed kernel feature is judged by this rule:

> **A construct belongs in the kernel only if removing it makes an important class of musical meanings impossible or
> unnatural to represent faithfully across multiple independent consumers.**
>
> "Musicians use this concept" is not enough. "It's convenient to parse this way" is not enough. "It's easier to
> implement this feature as another enum variant" is not enough. The burden is semantic necessity.
>
> Conversely, do not worship minimality. If a concept repeatedly requires convoluted encodings, duplicated conventions,
> or backend-specific reconstruction, that is evidence that the kernel is missing a genuine primitive.
>
> The target is not the *fewest constructors*. The target is the **smallest complete semantic basis**.

This rule is the acceptance test for every future kernel proposal, and for every deviation request against this
specification.

## The initial structural basis

Exactly three structural forms, plus named references for sharing:

- `timeline` — an ambient finite temporal region with facts supported within it;
- `sequence` — temporal succession (associative concatenation);
- `overlay` — simultaneous presence in a common ambient region (commutative, associative, **not** idempotent).

No primitive `note`, `rest`, `chord`, `motif`, `voice`, `repeat`, `transpose`, `key`, or `tempo` exists at this level.
Payloads are typed but musically opaque to the kernel: the kernel knows *where*, *when*, *for how long*, and *what
typed value* — never what a `Note` means.

## What the kernel deliberately is not

- Not a general-purpose programming language (no recursion, no general computation — §32).
- Not a semiring or ring; `sequence` does not distribute over `overlay` (`04-algebraic-laws.md`).
- Not assumed to be a monad; there is no canonical musically-correct `join`.
- Not infinite: patterns, loops, and live processes live **above** the finite kernel as producers of coherent finite
  observations.
- Not a provenance store: the kernel is a semantic *quotient* of richer source structure; provenance is preserved above
  it (`06-surface-elaboration.md`).

## The calculus, and why it does not cross that line

Prompt 46 adds `10-term-calculus.md` (governing since prompt 48): a *syntax* whose meanings are the timelines above. It
exists for three things values cannot express — sharing (`let`, so a canon's subject is stated once), deferred
observation (restricting before evaluating), and interchange (a syntax a second implementation can read) — and it adds
no semantic operation: every term denotes something `03-denotational-semantics.md` already defines.

It therefore stays under the "not a general-purpose programming language" line above rather than testing it. The
calculus has a binder but no abstraction: `let x = t in u` names a *value*, and there is no way to write a function, an
application, a conditional, or a recursion. That is why every closed well-formed term evaluates, deterministically and
in finitely many steps. `map f` is deliberately not a term for exactly this reason — naming `f` would require a syntax
for functions — so payload transformation stays above the kernel, where it already is.

## The pipeline

```text
musician-facing Musa source
            │
            ▼
    rich compositional HIR (motifs, repeats, transforms, provenance)
            │
            │ elaboration / finite observation
            ▼
┌───────────────────────────────┐
│      TEMPORAL KERNEL          │
│  exact ambient musical time   │
│  typed temporal occurrences   │
│  timeline / sequence / overlay│
│  restriction / normalization  │
└───────────────┬───────────────┘
                │
        normalized timeline
                │
      ┌─────────┼─────────┐
      ▼         ▼         ▼
   notation   analysis  performance
                          │
                    tempo realization
                          │
                          ▼
                         audio
```

## Document map

| File | Contents |
| --- | --- |
| `01-grammar.md` | The kernel interchange syntax (not the musician-facing syntax). |
| `02-static-semantics.md` | Well-formedness rules. |
| `03-denotational-semantics.md` | The denotation `(d, E)` and every operation's definition. |
| `04-algebraic-laws.md` | The laws, formally, cross-referenced to their property tests. |
| `05-normalization.md` | Canonical normal form, semantic equality, serialization. |
| `06-surface-elaboration.md` | How the existing surface constructs elaborate. |
| `07-backend-contract.md` | What downstream consumers may assume. |
| `08-open-questions.md` | What is deliberately undecided. |
| `09-performance.md` | The measured cost of the kernel path, prompt by prompt. |
| `10-term-calculus.md` | The term calculus: syntax, evaluation, soundness theorems. |
| `11-realization.md` | Seeded finite realization and its reproducibility laws. |
| `12-payload-admission.md` | Payload equality, schema ownership, law transport, and exact identity framing. |
