# Canonical framing is a governing bug

**Status: verified defect and repair design; governs nothing until the governing specification and code are amended.**
The independent K₃.1 review [39](39-proof-review-k3.1.md) found a counterexample in the current generic temporal kernel.
This note checks the defect, separates it from ordinary hash collisions, and chooses the smallest repair.

## 1. The exact defect

The current kernel has two different notions which its documentation identifies:

1. structured semantic equality compares

   ```text
   (extent, sorted multiset of (start,end,payload-key));
   ```

2. `write_canonical` prints each payload key bare inside

   ```text
   occurrence <payload-key> from <start> to <end>;
   ```

   and `semantic_hash` hashes those printed bytes.

`Canonical for String` returns the string unchanged. Therefore payload bytes can contain the record delimiters and
newlines used by the outer format. Let `M` have extent 2 and one occurrence carrying the string

```text
a from 0 to 1;\n  occurrence b
```

over `[1,2]`. Let `N` have extent 2, carrying `a` over `[0,1]` and `b` over `[1,2]`. Both print exactly:

```text
timeline 2 {
  occurrence a from 0 to 1;
  occurrence b from 1 to 2;
}
```

But `M` and `N` are not structurally semantically equal: their occurrence multisets have different cardinalities and
spans. The collision exists **before** FNV-1a is applied. It is not the documented possibility that two distinct byte
strings can share a finite digest.

## 2. What was verified

I inspected the current implementations:

- `Canonical for String` in `crates/musa-kernel/src/occurrence.rs` returns the string unchanged;
- `Timeline::semantic_eq` in `timeline.rs` compares extent and ordered `(start,end,key)` triples;
- `Timeline::write_canonical` interpolates the key without quoting, escaping, or length framing; and
- `Timeline::semantic_hash` feeds exactly that text to FNV-1a.

The governing `docs/rules/events/05-normalization.md` also contains a separate contradiction. N3 first requires keys to
be injective on Rust values, then explicitly permits `ScoreFact` keys to quotient away `definition_span` and declaration
id. The implemented equality follows the second idea: a payload owner's key declares the payload equality observed by
the kernel. That is a defensible design, but it is not injectivity on the complete stored Rust value.

I have not established a second concrete collision inside `ScoreFact::canonical_key`. Its construction contains several
ad-hoc textual delimiters and compiler `Debug` output, so it deserves a field-framing audit, but plausibility is not a
counterexample. The generic `String` example is already sufficient to refute the current N5 identity claim.

## 3. The natural definitions

Do not repair semantic identity by making a presentation printer pretend to be a data encoding. Define the payload
owner's admitted equality first:

```text
key_A : A → ByteString

a ≈_A b  iff  key_A(a) = key_A(b).
```

The owner must document why this is the intended quotient. The kernel requires `key_A` to be total and deterministic. If
the owner claims a pre-existing equality, it must additionally prove that the key is sound and complete for that
equality. It need not be injective on storage fields deliberately absent from `≈_A`.

Then define the exact semantic form as data, not text:

```text
Sem_A(M) =
  ( extent(M),
    sort [(start(o),end(o),key_A(payload(o))) | o∈occurrences(M)] ).

M ≡_A N  iff  Sem_A(M) = Sem_A(N).
```

This is the equality the current `semantic_eq` already computes.

When stable bytes are needed, use one injective, domain-separated encoding of this structure:

```text
encode_sem_A(M) =
  tag("musa.timeline.semantic")
  ++ schema_version
  ++ encode_ratio(extent)
  ++ encode_nat(number_of_occurrences)
  ++ concat [
       encode_ratio(start)
       ++ encode_ratio(end)
       ++ encode_nat(byte_length(key))
       ++ key
     ].
```

Every integer encoding has one canonical spelling; every variable byte field is length-prefixed; and the occurrence
count frames the list. The exact Rust representation can stream these fields to a sink rather than allocate the whole
byte string. The important property is unique decodability, not a particular varint scheme.

Human `Display` is a different projection. It may quote keys for readability, and its output may change without changing
semantic identity. Kernel interchange text remains its existing third representation. Conflating all three saved one
writer but destroyed the claimed invariant.

## 4. The repaired laws

### Lemma F1 — structured equality is decidable

If exact rational equality and byte-string equality are decidable, then `M≡_A N` is decidable for finite timelines.

*Proof.* Compare extents, finite list lengths, and corresponding sorted triples. Every component comparison is decidable
and the lists are finite. ∎

### Lemma F2 — the framed encoding is injective

For one schema version,

```text
encode_sem_A(M) = encode_sem_A(N)  iff  Sem_A(M)=Sem_A(N).
```

*Proof.* The tag and version fix the grammar. Canonical rational encodings decode uniquely. The occurrence count fixes
the number of records, and each payload length fixes exactly the next key bytes. Decoding equal byte strings therefore
yields equal structured forms. Conversely, equal structures are encoded componentwise by the same deterministic
functions. ∎

### Corollary F3 — hash agreement is one-way

With `h(M)=FNV1a128(encode_sem_A(M))`,

```text
M≡_A N  ⇒  h(M)=h(N).
```

The converse is false for cardinality reasons. A cache may use `h` as a lookup accelerator, but a hit must confirm the
complete structured key or its injective framed bytes at the same schema and implementation version.

## 5. Concrete repair

The repair should be one deliberate semantic-identity change:

1. amend N3 to say that a key is sound and complete for the payload owner's **admitted equality**, not injective on all
   stored fields;
2. amend N5 so human canonical display is not the definition of semantic identity;
3. define the domain-separated, versioned, framed encoding used by N6;
4. change `semantic_hash` to stream that encoding;
5. add the exact `Timeline<String>` collision above as a regression;
6. add property tests that length framing distinguishes arbitrary keys containing newlines, quotes, delimiters, and N5
   keywords; and
7. audit `ScoreFact::canonical_key` separately, replacing delimiter concatenation by field-framed writing if any field
   is not already canonically self-delimiting.

Stored hashes will change. That is correct: the old identity was not an identity. The schema/version boundary must make
old cache entries miss rather than reinterpret them.

