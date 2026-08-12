# T₂e — source-unit ownership and stable public types

**Status: fixed closure target for independent review; governs nothing.** Review [52](52-proof-review-t2d.md) accepted
T₂d's two project DAGs, exact nominal schemas, structural `Result` framing, and bidirectional boundary. It rejected the
rank base and the build-wide `Snapshot` owner. This note changes only those definitions. All term, typing, sealing,
encoding, and evaluation rules not named below are imported from [47](47-t2b-minimal-source-closure.md),
[49](49-t2c-source-closure.md), and [51](51-t2d-source-closure.md).

## 1. The nominal rank starts above the structural sentinel

For the reducibility proof, a type containing no nominal stamp has nominal rank zero. A user nominal with no nominal
field dependency has declaration rank **one**. In dependency-first topological order, every other nominal has rank

```text
1 + max(rank of every nominal stamp occurring in a field).
```

Thus a nominal stamp always has positive rank and every structural field type in its representation has strictly smaller
maximum nominal rank. In particular, if

```text
data Words { WordsValue(List Text) }
```

then `rank(Words)=1` while `r(List Text)=0`. T₂b's lexicographic induction on `(maximum nominal rank,type size)` applies
literally: the nominal clause descends in the first component, and structural clauses at fixed rank descend in size.

## 2. Ownership is per source unit, not per build world

There is no closure-wide snapshot owner and no source-level `world`. Resolution first partitions a finite build closure
into **source units**:

- one declared package and its declared module tree is one unit;
- the bundled standard library is one unit;
- one exact-pinned package and its declared module tree is one unit; and
- a loose or `[project]`-owned entry document plus the local libraries reached from that entry is one unit. Project
  metadata does not make unrelated pieces part of the entry's compilation unit.

A local import inside a unit resolves to an owner-local logical module. A package import resolves to another unit. The
governing static import graph is finite and acyclic, so external owner dependencies can be formed dependency first.

### 2.1 Exact source-unit manifests

Every unit has a uniquely framed `SourceUnitManifest`:

```text
SourceUnitManifest = {
  source_unit_format_version,
  source_language_version,
  compiler_owned_schema_versions,
  unit_kind,
  canonical_package_metadata : Option CanonicalData,
  modules : OrderedMap LogicalModulePath {
    exact_source_bytes,
    resolved_external_imports : OrderedList {
      source_site,
      imported_OwnerKey,
      imported_LogicalModulePath,
    },
  },
}.
```

Counts and variable fields are length-framed. Internal imports name only another key in the same module map and cannot
make the manifest recursively contain itself. External imports contain already formed dependency owners.

For a declared package, `LogicalModulePath` is exactly its declared `lib.musa`/`mod.musa` tree path. For a loose entry,
the entry is `$entry`; each relative import is lexically normalized from its importing logical path. Imports escaping
the entry root, following an external symlink, or colliding under the target filesystem's case policy are rejected, as
in the governing package rules. Absolute filesystem paths never occur. Consequently relocating a complete source unit
without changing its relative tree preserves its manifest.

The module map is keyed, not merely listed: two resolved documents in one unit cannot share a logical path. Every
owner-qualified nominal path is checked against this exact map.

### 2.2 Owner keys

```text
OwnerKey ::=
  Snapshot(exact_SourceUnitManifest)
| LocalPackage(StablePackageId,CanonicalPackageName)
| Bundled(DistributionKey,CanonicalPackageName)
| PinnedTree(CanonicalRemoteUrl,
             CanonicalPackageSubdirectory,
             exact_CanonicalPackageTree).
```

The cases are total for current and proposed sources:

- a loose entry, a `[project]` entry, and an existing `[package]` without `identity` receive `Snapshot` of **their own
  unit**;
- a local `[package]` with explicit identity receives `LocalPackage`;
- the embedded standard library receives `Bundled`; and
- a locked remote dependency receives `PinnedTree` after its fetched tree is verified.

Two byte-identical snapshot units with equal resolved dependency owners intentionally have equal applicative nominal
ownership. Units differing in any source byte, logical module name, imported owner, or schema/compiler version differ.
Because the key belongs to one source unit rather than the whole build, two packages with internal path
`theory.Practice.T` do not collide unless their complete unit manifests are equal; in that case their declarations and
dependencies are also equal by construction.

`PinnedTree` deliberately identifies package semantics by exact canonical package-tree bytes, remote, and subdirectory.
The lock's full commit object id is verification/retrieval evidence, not mathematical equality. Two commits with the
same exact admitted package tree denote the same pinned package owner; commit authorship and parent history are not Musa
type identity.

### 2.3 Stable local identity grammar

An identity-bearing local package contains:

```toml
[package]
name = "example.practice"
identity = "uuid:01234567-89ab-cdef-0123-456789abcdef"
```

`CanonicalPackageName` matches `[a-z][a-z0-9]*(\.[a-z][a-z0-9]*)*`. `StablePackageId` is `uuid:` followed by lowercase
hexadecimal groups of lengths 8-4-4-4-12. This is an exact user-asserted namespace, not a probabilistic equality proof.
Equal `(id,name)` occurrences with equal admitted public descriptors coalesce; equal keys with a conflicting descriptor
are a diagnostic. Copying a package intentionally preserves its owner. A fork that needs distinct ownership changes the
id.

## 3. Owner-qualified formation and exact registries

The symbolic nominal path remains

