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

N3 is the **equality** serialization, and that is all it is. It may — and for `ScoreFact` does — quotient away detail
the value carries: two facts differing only in their definition span and declaration id are the same fact for ordering,
equality, and hashing, so the key omits both. That is what makes the semantic hash *semantic*.

The consequence, discovered while implementing prompt 48: **N3 is not an interchange form.** An interchange form must
reproduce the value, so it carries what N3 drops, and the two are separate functions with separate jobs
(`Canonical::canonical_key` and `TextPayload::to_text`). The prompt asked for this to be checked before a second
function was written, and this is the answer. Where a payload has nothing to quotient — `Progress` below — the two
coincide, and one function serves.

### The canonical form of a `Progress`

```text
u₀/d₀:v₀/e₀,u₁/d₁:v₁/e₁,…
```

Breakpoints in order, each rational in reduced `p/q` form, `u` and `v` separated by `:` and pairs by `,`. Deterministic
and float-free by construction; injective because the breakpoints are strictly increasing in `u`, so no two distinct
curves produce the same string. A `Progress` therefore contributes stably to semantic equality (N4) and to the semantic
hash (N6), which is what makes two implementations reading the same interchange file agree that they read the same
piece.

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

N5 is **not** kernel-file syntax, and an earlier draft of this section claimed it was. Two differences, each of them
the point of the form it belongs to:

- N5 writes the payload's N3 key bare, where a file writes an interchange payload as a quoted string. The key is not
  parseable and does not need to be — nothing reads N5, it is hashed and compared.
- N5 has no version header, because it is not a file.

So the two serializations coexist: N5 for identity, `01-grammar.md` for exchange. `musa kernel --normalized` prints the
*interchange* spelling of the normal form — a single flat `timeline` in a kernel file — which is parseable, and which
`musa kernel --check` therefore accepts. Nothing about N5's bytes changed, and no golden moved.

## N6 — Semantic hashing

Because N5 is deterministic, `hash(serialize(normalize(C)))` is a well-defined **semantic hash**: equal-in-meaning
compositions hash equal regardless of how they were constructed. Uses: golden tests, cache keys, and deciding whether
work that depends on the meaning of a piece has to be redone.

`Timeline::semantic_hash` computes it. The algorithm is **FNV-1a, 128 bits**, over exactly the bytes N5 defines, and it
is named here rather than left to the consumer because an identity that varies between runs or processes is not an
identity — a stored digest has to still mean the same thing after a restart. The same writer produces the canonical
text and feeds the digest, so the two can never drift apart.

Invariants:

- **Stable.** Same timeline, same bytes, same digest — every run, every process, every machine. Rust's `DefaultHasher`
  is excluded by this: its output is not stable across releases and `HashMap`'s is randomly seeded.
- **Agrees with N4.** `M ≡ N ⟹ hash(M) = hash(N)`. The converse holds up to the collision probability of 128 bits, so
  an unequal digest *proves* the meanings differ — which is the direction a caller deciding whether to rebuild needs.
- **Not cryptographic.** FNV-1a resists accident, not an adversary. Signing a published score would need a different
  function, chosen then.
- **Covers whatever the payload key covers**, including provenance (N3). For musa's score facts that means source
  spans: re-indenting a file changes the digest. That is correct — the question is "is this the same compiled piece",
  not "does it sound the same" — and a sounds-the-same digest, if one is ever wanted, is a second function over a
  provenance-free projection, not a weaker reading of this one.

## N7 — What normalization forgets (on purpose)

Normalization erases: motif boundaries, repetition counts, transposition history, chord syntax, voice grouping,
file/import structure. All of it is recoverable where it matters — in provenance (source IDs, expansion paths, source
maps, HIR structure, §20) — and none of it may leak back into kernel equality. If a consumer finds itself reconstructing
forgotten structure from the normal form, that is evidence of a missing **payload-level** or **consumer-level** concept,
not of missing kernel nodes (§34).
