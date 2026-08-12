# 05 — Normalization, Semantic Equality, Serialization

Every finite kernel composition normalizes to one flat timeline (course correction §25). This document fixes the normal
form, canonical occurrence order, semantic equality, human canonical display, and separately framed semantic identity.

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

## N3 — Canonical payload equality key

Every payload type used in a normalized, compared, or hashed timeline supplies the admission record in
`12-payload-admission.md`: stable owner id, quotient version, and a canonical text key which is:

- **deterministic** — same value, same bytes, always (no addresses, no hash iteration order, no floats);
- **total** — every representable value serializes;
- **complete for admitted equality** — payload equality is exactly key equality.

N3 is the **equality projection**, and that is all it is. It may—and for `ScoreFact` does—quotient away detail the value
carries: two facts differing only in their definition span and declaration id are the same fact under current kernel
equality, so the key omits both. It is therefore not required or claimed to be injective on the raw stored struct.

The consequence, discovered while implementing prompt 48: **N3 is not an interchange form.** An interchange form must
reproduce the value, so it carries what N3 drops, and the two are separate functions with separate jobs
(`Canonical::canonical_key` and `TextPayload::to_text`). Where a payload has nothing to quotient—`Progress` below—the
two may coincide, but they retain different contracts.

### The canonical form of a `Progress`

```text
u₀/d₀:v₀/e₀,u₁/d₁:v₁/e₁,…
```

Breakpoints in order, each rational in reduced `p/q` form, `u` and `v` separated by `:` and pairs by `,`. Deterministic
and float-free by construction; complete because the breakpoints are strictly increasing in `u`, so no two distinct
curves produce the same string. A `Progress` therefore contributes stably to semantic equality (N4) and semantic
identity (N6).

## N4 — Semantic equality

```text
M ≡ N   ⟺   extent(M) = extent(N)  ∧  canonical-occurrences(M) = canonical-occurrences(N)
```

with occurrences compared as exact triples `(s, e, payload-serialization)` (§25). This is the only equality downstream
consumers may rely on. In particular, `sequence`/`overlay` trees that denote the same flat timeline are the same kernel
value: the kernel is a semantic quotient (§20), and structural history is provenance's job, not equality's.

## N5 — Canonical human display

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

N5 is deterministic text for people, logs, and display goldens. It is **not** a persisted semantic encoding and is not
kernel-file syntax. Two differences from interchange remain:

- N5 writes the payload's N3 key bare, where a file writes an interchange payload as a quoted string. The key need not
  be parseable and may itself contain newlines and N5 delimiters.
- N5 has no version header, because it is not a file.

Consequently distinct timelines can have equal N5 display text. This exact counterexample is retained as a regression:
one occurrence on `[1,2]` with string key `a from 0 to 1;\n  occurrence b`, versus occurrences `a` on `[0,1]` and `b` on
`[1,2]`. N4 distinguishes them; N5 does not. `musa kernel --normalized` prints the separate *interchange* spelling of
the normal form, which is quoted and parseable.

## N6 — Semantic hashing

N6 defines the private exact semantic encoding in `12-payload-admission.md` A7. It contains a domain tag, timeline
encoding version, payload owner and quotient version, exact rational extent, occurrence count, and every canonical
occurrence with length-framed payload-key bytes. It is separate from N5 display.

`Timeline::semantic_hash` computes FNV-1a-128 over exactly those version-2 bytes. Version 1 denotes the former unframed
N5 stream and is not reinterpreted as version 2. The named stable algorithm makes digests reproducible; the uniquely
decodable framed bytes, not the digest, carry exact identity.

Invariants:

- **Stable.** Same timeline, same bytes, same digest — every run, every process, every machine. Rust's `DefaultHasher`
  is excluded by this: its output is not stable across releases and `HashMap`'s is randomly seeded.
- **Agrees with N4.** `M ≡ N ⟹ hash(M) = hash(N)`. An unequal digest proves the framed bytes differ. An equal digest is
  only a candidate match and requires complete-byte or structured confirmation whenever a false hit changes a result.
- **Not cryptographic.** FNV-1a resists accident, not an adversary. Signing a published score would need a different
  function, chosen then.
- **Covers whatever the payload key covers**, including provenance (N3). For musa's score facts that means source spans:
  re-indenting a file changes the digest. That is correct — the question is "is this the same compiled piece", not "does
  it sound the same" — and a sounds-the-same digest, if one is ever wanted, is a second function over a provenance-free
  projection, not a weaker reading of this one.

## N7 — What normalization forgets (on purpose)

Normalization erases: motif boundaries, repetition counts, transposition history, chord syntax, voice grouping,
file/import structure. All of it is recoverable where it matters — in provenance (source IDs, expansion paths, source
maps, HIR structure, §20) — and none of it may leak back into kernel equality. If a consumer finds itself reconstructing
forgotten structure from the normal form, that is evidence of a missing **payload-level** or **consumer-level** concept,
not of missing kernel nodes (§34).
