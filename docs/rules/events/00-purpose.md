# 00 — Purpose of the Event-Track Core

**Status: governing since prompt 12; amended at prompt 127a.** This file is the entry point to `docs/rules/events/`; its
document map is at the bottom. Bound by `../constitution.md`, `../obligations.md`, and `../across-stages/`.

The directory is called `events/` after the value it specifies: the **finite event track**, one of the two core values
of `constitution.md` §3 and §4. It was called `kernel/` until the vocabulary amendment recorded in
`../../notes/research/core-calculus/23-events-vocabulary.md`, which retired *kernel* as a second name for this one thing
— the word had come to mean the crate, this directory, the quotation keyword, and the interchange format, none of which
it described. The other, the machine, is specified in `../across-stages/03-machine-calculus.md`, and the checked
scheduler that connects them is specified there too.

Musa maintains two pictures at once:

1. **The surface language** — expressive, concise, musician-oriented, programmable. Composers write notes, rests,
   chords, motifs, repeats, transpositions, voices, parts, keys, meters, phrases, sections, dynamics, loops, and
   aleatory constructions.
2. **The event-track core** — a very small, exact, backend-independent semantics that surface notation elaborates into.

The core exists to give every downstream consumer one precise answer to a single question:

> **What facts exist, and where do they exist in this coordinate of musical time?**

Everything else — how those facts were produced (motif? repetition? transposition?), how they are displayed (beaming,
rest glyphs, line breaks), how they sound (tempo realization, instruments, DSP) — belongs to layers above or beside the
core, never inside it.

Stating the facts is only half the job. A representation nobody can interrogate is ceremony: if every consumer that
needs "what is sounding at bar 12" or "which key is in force here" writes the scan itself, the core has centralized the
*storage* of temporal truth while leaving its *interpretation* scattered — and scattered interpretations disagree, which
is exactly what prompt 44 found. So the core is a representation **and** the interface for interrogating it
(`03-denotational-semantics.md` D10–D11): the second answer it gives is

> **What is in force at this instant, and what does it cover?**

The interface stays small in the same way the representation does. A query earns its place by being something more than
one consumer already computes by hand, and it adds no stored state, no constructor, and no denotation the operations
above do not already give.

## Why a small core at all

Without a small semantic basis, every surface-language construct becomes something each backend must independently
understand, each transformation must traverse, equality must account for, serialization must preserve, and tests must
special-case. That accretion is the failure mode this specification exists to prevent.

The anti-pattern is **not** domain-specific syntax; musician-friendly syntax is desirable. The anti-pattern is
**domain-specific semantic accretion without a stable lower algebra**.

## The foundational model

Musical time is ambient: it exists independently of what occurs within it (`docs/rules/constitution.md` §3). A finite
event track is:

1. a **coordinate** `C` saying whose time this is — written time, performed time, or physical seconds;
2. an ambient region of exact time `[0, d]`, `d ∈ ℚ≥0`, the track's **duration**; and
3. zero or more typed occurrences `(s, e, a)` supported within that region, `0 ≤ s ≤ e ≤ d`, payload `a : A`, where `A`
   is storable data.

If a region contains no note occurrence, that region is silent with respect to notes. **Nothing representing silence
needs to exist.** A rest glyph is a notation decision a backend makes about an uncovered region of a notated voice — it
is not core ontology (`07-backend-contract.md`).

Time is exact: positions form the abelian group `(ℚ, +, 0)` and durations the ordered commutative monoid `(ℚ≥0, +, 0)`.
Floats never represent symbolic musical time. Physical seconds are a *different coordinate*, reached by a named
conversion, and durations in two coordinates do not add.

Those are two structures, so they are two types: `Position<C>` for *when* and `Duration<C>` for *how much*. A position
plus a duration is a position, two durations add, two positions do not add at all, and their difference is a duration
only when it is nonnegative. `01-grammar.md` already lexes `position-literal` and `duration-literal` apart for the same
reason: one type for both would make beat 3 and three beats addable, which is the one arithmetic error a tagged rational
exists to catch.

The coordinate is the one type index the core carries, and `constitution.md` §8 says why it is the only one: every other
candidate index — part, voice, metre, tuning — has a diagnostic elsewhere, and a written beat added to a physical second
has none until the sound is wrong.

## The governing design rule

Every proposed core feature is judged by this rule:

> **A construct belongs in the event-track core only if removing it makes an important class of musical meanings
> impossible or unnatural to represent faithfully across multiple independent consumers.**
>
> "Musicians use this concept" is not enough. "It's convenient to parse this way" is not enough. "It's easier to
> implement this feature as another enum variant" is not enough. The burden is semantic necessity.
>
> Conversely, do not worship minimality. If a concept repeatedly requires convoluted encodings, duplicated conventions,
> or backend-specific reconstruction, that is evidence that the core is missing a genuine primitive.
>
> The target is not the *fewest constructors*. The target is the **smallest complete semantic basis**.

This rule is the acceptance test for every future proposal, and for every deviation request against this specification.

## The basis

Six operations form the basis:

