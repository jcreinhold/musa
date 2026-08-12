# T₂d — exact source closure

**Status: fixed closure target for independent review; governs nothing.** Review 50 accepted T₂c's bidirectional
checker, semantic calculus, `Text`/`Result` value encodings, and corrected probes. It found that type dependencies need
their own global DAG and that persistent package identity had been assumed where current Musa deliberately accepts
manifest-free and metadata-only projects. This note replaces those assumptions with exact formation judgments.

All unchanged rules and proofs are imported from [47](47-t2b-minimal-source-closure.md) and
[49](49-t2c-source-closure.md).

## 1. Two project graphs

Header collection initially assigns every user nominal declaration a unique symbolic path:

```text
NominalPath = (OwnerKey,canonical_module_path,canonical_structure_path,member_name).
```

Field syntax resolves to symbolic paths before any nominal stamp is constructed. Define two distinct finite graphs.

### 1.1 Nominal field-dependency graph

Vertices are all user nominal paths. Add an edge

```text
ν →_type μ
```

when a field of nominal `μ` contains nominal `ν`, including beneath product, `Option`, `List`, or `Result`. A
compiler-owned base/structural type adds no vertex. The direction is **dependency to consumer**. Reject a cycle,
including a self-edge. Choose the canonical topological order by taking the least owner-canonical path among ready
vertices.

In that order, construct each `NominalSchemaDescriptor μ` and exact stamp using only already constructed dependency
stamps. Its declaration rank is one plus the maximum dependency rank, or zero when it has none. Thus every nominal stamp
inside a field has strictly lower rank. The cross-structure example

```text
A.T contains B.T    B.T contains A.T
```

has a two-cycle and is rejected before any body is checked.

### 1.2 Value-definition graph

Vertices are every flattened ordinary/generated value definition. Add an edge

```text
dependency →_value consumer
```

for every free value reference in the consumer body, including sibling, qualified cross-structure, imported, private,
and generated-instance references. Reject a cycle. Return the canonical topological order, again choosing the least
owner-canonical ready path. Dependencies therefore precede consumers directly; no reverse-order convention is implicit.

### Lemma 1.1 — project rank and non-recursion are decidable

For a finite resolved build closure, both graph checks terminate. If they accept, every user nominal has a finite strict
rank and every value definition has a finite non-recursive dependency order.

*Proof.* Field/reference collection is structural over finite syntax. Cycle detection and canonically tie-broken
topological sorting are decidable on finite graphs. For the type graph, induction over its dependency-first order gives
every field dependency an already assigned smaller rank. For the value graph, every free dependency precedes its
consumer. ∎

## 2. Exact nominal schema identity

An accepted nominal declaration has this complete canonical descriptor:

```text
NominalSchemaDescriptor = {
  nominal_schema_format_version,
  ordered_constructors : List {
    constructor_name,
    ordered_fields : List FieldSchema,
  },
  equality_encoding_version,
}.

FieldSchema ::=
  Base(exact_compiler_base_schema)
| Nominal(dependency_StampKey)
| Product(List FieldSchema)
| Option(FieldSchema)
| List(FieldSchema)
| Result(FieldSchema,FieldSchema).
```

The descriptor is uniquely framed, with counts and length-framed variable fields. Constructor and field order are
identity-bearing. The `StampKey` is now:

```text
StampKey = {
  nominal_format_version,
  OwnerKey,
  canonical_module_path,
  canonical_structure_path,
  member_name,
  exact_NominalSchemaDescriptor,
}.
```

No manually maintained `member_schema_version` exists. A field/constructor/schema change mechanically changes the
complete descriptor and therefore the exact stamp. A digest may index a stamp table only after full `StampKey`
confirmation. A named migration may translate stored values between old and new stamps; no definitional equality or
silent reinterpretation results.

### Lemma 2.1 — schema edits cannot reuse a stamp

At a fixed owner/path, two nominal declarations have equal `StampKey`s iff their complete admitted schema descriptors
are equal.

