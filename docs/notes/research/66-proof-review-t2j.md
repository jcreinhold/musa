# Proof review: T₂j package-graph closure

**Status: independent review of the frozen T₂j research candidate; governs nothing.** I reviewed
[65](65-t2j-package-graph-closure.md) against every required closure item in [64](64-proof-review-t2i.md), the
package-graph definitions in [63](63-t2i-package-graphs.md), the T₂ source-calculus proof chain, the governing rules and
package plan, and current compiler/project behavior. I treated package dependency resolution and source-module imports
as different relations, because the governing package format does.

## Findings

### High

1. **Package descriptors still use a digest as proof of exact source-tree equality.**
   - **Location:** [65 §5](65-t2j-package-graph-closure.md), `Bundled` and `LockedGit` descriptors.
   - **Type:** false equality claim / governing-rule violation.
   - **Problem:** both descriptors retain only a source-tree digest, and the preceding rule says equal descriptors
     **must** select one graph node. The governing hash law in `docs/rules/obligations.md` §6 says the opposite:
     matching finite hashes may narrow a search but cannot prove equality or define type identity. Saying that the
     fetched tree must match its recorded digest checks consistency with the same finite summary; it does not compare
     two candidate trees after a collision.
   - **Exact counterexample:** let canonical source trees `X` and `Y` have different exact `(logical path, bytes)` maps
     but the same finite tree digest. Give them otherwise equal `LockedGit` descriptor fields. This is possible as a
     mathematical counterexample for every finite digest, regardless of cryptographic difficulty. Under T₂j the
     descriptors are equal and the resolver must coalesce them. Yet `X` can declare hidden type `T` and `Y` hidden type
     `U`, or their manifests can name different module trees. The selected node then has no unique manifest, source,
     public environment, or checked program. Line 177's requirement that an active descriptor have exactly one tree
     rejects the situation only if it compares exact trees, a comparison the descriptor rule never defines.
   - **Why it matters:** this reintroduces precisely the digest-only identity the package graph was meant to eliminate.
     It can collapse different nominal owners before canonical graph construction, so no later exact table-byte
     comparison repairs it.
   - **Suggested repair:** make the admitted instance record retain the complete canonical source-tree descriptor—a
     sorted, length-framed map from logical paths to exact bytes, including the manifest—and compare it after digest
     lookup. An interned tree handle is also sound if its registry keeps those exact bytes and rejects one handle/digest
     paired with unequal descriptors. Git repository and object ids remain useful provenance and retrieval fields; they
     do not replace the exact collision check.

2. **One edge relation is being used for two different graphs: locked package dependencies and source module imports.**
   - **Location:** [65 §4 “Edge labels” and §5 lines 165–166](65-t2j-package-graph-closure.md).
   - **Type:** wrong object / cross-document inconsistency.
   - **Problem:** T₂j defines every edge label as `(dependency alias, requested logical module path)`, then says these
     are the locked outgoing dependency edges. The governing format has one lock-graph edge for each declared package
     dependency. A source package can request zero, one, or several modules through that dependency. The requested
     module is therefore not a field of the package dependency edge.
   - **Exact multiple-module counterexample:** package `P` declares one dependency `d -> D` and imports `d::x` and
     `d::y`. The lock graph has one package edge `d -> D`. T₂j's graph requires two distinct edges `(d,x) -> D` and
     `(d,y) -> D`. Conversely, if `P` declares and locks `d` but presently imports no `D` module, the lock graph still
     has its declared dependency edge while T₂j cannot form an edge at all: its requested module must be a nonempty
     identifier list.

     This is not hypothetical pressure. Current Musa files routinely import several modules from the one bundled
     `std` package, such as `examples/rule-of-the-octave.musa` and `examples/tonal-construction.musa`.
   - **Local imports expose the other direction:** a quoted module imported within the current package is a module edge
     or source dependency, not a dependency on a second package instance. Representing it as a package self-edge would
     violate the package DAG's acyclicity even though the declared module graph can be acyclic.
   - **Why it matters:** canonical addresses, selected-node embeddings, sharing, descriptor coalescing, and lockfile
     reproducibility are claimed for the exact **package** graph. They cannot be checked against `musa.lock` while the
     graph's edges are actually module requests. The two relations have different multiplicity and different cycle
     conditions.
   - **Suggested repair:** define package-instance edges as

     ```text
     PackageDependencyEdge = { dependency alias, target package node }
     ```

     with one functional target per alias. Put requested module paths in the exact root source/module map and in
     `PublicEnvironment`, where client name resolution uses them. If a separate import graph is useful, define it as a
     second relation over source modules and prove how each import's package alias factors through the package edge.
     Canonical package-node addresses can use dependency aliases alone; aliases are unique at one manifest node.

### Medium