```text
empty        : Duration<C> -> EventTrack<C,A>
event        : Duration<C> -> A -> EventTrack<C,A>
follow       : EventTrack<C,A> × EventTrack<C,A> -> EventTrack<C,A>
together     : EventTrack<C,A> × EventTrack<C,A> -> EventTrack<C,A>
map_payloads : (A -> B) × EventTrack<C,A> -> EventTrack<C,B>
duration     : EventTrack<C,A> -> Duration<C>
```

`follow` is temporal succession (associative concatenation, durations add). `together` is simultaneous presence in a
common ambient region (commutative, associative, **not** idempotent, longer duration wins).

Three further operations are **retained beyond the basis because they have named callers**, not because the basis needs
them, and each is stated in `03-denotational-semantics.md` with its laws:

- `scale_r` — exact positive rational time scaling. Its caller is augmentation and diminution (`stretch`), which cannot
  be written above the core because it moves spans.
- `restrict` — observation through a window, with whole spans preserved. Its callers are windowed observation and the
  editor's selection.
- `covering` and `prevailing` — the two queries of D10–D11. Their callers are four consumers that used to answer the
  same question privately and disagree.

Adding to that list requires the same evidence as adding to the basis. Removing from it requires only that the caller
goes away, which is what happened to `extend` at prompt 37.

No primitive `note`, `rest`, `chord`, `motif`, `voice`, `repeat`, `transpose`, `key`, or `tempo` exists at this level.
Payloads are typed but musically opaque: the core knows *where*, *when*, *for how long*, and *what typed value* — never
what a `Note` means.

## What the core deliberately is not

- Not a general-purpose programming language. The source language above it is (`../language/`), and the two are separate
  stages, not two cores.
- Not a semiring or ring; `follow` does not distribute over `together` (`04-algebraic-laws.md`).
- Not assumed to be a monad; there is no canonical musically-correct `join`.
- Not infinite: patterns, loops, and live processes live **above** the finite core as producers of coherent finite
  observations.
- Not a provenance store: the core is a semantic *quotient* of richer source structure; provenance is preserved above it
  (`06-surface-elaboration.md`).
- **Not the sound layer.** A machine is not an event track and an audio history is not a payload. Where they meet is the
  checked scheduler, and it is specified next door.

## The calculus, and why it does not cross that line

`10-term-calculus.md` (governing since prompt 48) is a *syntax* whose meanings are the tracks above. It exists for three
things values cannot express — sharing (`let`, so a canon's subject is stated once), deferred observation (restricting
before evaluating), and interchange (a syntax a second implementation can read) — and it adds no semantic operation:
every term denotes something `03-denotational-semantics.md` already defines.

It therefore stays under the "not a general-purpose programming language" line above rather than testing it. The
calculus has a binder but no abstraction: `let x = t in u` names a *value*, and there is no way to write a function, an
application, a conditional, or a recursion. That is why every closed well-formed term evaluates, deterministically and
in finitely many steps. `map_payloads f` is deliberately not a term for exactly this reason — naming `f` would require a
syntax for functions — so payload transformation stays above the core, where it already is.

## The pipeline

```text
musician-facing Musa source
            │
            │ read, expand bounded syntax adapters, resolve, infer, evaluate
            ▼
    one total source language — ordinary values, event tracks, machines
            │
            │ track construction
            ▼
┌───────────────────────────────────────┐
│        EVENT-TRACK CORE               │
│  exact time, tagged by coordinate     │
│  typed occurrences over storable data │
│  empty / event / follow / together    │
│  map_payloads / duration              │
│  scale / restrict / normalization     │
└───────────────┬───────────────────────┘
                │
       normalized event track
                │
      ┌─────────┼─────────────────────────┐
      ▼         ▼                         ▼
   notation   analysis         performance interpretation
                                          │
                                 EventTrack<PerformedTime,Gesture>
                                          │
                          schedule(format, policy, time map, track)
                                          │
                                          ▼
                          Machine<AudioFrameStep, …>  ── step ──▶ audio history
```

The last two arrows leave this directory. `../across-stages/03-machine-calculus.md` owns them.

## Document map

| File | Contents |
| --- | --- |
| `01-grammar.md` | The interchange syntax (not the musician-facing syntax). |
| `02-static-semantics.md` | Well-formedness rules. |
| `03-denotational-semantics.md` | The denotation `(d, E)` and every operation's definition. |
| `04-algebraic-laws.md` | The laws, formally, cross-referenced to their property tests. |
| `05-normalization.md` | Canonical normal form, semantic equality, serialization. |
| `06-surface-elaboration.md` | How the surface constructs elaborate. |
| `07-backend-contract.md` | What downstream consumers may assume. |
| `08-open-questions.md` | What is deliberately undecided. |
| `09-performance.md` | The measured cost of the core path, prompt by prompt. |
| `10-term-calculus.md` | The term calculus: syntax, evaluation, soundness theorems. |
| `11-realization.md` | Seeded finite realization and its reproducibility laws. |
| `12-payload-admission.md` | Payload equality, schema ownership, law transport, and exact identity framing. |