*Proof.* The path components are fixed. Unique record framing reduces stamp-key equality to exact descriptor-byte
equality, which reduces structurally to equality of constructor names/order, field schemas/order, and encoding version.
Conversely equal descriptors deterministically encode equally. ∎

The `Item(Nat)`/`Item(Text)` revisions therefore have unequal stamps without asking an author to remember a version
bump.

## 3. Owner identity without rejecting loose projects

Current Musa has three local situations, not one. Define:

```text
OwnerKey ::=
  Snapshot(exact_source_closure_manifest)
| LocalPackage(stable_package_id,canonical_package_name)
| Bundled(distribution_key,canonical_package_name)
| PinnedGit(canonical_remote_url,
            commit_algorithm,full_commit_object_id,
            canonical_package_subdirectory).
```

### 3.1 Snapshot owners

A loose `.musa` file or a directory governed only by the current metadata `[project]` manifest receives `Snapshot`. Its
exact source-closure manifest is a count/length-framed list of canonical logical document names and exact source bytes,
plus compiler/source-language version. It contains complete bytes, not only a digest. Logical names are the resolved
closure's module/document names, not absolute filesystem paths.

Snapshot nominal values are valid, exact, and deterministic within that source snapshot. Any source edit may change
their owner key even when the nominal declaration is unchanged. They are therefore marked **snapshot-local** and may not
be exported as a persistent nominal API or decoded under another snapshot owner. This preserves “one file until it needs
a package”: ordinary loose projects compile and can use nominal abstraction internally; stable external nominal identity
requires opting into a package.

### 3.2 Identity-bearing local packages

A source package which exports persistent nominal types adds explicit package identity to its existing `[package]`
manifest shape:

```toml
[package]
name = "example.practice"
identity = "uuid:…"
```

The exact finite UUID grammar and package-name grammar are fixed by the implementing prompt; malformed or duplicate
identity is a diagnostic. Moving the directory preserves the owner. Copying both fields intentionally preserves nominal
ownership; a fork which wants distinct public types changes `identity`. Existing metadata-only `[project]` manifests are
unchanged and remain `Snapshot` owners.

### 3.3 Bundled and pinned owners

`Bundled` uses a versioned distribution key resolved to one exact bundled package descriptor and canonical manifest.
`PinnedGit` uses the exact lock edge and resolves to one fetched package descriptor plus a complete canonical tree
manifest. Commit/tree digests are lookup/verification fields, never exact equality substitutes.

## 4. Well-formed owner table and merge

One immutable finite `OwnerTable` maps every `OwnerKey` in a build to exactly one canonical `OwnerDescriptor`:

```text
OwnerDescriptor = {
  owner_kind,
  canonical_package_metadata,
  immutable_manifest : Option CanonicalBytes,
}.
```

`Snapshot` and `LocalPackage` descriptors record their exact key/metadata; only `Snapshot` carries changing source bytes
inside its key. Bundled/pinned descriptors retain the complete immutable manifest/tree bytes against which distribution,
commit, and tree digests were verified.

`OwnerTableWellFormed` requires:

1. unique map keys and functional lookup;
2. descriptor kind/key agreement;
3. every referenced logical module path exists in the resolved source closure;
4. every bundled/pinned immutable manifest is present and verified;
5. every collected nominal path is unique; and
6. every equal nominal `StampKey` resolves to a byte-identical complete schema descriptor.

Merging two artifact owner tables compares full keys and full canonical descriptors. An equal key with unequal
descriptor/immutable manifest, or an equal stamp with unequal nominal schema descriptor, is rejected. Digests may locate
candidates but never bypass these exact comparisons. The merged table is immutable during typechecking and lineage or
value decoding.

### Lemma 4.1 — owner formation is total and collision-safe

