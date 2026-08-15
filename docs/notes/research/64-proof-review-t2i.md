# Proof review: T₂i package graphs

**Status: independent review of the frozen T₂i research candidate; governs nothing.** I reviewed
[63](63-t2i-package-graphs.md) against reviews [58](58-proof-review-t2g.md) and [60](60-proof-review-t2h.md), the
T₂b–T₂h source-metatheory chain, the governing documents under `docs/rules/`, the implementation plan and code map under
`docs/plan/`, and current compiler/project code. The central question was whether the new exact object really is a
rooted package DAG, rather than another summary that forgets how nominal owners are shared.

## Findings

### High

1. **The required root-preserving interface inclusion does not exist for an ordinary import.**
   - **Location:** [63 §4](63-t2i-package-graphs.md), before Lemma 4.1.
   - **Type:** false formation rule / wrong morphism.
   - **Problem:** let project package `P` import package `A`. The stored public graph of `A` is rooted at `A`, so `A`
     has the empty address in that graph. In `P`'s checking graph, the empty address belongs to `P`; `A` has the address
     of the `P -> A` edge. The inclusion needed to instantiate `A`'s interface must therefore send the source root `A`
     to the **selected `A` node**, not to the ambient root `P`. Section 4 nevertheless says the loader accepts the
     inclusion only when it preserves the root. A root-preserving map sends `A` to `P` and cannot preserve their package
     labels. The only import for which the stated inclusion can exist is the degenerate case in which the imported
     public graph is already rooted at the checking root.
   - **Address-bearing labels expose a second literal mismatch:** suppose `A` owns hidden nominal `T` and exports
     `x : T`. In `A`'s stored graph, the owner address written in `x`'s type is empty. In `P`'s graph it is the address
     of `P -> A`. Raw node labels are unequal. The intended operation is clearly to **transport** `A`'s interface body
     along the node map and compare the transported label with `P`'s row, but the stated condition instead says the
     inclusion preserves node labels while the preceding paragraph says applying it changes addresses.
   - **Why it matters:** this is the load-bearing construction meant to repair transitive exported types. As stated, `P`
     cannot instantiate even a one-node public interface from `A`, so Lemma 4.1 is vacuous for the ordinary import case
     and Theorems 5.1 and 7.2 do not have their required interface input.
   - **Suggested repair:** use a **pointed subgraph embedding** `j : G_A -> G_P` whose distinguished source root maps to
     the target selected by `P`'s import edge. Require injectivity, edge commutation, and preservation of node sharing.
     Define label preservation as the commuting equation

     ```text
     transport_j(interface_body_A(v)) = interface_body_P(j(v)).
     ```

     Do not require preservation of the ambient root. State whether `G_P` here is the resolved graph, the checking
     graph, or its dependency-interface subgraph; only the last two have comparable interface-body labels.

2. **Theorems 5.1 and 8.1 omit resource-state premises required by their own replay semantics.**
   - **Location:** [63 Theorem 5.1 and Theorem 8.1](63-t2i-package-graphs.md).
   - **Type:** false statements / missing hypotheses.
   - **Checking counterexample:** take one package whose successful check requests 100 logical reduction steps. Two
     checks have the same `CheckedKey`. The first starts with a fresh project meter and produces a checked artifact. The
     second starts after earlier packages have consumed all but 50 steps; it stops at the charge and produces no
     artifact. Thus equal `CheckedKey` values do not, without qualification, "produce equal checked artifacts." Theorem
     5.1's key correctly determines the successful artifact and its exclusive replay list, but project-meter state still
     determines whether that artifact is released.
   - **Execution counterexample:** an execution needs 100 steps. Compare a cache hit starting with 50 steps of remaining
     budget with fresh execution starting from an empty meter. The hit rejects and the fresh execution succeeds, despite
     equal requested `ExecutionKey` bytes. Theorem 8.1 invokes Lemma 7.1, but Lemma 7.1 explicitly assumes equal initial
     meter **and diagnostic** states; Theorem 8.1 does not.
   - **Why it matters:** these are theorem-statement errors, not a failure of replay events. Lemma 7.1 and Theorem 7.2
     correctly compare warm and cold work from equal states. The broader statements silently drop precisely that
     premise.
   - **Suggested repair:** state Theorem 5.1 as equality of the **successful, pre-resource artifact and replay event
     list** determined by equal checking keys, or add "if both checks finish successfully." State Theorem 8.1 with the
     hit and fresh execution beginning at equal project-meter and diagnostic states. If execution emits no ordinary
     diagnostics, say so and retain only the equal-meter premise.

