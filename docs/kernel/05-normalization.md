# 05 — Normalization, Semantic Equality, Serialization

Every finite kernel composition normalizes to one flat timeline (course correction §25). This document fixes the normal
form, the canonical occurrence order, semantic equality, and the canonical text serialization used for golden tests and
semantic hashing.

## N1 — Normal form

For every finite kernel composition `C`:

```text
normalize(C) = timeline d {
    occurrence <payload₁> from <s₁> to <e₁>;
    …
    occurrence <payloadₙ> from <sₙ> to <eₙ>;
}
```

with no remaining `sequence`, `overlay`, or composition references, where `(d, E) = ⟦C⟧` per
`03-denotational-semantics.md`. Normalization is total and finite: the only operations are translation (τ), multiset
union, and `max` on extents, all closed over finite timelines.

Implementation note: prompt 09's `Timeline<A>` stores flat timelines directly — construction **is** normalization.
`normalize` therefore re-canonicalizes occurrence order; it never needs to flatten a tree, because the tree never
persists as kernel state. Un-normalized `sequence`/`overlay` expressions exist only in the interchange grammar
(`01-grammar.md`) and in HIR above the kernel.

## N2 — Canonical occurrence order

Occurrences in a normal form are sorted ascending by:

1. start `s`;
2. end `e`;
3. canonical payload serialization (lexicographic byte order).

The payload key is last so that musically identical spans produced by different payloads never reorder relative to one
another except by a total, deterministic rule. Duplicate occurrences (equal in all three) are adjacent and are **both
retained** — the multiset is preserved through sorting (§6, K6).

## N3 — Canonical payload serialization

Every payload type used in a normalized, compared, hashed, or serialized timeline must provide a canonical text form
that is:

- **deterministic** — same value, same bytes, always (no addresses, no hash iteration order, no floats);
- **total** — every representable value serializes;
- **injective on values** — distinct values serialize distinctly (so semantic equality of payloads is equality of
  serializations).

For the first-order payload schemas of `01-grammar.md` this is: fields in declaration order, `name = value;` pairs,
rationals in reduced `p/q` form, text escaped minimally and consistently.

## N4 — Semantic equality

```text
M ≡ N   ⟺   extent(M) = extent(N)  ∧  canonical-occurrences(M) = canonical-occurrences(N)
```

with occurrences compared as exact triples `(s, e, payload-serialization)` (§25). This is the only equality downstream
consumers may rely on. In particular, `sequence`/`overlay` trees that denote the same flat timeline are the same kernel
value: the kernel is a semantic quotient (§20), and structural history is provenance's job, not equality's.

## N5 — Canonical text serialization

A normalized timeline serializes deterministically as:

```text
timeline <d> {
    occurrence <payload₁> from <s₁> to <e₁>;
    occurrence <payload₂> from <s₂> to <e₂>;
    ...
}
```

Rules:

- Occurrences in canonical order (N2), one per line, two-space indentation.
- Rationals in reduced form; integers (denominator 1) print without `/1`. Negative values never occur.
- Payloads in canonical payload serialization (N3).
- Trailing newline after the closing brace; no timestamps, no comments, no version headers.

This is a strict subset of the `01-grammar.md` syntax (a `timeline-expression` alone), so a future kernel-file parser
reads today's golden files unchanged.

## N6 — Semantic hashing

Because N5 is deterministic, `hash(serialize(normalize(C)))` is a well-defined **semantic hash**: equal-in-meaning
compositions hash equal regardless of how they were constructed. Uses: golden tests, cache keys, differential testing
between semantic paths (prompt 11). The hash algorithm is a consumer choice; the serialized bytes are the contract.

## N7 — What normalization forgets (on purpose)

Normalization erases: motif boundaries, repetition counts, transposition history, chord syntax, voice grouping,
file/import structure. All of it is recoverable where it matters — in provenance (source IDs, expansion paths, source
maps, HIR structure, §20) — and none of it may leak back into kernel equality. If a consumer finds itself reconstructing
forgotten structure from the normal form, that is evidence of a missing **payload-level** or **consumer-level** concept,
not of missing kernel nodes (§34).
