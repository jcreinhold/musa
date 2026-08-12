# T₂i — the cache key is the package graph

**Status: research. Governs nothing.** Reviews [58](58-proof-review-t2g.md) and [60](60-proof-review-t2h.md) found the
same fault in two forms. A package interface was summarized as a list of imports, but type checking also depends on how
those imports share package instances. The summary had forgotten part of the input and the cache theorem was false.

This note has one purpose: define the exact package graph on which checking and execution depend.

It keeps T₂h's source packages, build-local private types, resource replay, and rejection of persistent private values.
It replaces T₂h's `Self` and direct `Import` names with names bound by the whole resolved graph.

## 1. Why a graph is necessary

Five examples fix the definition.

| Example | What the design must retain |
| --- | --- |
| One package with no dependencies | Its own nominal types need no permanent global id. |
| Package `P` imports `A.T` | `A.T` must remain distinct from an equal-looking type in another package instance. |
| `A` exposes a value of type `D.T`, and `P` imports `A` | `P` must be able to name the type owned by the transitive dependency `D`. |
| Aliases `a` and `b` point to one shared instance in one build and two equal-looking instances in another | The two graphs must have different checking keys because `b.consume(a.x)` has different typing results. |
| A dependency changes a private function body but not its public interface | A dependent package may reuse checking, but execution must use the changed body. |

An ordered list of interface keys passes the first two examples and fails the next two. A permanent package-owner id
would distinguish them, but it would restore the binary-compatibility promise Musa has chosen not to make. The needed
datum is smaller: the exact finite graph selected for this build.

### Level audit

The **exact object** is the rooted package graph with labelled edges and preserved node sharing. The earlier import list
was a **summary** of that graph. It forgot whether two paths ended at one node or two and forgot the scope of transitive
type names. The checking theorem is false for the summary and substantive for the graph. This note therefore stops
patching the list and states the theorem on the graph.

## 2. The resolved package graph

For one project, the resolver produces a finite rooted directed acyclic graph `B`.

- A node is one selected source-package instance.
- The root is the project package being built.
- An edge records a dependency alias and a requested logical module.
- Every node is reachable from the root.
- Outgoing edge labels at one node are unique.
- Two edges may end at the same node. That equality is part of the graph.

Each node has a trusted source root and one exact source tree. Absolute machine paths are display data and do not enter
the graph.

The resolver selects instances by these rules:

1. One bundled package in one Musa release selects one node.
2. Repeated uses of one exact locked Git package select one node.
3. Repeated uses of one editable local package root select one node.
4. Different Git revisions or different local package roots select different nodes, even when their source or public
   interfaces happen to match.
5. Package aliases choose edges, not type identity. Two aliases may point to the same node.

Local roots are normalized logical paths inside the project source tree. A move of the complete project tree therefore
preserves the graph. Imports that escape the trusted root, ambiguous case or Unicode-normalization collisions, cycles,
and unresolved modules are rejected before checking.

These rules decide identity only inside one resolved build. They do not say that a private value written by an old build
belongs to a new one.

## 3. Canonical local names for graph nodes

A cache file needs deterministic names for graph nodes, but those names need not be global.

Fix a byte order for framed edge labels. For a node `v`, consider every edge-label path from the root to `v`. The graph
is finite and acyclic, so this set is finite and nonempty. Define the **address** of `v` to be its least path. The
root's address is the empty path.

Two different nodes cannot have one address: following one edge-label path from the root has at most one result. A
shared node has one address even when several other paths also reach it. Two distinct equal-looking nodes have different
addresses.

The canonical graph table contains one row per node, sorted by address. Each row records:

- the node address;
- the node label required by the current operation; and
- every outgoing edge label and the target node's address.

Rows and edges use exact length-framed encodings. A digest may locate a table, but exact table bytes decide equality.

**Lemma 3.1 (the table preserves the graph).** Two canonical tables are equal exactly when their rooted, edge-labelled,
node-labelled graphs are isomorphic.

**Proof.** An equal table reconstructs the same set of node addresses, the same root at the empty address, the same
labels, and the same target address for every edge. Thus it reconstructs one graph. Conversely, a rooted isomorphism
preserves every edge-label path and hence every least path. It therefore preserves addresses, rows, labels, and edges,
so both graphs write the same table. ∎

The address is a bound name inside one table. It is not a package id and must not appear in a runtime private value.

## 4. Types name nodes in a public graph

An exported nominal type is named by:

```text
NominalType = {
    owning node address,
    logical module,
    declaration path,
}
```

A compiler-owned type instead uses its name and language version. Structural types such as products, `Option`, `List`,
`Result`, and arrows contain these names recursively.

An **interface body** is one package node's exported names and complete types. Its type names may refer to any node
reachable from that package node, including itself. This handles a package that exposes a value whose type is owned by a
dependency of a dependency.

