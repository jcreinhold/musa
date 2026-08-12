# Identity and storage architecture

## 1. Four equalities

The implementation must name rather than conflate:

| Equality | Example owner | What it may forget |
| --- | --- | --- |
| Presentation semantic equality | `Timeline<A>`, a notation plan schema | construction/display history chosen by its schema |
| Derivation equality | project artifact registry | insertion order, exact duplicate paths |
| Execution equality | `musa-audio` prepared plan schema | presentation-only lineage fields |
| Observation equality | renderer/engine conformance test | only what its explicit tolerance or bit contract states |

A type called `SemanticHash` is not enough to bridge rows. Each conversion theorem names both relations.

## 2. Canonical-data pattern

Every persisted identity-bearing record implements one internal contract equivalent to:

```text
CanonicalData X = {
  schema,
  encode : X → bytes,
  compare : X×X → Ordering,
}
```

Encoding equality and comparison equality coincide. Records use fixed domain/version tags and length-frame every
variable child. Display/diagnostic formatting is separate. This pattern applies recursively to temporal payload schemas,
nominal stamps, presentation refs, anchors, lineage paths, bindings, seeds, options, and prepared result schemas.

The concrete Rust API need not be one public trait. Prefer private writer functions and narrow owner methods; stabilize
a shared facade only when multiple real owners need the same implementation.

## 3. Temporal identity migration

The current `Canonical::canonical_key() -> String` remains the payload's admitted semantic key for now, but its contract
changes from “injective on stored Rust values” to “complete for the declared equality class.” It gains owner type and
quotient version metadata. `Timeline::semantic_hash` writes a versioned framed semantic record; it no longer hashes
`Display` output.

Migration steps:

1. add schema metadata to every current `Canonical` implementation;
2. add the known newline/delimiter counterexample before changing the writer;
3. implement one private framed semantic writer and hash it;
4. retain stable display output only for human/golden consumers;
5. change stored-cache versions so old N5 digests cannot be interpreted as new identity; and
6. add property tests over arbitrary payload key strings and occurrence multiplicity.

The compiler's `ScoreFact` key deliberately omits definition/declaration presentation fields. That quotient is recorded
and versioned; it is not described as injective on the full struct.

## 4. Cache records

A correctness-sensitive cache stores:

```text
(digest, exact_argument_bytes, exact_result)
```

The digest selects a bucket. Exact bytes confirm a candidate. The operation version, all argument schema versions,
sample/channel/tick options, bindings, seed, and semantic gesture value occur in the argument record. Cache insertion
happens only after the named pure operation returns.

Lineage is cached separately when its inputs include presentation data which execution semantics quotient away. A
lineage cache hit cannot replace an execution result, and an execution cache hit does not imply equal lineage.

## 5. Artifact registry storage

One stored artifact contains or resolves:

- its `(PresentationId,ArtifactVersion)`;
- exact presentation descriptor and schema;
- canonical anchor table/root/sites;
- canonical manifest bytes;
- pass descriptors used by lineage; and
- normalized complete lineage paths and loss records.

Digests may index manifests. Loading/merging validates exact descriptors after lookup and rejects an id conflict. The
registry is immutable while a derivation is checked; a source edit creates a new artifact version rather than mutating
the meaning behind an existing reference.

## 6. Migration and compatibility

Schema changes are explicit. A reader either:

- reads the old version and applies a named checked migration which records loss;
- recomputes from canonical source/package inputs; or
- rejects the artifact with a version diagnostic.

It never reuses a version number for a new quotient, ignores an unknown field which affects equality, or accepts a
digest because its length “looks right.”