### Medium

1. **The interface-body descriptor is not yet the exact public checking contract.** Section 4 calls it "exported names
   and complete types." The source proposal also needs declaration kind, abstract nominal declarations, visibility and
   sealing, structure/signature membership, and every other public fact on which name resolution or checking can branch.
   A list of value names and types cannot tell an exported abstract type declaration from an unavailable name, nor can
   it establish that a hidden constructor is absent from the public source environment. T₂b's reviewed boundary was the
   functional `Σ_public` environment plus retained private metadata, not just value signatures. Define `InterfaceBody`
   as the exact canonical encoding of that public environment, with graph-relative nominal names, and prove that
   dependency source is used by a client checker only through this descriptor. Documentation-only fields may remain
   outside if they do not affect compiler diagnostics.

2. **The two-phase relationship between addresses and node labels is implicit.** Least paths depend only on the rooted
   edge-labelled graph, while checking/public node labels can themselves contain those addresses. Lemma 3.1 is sound if
   canonicalization first computes addresses from edges and only then rewrites and encodes labels. Without that order,
   "node-labelled graph" appears circular: graph equality is used to obtain addresses while label equality already
   mentions them. State the two phases and define graph isomorphism on address-bearing labels by transport. This is an
   exposition and definition gap, not a counterexample to the least-path construction.

3. **Exact resolver coalescing needs exact instance descriptors.** "One exact locked Git package" should be the
   governing lock descriptor: package id, immutable repository, revision algorithm and object id, verified source-tree
   identity, and locked dependency edges. "One editable local package root" needs one canonical admitted manifest/root
   identity after lexical normalization, case/Unicode checks, and the symlink policy. Otherwise two implementations can
   disagree about whether different URL spellings, repository mirrors, lexical local paths, or in-root links coalesce.
   The graph theorem is correct once selection is functional, but the resolver rules must define the equality whose
   sharing the graph records. The governing exact-Git design provides most of these fields and should be imported
   literally.

4. **Diagnostic storage and replay need one publication rule.** A checked artifact contains canonical diagnostics, and
   its exclusive replay list contains `Emit` events for those diagnostics. On a hit, replay appends the diagnostics;
   releasing the artifact must not append its diagnostic list again. Conversely, consumers may still need the list as
   artifact data. State that replay events alone mutate the project diagnostic state and that the artifact's list is a
   read-only copy/projection, or remove the duplicate field. This is a possible double-publication bug, not a defect in
   the `Emit` event design.

5. **Replay must cover all acceptance-affecting work at the declared operation boundary.** T₂i clearly fixes the earlier
   composition ambiguity: checking entries are exclusive per node, and execution entries are inclusive for the whole
   rooted operation with no descendant replay. The remaining boundary question is whether resolver, interface inclusion
   validation, checked-artifact decoding, and graph construction request any governing language-resource charges or emit
   canonical diagnostics. If they do, their events must occur at one specified node or outside both cached operations in
   exactly the same way on warm and cold runs. If they are compiler-administration work outside the source meter, say
   so. The current core meter covers checking and evaluation, not wall-clock resolver work, so the latter choice is
   natural.

### Low

1. **Canonical edge-label bytes need their full grammar.** Alias plus requested module is enough to distinguish the
   examples only if the requested package coordinate/resolution edge is already fixed by the node target. Define Unicode
   normalization, case policy, module-path segmentation, and duplicate import handling before declaring the byte order.
   Length framing alone does not choose those source-level equivalences.

