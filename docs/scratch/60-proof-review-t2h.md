# Proof review: T₂h relocatable checking

**Status: independent closure review of the frozen T₂h candidate; governs nothing.** I reviewed
[59](59-t2h-relocatable-checking.md) against T₂g and review [58](58-proof-review-t2g.md), the relevant T₂b–T₂f
metatheory, the governing package and resource rules, and current compiler/project behavior. The review distinguishes an
interface's structural contents from the identity and sharing of the package instances that instantiate it.

## Findings

### High

1. **A direct `Import(i, path)` cannot name a nominal type re-exported through an imported interface.**
   - **Location:** [59 §§1–2](59-t2h-relocatable-checking.md), especially `RelativeTypeName` and Lemma 2.1.
   - **Type:** wrong object / incomplete name grammar.
   - **Problem:** an exported value or function may have a type owned by its package's dependency, even though T₂h
     correctly does not add transparent source type aliases. Let package `D` export hidden nominal `T`; let package `A`
     import `D` and export `x : D.T` and `use : D.T -> Nat`; and let package `P` import `A`. The public interface of `A`
     must retain that `T` is owned by `D`.

     In `A`'s interface, `Import(0, D_module, T)` can mean `D.T` because parameter zero is scoped to `A`. Once `P`
     imports that interface, neither available reading works:

     1. copying `Import(0, D_module, T)` makes zero refer to `P`'s direct parameter, which is `A`, not `D`; or
     2. rewriting it to `Import(i_A, D_module, T)` asks the loader for declaration `D_module.T` in `A`'s build node,
        where that nominal was not declared.

     A value declaration such as `A.x` is not itself a nominal declaration path, so using its exported path does not
     repair the displayed name grammar. The same failure occurs for an imported nominal nested under `List`, `Result`,
     a product, or an arrow.
   - **Why Lemma 2.1 does not prove existence:** its proof says a direct target interface gives every exported
     declaration path one meaning. That establishes lookup of a declaration owned by the direct target. It does not turn
     a nominal owned by a transitive dependency into a declaration of that target, nor preserve the lexical scope of the
     target interface's own parameter numbers.
   - **Suggested repair:** make a public interface an explicitly scoped interface scheme. Its imported package-instance
     variables are binders, and every external nominal is named by one of those variables, not by an unscoped direct
     import number. When one interface is imported into another, instantiate its variables through an explicit mapping.
     Equivalently, use `External(node variable, module, declaration path)` over a canonical public-instance DAG. Include
     the binder table in `PublicInterfaceKey`; bare relative type syntax is not complete interface data.