Every current loose/metadata project, identity-bearing local package, bundled package, and exact-pinned package in an
accepted build has exactly one owner lookup. Under `OwnerTableWellFormed`, finite digest collisions cannot identify
unequal owner or nominal descriptors.

*Proof.* The root classification is total and the four tagged constructors are disjoint. Functional lookup gives one
descriptor. Snapshot equality includes complete manifest bytes. Local equality uses its exact stable identity while
nominal schema equality remains in `StampKey`. Bundled/pinned equality is admitted only in a table retaining and
comparing complete descriptors/manifests. Thus a digest collision may select an additional candidate, but exact merge or
lookup rejects it. ∎

Import aliases are erased by resolution before `OwnerKey`, module path, or `StampKey` formation, so they cannot change
nominal identity.

## 5. Unambiguous structural result schemas

The canonical structural schema and value encoding for `Result A E` are exactly:

```text
ResultSchema = record(
  result_type_id,
  result_schema_version,
  length(enc_schema(A)), enc_schema(A),
  length(enc_schema(E)), enc_schema(E),
)

Ok(a) = record(ResultSchema,OkTag, length(enc_A(a)),enc_A(a))
Err(e)= record(ResultSchema,ErrTag,length(enc_E(e)),enc_E(e)).
```

`record` begins with a field count and length-frames every variable field; fixed tags have one versioned fixed-width
encoding. Thus `(A="a",E="bc")` and `(A'="ab",E'="c")` cannot share schema bytes. T₂c Lemma 3.1 now applies without an
implicit reading of `frame`.

## 6. Bidirectional completeness boundary

T₂c's checker is sound and decidable, and intentionally complete only up to annotations. A check-only `Ok`/`Err` nested
where synthesis is required gets a diagnostic suggesting an exact annotation such as `(: Result Nat Text)`. This is the
same annotation discipline used for empty lists/options; no unification variable or implicit generalization is
introduced.

## 7. Closed project judgment

The project algorithm is:

1. build and validate the exact owner table/source closure;
2. collect unique symbolic nominal/value headers;
3. resolve and check the nominal field DAG; in dependency order construct complete schemas/stamps/ranks;
4. bidirectionally check signature/structure/value bodies with sealed public and retained core environments;
5. flatten all value bodies and check the global value DAG;
6. emit dependency-first non-recursive core lets; and
7. retain exact owner/nominal descriptors beside separately compiled or persisted nominal values.

### Theorem 7.1 — T₂d source closure

Assume the governing foreign-operation, contextual-`Music`, deterministic resource, and immutable build-input contracts.
Every project accepted by the displayed T₂d judgment has decidable functional checking, a finite rank-ordered nominal
family, a finite well-typed non-recursive core program, representation opacity, deterministic type-preserving strongly
normalizing evaluation, exact canonical nominal values, and alias-independent owner identity.

Persistent nominal APIs/values additionally require a non-`Snapshot` owner; attempting to export one from a snapshot
owner is a static diagnostic.

*Proof.* T₂c Lemma 1.1 gives bidirectional checking. Lemma 1.1 here supplies both strict nominal ranks and dependency-
first value lets. T₂b/T₂c sealing preserves source opacity and target typing. The reviewed T₂b reducibility proof
applies to the rank-ordered family and the governing topological-let proof applies to the value program, giving safety
and strong normalization. T₂c `Text`, §5 `Result`, and T₂b nominal canonical-data induction, with complete stamps from
Lemma 2.1, give exact value identity. Lemma 4.1 supplies total collision-safe owner lookup; resolution erases aliases.
Snapshot export rejection isolates its deliberately source-snapshot identity from persistent APIs. ∎

## 8. Recommendation if accepted

If the proof closes, schedule one source-language implementation prompt for exactly this fragment and package-identity
opt-in. Keep the two reviewed mechanism probes and add one real algorithm per materially different theory owner before
graduation. The proof still does not justify abstract-member templates, aliases, recursive data, dependency, worlds, or
theta-links, and it proves no cultural adequacy claim.