1. **Equal source descriptors need a functional locked-edge condition.** Even after separating package edges, the
   resolver must reject a lock artifact that presents one exact package instance with two different outgoing mappings
   for the same dependency alias. T₂j says equal descriptors coalesce before graph construction, while also saying a
   descriptor plus its locked edges forms the node. A hostile lock could otherwise repeat equal package/source fields
   once with `d -> D₁` and once with `d -> D₂`. Coalescing gives one node with two targets; keeping both gives two nodes
   despite equal descriptors. The governing exact manifest should make the mapping functional, but T₂j must import that
   check explicitly: verify the lock edge map against the exact manifest, and reject equal instance records with unequal
   resolved edge maps.

2. **`transport_j` is defined on structured environments but written as acting directly on encoded bodies.** Section 2
   defines `InterfaceBody` as bytes. Section 3 recursively transports nominal references inside it. The intended
   operation is sound: decode a versioned `PublicEnvironment`, transport its structured type names, and canonically
   re-encode it. State that operation and require the decoder to reject malformed, unknown-version, and trailing data.
   Literal mutation of an opaque byte string would not justify the commuting equation. This is a specification gap, not
   a counterexample to the selected-node embedding.

### Low

1. **Selected-node embedding rule 4 is redundant.** Injectivity plus edge preservation already sends one source node to
   one target and keeps distinct source nodes distinct. Keeping the path-sharing sentence is useful explanation, but it
   should be identified as a consequence rather than an independent check so an implementation does not invent a second,
   possibly divergent test.

2. **The exact tree comparison can remain private.** The repair to finding 1 does not require putting complete source
   bytes into every cache key occurrence or public API. A collision-checked intern table can share the exact tree once;
   canonical graph rows can refer to a locally framed table entry. The invariant is that equality dereferences to the
   complete descriptor, not that the representation must duplicate it.

## Verdict

- **Decision:** **Incorrect.** T₂j closes all five explicit proof gaps from review 64, and its graph, transport, and
  replay proofs are otherwise sound. But its new package-instance descriptors violate the governing no-hash-identity
  law, and its exact edge grammar conflates the lockfile's package graph with source module imports. Both errors occur
  before the reviewed canonical table is built, so its correctness cannot compensate for them.
- **Basis:** I checked every definition and inference in T₂j; repeated, transitive, shared, split, and two-version
  package graphs; direct and re-exported nominal types; interface transport; exact and colliding package descriptors;
  zero/one/multiple module requests; successful and resource-rejected cache runs; diagnostic event ordering; execution
  replay; codec/trust assumptions; and transport of the source safety results.
- **Limits:** user nominal types, package-instance graphs, exact locked Git packages, public-interface artifacts, replay
  events, and persistent source caches are not implemented. The T₂ source safety proof and T₂h renaming lemmas remain
  named mathematical dependencies rather than current executable evidence.

## Clean passes

### Every explicit review-64 closure item is repaired

1. The selected-node embedding sends the imported graph's root to the dependency node actually selected by the client,
   is injective, preserves exact outgoing structure, and compares interface labels only after address transport. The
   ordinary `P -> A` case now exists.
2. `PublicEnvironment` is stated as the complete finite source-checking view: module/name order, declaration kind and
   visibility, abstract nominals, signatures, structures, sealing, any public constructors/fields, and diagnostic
   anchors. Client checking is required to factor through it.
3. Canonicalization first computes least-path addresses solely from root and edges, then transports and encodes
   address-bearing labels. This removes the earlier circular presentation.
4. Theorem 6.1 requires both equal-key checks to succeed. Lemma 6.2, Theorem 6.3, and Theorem 9.1 compare current runs
   from equal meter and diagnostic states. The earlier run that created a valid cache entry may have begun elsewhere.
5. `Emit` events are the only mutation of the canonical diagnostic stream; an artifact's diagnostic list is read-only
   query data. Resolver, graph validation, lookup, key comparison, decoding, codec validation, and malformed-entry
   deletion are placed outside the source-language meter and diagnostic stream.

### The selected-node embedding and nominal lemma are correct

For a well-formed target dependency subgraph, the selected root fixes the image of the imported root. Unique outgoing
dependency aliases then determine the rest of the map recursively; diamonds map consistently exactly when the target
preserves sharing. Injectivity prevents two distinct imported nominal owners from collapsing. Recursing transport
through every structural type former carries transitive and re-exported types without changing their owner.

Lemma 3.1 is therefore correct. Equality of nominal names is equality of owner node, module path, and declaration path;
an injective owner map preserves and reflects that equality. A shared dependency stays shared, while two equal-looking
versions remain distinct.

### Two-pass canonicalization remains correct