2. **`CheckedKey` erases the package-instance sharing relation on which nominal equality depends.**
   - **Location:** [59 §§3 and 7](59-t2h-relocatable-checking.md), affecting Theorems 7.1 and 7.2.
   - **Type:** exact counterexample / false theorem.
   - **Problem:** two `ImportParameter` records contain aliases, requested modules, and public-interface keys, but not
     whether their active targets are one `BuildNodeId` or two distinct nodes with equal public interfaces. T₂h makes
     nominal equality depend on that distinction: one node receives one fresh `TypeId` per declaration, while two nodes
     receive two ids. Public-interface equality deliberately does not settle package-instance equality.
   - **Exact direct counterexample:** let interface `I` export hidden nominal `T`, `x : T`, and `consume : T -> Nat`.
     Package `P` imports `I` through aliases `a` and `b` and checks:

     ```text
     let answer : Nat = b.consume(a.x)
     ```

     In accepted graph `G_shared`, both aliases target one node, so `a.T = b.T` and the definition checks. In
     `G_split`, the aliases target two nodes with byte-identical public interfaces, so the fresh ids differ and checking
     rejects the argument. Every displayed `CheckedKey(P)` field can nevertheless be equal: `P`'s exact source and
     manifest, both alias/module records, both equal public-interface keys, options, and all versions.

     T₂g/T₂h build formation does not require two aliases of one exact reference to coalesce; it only says a
     `BuildNodeId` *lets* a diamond share. Thus both graphs satisfy the stated formation rules. Even if a stronger
     resolver law made identical direct references coalesce, the problem remains for editable local packages. Keep
     `P`'s paths and manifest fixed, let local packages `A` and `B` expose the same signatures, and change `B`'s local
     dependency from the node shared with `A` to a second node having the same public interface. `P`'s displayed key is
     unchanged while `B.consume(A.x)` changes from accepted to rejected.
   - **Diamonds and two versions:** a shared diamond must identify the dependency-owned nominal reached along its two
     arms. Two simultaneous versions with equal-looking interfaces must not. The required datum is the equality pattern
     of formal package-instance variables. It is neither a permanent package owner nor a build-local number; it is part
     of the instantiated public dependency graph for this check.
   - **Execution consequence:** the displayed recursive `ExecutionKey` also does not distinguish one shared node from
     two equal-key nodes. It describes an unfolded dependency tree, not an exact rooted DAG. Besides nominal wiring,
     this can change whether package initialization and its logical resource charges occur once or twice. Therefore the
     proof phrase "following the finite dependency graph" uses graph data that the key has not encoded.
   - **Suggested repair:** include in `CheckedKey` a canonical rooted **public-instance DAG**. Label each node by its
     public interface scheme/key, label edges by alias and requested module, and retain exact node sharing even when two
     labels are equal. Canonical node numbers are local binders, not persistent identities. Use those binders in
     relative type names. Define the `ExecutionKey` analogously, labeling nodes by exact checked/execution inputs and
     preserving the same sharing. Alternatively, add a build-formation theorem that canonically coalesces every equal
     exact package reference, but that alone does not handle two distinct equal-interface versions or transitive
     interface instantiation.

### Medium

1. **The resource proof establishes the resource diagnostic, not every canonical diagnostic claimed by Theorem 7.1.**
   - **Location:** [59 §5 and Theorem 7.1](59-t2h-relocatable-checking.md).
   - **Type:** proof gap.
   - **Problem:** a resource trace records only charges and their locations. A cold check may emit an ordinary warning
     or another canonical diagnostic before a later charge exhausts the meter. On a hit, publishing all diagnostics
     stored by the earlier successful run can publish diagnostics that lie after the new rejection point; publishing
     none can omit diagnostics that the cold run emitted before it. Lemma 5.1 proves equality of the first resource
     diagnostic but says nothing about other diagnostic emissions.
   - **Suggested repair:** either make diagnostics transactional and specify that a resource-rejected operation exposes
     only its resource diagnostic, both cold and warm, or record diagnostic-emission events in the ordered replay trace.
     If checking computes one canonical diagnostic batch independently of resource state, state and prove that stronger
     premise. Current compiler behavior includes warnings, so this is not an empty concern.

2. **Package-level trace composition is undefined.** The cost model fixes a canonical package order, but the note does
   not say whether a package execution trace includes its reachable dependencies or only its own work. If `D` is
   restored and charged separately and the cached trace for `P` also includes the cold evaluation of `D`, a warm build
   charges `D` twice. If all traces are exclusive, a root hit must still replay every reachable node exactly once in
   canonical DAG order. This matters especially for diamonds. Define one of two protocols: a root entry contains one
   inclusive whole-operation trace and no child entry is separately replayed, or entries contain exclusive node traces
   and the canonical instantiated DAG schedules each node once.

3. **`valid cache hit` hides the association between the key, trace, codec, and stored bytes.** Section 6 gives a strong
   left-inverse and canonical-encoding law for values, but no cache-entry record states that the bytes being decoded are
   exactly `encode(v)` for the result inserted under this `ExecutionKey`, nor where the codec version is framed.
   Replacing a valid encoding of `0` by the valid encoding of `1` is not detected by the codec law alone. This may be
   outside the intended corruption threat model, but then `valid` must explicitly mean an unmodified entry inserted by a
   successful equal-key run. For self-validating cache files, define an exact framed record containing the full key,
   trace, codec id/version, and result bytes, and require `decode(b) = v` only for a canonical image satisfying
   `encode(v) = b`.

