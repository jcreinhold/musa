# T₂h — relocatable checking and honest cache costs

**Status: candidate closure for independent review; governs nothing.** Review [58](58-proof-review-t2g.md) accepted
T₂g's source-package model, trusted roots, build-local private types, and dependency-complete execution key. It found
two faults. A checked package could not move from one build slot to another, and a cache hit could skip the language's
resource charges.

This note repairs those faults. It has one purpose: make a warm build accept and reject the same programs as a cold
build without giving private compiled values a permanent identity.

Sections 1–7 of T₂g remain in force except where this note replaces them.

## 1. Public interfaces do not contain build-local slots

A resolved build still gives each package instance one private `BuildNodeId`. The id distinguishes two installed
versions and lets a diamond dependency share one instance. It exists only for that build and is never written into a
cache file.

A stored public interface uses relative type names:

```text
RelativeTypeName =
    Self(logical module, declaration path)
  | Import(parameter number, logical module, declaration path)
  | Compiler(type name, type version)
```

`Self` means a type declared by the package being checked. `Import(i,...)` means a type reached through direct import
parameter `i`. `Compiler` means a versioned type owned by Musa, such as `Music`.

The import parameters form an ordered table:

```text
ImportParameter = {
    source alias,
    requested logical module,
    public interface key,
}
```

The source alias and requested module are part of the record. Sorting a bare set of interface keys would be wrong: if
aliases `a` and `b` were swapped, the same source name could refer to a different type.

A checked artifact contains:

```text
CheckedArtifact = {
    relocatable checked program,
    relocatable public interface,
    canonical diagnostics,
    logical resource trace,
}
```

The checked program uses the same `Self` and `Import` names. Canonical diagnostics use logical document names and byte
ranges. The project or editor adds an absolute display path only after loading the artifact. Moving a source tree can
therefore preserve the checked artifact without making `/work/piece.musa` and `/tmp/piece.musa` the same message on
screen.

## 2. Loading a checked artifact

Loading occurs only after the package resolver has produced one accepted build graph.

For each build node, the loader creates one fresh internal `TypeId` for every local nominal declaration. It then
replaces relative names as follows:

```text
Self(path)       -> TypeId for path in this build node
Import(i,path)   -> TypeId for path in the build node selected by parameter i
Compiler(name,v) -> compiler-owned TypeId for (name,v)
```

The loader rejects the artifact if an import alias, requested module, interface key, declaration path, or compiler type
does not match the active build. It never guesses from a similar field list.

**Lemma 2.1 (loading is unambiguous).** In an accepted build, a valid checked artifact has exactly one replacement for
every relative type name.

**Proof.** Build formation gives each direct alias one target node and logical module. The accepted interface for that
node gives each exported declaration path one meaning. Local namespace checks do the same for `Self`, and the compiler
table does the same for a versioned compiler type. The three name forms are disjoint. Therefore every admitted name has
one replacement. ∎

Two loads may choose different integer `TypeId`s. That difference has no source-level meaning.

## 3. The checking key

Replace T₂g's `InterfaceKey` and `CheckedKey` with:

```text
PublicInterfaceKey(package) = encode {
    interface format version,
    complete relocatable public interface,
}

CheckedKey(package) = encode {
    checked-artifact format version,
    checker, language, and resource-cost-model versions,
    exact package source tree and manifest,
    checking options,
    ordered ImportParameter records,
}
```

Neither key contains a `BuildNodeId`. The package's own types use `Self`. Imported types use parameter numbers whose
alias, requested module, and public interface are fixed by `CheckedKey`.

A private body edit in dependency `D` may leave `PublicInterfaceKey(D)` unchanged. A dependent package `P` may then
reuse its checked artifact. If `D` is replaced by a different build node with the same exact public interface, loading
replaces `P`'s `Import` names with that node's fresh type ids. This is safe because checking used only the public
interface.

The execution key remains dependency-complete:

```text
ExecutionKey(package) = encode {
    execution format and compiler versions,
    CheckedKey(package),
    ordered {
        source alias,
        requested logical module,
        dependency ExecutionKey,
    },
    execution options,
}
```

Changing any reachable package body changes its own `CheckedKey`, then every parent `ExecutionKey`. It need not change a
parent's `CheckedKey` when the imported interface stays equal.

## 4. Renaming internal type numbers changes no behavior

The source language cannot inspect a `TypeId`, convert one to `Nat` or `Text`, or branch on its numeric value. It can
only require two nominal types to be the same and use constructors permitted by the checked program.

Let `ρ` be a one-to-one renaming of internal type ids that preserves every nominal declaration and constructor table.
Apply `ρ` to all type annotations, constructor owners, and nominal values in a checked program.

**Lemma 4.1 (checking ignores the chosen numbers).** A program is well typed before the renaming exactly when its
renamed program is well typed after it, with the correspondingly renamed result type.

**Proof.** Induct over the typing derivation. Ordinary term rules do not mention `TypeId`. A nominal constructor or
match rule looks up one owner id and its constructor table. The one-to-one renaming preserves that lookup and every
field type. Rebuild the same rule with renamed annotations. Applying the inverse renaming proves the other direction. ∎

**Lemma 4.2 (evaluation ignores the chosen numbers).** If program `p` evaluates to `v`, then `ρ(p)` evaluates to `ρ(v)`.

**Proof.** Induct over the deterministic evaluation derivation. Variables, functions, products, lists, options, results,
and compiler primitives take the same step because none reads an internal type number. A nominal constructor keeps the
same constructor position under its renamed owner. A nominal match therefore selects the same arm, after which the
induction hypothesis applies. ∎

These lemmas justify loading one checked artifact with fresh ids in different builds. They do not make a nominal value
portable: such a value still contains the ids chosen for its build and has no admitted cross-build encoding.

## 5. A cache must replay logical resource use

Musa's resource limits are part of the language. Checking and evaluation use one project meter. A cache must therefore
preserve the logical charges that a cold operation would make.

One charge record is:

```text
Charge = {
    metric,
    amount,
    operation,
    logical document,
    byte range,
}
```

The metrics are the ones named by the governing core: reduction steps, value nodes, logical value bytes, compiler
instances, music occurrences, and any later versioned addition. A **resource trace** is the ordered finite list of
charges requested by one successful cacheable operation.

The resource-cost-model version fixes:

- every metric and limit;
- the order and amount of charges for each language operation;
- the canonical order in which packages and declarations are processed; and
- the rule for locating a charge in logical source.

A cache entry is inserted only after the operation succeeds and records its complete trace. Until resource rejection,
the next charge depends only on the operation's semantic inputs, not on the current remaining budget. A failed operation
is not cached.

On a miss, the operation charges the project meter normally. On a hit, Musa replays the stored charges in order before
it releases the stored result. If a charge crosses a limit, replay stops, reports that same operation, metric, attempted
amount, logical location, and limit, and discards the result. The project layer then attaches the current display path.

An implementation may compress a trace only if expanding it yields the same ordered charge records. A total alone is not
always enough: Musa promises to report the first operation that crosses a limit.

**Lemma 5.1 (resource replay).** Start a cold operation and a cache hit with equal project-meter states and equal
semantic inputs. Suppose the hit contains the complete trace and result of a previous successful run under the same
cost-model version. Then both runs either:

1. stop at the same first charge with the same canonical resource diagnostic; or
2. succeed with equal final meter states and equal results.

**Proof.** Induct over the stored charge list. Before the first charge, the meter states are equal. At each step, equal
metric, amount, and limit either reject both requests with the same attempted total or advance both meters to the same
state. If rejection occurs, the record supplies the same operation and logical location. If every charge succeeds, the
states are equal after the list. Determinism on equal semantic inputs gives the stored result. ∎

