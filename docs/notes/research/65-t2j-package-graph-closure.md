# T₂j — close the package-graph argument

**Status: research. Governs nothing.** Review [64](64-proof-review-t2i.md) found two false claims in
[63](63-t2i-package-graphs.md). The graph was right, but the map between graphs preserved the wrong root, and two cache
theorems forgot that Musa has one project-wide resource meter.

This note has one purpose: repair those claims without changing the package-graph design.

## 1. What stays and what changes

The resolved input is still one finite package graph. The graph retains package sharing, exact package versions, and the
owner of every nominal type. Its node addresses remain local names inside one graph. They are not permanent package ids
and do not appear in private runtime values.

Five points change:

1. An imported public graph maps its root to the selected dependency node, not to the project root.
2. A public interface contains the full environment used to check clients, not just value names and types.
3. Graph addresses are assigned before address-bearing interface labels are encoded.
4. Cache theorems compare runs that start with the same resource and diagnostic state.
5. Cache lookup is compiler housekeeping. It neither spends the source-language budget nor publishes source diagnostics.

These corrections leave the implementation order from [63 §10](63-t2i-package-graphs.md) unchanged.

## 2. A public interface is the client's whole view

For one package node, `PublicEnvironment` is the exact finite environment through which a client resolves names and
checks types. It contains:

- the public module tree and declaration order;
- every public declaration's path, kind, visibility, and type;
- every public abstract nominal type, including its kind and type parameters;
- the public members of signatures and structures, including sealing facts;
- the constructors and fields a client may use, if the language permits public constructors or fields; and
- every logical source anchor that can affect a client-facing diagnostic.

Private definitions and private constructors are absent. Function bodies and constant values are absent unless the
language later gives them a checked, public compile-time meaning. Documentation may be stored beside the environment,
but it enters the checking key only if the checker uses it to choose a diagnostic.

An `InterfaceBody` is the versioned, length-framed encoding of one `PublicEnvironment`. Nominal types in that encoding
name nodes in the public graph that contains the body.

This is a real software boundary, not just a file format. Once dependency interfaces have been built, the client checker
may read dependency facts only through `PublicEnvironment`. If client checking can inspect some other field, that field
belongs in `PublicEnvironment` or the checking theorem does not apply.

## 3. Move an imported graph to the selected node

Let `G_A` be the public graph stored for package `A`. Its root is `A`. Let `G_P` be the dependency-interface part of the
checking graph for package `P`. Suppose an import edge in `G_P` selects node `a`, which is the instance of `A` used by
`P`.

The loader supplies a node map

```text
j : nodes(G_A) -> nodes(G_P)
```

with these rules:

1. `j(root(G_A)) = a`. It does **not** equal `root(G_P)` unless `A` and `P` are the same package node.
2. `j` is one-to-one.
3. For every edge `v -label-> w` in `G_A`, `G_P` has exactly the edge `j(v) -label-> j(w)`. The set of outgoing public
   dependency edges is the same on both sides.
4. If two paths in `G_A` meet at one node, their images meet at one node. If they end at distinct nodes, their images
   remain distinct.

Call this the **selected-node embedding**. In plain terms, it places `A`'s whole public dependency graph at the `A` node
that `P` actually imported.

The map also moves type names. If node `v` owns a nominal type, define

```text
transport_j(address_G_A(v), module, declaration)
  = (address_G_P(j(v)), module, declaration).
```

Apply `transport_j` recursively inside products, lists, options, results, arrows, signature members, and every other
type-bearing part of a public environment.

The loader accepts the embedding only when, for every node `v`,

```text
transport_j(InterfaceBody_G_A(v)) = InterfaceBody_G_P(j(v)).
```

It compares the transported bodies. It does not compare their raw stored bytes, because the two graphs use different
local addresses.

**Lemma 3.1 (moving an interface preserves nominal equality).** Two nominal type names are equal in `G_A` exactly when
their transported names are equal in `G_P`.

**Proof.** Equal names have the same owner, module, and declaration path, so transport gives equal fields. For the
reverse direction, equal transported names have equal modules and declaration paths. Their target owners are also equal.
Since `j` is one-to-one, their source owners were equal. Thus the original names were equal. ∎