Least edge-label paths are finite and nonempty for every reachable node. Unique outgoing labels make following one path
functional. Rooted isomorphism preserves path sets and their least elements, so the canonical address table preserves
the exact finite DAG and its sharing. Interface labels are encoded only after addresses exist, and equality across
different local scopes is correctly defined by transport.

After the edge-object repair above, using the unique dependency alias as the package edge label retains this proof. A
source import of another module does not need to rename the package node.

### The resource and diagnostic theorems are correct

For one fixed key and cost-model version, the event requests are independent of remaining budget. A successful run can
therefore store the complete deterministic list. Replaying it from the same **current** state as fresh work advances or
rejects at the same event. The stored run may have started from a different meter state; only its successful complete
trace matters.

Theorem 6.1 correctly claims artifact/event equality only for two successful equal-key checks. Lemma 6.2 follows by
induction over `Emit` and `Charge`. Theorem 6.3 follows by induction over the fixed dependency-first node order, with
each shared node scheduled once. Since result release happens only after the event list finishes, neither a hit nor a
miss publishes a partial artifact.

Theorem 9.1 also has the needed premises. An inclusive root execution event list is replayed once, not once per child.
Equal exact execution graphs give equal checked programs up to internal-id renaming; evaluation is equivariant under
that renaming; deterministic execution yields the stored event list/result; and an admitted codec preserves the stage's
chosen result equality.

### Cache housekeeping is separated honestly

Package resolution, graph validation, selected-node embedding, and key construction run before lookup in both warm and
cold paths. Lookup and malformed-entry handling affect optimization state only. Exact key comparison after digest lookup
and the inherited valid-entry premise keep a local cache hit tied to the compiler operation that inserted it. Optional
debug logging is correctly outside source diagnostics.

### Source metatheory transport remains valid

T₂j adds no source term, type former, value, or reduction. Under the previously reviewed T₂ premises, finite
non-recursive nominal data, hidden constructors, exhaustive internal matching, `Text`, structural `Result`, abstract
type members, bidirectional checking, sealing, preservation, progress, deterministic evaluation, and strong
normalization retain their proofs. Graph-local names replace internal integers only in artifacts; the T₂h renaming
lemmas transport typing and evaluation. No persistent private nominal value or owner identity returns.

## Current implementation distinction

- Current lexer identifiers are exactly ASCII `[a-zA-Z_][a-zA-Z_0-9]*`, so T₂j's identifier and Unicode statement is
  accurate today.
- Current source routinely imports several modules from the bundled `std` package, confirming that module requests and
  package dependency edges have different multiplicity.
- Current imports are lexical over a caller-supplied source map. Package graphs, exact Git lock resolution, simultaneous
  versions, graph-relative nominal names, public interface files, replay events, and persistent source caches are
  absent.
- Current internal declaration keys are transient and explicitly non-serializable. User-defined nominal data and
  abstract type members are absent.
- Current `WorkMeter` implements the governing deterministic source limits. It has no cache replay layer.

I ran the language lexer filter and the compiler suite filters for all 13 import laws, all 15 module laws, core laws,
resource validation, and template laws; and the project suite filters for all 22 project-file laws, all 18 project laws,
and the resource-session law. All passed. These establish the current compatibility surface, not T₂j's unimplemented
claims.

## Verified, judged, and not checked

### Verified

- Every T₂j definition, lemma, theorem, and proof against all required closure items from review 64.
- Governing exact-identity/hash rules, source/package/module boundaries, exact-Git lock plan, deterministic resource
  acceptance, diagnostic order, and source-core metatheory.
- Current ASCII identifier grammar, multi-module import examples, import/module/resource implementation surfaces, code
  map, package prompt, and focused suite results.

### Judged

- The digest-collision and package-edge/module-request counterexamples.
- Correctness of the repaired selected-node embedding, nominal transport, canonical table, and replay induction.
- Sufficiency of the public-environment boundary and the source-metatheory transport.
- The smallest exact-tree and split-edge repairs.

### Not checked

- Any executable T₂j resolver, package graph, exact-tree registry, interface encoder/transport, nominal loader, replay
  scheduler, persistent cache, or result codec, because none exists.
- Remote Git fetch, hostile repository/archive, lockfile, and offline-cache behavior planned by prompt 162.
- Musical or cultural adequacy of the future theory packages.

## Required closure

1. Make exact canonical source-tree data, not its digest, authoritative for package-instance equality; retain digests
   only for lookup and verification followed by an exact collision check.
2. Separate locked package-dependency edges from source module-import requests, and use the former for package graph
   addresses, sharing, and cycle checks.
3. Verify each node's functional outgoing dependency map against its exact manifest and reject equal instance records
   paired with unequal lock edges.
4. Define interface transport as decode, structured graph-name transport, and canonical re-encode.

Those changes preserve the selected-node and cache proofs. They repair the input object on which those now-correct
proofs operate.