A stored public interface is not one interface body in isolation. It is a rooted **public graph**:

- every reachable package node occurs once;
- each node is labelled by its interface body; and
- every nominal type in every body names an address in that same graph.

Suppose `D` owns `T`, `A` exports `x : D.T`, and `P` imports `A`. In `A`'s public graph, `x` refers to `D`'s node
address. When `A` is used while checking `P`, the resolver supplies the inclusion from `A`'s resolved subgraph into
`P`'s graph. Applying that inclusion changes the local addresses in `A`'s stored interface to the addresses in `P`'s
graph. It does not turn `D.T` into a type owned by `A`.

The loader accepts this inclusion only when it preserves the root, node labels, dependency edges, and node sharing. It
therefore cannot map one node to two nodes or identify two distinct nodes.

**Lemma 4.1 (interface inclusion preserves nominal equality).** Let `j` be an accepted inclusion of one public graph
into another. Two nominal type names are equal before applying `j` exactly when their images are equal afterward.

**Proof.** Nominal names compare their owning node, module, and declaration path. The inclusion preserves the latter two
fields and is one-to-one on nodes. It therefore preserves and reflects equality of the three fields. ∎

## 5. The checking input

To check package node `p`, Musa builds a rooted **checking graph** from the subgraph reachable from `p`.

- The root row contains `p`'s exact source tree, manifest, logical module map, and checking options.
- Every other row contains the dependency node's complete interface body.
- All rows contain the exact dependency edges and their sharing.
- Every type name has already been moved into this graph's addresses.

The key is:

```text
CheckedKey(p) = encode {
    checked-artifact format version,
    resolver and graph format versions,
    checker, language, and resource-cost-model versions,
    canonical checking graph for p,
}
```

Checking creates fresh internal `TypeId`s for the graph's nominal declarations. The stored artifact does not contain
those numbers. It contains graph addresses, logical modules, and declaration paths. Loading the artifact into an equal
checking graph assigns fresh ids and replaces every stored nominal name consistently.

The checked artifact contains:

- the checked program with graph-relative nominal names;
- the root interface body;
- canonical diagnostics in logical documents and byte ranges; and
- the replay events defined in §7.

Absolute display paths are added by the project or editor after loading.

**Theorem 5.1 (checking depends on the checking graph).** Equal `CheckedKey` values produce equal checked artifacts.

**Proof.** Equal keys give the same resolver, graph, checker, language, cost-model, source, manifest, options,
dependency interfaces, type names, and node-sharing relation. The checker is deterministic on those inputs. It may
choose different internal integers for `TypeId`, but replacing each integer with its graph-relative name makes the
results equal. The renaming lemmas from T₂h show that this replacement preserves typing and evaluation. ∎

This theorem is deliberately conservative. A future implementation may prove that some unused part of the public graph
can be removed from the key. It may not remove that part merely because no current test notices.

## 6. The execution input

Checking may ignore private dependency bodies. Execution may not. For root node `p`, form an **execution graph** with
the same nodes, edges, and sharing as the resolved graph. Label every node by its exact `CheckedKey` and the version of
the operation that evaluates its checked program.

```text
ExecutionKey(p) = encode {
    execution and result format versions,
    execution options,
    canonical execution graph for p,
}
```

A private body edit changes the edited node's `CheckedKey` because that key contains its exact source. Lemma 3.1 then
makes the root execution graph unequal. The dependent package's own `CheckedKey` may remain equal if every imported
interface and its sharing remain equal.

**Corollary 6.1.** A private dependency edit may preserve dependent checking. It cannot preserve an execution result
that can observe the edited body.

**Proof.** The edit may leave the dependency node's interface body unchanged, so the parent's checking graph may remain
equal. It changes the dependency's own source and hence its `CheckedKey`. That changed node label changes every rooted
execution graph that reaches it. ∎

An execution cache entry represents the whole rooted execution. On a root hit, Musa does not also replay child execution
entries. This inclusive rule prevents a shared diamond from being charged or initialized twice.

## 7. Cache hits replay work that affects acceptance

Musa's resource limits are language rules. A warm cache must accept and reject the same builds as a cold compiler.

For checking, the compiler processes package nodes once in dependency-first order. When several nodes are ready, it
orders them by their canonical graph addresses. Each node's checked entry records the events produced while checking
that node alone:

```text
ReplayEvent =
    Charge(metric, amount, operation, logical document, byte range)
  | Emit(canonical diagnostic)
```

A miss performs the work and records its exclusive event list. A hit replays that list. A charge that crosses a limit
stops replay and produces the same resource diagnostic as a cold check. Later events are not emitted and the cached
artifact is not released. Thus an ordinary warning before exhaustion appears in both runs, while a warning after
exhaustion appears in neither.

