# T₂f — selecting one package implementation

**Status: fixed closure target for independent review; governs nothing.** Review [54](54-proof-review-t2e.md) accepted
T₂e’s rank repair, per-unit snapshot owners, exact tree identity for pinned packages, owner-qualified paths, and
recursive ban on snapshot types in stable APIs. It found that T₂e still failed to choose one active implementation for a
stable package and used one rule for two different questions: which types may appear in a public API, and which values
may be saved as canonical data.

This note changes only those points. It imports every other rule and proof from T₂b through T₂e.

## 1. Stable owner and active implementation are different ids

A stable local package keeps the owner key defined in T₂e:

```text
OwnerKey = LocalPackage(stable package id, canonical package name).
```

That key answers “which package owns this nominal type?” It should survive an edit to a function body.

The current package body has a separate exact id:

```text
ImplementationKey = encode {
    compiler and source-language versions,
    exact source-unit manifest,
    resolved external owner keys,
    public interface descriptor,
}.
```

This key answers “which code did this build use?” A source or dependency edit changes it even when the package owner and
public nominal schema stay the same.

### The active owner table

One accepted build has an immutable finite table:

```text
ActiveOwnerTable : OwnerKey -> ActiveImplementation

ActiveImplementation = {
    implementation key,
    exact module map,
    exact public interface,
    active nominal path -> complete StampKey,
    checked value definitions,
}.
```

The table is valid only if:

1. every resolved package reference selects exactly one table entry;
2. an owner key has exactly one active implementation;
3. byte-identical repeats coalesce;
4. equal owner keys with different implementation keys are a build conflict;
5. every active nominal path maps to exactly one current stamp; and
6. every module, definition, and public member used by checking belongs to that active implementation.

Thus two copied packages with the same stable id either contain the same exact implementation and coalesce, or differ
and cause a clear conflict. A dependency resolver may choose a new implementation in a later build. Within one build it
cannot mix the two.

### Historical nominal data remains separate

The `NominalRegistry` from T₂e stores complete stamps and schemas needed to decode old values. It may contain several
schema versions from the same stable owner and source path. It does not choose code or resolve a current source name.

Source checking uses `ActiveOwnerTable`. Decoding a saved nominal value uses `NominalRegistry`. A linkage or execution
cache includes `ImplementationKey`, so equal public type stamps do not cause a stale function body to be reused.

**Lemma 1.1.** In one valid active owner table, every resolved stable package member has one type and one checked body.

**Proof.** The package reference selects one owner key by rule 1. Rules 2 and 4 select one active implementation. The
owner-qualified module map and rules 5–6 then select one current member, stamp, and body. ∎

## 2. Public interface types and saved value types are different checks

The old `StablePublic` rule tried to do both jobs. Replace it with two rules.

### Types allowed in a stable public interface

`StableInterface(A)` means that type `A` may appear in the public signature of a stable, bundled, or pinned package. It
contains:

```text
StableInterface(B)                         for every versioned compiler-owned source type B,
                                           including Music, Pitch, Duration, Text, and the existing base types
StableInterface(A × B)                     when both children pass
StableInterface(Option A)                  when A passes
StableInterface(List A)                    when A passes
StableInterface(Result A E)                when both children pass
StableInterface(A -> B)                    when both sides pass
StableInterface(Nominal μ)                 when μ has a non-Snapshot owner and every field type in μ passes.
```

Compiler-owned source types have fixed versioned type ids. This rule allows the existing signatures

```text
subject : Music
answer  : Music -> Music
```

without claiming that a `Music` value or function closure has a canonical saved-data encoding.

### Types whose values may be saved as canonical data

`StorableValue(A)` is narrower:

```text
StorableValue(B)                           for compiler-owned finite canonical-data base types B
StorableValue(A × B)                       when both children pass
StorableValue(Option A)                    when A passes
StorableValue(List A)                      when A passes
StorableValue(Result A E)                  when both children pass
StorableValue(Nominal μ)                   when μ has an exact complete schema and every field passes.
```

Functions do not pass. `Music` passes only if a later, separately reviewed canonical-data schema is defined for it; the
current rule does not assume one. Snapshot nominals may be stored in a cache scoped to their exact snapshot because
their complete owner and schema travel with the value. They may not appear in a stable public interface.

**Lemma 2.1.** Both checks terminate.

**Proof.** Structural cases recurse on strict type children. Nominal cases recurse only into fields with smaller
positive nominal rank. The pair `(largest nominal rank, type size)` decreases at every call. ∎

## 3. Logical paths for loose entries with `../` imports

Current loose-file fixtures may import a sibling library with `../library/file.musa`. T₂e’s synthetic `$entry` root did
not define a logical parent for that path.

For one resolved loose-entry source unit:

1. resolve its finite local import graph using the governing path checks;
2. find the smallest filesystem directory containing the entry and every local imported document;
3. use normalized paths relative to that directory as logical document names;
4. record which logical name is the entry; and
5. omit the absolute containing directory from the source-unit manifest.

For example, `/work/pieces/main.musa` importing `../library/forms.musa` yields logical names `pieces/main.musa` and
`library/forms.musa`. Moving the complete tree to `/tmp/copy` preserves both names. Two documents with one logical name,
case-fold collisions, and forbidden symlink escapes remain errors.

The exact source bytes still include the written import spelling. Renaming an alias or rewriting a relative path may
therefore change snapshot identity even when resolution reaches the same document. Only the fixed-owner alias rule from
T₂e is claimed.

## 4. Snapshot dependency graphs are stored with sharing

Snapshot owners may depend on other snapshot owners. Their logical definition is the finite acyclic graph already formed
by import resolution; it is not a recursively copied tree of bytes.

The canonical graph encoding writes each distinct owner node once in dependency-first order. A node records its local
source-unit fields and refers to dependency nodes by earlier integer indices. Ready nodes are ordered by their complete
local fields and already assigned dependency-index lists; exact duplicate nodes coalesce. The root index selects the
source unit whose owner is being formed.

This encoding is finite and at most linear in the owner graph plus its source bytes. Diamond dependencies are shared
rather than copied once per path. Equality remains exact because the complete node table and root index are compared;
the implementation may hash the encoding only as a lookup aid.

## 5. Repaired project theorem

**Theorem 5.1.** Under the contracts and accepted rules imported from T₂e, every project accepted by T₂f has:

1. decidable deterministic type checking, complete up to the stated annotations;
2. one active checked implementation for every package owner in the build;
3. a finite positive-rank nominal family and finite non-recursive core program;
4. representation hiding, type preservation, progress, deterministic evaluation, and strong normalization;
5. exact versioned canonical nominal values for every `StorableValue` type;
6. support for current `Music`-bearing public signatures;
7. no snapshot nominal hidden inside a stable public interface; and
8. relocation-stable logical names for unchanged loose-entry trees, including sibling `../` imports.

**Proof.** T₂e supplies items 1, 3, 4, and the nominal encoding part of item 5. Lemma 1.1 gives item 2 and makes the
selected definitions functional. The `StorableValue` rules and T₂e’s exact encoders give the rest of item 5. The
explicit compiler-owned `Music` case gives item 6. Induction over `StableInterface`, using positive nominal rank, gives
item 7. The common-directory construction gives item 8 because relocation changes only the omitted absolute prefix. ∎

No new source term or type former was added. The remaining musical test is still practical: implement complete theory
operations in at least two packages and judge whether the public definitions are short, clear, and faithful to the
practices they claim to model.