The premise that the stored trace came from a successful equal-input run is part of cache admission. A changed cost
model changes the key and cannot reuse the trace.

## 6. Exact codecs for stored results

Each result type that crosses a build boundary supplies a private versioned codec:

```text
Codec A = {
    type and codec version,
    encode : A -> bytes,
    decode : bytes -> A or format error,
    equality : A × A -> Bool,
}
```

For every admitted value `v`, decoding `encode(v)` returns a value equal to `v`. Decoding rejects malformed records,
unknown versions, and trailing bytes. Two encodings are byte-equal exactly when their admitted values are equal.

Compiler-owned finite values may receive a codec after review. Functions, closures, and values containing build-local
user nominal types do not. An in-memory cache scoped to one build may hold them, but a persistent cache may not.

## 7. Repaired cache results

**Theorem 7.1 (checking-cache safety).** Suppose two package checks have equal `CheckedKey`s and begin at equal
project-meter states. A valid cache hit and a cold check have the same acceptance result, canonical diagnostics,
resource charges, relocatable checked program, and relocatable public interface.

**Proof.** Equal keys give equal checker and language versions, exact source, manifest, checking options, cost model,
and alias-to-interface parameter table. The checker is deterministic on those inputs. It therefore produces the same
relative names, program, public interface, diagnostics, and charge trace. Lemma 5.1 gives equal resource acceptance and
meter state. Lemma 2.1 makes later loading unambiguous. ∎

**Theorem 7.2 (execution-cache safety).** Suppose two executions have equal `ExecutionKey`s, begin at equal project-
meter states, and the result type has an admitted codec. A valid persistent cache hit and cold execution have the same
acceptance result and equal decoded result.

**Proof.** Equal root keys give equal relocatable checked code, execution options, dependency edges, and dependency
execution keys. Following the finite dependency graph gives the same facts for every reachable package. Lemmas 2.1, 4.1,
and 4.2 show that choosing fresh internal type numbers changes neither typing nor evaluation. Determinism gives equal
results. Lemma 5.1 preserves resource acceptance. The codec returns a value equal to the one originally stored. ∎

**Corollary 7.3.** A dependency body edit may preserve a dependent package's checking cache. It cannot preserve an
execution cache that can observe the changed body.

## 8. Exact scope of the source-language result

This design keeps these parts of T₂b–T₂f:

- finite `Text` and structural `Result`;
- finite, non-recursive nominal declarations with positive ranks;
- private constructors and exhaustive internal matching;
- unique declaration namespaces;
- separate public and retained private checking environments;
- bidirectional checking with the stated annotations;
- finite acyclic value and nominal-dependency graphs;
- sealing, preservation, progress, deterministic evaluation, and strong normalization; and
- public signatures containing compiler-owned `Music` and functions.

It replaces persistent `StampKey` equality with build-local `TypeId` equality and Lemmas 4.1–4.2. It deletes the old
claims about `OwnerKey`, `ActiveOwnerTable`, `NominalRegistry`, cross-build private nominal encodings,
`StableInterface`, and `StorableValue`. Section 6's codec rule now decides which compiler-owned results may cross a
build boundary.

The W and K examples remain mechanism tests. They show that two packages can expose different hidden types. They do not
prove that either package models a musical practice well. That test still requires real package code and review by
people who know the practices represented.

## 9. First implementation boundary

The first package release supports:

- bundled source packages;
- editable local source packages; and
- exact Git source packages recorded in `musa.lock`.

It does not include a package registry, version ranges, a solver, native package code, or an implicit network fetch.
Normal checking and rendering remain offline. This matches the current governing package decision.

Persistent checked and execution caches should be implemented only after measurements show that cold compilation needs
them. The definitions above state how to do so safely; they do not claim that a cache is needed before that evidence.

The source-language extension itself needs no persistent cache. A first implementation may compile the exact locked
source graph together and keep only in-memory build-local values. This is enough to test the two real theory packages
before committing to an artifact format.