The cost-model version fixes the metrics, limits, charge points, diagnostic order, and node order. Failed checking
results are not cached. A successful checked entry may later fail during replay when earlier packages have consumed more
of the project budget; this is the same failure a cold check would have at that point.

Execution uses one inclusive event list for the whole rooted operation. A root hit replays that list and returns its
result only if replay finishes. It never separately replays a descendant execution entry.

**Lemma 7.1 (one event list replays exactly).** Start a miss and a hit at equal project-meter and diagnostic states, on
equal operation inputs and under one cost-model version. If the hit was inserted by a successful earlier run on those
inputs, both current runs either stop at the same event or finish with equal meter and diagnostic states.

**Proof.** Induct over the stored event list. An `Emit` appends the same canonical diagnostic. A `Charge` sees equal
meter states and equal limits, so both runs either reject at that charge with the same attempted total and location or
advance to equal states. If every event succeeds, both final states are equal. ∎

**Theorem 7.2 (warm and cold checking agree).** Checking the same resolved graph cold or with any set of valid checked
cache hits yields the same acceptance result, canonical diagnostics, public interfaces, and final resource state.

**Proof.** Induct over the fixed dependency-first node order. Before the first node, both compiler states are equal. At
each node, a miss is deterministic by Theorem 5.1 and a hit has the same effect by Lemma 7.1. Thus the states remain
equal or both runs reject at the same event. Every node is processed once, so shared dependencies are neither skipped
nor counted twice. ∎

## 8. Stored cache entries

A persistent cache entry is one framed record:

```text
CacheEntry = {
    entry format version,
    complete key bytes,
    replay-event format and events,
    result codec name and version,
    result bytes,
}
```

Lookup uses a digest only to find candidates. It then compares the complete key bytes, decodes every field, rejects
unknown versions and trailing data, and checks that the result bytes are the canonical encoding of the decoded result. A
malformed entry is a miss and may be deleted.

The correctness theorem assumes a **valid entry**: this exact record was inserted by the named compiler operation on the
recorded key. The local cache is disposable compiler output, not an untrusted package input. Deliberate tampering with
the compiler or its cache is outside the language semantics. Verified package source and assets remain untrusted inputs
and cross a separate validation boundary.

Only types with a reviewed versioned codec may be stored across builds. A checked artifact qualifies because it contains
graph-relative names rather than internal `TypeId`s. A runtime value containing a user nominal type, a closure, or a
function does not qualify. Such values may remain in an in-memory cache for one build.

**Theorem 8.1 (execution-cache safety).** Let a valid execution cache entry have a complete `ExecutionKey`, an admitted
result codec, and a replay list. A hit on equal requested key bytes either produces the same resource failure as fresh
execution or returns a result equal to fresh execution.

**Proof.** Lemma 3.1 gives the same rooted execution graph, including node sharing. Theorem 5.1 gives equal checked
programs up to a consistent renaming of internal type ids. T₂h's renaming lemma makes evaluation invariant under that
renaming. Determinism therefore gives equal results and the same event list. Lemma 7.1 preserves resource acceptance,
and the codec preserves the chosen result equality. ∎

## 9. What this changes—and what it does not

This note changes no source term, type former, or reduction rule. It keeps the T₂ source proposal's:

- finite non-recursive nominal data;
- private constructors and exhaustive internal matches;
- `Text` and structural `Result`;
- structures with abstract type members;
- finite acyclic declaration graphs;
- bidirectional checking with stated annotations; and
- preservation, progress, deterministic evaluation, and strong normalization.

It replaces every persistent nominal owner or direct-import parameter with one finite graph of locally bound package
nodes. Private runtime values still have identity only inside one build.

The W and K examples remain language-mechanism tests, not claims of cultural adequacy. The next musical gate is still
two real source packages: one familiar launch theory and one practice whose primary public objects are phrases,
gestures, tunings, or another non-chordal organization. People who know the represented practices must review the second
package.

## 10. Implementation recommendation

Implement the language feature before the persistent cache format:

1. Add finite nominal data, hidden constructors, and abstract type members using build-local ids.
2. Resolve bundled, editable local, and exact locked Git source packages into one immutable graph.
3. Check the whole graph together and test shared diamonds, split equal interfaces, two exact versions, and transitive
   exported types.
4. Write the two real theory packages and judge whether their public definitions are clear and faithful.
5. Measure cold and repeated compilation on those packages.
6. Add persistent interface and checked-artifact caches only if the measurement justifies them, using the graph and
   replay rules above.

This order preserves Git collaboration, installable source packages, and reproducible builds from the start. It does not
make cache machinery a prerequisite for learning whether the proposed language is musically useful.
