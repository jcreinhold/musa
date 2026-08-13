# 05 — Normalization, Semantic Equality, Serialization

Every finite kernel composition normalizes to one flat event track. This document fixes the normal form, canonical
occurrence order, semantic equality, human canonical display, and separately framed semantic identity.

## N1 — Normal form

For every finite kernel composition `C`:

```text
normalize(C) = track d {
    occurrence <payload₁> from <s₁> to <e₁>;
    …
    occurrence <payloadₙ> from <sₙ> to <eₙ>;
}
```

with no remaining `follow`, `together`, or composition references, where `(d, E) = ⟦C⟧` per
`03-denotational-semantics.md`. Normalization is total and finite: the only operations are translation (τ), multiset
union, and `max` on lengths, all closed over finite tracks.

Implementation note: prompt 09's flat track value stores occurrences directly — construction **is** normalization.
`normalize` therefore re-canonicalizes occurrence order; it never needs to flatten a tree, because the tree never
persists as core state. Un-normalized `follow`/`together` expressions exist only in the interchange grammar
(`01-grammar.md`) and in HIR above the core.

## N2 — Canonical occurrence order

Occurrences in a normal form are sorted ascending by:

1. start `s`;
2. end `e`;
3. canonical payload serialization (lexicographic byte order).

The payload key is last so that musically identical spans produced by different payloads never reorder relative to one
another except by a total, deterministic rule. Duplicate occurrences (equal in all three) are adjacent and are **both
retained** — the multiset is preserved through sorting (K6).

## N3 — Canonical payload equality key

Every payload type used in a normalized, compared, or hashed track supplies the admission record in
`12-payload-admission.md`: stable owner id, quotient version, and a canonical text key which is:

- **deterministic** — same value, same bytes, always (no addresses, no hash iteration order, no floats);
- **total** — every representable value serializes;
- **complete for admitted equality** — payload equality is exactly key equality.

N3 is the **equality projection**, and that is all it is. It may—and for `ScoreFact` does—quotient away detail the value
carries: two facts differing only in their definition span and declaration id are the same fact under current core
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
M ≡ N   ⟺   length(M) = length(N)  ∧  canonical-occurrences(M) = canonical-occurrences(N)
```

with occurrences compared as exact triples `(s, e, payload-serialization)`. Tracks in different coordinates are
different types, so the question does not arise between them; where coordinate is carried as data rather than as a type
parameter — in the exact bytes of N6 — it is compared first (`../across-stages/04-identity-and-realization.md` §2).

This is the only equality downstream consumers may rely on. In particular, `follow`/`together` trees that denote the
same flat track are the same core value: the core is a semantic quotient, and structural history is provenance's job,
not equality's.

## N5 — Canonical human display

A normalized track serializes deterministically as:

```text
track <d> {
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

Consequently distinct tracks can have equal N5 display text. This exact counterexample is retained as a regression: one
occurrence on `[1,2]` with string key `a from 0 to 1;\n  occurrence b`, versus occurrences `a` on `[0,1]` and `b` on
`[1,2]`. N4 distinguishes them; N5 does not. `musa kernel --normalized` prints the separate *interchange* spelling of
the normal form, which is quoted and parseable.

## N6 — Semantic hashing

N6 defines the private exact semantic encoding in `12-payload-admission.md` A7. It contains a domain tag, track encoding
version, coordinate tag, payload owner and quotient version, exact rational length, occurrence count, and every
canonical occurrence with length-framed payload-key bytes. It is separate from N5 display, and its field order is the
one `../across-stages/04-identity-and-realization.md` §3 states.

`semantic_hash` computes FNV-1a-128 over exactly those bytes. Earlier encoding versions are refused, not reinterpreted:
version 1 denotes the former unframed N5 stream, and version 2 the framed-but-untagged bytes that predate the
coordinate. The named stable algorithm makes digests reproducible; the uniquely decodable framed bytes, not the digest,
carry exact identity.

Invariants:

- **Stable.** Same track, same bytes, same digest — every run, every process, every machine. Rust's `DefaultHasher` is
  excluded by this: its output is not stable across releases and `HashMap`'s is randomly seeded.
- **Agrees with N4.** `M ≡ N ⟹ hash(M) = hash(N)`. An unequal digest proves the framed bytes differ. An equal digest is
  only a candidate match and requires complete-byte or structured confirmation whenever a false hit changes a result.
- **Not cryptographic.** FNV-1a resists accident, not an adversary. Signing a published score would need a different
  function, chosen then.
- **Covers whatever the payload key covers**, including provenance (N3). For musa's score facts that means source spans:
  re-indenting a file changes the digest. That is correct — the question is "is this the same compiled piece", not "does
  it sound the same" — and a sounds-the-same digest, if one is ever wanted, is a second function over a provenance-free
  projection, not a weaker reading of this one.
- **Not a cache key on its own.** A digest names a candidate; a cache compares the exact bytes before returning a hit,
  and a preparation cache keys on every argument rather than on a track digest
  (`../across-stages/04-identity-and-realization.md` §7).

## N7 — What normalization forgets (on purpose)

Normalization erases: motif boundaries, repetition counts, transposition history, chord syntax, voice grouping,
file/import structure. All of it is recoverable where it matters — in provenance (source IDs, expansion paths, source
maps, HIR structure) — and none of it may leak back into core equality. If a consumer finds itself reconstructing
forgotten structure from the normal form, that is evidence of a missing **payload-level** or **consumer-level** concept,
not of missing core operations.