```text
(OwnerKey,LogicalModulePath,canonical_structure_path,member_name).
```

For every accepted unit, `OwnerWellFormed` requires:

1. the owner constructor agrees with the unit classification;
2. each `(OwnerKey,LogicalModulePath)` resolves in that owner's exact module map or bundled/pinned descriptor;
3. nominal paths within that owner-qualified module are unique;
4. every external import target resolves in the immutable build owner table; and
5. an equal complete `StampKey` always has a byte-identical nominal schema descriptor.

The artifact `NominalRegistry` is a finite map from complete `StampKey` to the complete uniquely framed schema
descriptor and its transitive dependency descriptors. Merge uses a digest only to find candidates, then compares full
keys and descriptors. Equal entries coalesce; an equal key with unequal bytes is rejected. Different schemas at the same
stable owner/path have different stamps and may coexist as versioned stored values because the complete schema is inside
the stamp; the current build's resolved path selects only its current stamp.

Snapshot source aliases are presentation bytes, so renaming an alias may change a snapshot owner. The law is therefore
narrow and exact: **after resolution in one fixed owner table**, every reference alias to the same
`(OwnerKey,LogicalModulePath,member path)` produces the same stamp. Client alias edits do not affect stable, bundled, or
pinned dependency owners. No stronger alpha-invariance is claimed for exact-source snapshots.

## 4. Stable public types are a recursive judgment

Snapshot nominal abstraction is valid inside its exact unit. It must not leak into a separately compiled stable API.
Define `StablePublic(τ)` over an accepted acyclic nominal registry:

```text
StablePublic(Base b)                       when b is compiler-owned and versioned
StablePublic(A × B)                        when StablePublic(A), StablePublic(B)
StablePublic(Option A)                     when StablePublic(A)
StablePublic(List A)                       when StablePublic(A)
StablePublic(Result A E)                   when StablePublic(A), StablePublic(E)
StablePublic(A → B)                        when StablePublic(A), StablePublic(B)
StablePublic(Nominal μ)                    when owner(μ) is not Snapshot
                                           and every field type in schema(μ) is StablePublic.
```

The nominal clause is decidable by induction over the dependency-first nominal rank. Require `StablePublic` of every
type exposed by a stable local, bundled, or pinned public signature. This rejects both

```text
Stable.Box(Snapshot.Temp)
Snapshot.Temp → Text
```

and the same leak under any structural wrapper. An identity-less package may compile and use private nominal
abstraction, but receives a targeted diagnostic if it exports such a type across its snapshot boundary; adding
`[package].identity` is the explicit opt-in.

Exact serialization is a different property from stable public ownership. Any accepted nominal value is exact canonical
data when stored with its complete `StampKey` and registry dependencies. A cache scoped to the exact snapshot may
therefore serialize it safely. Importing it under a different snapshot is a type mismatch, not reinterpretation.

### Lemma 4.1 — stable-public checking terminates and closes transitively

For every accepted type, `StablePublic` is decidable. If it accepts a type, no nominal stamp reachable through its
structural children or nominal representations has a `Snapshot` owner.

*Proof.* Structural cases recurse on strict type subexpressions. A nominal case recurses only into field nominals of
strictly lower positive rank. The lexicographic pair `(maximum nominal rank,type size)` therefore decreases. The closure
property follows by the same induction and the premises of each rule. ∎

## 5. Repaired closure theorem

The T₂e project algorithm is T₂d's seven steps with three refinements: form source units and owners before nominal
headers, begin nominal ranks at one, and check `StablePublic` on every stable public signature before emission.

### Theorem 5.1 — T₂e source closure

Assume the governing foreign-operation, contextual-`Music`, deterministic-resource, immutable-build-input, and exact
compiler-owned canonical-data contracts. Every project accepted by T₂e has:

1. decidable functional bidirectional checking, complete up to the stated annotations;
2. a finite positive-rank nominal family and finite dependency-ordered non-recursive core program;
3. representation opacity, preservation, progress, deterministic evaluation, and strong normalization;
4. exact, schema-versioned canonical nominal values;
5. total owner formation for loose entries, metadata projects, identity-less packages, identity-bearing packages,
   bundled packages, and exact-tree-pinned packages;
6. owner-qualified paths invariant under importing aliases in a fixed owner table and under relocation of an unchanged
   logical source tree; and
7. no transitive snapshot nominal in a stable public signature.

*Proof.* Item 1 is T₂c's bidirectional theorem plus the two finite T₂d graph checks. Item 2 follows from
dependency-first topological order and the repaired positive rank. Item 3 is T₂b's reducibility proof, whose nominal
call now strictly decreases rank. Item 4 follows by induction over the nominal DAG and uniquely framed child encodings.
Item 5 is by case analysis on the six listed source situations and the four owner constructors; the three snapshot
situations share one constructor but distinct per-unit manifests. Item 6 follows because paths contain owners and
owner-local logical module keys, never importing aliases or absolute paths; the stated snapshot alias qualification is
part of the premise. Item 7 is Lemma 4.1 applied to every exported type. ∎

## 6. What this still does not prove

T₂e adds no dependent type, recursive type, first-class module, world, or link. It does not prove the cultural adequacy
of a music-theory package. The W and K programs from [47](47-t2b-minimal-source-closure.md) remain mechanism probes: the
next gate after metatheoretic closure is implementing at least two complete, differently framed theory packages and
testing whether their definitions remain simple.