4. **The resolver and canonical-graph format versions are absent from the displayed keys.** Checker and language
   versions need not change when package coalescing, logical module normalization, or canonical graph numbering changes.
   Once the public-instance DAG repair is made, its resolver/graph-format version must be included in both interface
   instantiation and execution keys. Otherwise equal bytes can be interpreted with different instance wiring.

### Low

1. **The import-parameter order is deterministic but not defined.** `CheckedKey` says the records are ordered, but it
   does not select source order, canonical alias order, or dependency-first order. Any one deterministic choice works;
   it must be part of the artifact format so two implementations produce the same parameter numbers.

2. **Lemma 4.2 should state the environment/substitution case.** The claimed renaming theorem is correct under the
   retained T₂b primitive-opacity premise, but the proof mentions only terms and results. Closures, captured
   environments, pattern-bound fields, and simultaneous substitutions must be renamed recursively. This is routine and
   does not threaten the lemma.

## Verdict

- **Decision:** **Incorrect.** `Self` repairs the original own-slot bug, and ordered alias/interface records repair the
  simple alias-swap bug, but the new relative-name grammar cannot carry a dependency-owned nominal through a public
  interface. Independently, the key forgets whether equal interface parameters instantiate one package node or two. That
  equality pattern changes typechecking, so Theorem 7.1 is false. The same missing exact-DAG object prevents the
  displayed `ExecutionKey` from supporting its proof as stated.
- **Basis:** I checked the literal relative-name grammar and all loader cases; direct, transitive, and re-exported
  imported nominals; repeated aliases; shared and split diamonds; simultaneous equal-interface versions; private body
  edits; current and prior resource-meter states; warning/resource ordering; evaluator renaming; codec laws; exact
  source-package governance; and every retained/deleted T₂b–T₂f claim.
- **Limits:** T₂h has no implementation. I treated the previously reviewed positive-rank source calculus and
  compiler-primitive opacity as dependencies. No current code can execute the nominal/interface or cache examples.

## Clean passes

### `Self` closes the original own-slot counterexample

A package's local nominal names no longer contain its build-local node id. Rechecking the same dependency-free package
under node ids `s` and `t` produces the same relocatable `Self(module,path)` interface. Loading assigns fresh internal
ids after the current build graph exists. This closes review 58's first exact counterexample for local declarations.

### Resource replay works across different prior meter states

Lemma 5.1 has the right current-state hypothesis. The cache entry may have been created by a successful run beginning at
some earlier state `S₀`; the present cold operation and hit need only begin together at state `S₁`. Since T₂h explicitly
requires the next pre-rejection charge to depend only on semantic inputs, the stored complete trace is also the cold
operation's charge sequence. Replaying it from `S₁` either crosses the same first limit or reaches the same final state.
The ordered trace, rather than metric totals, correctly preserves the located first-exhaustion diagnostic.

This closes review 58's second high finding for one cacheable operation, subject to the diagnostic and composition
qualifications above.

### Internal `TypeId` renaming is semantically invisible

Lemmas 4.1 and 4.2 are valid under the imported rules: user programs cannot inspect numeric ids; constructor and match
rules use the renamed owner table consistently; the foreign primitive registry cannot inspect user nominals; and the
calculus is deterministic. A bijection between the ids occurring in two loaded programs transports typing and
evaluation. Because an admitted persistent result cannot contain a user nominal id, the renamed result agrees exactly at
the codec boundary.

### The codec boundary is appropriately narrow

The stated codec laws provide versioning, `decode(encode(v)) = v` up to the admitted equality, canonical bytes,
malformed/unknown/trailing-data rejection, and exclusion of functions, closures, and user nominal values. Compiler-
owned finite results can satisfy those laws after individual review. The medium finding concerns cache-entry framing,
not the decision to exclude private values.

### Body edits propagate through execution keys

A reachable dependency body edit changes that dependency's exact source in its `CheckedKey`; the change propagates
through every parent `ExecutionKey`. A parent `CheckedKey` may remain stable when the imported public interface and its
instance wiring remain stable. Corollary 7.3 is correct after the exact public-instance DAG is added.

