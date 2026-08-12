# T₂g — source packages and safe caches

**Status: candidate repair for independent review; governs nothing.** The earlier drafts tried to give user-defined
types a lasting compiled identity. Musa does not yet need that promise. This note keeps the useful goals—source
packages, imports, separate checking, and fast caches—without claiming that a private compiled value will survive an
unrelated rebuild.

This note has one purpose: state the smallest package and cache rules needed by the proposed source language.

It keeps the term language and safety proof from T₂b–T₂f. In particular, the source language still has finite nominal
data, hidden constructors, non-recursive definitions, structures, signatures, and total functions. This note changes
only package resolution, type identity, and cache keys.

## 1. What Musa promises

Musa promises that musicians can:

1. keep projects and libraries in Git;
2. install and import source packages;
3. lock every external package to exact source;
4. reproduce a build from its source and lockfile; and
5. reuse cached work when all inputs relevant to that work are unchanged.

Musa does not yet promise that a saved private value or compiled binary from one build can be loaded by another build.
That later feature would need its own file format, compatibility policy, and migration rules.

## 2. A resolved build

Before type checking, the package resolver produces a finite directed acyclic graph:

```text
ResolvedBuild = {
    root package,
    package instances,
    dependency edges,
}

PackageInstance = {
    slot,
    package name,
    exact source tree,
    logical module map,
    dependency aliases,
}
```

A `slot` names one package instance in this build. Two versions of one library may occupy different slots. Every import
resolves through a declared dependency alias to one slot and one logical module.

Each package instance has a trusted source root before imports are read:

- for a package, the root is the directory containing its manifest;
- for a project, the root is the directory containing its project file;
- for an editor buffer or loose file, the caller supplies a finite virtual source map and a virtual root.

An import may not escape that root. The resolver checks normalized paths, duplicate logical names, case collisions, and
forbidden links before it forms the module map. Absolute machine paths never enter semantic identity.

External packages are exact:

- a released package is fixed by its registry, name, version, and verified source tree;
- a Git package is fixed by its repository and verified tree;
- a bundled package is fixed by the Musa release; and
- an editable local package uses its current source tree.

A digest may find a source tree, but exact bytes settle equality. The lockfile records enough information to fetch and
verify every external tree.

**Build formation rule.** Musa accepts a resolved build only when every slot has one exact package instance, every
import has one target, all source paths stay inside their trusted roots, and the dependency graph is finite and acyclic.

This rule replaces T₂d–T₂f's `OwnerKey`, `ActiveOwnerTable`, and snapshot-owner graph. Those objects solved a stronger
problem that this design does not claim to solve.

## 3. Type names live for one build

The checker assigns a fresh `TypeId` to each user-defined nominal type in an accepted build. A type declaration also has
a symbolic name:

```text
TypeName = (package slot, logical module, declaration path)
```

Within one build, `TypeName` maps to exactly one `TypeId`. Different declarations get different ids, even when their
fields happen to match. Hidden constructors remain hidden after a structure is sealed.

Across builds, Musa compares public interfaces by symbolic names and complete type descriptions. It does not compare the
fresh ids. A cached checker result may therefore be restored only after its symbolic names have been resolved to the
current build's fresh ids.

This gives packages useful abstract types without claiming a permanent binary identity for their runtime values.

## 4. One cache key cannot answer every question

Musa uses different keys for different work.

### Parsing

```text
ParseKey(file) = encode {
    parser version,
    language version,
    exact source bytes,
}
```

Equal parse keys produce the same concrete syntax tree and parse diagnostics.

### Public interfaces

After checking a package, Musa writes a complete public interface. It contains exported names and types, including the
full descriptions of public nominal types. It omits private function bodies.

```text
InterfaceKey(package) = encode {
    interface format version,
    package slot,
    complete public interface,
}
```

The encoding uses symbolic type names. Exact encoded data, not a bare hash, decides equality.

### Type checking

```text
CheckedKey(package) = encode {
    checker and language versions,
    exact package source and manifest,
    checking options,
    ordered imported InterfaceKeys,
}
```