2. **The valid-cache trust premise is intentionally narrow but should name accidental corruption.** Section 8 honestly
   treats a valid entry as an unmodified record inserted by the named compiler operation and excludes deliberate cache
   tampering from language semantics. Canonical decoding catches malformed bytes but cannot detect replacing one valid
   result encoding with another valid encoding. That is sound under the trust premise. If Musa wants bit-rot detection,
   add a verified digest over the complete record; do not describe it as proof against a malicious local user.

## Verdict

- **Decision:** **Incorrect.** The central replacement object is now the right one, and the canonical graph lemma itself
  survives. However, the literal public-interface inclusion rejects every proper imported root, and two stated cache
  theorems omit resource-state premises required by the replay lemma. These are local repairs rather than reasons to
  return to permanent nominal owners or import lists.
- **Basis:** I checked the graph definition and canonical-address proof; resolver coalescing and relocation; direct,
  transitive, and re-exported nominal types; shared and split diamonds; equal-interface versions; checking and execution
  graph fields; exclusive/inclusive replay composition; ordinary and resource diagnostics; cache framing and trust; and
  the exact T₂ source-metatheory claims transported in section 9.
- **Limits:** T₂i is not implemented. The positive-rank nominal calculus, sealing proof, and `TypeId`-renaming lemmas
  are previously reviewed mathematical dependencies rather than executable current code. Remote package resolution and
  lockfiles remain planned work.

## Clean passes

### The least-path canonical table preserves exact sharing

Lemma 3.1 is correct for the stated finite rooted DAG once node labels are encoded after address assignment. Every node
has at least one finite root path. Unique outgoing edge labels make following one label path functional, so two nodes
cannot share one address. A rooted edge-labelled isomorphism preserves the full path set and therefore its least member.
Sorting rows by those addresses and recording every target address reconstructs the exact rooted graph. Full table-byte
comparison, rather than digest equality, satisfies the governing exact-identity rule.

In particular, the table distinguishes:

- two aliases whose edges meet at one node from two aliases ending at distinct equal-labelled nodes;
- a shared dependency in a diamond from two copies with equal public interfaces; and
- two exact package versions with equal source-level signatures.

The canonical addresses are local binders. Moving the complete project tree leaves normalized local roots, dependency
edge labels, and addresses unchanged; adding or renaming a dependency may change addresses and invalidate a cache, which
is conservative but sound.

### Public graphs contain the data needed for transitive nominal names

After replacing the faulty root condition by a pointed embedding, a public graph solves the T₂h counterexample. If `D`
owns `T`, `A` exports `x : D.T`, and `P` imports `A`, the type continues to name `D`'s graph node. The embedding moves
the address into `P`'s graph without changing ownership. Its injectivity preserves and reflects nominal equality, so it
neither splits a shared node nor identifies distinct versions. Lemma 4.1 is correct for such an injective node map.

### Checking and execution keys now retain the right dependency distinctions

The checking graph includes the root's exact source, manifest, logical module map, options, all reachable public
interfaces, exact edges, and node sharing, with resolver, graph, checker, language, artifact, and cost-model versions.
That closes the previous `b.consume(a.x)` false hit. It is deliberately conservative to include unused reachable public
nodes.

The execution graph has the same exact sharing and labels every node with its source-sensitive `CheckedKey` and
operation version. A private dependency body edit changes that node's checked key and therefore the exact root execution
table, while a parent's checking graph can remain stable when public inputs and sharing remain stable. Corollary 6.1
survives.

### Replay event composition is coherent

Checking schedules every node once in dependency-first order with a deterministic address tie-break. Exclusive per-node
events allow any mixture of hits and misses without double charging a shared dependency. `Emit` events place ordinary
diagnostics in the same order as cold checking, and charge replay stops before later events. Execution's one inclusive
root list, coupled with the rule never to replay child execution entries on a root hit, closes the previous diamond-
composition ambiguity.

Lemma 7.1 and Theorem 7.2 are correct when warm and cold builds start from equal meter and diagnostic states. The stored
list may have been created from a different earlier meter state: because pre-rejection events depend only on equal
semantic inputs, replaying that list from the current shared state rejects at the same first event or reaches the same
final state.

