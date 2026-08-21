# Equality, hashes, audio preparation, and caches

**Status: governing.** When two stored results are equal, and when Musa may reuse a prepared one instead of recomputing
it.

This chapter answers one question: when may Musa safely reuse a result?

The short answer is that each type defines exact equality, a hash only finds candidates, and a cache checks the full
arguments before returning a hit.

## 1. Equality for event-track payloads

An event track can carry any admitted payload type `A`. `A` must be storable data, and it supplies:

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

The key may ignore fields on purpose. For example, an occurrence comparison may ignore fields used only to navigate back
to source. The schema must list those omitted fields. If the chosen fields or their encoding changes, the equality
version changes.

## 2. Equality for event tracks

For a track `M = (d, E)` in coordinate `C`, its semantic value contains:

- the coordinate tag `C`;
- the exact duration `d`; and
- the multiset of triples `(start, end, payload key)`, sorted into a fixed order.

Two tracks are equal when those values are equal. Construction order and human display formatting do not matter.
Duplicate occurrences remain duplicated. Tracks in different coordinates are never equal, because they are not even the
same type.

## 3. Exact byte encoding

Musa needs bytes that can be decoded in only one way. The track encoding contains:

1. a fixed tag saying that these are event-track semantic bytes;
2. the track format version;
3. the coordinate tag;
4. the payload owner id and equality version;
5. the exact rational duration;
6. the number of occurrences; and
7. each occurrence’s exact endpoints and payload key.

Every string or variable-length field is preceded by its byte length. Rational numbers have a unique reduced form. Human
display text is separate and may contain arbitrary newlines or words such as `from` and `occurrence`.

**Theorem I1: the encoding preserves and reflects track equality.** Fix one valid payload schema, one coordinate, and
one track format version. Two tracks have equal encoded bytes if and only if they have equal semantic values.

**Proof.** The fixed tags and field order identify each record field. Counts identify list boundaries, and byte lengths
identify variable fields. Reduced rationals have one representation. The bytes can therefore be decoded into exactly one
semantic value. Conversely, equal semantic values write the same fields in the same order. ∎

## 4. Equality for machines

A machine has two equalities, and Musa decides only one.

**Structural equality** compares, within one build's primitive registry, the primitive ids, versions, and exact
configurations at the leaves and the exact shape of the wiring tree. It has one meaning inside a build because
registration rejects one `(name, version)` pair with two definitions. It is decidable and it is what a cache key uses.

**Behavioural equality** says two machines produce the same output for every input history. It is the relation the laws
of `03-machine-calculus.md` §7 are stated in, and particular instances of it are proved there. **Neither the type
checker nor a cache may attempt to decide it.** Two structurally different machines may behave identically; a cache that
assumed the converse would return a result built from a different description.

A `Schedule<A>` adds a third, narrower relation: two are equal **up to handle renaming** when a consistent one-to-one
renaming of their private handles makes every frame's batch equal. Every instrument primitive must respect it
(`obligations.md` §15).

## 5. Hashes are indexes

Let `encode(M)` be the exact bytes above. Musa computes

```text
semantic_hash(M) = H(encode(M))
```

with a fixed hash function `H`. If two hashes differ, the encoded bytes differ. Equal hashes do not prove equal bytes,
because every finite hash has collisions.

Correctness-sensitive code uses the hash to choose a small candidate set and then compares the exact encoded values.
Display output is never used as the identity encoding.

## 6. Deterministic audio preparation

Audio preparation is one pure operation:

```text
prepare_execution(gesture track, bindings, seed, options)
    -> prepared machine or preparation error
```

`options` contains sample rate, channel layout, batching policy, render bounds, quality policy, and every other choice
that can change acceptance or execution. Bindings, seed, options, and the complete result each have a versioned equality
rule. The gesture track is the exact `EventTrack<PerformedTime,Gesture>` projection — not a full presentation and not a
digest.

Origin data used only for editor navigation is handled by a separate operation:

```text
prepare_lineage(gesture presentation, prepared machine)
    -> origin paths and recorded losses
```

It cannot change the prepared machine.

**Theorem R1: equal preparation arguments give equal results.** If all four arguments to `prepare_execution` are equal,
two calls return equal complete results.

**Proof.** `prepare_execution` is a pure deterministic function of exactly those arguments. Replacing any argument with
an equal value cannot change the function result. ∎

**Theorem R1-frames: equal prepared machines need more premises to produce equal samples.** Suppose preparation
succeeds, allocation follows the same rules, external input histories match, initial state and configurations match, and
every primitive obeys its registered deterministic contract. Then both runs produce equal output frames.

**Proof.** The runs begin with equal machines and equal start states. Apply machine Theorem M1 to the first equal input
frame, then repeat on the equal next states. Device-level floating-point equality is only as strong as the primitive
contract. ∎

**Theorem R1-batch: a valid batch changes nothing.** If a machine's `batch(n)` method satisfies the contract in
`03-machine-calculus.md` §4 for every valid state and input block, replacing `n` repeated steps by one `batch(n)` call
leaves the state and the outputs unchanged, so a caller's chosen partition of the same frames is unobservable.

**Proof.** This is the stated whole-machine contract, applied once per block and composed over the partition. What is
*not* proved is that any particular implementation satisfies it; that is per-primitive evidence, and feedback does not
inherit it from its children. ∎

## 7. Correct cache lookup

A preparation cache key contains every argument and version:

```text
ExecArgs = {
    preparation operation version,
    track format, coordinate, and payload versions,
    gesture track,
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

A key built from `semantic_hash(track) ⊕ bindings ⊕ seed` is **not** an instance of this theorem: it omits the options
and trusts a finite digest, and it fails on exactly the two counts this chapter exists to rule out.