If a dependency changes a private function body but keeps the same public interface, a dependent package may reuse its
checked result. When Musa loads that result, it resolves the stored symbolic type names to the active build's fresh
`TypeId`s. A missing name or unequal interface rejects the cache entry.

### Linking and evaluation

Code can observe dependency bodies, so execution needs a stronger key. Form it in dependency-first order:

```text
ExecutionKey(package) = encode {
    execution format and compiler versions,
    CheckedKey(package),
    ordered {
        dependency alias,
        dependency ExecutionKey,
    },
    execution options,
}
```

This is a key for the whole reachable code graph. Implementations may store that graph with shared nodes. Exact rooted
graph data decides equality; digests only speed lookup.

Suppose package `D` changes `tone : Nat` from `0` to `1`, while package `P` still defines `answer = D.tone`.
`InterfaceKey(D)` may stay equal, so `P` need not be checked again. `ExecutionKey(D)` changes, which changes
`ExecutionKey(P)`. Musa must therefore recompute `P.answer`. This is the case T₂f got wrong.

### Values stored in caches

A valid key does not by itself make every result safe to store. A stage may store a value across builds only when that
stage defines an exact, versioned encoding for the value's type.

Compiler-owned values such as exact timeline data may qualify under their own reviewed encoding. A value containing a
build-local `TypeId`, a function closure, or a user-defined private value does not qualify. Musa may keep such a value
in memory for the current build, but it must not restore it into another build.

Rendering and audio preparation keep their separate complete keys. In particular, a prepared execution key must still
include the complete musical input, bindings, seed, options, and versions required by R1.

## 5. Cache claims

The next results say precisely what these keys buy us. They assume deterministic parsing, checking, and evaluation, as
proved for the finite total source core in T₂b–T₂f.

**Theorem 5.1 (checking-cache safety).** If two package checks have equal `CheckedKey`s, then they produce the same
public interface, diagnostics, and checked program, apart from a consistent renaming of fresh `TypeId`s.

**Proof.** Equal keys give equal checker versions, language versions, package source, manifest, options, and imported
public interfaces. The checker is deterministic on those inputs. It may choose different fresh ids in the two runs, but
each id is tied to the same symbolic declaration name. Replacing each fresh id by that name makes the checked results
equal. Resolving the names in either accepted build gives a one-to-one renaming between the ids. ∎

**Theorem 5.2 (execution-cache safety).** If two package executions have equal `ExecutionKey`s and the cached result's
type has an admitted exact encoding, restoring one execution result for the other is sound.

**Proof.** Equal root keys give equal checked code, execution options, and dependency edges. Following the finite keys
down the dependency graph gives equal checked code and options for every reachable package. The deterministic evaluator
therefore produces equal results. The admitted encoding preserves and reflects the value equality chosen by that stage,
so decoding the stored result preserves that equality. ∎

**Corollary 5.3.** A body-only edit in a dependency may preserve a dependent package's checking cache but cannot
preserve its execution cache.

## 6. What remains true of the source language

Under the accepted premises from T₂b–T₂f, every accepted build still has:

1. decidable type checking, complete up to the stated type annotations;
2. finite non-recursive nominal types with hidden constructors;
3. type preservation, progress, deterministic evaluation, and strong normalization;
4. ordinary public signatures containing `Music` and functions such as `Music -> Music`; and
5. enough abstraction to define competing theory packages without putting Western harmony in the kernel.

The package and cache rules add no term, type former, or reduction rule. They therefore do not alter the source-core
safety proof.

## 7. What Musa should build now

The first package implementation should provide:

1. a manifest and lockfile with exact source dependencies;
2. trusted package roots and logical module paths;
3. one immutable resolved-build graph;
4. public interface files with exact, versioned encodings;
5. separate parsing, checking, and execution cache keys; and
6. cache admission rules that reject values without an exact stable encoding.

It should not yet provide:

- a stable binary interface for user-defined types;
- serialized private package values;
- dynamic linking of independently compiled Musa binaries; or
- migration between old and new private nominal schemas.

Those features may be useful later. None is required for Git collaboration, installable source packages, reproducible
builds, or sound incremental checking.
