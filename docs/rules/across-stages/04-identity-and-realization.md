# Equality, hashes, audio preparation, and caches

This chapter answers one question: when may Musa safely reuse a result?

The short answer is that each type defines exact equality, a hash only finds candidates, and a cache checks the full
arguments before returning a hit.

## 1. Equality for timeline payloads

A timeline can carry any admitted finite payload type `A`. That type supplies:

```text
PayloadSchema<A> = {
    owner type id,
    equality version,
    key : A -> bytes,
}
```

The key is total and deterministic. It defines equality for this use:

```text
a ≡_A b  exactly when  key(a) = key(b)
```

The key may ignore fields on purpose. For example, an event comparison may ignore fields used only to navigate back to
source. The schema must list those omitted fields. If the chosen fields or their encoding changes, the equality version
changes.

## 2. Equality for timelines

For a timeline `M = (d, E)`, its semantic value contains:

- exact length `d`; and
- the multiset of triples `(start, end, payload key)`, sorted into a fixed order.

Two timelines are equal when those values are equal. Construction order and human display formatting do not matter.
Duplicate occurrences remain duplicated.

## 3. Exact byte encoding

Musa needs bytes that can be decoded in only one way. The timeline encoding contains:

1. a fixed tag saying that these are timeline-semantic bytes;
2. the timeline format version;
3. the payload owner id and equality version;
4. the exact rational timeline length;
5. the number of occurrences; and
6. each occurrence’s exact endpoints and payload key.

Every string or variable-length field is preceded by its byte length. Rational numbers have a unique reduced form. Human
display text is separate and may contain arbitrary newlines or words such as `from` and `occurrence`.

**Theorem I1: the encoding preserves and reflects timeline equality.** Fix one valid payload schema and one timeline
format version. Two timelines have equal encoded bytes if and only if they have equal semantic values.

**Proof.** The fixed tags and field order identify each record field. Counts identify list boundaries, and byte lengths
identify variable fields. Reduced rationals have one representation. The bytes can therefore be decoded into exactly one
semantic value. Conversely, equal semantic values write the same fields in the same order. ∎

## 4. Hashes are indexes

Let `encode(M)` be the exact bytes above. Musa computes

```text
semantic_hash(M) = H(encode(M))
```

with a fixed hash function `H`. If two hashes differ, the encoded bytes differ. Equal hashes do not prove equal bytes,
because every finite hash has collisions.

Correctness-sensitive code uses the hash to choose a small candidate set and then compares the exact encoded values.
Display output is never used as the identity encoding.

## 5. Deterministic audio preparation

Audio preparation is one pure operation:

```text
prepare_execution(gesture meaning, bindings, seed, options)
    -> prepared execution or preparation error
```

`options` contains sample rate, channels, semantic step policy, render bounds, quality policy, and every other choice
that can change acceptance or execution. Bindings, seed, options, and the complete result each have a versioned equality
rule.

Origin data used only for editor navigation is handled by a separate operation:

```text
prepare_lineage(gesture representation, prepared execution)
    -> origin paths and recorded losses
```

It cannot change the prepared execution.

**Theorem R1: equal preparation arguments give equal results.** If all four arguments to `prepare_execution` are equal,
two calls return equal complete results.

**Proof.** `prepare_execution` is a pure deterministic function of exactly those arguments. Replacing any argument with
an equal value cannot change the function result. ∎

**Theorem R1-frames: equal prepared plans need more premises to produce equal samples.** Suppose preparation succeeds,
allocation follows the same rules, external input histories match, initial state and parameters match, and every
processor obeys the deterministic process contract. Then both runs produce equal output steps.

**Proof.** The runs begin with equal graphs and state. Apply process Theorem P1 to the first equal input step, then
repeat on the equal next states. Device-level floating-point equality is only as strong as the processor contract. ∎

## 6. Correct cache lookup

A preparation cache key contains every argument and version:

```text
ExecArgs = {
    preparation operation version,
    timeline and payload versions,
    gesture meaning,
    binding version and values,
    seed,
    option version and values,
}
```

The cache stores:

```text
(hash(exact ExecArgs bytes), exact ExecArgs bytes, complete result)
```

Lookup first uses the hash, then compares the exact `ExecArgs` bytes. Only an exact match returns a hit.

**Theorem C1: a returned cache hit equals recomputation.**

**Proof.** Exact argument equality gives the same operation version and the same value for every argument. The cache
stores only results produced by that operation on those stored arguments. Theorem R1 therefore makes the stored result
equal to recomputation. A hash collision merely adds a candidate whose exact bytes fail the second check. ∎