For example, if `D` owns `T`, `A` exports `x : D.T`, and `P` imports `A`, the map sends `A`'s root to the selected `A`
node and `D`'s node to the selected `D` node. The transported type of `x` still belongs to `D`. It does not become `A.T`
or `P.T`.

## 4. Build the canonical table in two passes

Interface bodies contain node addresses, while addresses depend on graph edges. The construction therefore has two
passes.

First, use only the root and the labelled edges. Assign each node the least edge-label path from the root, as in
[63 §3](63-t2i-package-graphs.md). This pass does not inspect an interface body.

Second, replace every internal node reference in each body with the address from the first pass. Then encode node rows
and sort them by address.

Equality of two address-bearing graphs means that a root-and-edge map exists and that labels are equal after this
transport. It never means that two labels written under different local address scopes have equal raw bytes.

With that order, [63 Lemma 3.1](63-t2i-package-graphs.md) still holds. The canonical table preserves the exact finite
graph, including shared nodes.

### Edge labels

An edge label is a framed pair:

```text
EdgeLabel = {
    dependency alias,
    requested logical module path,
}
```

The alias is one accepted Musa identifier. A module path is a nonempty list of accepted Musa identifiers. Current Musa
identifiers use the ASCII grammar `[a-zA-Z_][a-zA-Z_0-9]*`, so case is exact and Unicode normalization does not arise in
these fields. A later identifier grammar must change the graph-format version and state its normalization rule.

The target package is recorded by the target row, not repeated in the edge label. Two outgoing edges at one node with
the same complete label are rejected.

## 5. Decide which package instances are shared

Before graph construction, the resolver assigns each selected source tree one exact instance descriptor. Equal
descriptors must select one node. Unequal descriptors select different nodes unless a governing conflict rule rejects
the build.

The initial descriptors are:

```text
Bundled = {
    package id,
    Musa release,
    embedded source-tree digest,
}

LockedGit = {
    package id,
    canonical immutable repository URL,
    revision algorithm and full object id,
    verified source-tree digest,
}

Local = {
    package id,
    normalized logical root relative to the trusted project root,
}
```

The graph records the locked outgoing dependency edges. Together, the node descriptor and those edges are the package
node described by `musa.lock`.

The resolver version fixes URL parsing and serialization. Repository mirrors are not equal merely because they contain
the same tree. A Git revision uses the algorithm and full object id recorded by the lockfile, and the fetched tree must
match its recorded digest.

A local logical root removes `.` components, resolves `..` without allowing escape, and uses the path separators fixed
by the resolver format. The resolver rejects case or Unicode-normalization collisions and rejects a second logical root
that reaches the same directory through a symlink alias. Moving the whole trusted project tree leaves the logical root
unchanged.

For every accepted descriptor, the active build must contain exactly one manifest and one source tree. Historical cache
entries do not take part in this choice.

## 6. Separate deterministic work from the current budget

For a fixed checking input, the checker is deterministic. As it works, it requests a finite ordered list of events:

```text
ReplayEvent =
    Charge(metric, amount, operation, logical document, byte range)
  | Emit(canonical diagnostic)
```

The event requests depend on the checking input and the cost-model version. They do not depend on how much of the
project budget earlier packages have spent. The current meter decides whether the next `Charge` succeeds. If it fails,
the operation stops and releases no artifact.

A successful run stores its complete event list. A later hit may start with less budget and fail while replaying that
same list. That is expected. The equivalent cold run, started from the same state, reaches the same charge and fails
there too.

**Theorem 6.1 (equal successful checks).** If two cold checks have equal complete `CheckedKey` bytes and both checks
succeed, they produce equal checked artifacts and equal exclusive event lists, after replacing internal `TypeId` numbers
with graph-relative names.

**Proof.** Equal keys give equal checking graphs, source, manifests, options, public environments, package sharing, and
checker, language, graph, artifact, and cost-model versions. The deterministic checker therefore makes the same rule
choices and requests the same events. Both runs finish, so both release their result. Fresh internal `TypeId` numbers
may differ, but the T₂h renaming lemmas replace them consistently with equal graph-relative names. ∎

The success premise matters. Equal keys can succeed from a fresh project meter and fail after earlier packages have
spent most of the budget.

