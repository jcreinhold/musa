# Typed derivation diagrams

## 1. Artifacts and registries

A finite immutable artifact registry maps a versioned presentation reference to exactly one descriptor:

```text
PresentationRef = (PresentationId,ArtifactVersion)

PresentationDescriptor = {
  kind,
  schema,
  root,
  sites,
  anchor_table,
  canonical_manifest,
}.
```

Anchor ids are local to one versioned presentation. A qualified anchor is `(PresentationRef,AnchorId)`. The root and
every generation site occur in the descriptor's anchor table. Equal presentation references loaded from separate
artifacts must have exact equal descriptors and manifests; otherwise registry merge rejects an identity conflict.

A pass registry maps each `PassId` to one exact versioned descriptor containing its source kind, target kind, evidence
schema, region schema, and operation version. Equal ids with unequal descriptors are rejected.

## 2. Passes

A primitive pass has the semantic shape

```text
Pass S T E =
  Presentation S
  → Result {
      target  : Presentation T,
      lineage : Lineage S T,
      losses  : List E,
    }
    Diagnostic.
```

This is metalanguage notation for a compiler or package operation, not a required first-class source type. `losses`
records forgotten, approximated, selected, or externally supplied structure. Empty loss does not imply an isomorphism;
an inverse and its laws must be provided separately.

## 3. Primitive lineage hops

A lineage path alternates qualified anchors and typed hops. Primitive hop forms are:

```text
Preserved(pass,source,target,region,evidence)
Generated(pass,source_root,generation_site,target,region,evidence)
Combined(pass,sources,target,region,evidence)
```

Every displayed anchor resolves in the registry. The pass descriptor's source and target kinds must match. A generated
hop retains the root and generation site which justify its lack of an ordinary source anchor. A combined hop retains all
of its finite source anchors in canonical order.

## 4. Paths and lineages

A path is well formed when adjacent hops share the exact intermediate qualified anchor. Identity is the length-zero path
at an anchor. Composition is concatenation at an equal endpoint:

```text
p : a→b    q : b→c
────────────────── Path-Compose
q∘p : a→c.
```

Composition does not erase `b` or either primitive hop. Associativity follows from list concatenation; zero-length paths
are identities. Thus well-formed lineage paths form a category for one fixed well-formed registry.

A lineage artifact is a finite set of complete canonical paths. Canonical ordering and exact duplicate removal make
insertion order irrelevant. If two operational derivations with otherwise equal fields must survive, their stable
`DerivationId` fields differ; the representation is not simultaneously a bag and a set.

## 5. Pass composition

Given accepted passes `f:S→T` and `g:T→U`, their composite:

1. runs `f`;
2. on success runs `g` on `f.target`;
3. composes every compatible lineage path while retaining the intermediate `T` anchors;
4. concatenates typed loss records in pass order; and
5. returns the first canonical diagnostic on failure.

The function and path identity/associativity laws hold under pure deterministic pass execution and one well-formed
merged registry. They do not imply that separately developed passes commute.

## 6. The coherent project object

The coherent project artifact is the finite diagram containing its presentation descriptors, typed primitive pass edges,
lineage paths, and loss evidence. Its modest universal property is the free category of recorded paths: an assignment of
primitive edges to composable maps extends uniquely to paths.

This organizes provenance and comparison. It is not a universal musical object and proves no claim about harmony,
phrase, timbre, or perception.