### Diagnostics, governance, and metatheory scope are substantially repaired

Logical document names and byte ranges are the correct cached diagnostic coordinates; current absolute display paths
belong to the project/editor projection. Section 9 now matches the governing bundled/local/exact-Git, offline,
no-registry/no-solver scope and defers persistent caches pending measurement.

Section 8 also gives an honest transport table. The finite syntax, `Text`, structural `Result`, positive nominal ranks,
sealing, namespace and checking rules, finite dependency graphs, preservation, progress, determinism, and strong
normalization survive replacement of nominal atoms. Persistent `StampKey`, owner/active registries, private nominal
encoding, `StableInterface`, and `StorableValue` are explicitly deleted. The W/K examples are correctly limited to
mechanism tests.

## Current implementation distinction

- Current imports are lexical resolution over a caller-supplied finite `ImportSources` map. They provide no package-
  instance DAG, public-interface scheme, imported nominal type, or simultaneous package version.
- Current package code implements the bundled standard library's declared `lib.musa`/`mod.musa` tree. Exact Git
  fetching, editable dependency graphs, lockfile loading, package coalescing, and interface files are not implemented.
- Current declaration handles are transient and explicitly not serializable. There is no user-nominal `TypeId`, checked-
  artifact loader, or cross-build nominal value.
- Current `WorkMeter` has the governing fixed metrics and is threaded through checking/evaluation within one
  compilation. There is no resource trace, package-level replay scheduler, or persistent checked/execution cache.
- Current compiler diagnostics include warnings, and `SourceDocument` carries a display name. The proposed logical-
  coordinate/display-path separation is therefore useful but unimplemented.

I ran the compiler package-tree tests, all 13 import-law tests, all 15 module-law tests, the core-law, resource-
validation, and template-law suites, all 22 project-file laws, and all 18 project laws. All passed. These tests
establish the present compatibility surface, not T₂h's unimplemented claims.

## Verified, judged, and not checked

### Verified

- Every displayed T₂h definition, lemma, theorem premise, and proof step against review 58's required closure list.
- Governing flat imports, declared package module trees, exact-pinned Git/no-solver rules, exact rational source-core
  evaluation, deterministic resource acceptance, and diagnostic ordering requirements.
- Current import/package traversal, transient identities, diagnostic/display-name surface, work meter, project
  compilation path, and the focused tests listed above.
- The exact T₂b–T₂f claims that section 8 retains, replaces, and deletes.

### Judged

- The transitive-interface and shared-versus-split-instance counterexamples.
- The canonical public-instance DAG/interface-scheme repair.
- The TypeId-renaming proof, resource-replay induction, diagnostic-event gap, and cache-framing requirements.
- The source-core metatheory transport after replacing nominal atoms.

### Not checked

- Any executable T₂h resolver, interface scheme, checked artifact, loader, graph key, resource trace, codec, or cache,
  because none exists.
- Remote Git fetch and lockfile behavior planned by later prompts.
- Musical or cultural adequacy of the proposed W/K packages.

## Required closure

1. Replace unscoped direct `Import` names with lexically scoped interface parameters or canonical public-instance node
   variables capable of naming transitive dependency-owned nominals.
2. Put the complete binder/import table in `PublicInterfaceKey` and define interface instantiation composition.
3. Put the canonical public-instance DAG, including node sharing, in `CheckedKey`; put the analogous exact rooted DAG in
   `ExecutionKey`; version the resolver and graph encoding.
4. State the build coalescing law for repeated exact package references and test shared diamonds against two distinct
   equal-interface versions.
5. Define package trace composition and the relationship between resource replay and ordinary diagnostic publication.
6. Frame a valid cache entry with its full key, trace, codec id/version, and exact result bytes, or state the trusted-
   cache admission premise explicitly.

These repairs preserve T₂h's main simplification. Canonical package-instance variables are local binders describing one
resolved graph; they do not revive permanent owner identities or portable private nominal values.
