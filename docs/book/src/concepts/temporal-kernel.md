# The temporal kernel

Musa maintains two pictures at once. The surface language is expressive and musician-oriented: notes, motifs, repeats,
transpositions, voices, keys. The temporal kernel is a small, exact semantics the surface language elaborates into.
The kernel exists to give every downstream consumer one precise answer:

> What musical facts exist, and where do they exist in musical time?

How the facts were produced, how they are displayed, and how they sound all belong to layers above or below the
kernel, never inside it.

## The model

Musical time is ambient: it exists independently of what occurs within it. A kernel object is a finite region of exact
musical time `[0, d]` plus zero or more typed occurrences `(s, e, a)` supported within it — each with a start, an end,
and a payload.

A region with no note occurrence is silent with respect to notes. Nothing represents silence: a rest glyph is a
notation decision a backend makes about an uncovered region, not kernel ontology.

## Three structural forms

- `timeline` — an ambient region with facts supported in it;
- `sequence` — temporal succession, associative concatenation;
- `overlay` — simultaneous presence in a common region (commutative, associative, not idempotent).

There is no primitive `note`, `rest`, `motif`, `voice`, `repeat`, `key`, or `tempo` at this level. Payloads are typed
but musically opaque: the kernel knows where, when, for how long, and what typed value — never what a note means.

## What the kernel is not

- Not a programming language: no recursion, no general computation. Every closed term evaluates, deterministically, in
  finitely many steps.
- Not a semiring: `sequence` does not distribute over `overlay`.
- Not a monad: there is no canonical musically-correct `join`.
- Not infinite: loops and patterns live above the kernel as producers of finite observations.
- Not a provenance store: the kernel is a semantic quotient of richer source structure, and provenance is preserved
  above it (see [Source and provenance](provenance.md)).

## Why a kernel at all

Without a small semantic basis, every surface construct becomes something each backend must independently understand,
each transformation must traverse, and equality must account for. The kernel is the smallest complete semantic basis:
a construct earns kernel status only when removing it makes an important class of musical meanings impossible to
represent faithfully across independent consumers.

The full specification — grammar, denotational semantics, algebraic laws, normalization, elaboration — lives in
`docs/kernel/` in the repository.