### Cache framing and trust are honest

The cache entry frames its exact key bytes, event format/list, codec name/version, and result bytes; lookup confirms the
complete key after digest lookup and rejects unknown versions, malformed fields, and trailing data. The result is
checked for canonical encoding. The theorem explicitly assumes an unmodified entry inserted by the named compiler
operation, and distinguishes disposable local compiler output from untrusted package source. This closes review 60's
cache-association gap without pretending canonical encoding authenticates an adversarial cache.

### Source safety transports

Package graphs, graph-relative names, cache keys, and replay schedules add no source term, type former, value, or
reduction. Under the previously reviewed T₂ premises, finite non-recursive nominal data, private constructors,
exhaustive internal matching, `Text`, structural `Result`, abstract structure members, finite acyclic declaration
graphs, bidirectional checking, preservation, progress, deterministic evaluation, and strong normalization therefore
retain their proofs. Fresh `TypeId`s are internal opaque nominal atoms, and the T₂h renaming lemmas transport typing and
evaluation between equal graph-relative artifacts. No persistent private runtime identity is reintroduced.

The candidate also correctly keeps W and K as mechanism tests and makes real culturally reviewed theory packages the
next adequacy gate.

## Current implementation distinction

- Current compiler imports are lexical resolution over a caller-supplied finite source map. They have no immutable
  package-instance DAG, public graph, graph inclusion, transitive package type, simultaneous version selection, or
  lockfile.
- Current package code traverses the bundled standard library's declared `lib.musa`/`mod.musa` tree. Exact locked Git
  fetching and the resolver/cache work remain in pending prompt 162.
- User-defined nominal data, hidden constructors, and abstract type members are absent. Current internal `DeclKey`
  values are transient and explicitly must not be serialized.
- Current `WorkMeter` enforces the governing source limits inside checking/evaluation. There are no replay events,
  graph-keyed checked artifacts, persistent source execution entries, or result codecs implementing T₂i.
- The project code map accurately marks theory-owned nominal types and exact package/cache infrastructure as absent.

I ran the compiler suite filters for all 13 import laws, all 15 module laws, core laws, resource validation, and
template laws; and the project suite filters for all 22 project-file laws, all 18 project laws, and the resource-session
law. All passed. They verify the present compatibility surface, not T₂i's unimplemented theorems.

## Verified, judged, and not checked

### Verified

- Every T₂i graph, address, interface, checking, execution, replay, cache-entry, and transport definition against the
  prior review obligations.
- Governing exact identity/hash rules, source authority, package shape, exact Git/no-solver plan, deterministic resource
  acceptance, diagnostic ordering, and source-core metatheory.
- The reorganized documentation precedence, implementation code map, prompt 162 package boundary, current compiler and
  project surfaces, and the focused suites listed above.

### Judged

- The root-preservation and missing-meter counterexamples.
- Correctness of least-path canonicalization and nominal equality under a pointed injective embedding.
- Sufficiency of exact checking/execution DAGs for shared versus split interfaces and private-body invalidation.
- Replay event composition, codec/trust boundary, and transport of the source safety proof.

### Not checked

- Any executable package resolver, public/checking/execution graph, graph codec, interface inclusion, checked artifact,
  replay scheduler, persistent cache, or user-nominal loader, because none exists.
- Network fetch, lockfile, offline package-cache, and hostile archive behavior planned by prompt 162.
- Musical or cultural adequacy of the future W/K source packages.

## Required closure

1. Replace root-preserving public-graph inclusion by a pointed injective embedding into the selected dependency node,
   and state the exact transported-label commuting law.
2. Define the complete graph-relative public checking environment encoded by an interface body.
3. State the two-phase edge-address-then-label canonicalization and exact resolver instance descriptors.
4. Add successful-check/equal-current-state hypotheses to Theorems 5.1 and 8.1.
5. State that replay events alone publish cached diagnostics and place any resolver/decoding events at an exact cache
   boundary.

After those repairs, the package graph is a sound and substantially simpler basis for source-package checking and future
caches. None of them requires permanent owner keys or portable private nominal values.