**Lemma 6.2 (fresh work and replay have the same effect).** Compare fresh work and a valid hit for one key. If they
start with equal meter and diagnostic states, they either stop at the same event or finish with equal states. If they
finish, they release equal results.

**Proof.** Fresh work requests the event list stored by the valid successful entry. Induct over that list. `Emit`
appends the same diagnostic. `Charge` sees equal current totals and limits, so it either advances both meters equally or
rejects both runs at that charge. No result is released before the whole list succeeds. If it succeeds, Theorem 6.1 or
deterministic execution gives equal results. ∎

**Theorem 6.3 (warm and cold builds agree).** Start warm and cold checking of the same resolved graph with equal project
meter and diagnostic states. Any mixture of valid successful checked-artifact hits yields the same acceptance result,
diagnostics, public interfaces, and final meter state as the cold build.

**Proof.** Process nodes once in the fixed dependency-first order. Before each node, the states are equal. A miss runs
the same deterministic checker. A hit has the same effect by Lemma 6.2. Thus both builds either reject at the same event
or advance to equal states and equal artifacts. Shared nodes occur once in the order, so neither run charges them twice.
∎

## 7. Publish each diagnostic once

A checked artifact may retain its canonical diagnostic list for later queries. That list is read-only result data.

Only `Emit` events append to the project's diagnostic stream. A cold run publishes each diagnostic as its event occurs.
A hit publishes it while replaying the same event. Releasing the artifact does not publish its stored list again.

This rule prevents a cache hit from showing each warning twice.

## 8. Put cache work outside the source meter

The cached operation starts after package resolution, graph validation, selected-node embedding, and key construction.
Those steps either produce the exact checking or execution input or report the same ordinary resolver error before any
cache lookup.

Digest lookup, exact-key comparison, entry decoding, codec checks, and deletion of malformed local entries are compiler
housekeeping. They do not spend the source-language meter and do not emit canonical source diagnostics. A malformed or
unknown local entry is a miss; an optional debug log is outside Musa program semantics.

Checking and evaluation begin after that boundary. Their `Charge` and `Emit` events contain every action that can change
language acceptance or the canonical diagnostic stream. A cache hit replays those events before it releases a result.

This boundary is deliberate. Disk speed, cache corruption, and the cost of hashing must not change which Musa programs
the language accepts.

## 9. Correct execution-cache theorem

An execution entry remains inclusive: it records the events and result for the whole rooted execution graph. A root hit
does not replay child execution entries.

**Theorem 9.1 (execution-cache safety).** Let a valid execution entry contain the complete `ExecutionKey`, an admitted
result codec, and the complete event list. Compare a hit and fresh execution with equal requested key bytes and equal
starting meter and diagnostic states. They either stop with the same resource failure and diagnostics or finish with
equal final states and equal results under the codec's stated result equality.

**Proof.** Equal exact key bytes reconstruct the same rooted execution graph, node sharing, checked inputs, operation
versions, and options. The checking theorem gives equal checked programs up to a consistent renaming of internal
`TypeId`s, and the T₂h evaluation-renaming lemma preserves evaluation. Determinism gives the same execution events and
result. Lemma 6.2 gives the same effect from equal starting states. The admitted codec preserves the chosen result
equality. ∎

The starting-state premise compares the current hit and miss. The earlier run that created the entry may have started
from another state; it only had to finish successfully and store the deterministic complete event list.

## 10. What this means for packages and caching

The design still supports ordinary source collaboration. A package is a checked Musa source tree with a manifest and a
module tree. An exact Git pin and lockfile make it installable and reproducible without a registry or version solver.
None of that depends on stable private runtime identities.

The graph also gives caching the input it needs. If source, selected package graph, options, and compiler rules stay the
same, exact checked keys stay the same across builds and may reuse artifacts. A private dependency edit can preserve a
client's checking key when the public graph stays the same, while the dependency-closed execution key changes. Shared
and split package instances never collide.

The cache is an optimization, not part of package meaning. Musa should first implement nominal types and exact package
graphs, write the two trial theory packages, and measure real builds. Persistent caches come after that evidence.

This note does not yet govern the language. It should replace [63](63-t2i-package-graphs.md) only if an independent
review finds no remaining counterexample.
