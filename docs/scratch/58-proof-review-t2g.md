# Proof review: T₂g source packages and safe caches

**Status: independent review of the frozen T₂g candidate; governs nothing.** I reviewed
[57](57-t2g-source-packages-and-caches.md) against the T₂a–T₂f chain and reviews [46](46-proof-review-t2a.md),
[48](48-proof-review-t2b.md), [50](50-proof-review-t2c.md), [52](52-proof-review-t2d.md), [54](54-proof-review-t2e.md),
and [56](56-proof-review-t2f.md). I also compared its package, import, resource, and cache claims with the governing
language documents and current compiler/project code.

## Verdict

**Incorrect as written.** T₂g makes two good simplifications: it gives up persistent identity for private user values,
and it replaces T₂f's unsound implementation key by a dependency-closed execution key. Those choices close the central
T₂f counterexample. However, Theorem 5.1 is false under T₂g's own definition of a build-local package slot, and neither
cache theorem accounts for the governing whole-compilation resource meter. A cache implementation built from the
displayed rules could consequently reject a valid checked artifact after a harmless slot renaming, or make language
acceptance depend on whether the cache was warm.

Neither defect requires restoring `OwnerKey`, stable private nominal values, or the T₂d–T₂f registry machinery. The
repair is to make checked artifacts relocatable with respect to package slots, and to make cache hits replay a certified
logical resource charge.

## Findings

### High

1. **Theorem 5.1 is false because `CheckedKey` omits the package's own build-local slot.**
   - **Location:** [57 §§3–5](57-t2g-source-packages-and-caches.md).
   - **Type:** exact counterexample / false theorem.
   - **Problem:** `TypeName` and `InterfaceKey` contain `package slot`, but `CheckedKey` contains only the package
     source, manifest, options, and imported interface keys. T₂g gives no cross-build stability law for a slot; on the
     contrary, a slot names an instance "in this build." Renaming slots is therefore a legal renaming of a resolved
     build.
   - **Exact counterexample:** let dependency-free package `Q` have source

     ```text
     data T { Mk(Nat) }
     ```

     and export `T`. Check the same exact source and manifest in accepted build `B₀`, where `Q` has slot `s`, and in
     accepted build `B₁`, obtained only by renaming that slot to `t`. The builds have equal `CheckedKey(Q)`: there are
     no imported interface keys, and every displayed field is equal. Nevertheless, the produced public interfaces
     contain `(s, M, T)` and `(t, M, T)`, respectively. They are not equal up to a renaming of fresh `TypeId`s, because
     the unequal component is the stored symbolic `TypeName`, not a `TypeId`. A cached checked artifact from `B₀`
     cannot resolve `(s, M, T)` in `B₁` and is rejected by T₂g's own load rule.
   - **Two-version pressure:** distinct slots are necessary to distinguish two instances of one package in a build. A
     slot allocation ordinal is therefore not a harmless implementation detail in the current interface format. Adding
     an unrelated instance can renumber a package and invalidate its interface. Making the slot a digest of the exact
     source avoids renumbering, but then a dependency's private body edit changes its public symbolic names and defeats
     the advertised interface-preserving checking reuse.
   - **Repair:** serialize a relocatable interface. Use a package-relative `Self` symbol for declarations owned by the
     checked package and explicit imported-interface parameters for external nominals. Instantiate `Self` and those
     parameters with the active build's slots when loading. The checked key must contain the exact ordered
     `alias -> InterfaceKey` environment. A less attractive alternative is a canonical stable slot law plus the own slot
     in `CheckedKey`; that law must distinguish simultaneous versions without depending on private bodies or unrelated
     graph allocation.

2. **The cache theorems omit the deterministic resource state that is part of governing language acceptance.**
   - **Location:** [57 §§5–6](57-t2g-source-packages-and-caches.md) against
     [`02-core-calculus.md` §4](../language/02-core-calculus.md).
   - **Type:** missing hypothesis / cache-state-dependent language semantics.
   - **Problem:** the governing core uses one deterministic meter over checking and evaluation. It rejects at the next
     over-budget operation and makes that operation part of the diagnostic. A package's outcome is consequently not a
     function of the displayed `CheckedKey` alone if packages consume a shared build meter. More seriously, physically
     skipping work on a cache hit must not skip its logical charge.
   - **Exact cold/warm counterexample:** choose package `D` whose accepted checking and evaluation consumes 120,000
     logical reduction steps, and root package `P` whose other work consumes 100,000. With the governing 200,000-step
     limit, a cold build rejects when the aggregate next charge passes the limit. If restoring `D` from a checking or
     execution cache charges zero, the otherwise identical warm build consumes only `P`'s 100,000 steps and succeeds.
     Thus source, lockfile, options, and every displayed cache key are equal while acceptance differs. The same problem
     appears between two equal `CheckedKey(D)` checks that begin with different accumulated meter states: one can return
     a checked program while the other returns a `resource-limit` diagnostic.
   - **Why the determinism premise does not repair it:** determinism says equal complete machine states take the same
     transition. It does not show that the key contains the starting meter state, nor that a cache hit is resource-
     transparent. "The package and cache rules add no reduction rule" is insufficient because caching changes which
     logical operations are physically performed.
   - **Repair:** each cacheable stage must produce a deterministic logical cost certificate in every governing meter
     dimension. Charge that same cost, at the same semantic boundary, on a cache hit and on a miss. Whole-build resource
     acceptance must be computed from those certified costs independently of cache warmth. Version the cost model and
     include its policy and limits in the relevant semantic inputs. If package checks intentionally receive a fresh
     meter and only a later whole-build preflight aggregates costs, state and prove that architecture instead.

### Medium

1. **The imported-interface component does not define its alias association.** `CheckedKey` contains "ordered imported
   `InterfaceKey`s," but it does not say that the order retains the source dependency alias or resolved module target.
   Consider unchanged source that refers to `a.T` and a resolver environment with aliases `a -> A, b -> B`. If a second
   graph swaps the targets while a canonical key merely sorts the multiset `{InterfaceKey(A), InterfaceKey(B)}`, the
   checked source resolves `a.T` to a different nominal while the displayed checked key remains equal. The intended
   repair is an exact ordered list of `(source alias, resolved module, InterfaceKey)` records. This is a proof gap
   rather than a counterexample if "ordered" was intended to mean precisely that record order.

2. **Theorem 5.2 needs an evaluator-equivariance lemma and an actual codec contract.** Equality of execution keys gives
   checked programs only up to fresh-`TypeId` renaming, not literal equality. Sound reuse follows because the source
   calculus has no type reflection and evaluation should commute with a bijection of build-local nominal ids, while an
   admitted cross-build result cannot contain such an id. That argument is sound but absent. State it as:

   ```text
   eval(rename_ρ(program)) = rename_ρ(eval(program)).
   ```

   "Admitted exact encoding" must also name a versioned codec with a partial decoder, rejection of malformed or
   trailing data, `decode(encode(v)) = v`, and reflection of the stage's chosen value equality. The codec/schema version
   must be part of the cache entry or its key. Exact `List`, `Option`, `Result`, and compiler-owned base values can meet
   this contract. Functions, user nominals containing `TypeId`, and private values correctly do not.

3. **The metatheory transport is under-specified because T₂g changes, rather than merely supplements, nominal
   identity.** T₂b–T₂f state rules using `StampKey`, `OwnerKey`, `ActiveOwnerTable`, `StableInterface`, and
   `StorableValue`; T₂g expressly replaces or abandons those objects. The preservation, progress, determinism, and
   strong-normalization proofs can still transport: fresh `TypeId`s are opaque nominal atoms, positive nominal ranks
   still make representation reducibility well founded, and no source reduction can inspect identity. But the document
   must enumerate the imported theorem surface. Retain the term syntax, ranked finite nominal schemas, sealing,
   namespace rules, bidirectional checking, and reduction semantics. Replace nominal equality by build-local `TypeId`
   equality. Delete the old persistent nominal encoding and stable-owner conclusions. Prove renaming equivariance for
   the replacement. The current statement "keeps the term language and safety proof" does not identify enough premises
   to be a proof.

4. **Exact diagnostics need a semantic/presentation split.** T₂g deliberately excludes absolute machine paths from
   semantic identity. Current `SourceDocument`, however, has a caller-supplied display name, often an absolute path, and
   diagnostics can expose that context. Relocating equal source from `/work/Q` to `/tmp/Q` may therefore leave
   `CheckedKey` equal while the rendered diagnostics differ. Theorem 5.1 should claim equality of canonical structured
   diagnostics using logical module paths and relative spans. Host display paths can be attached after cache lookup and
   are outside the theorem. Putting absolute display paths into the key would be sound but would defeat relocation.

5. **A released-package registry conflicts with the governing package decision.** T₂g admits a released package fixed by
   registry/name/version and tells Musa to build source-package support now. The roadmap and
   [`09-assets-and-packages.md` §2](../language/09-assets-and-packages.md) explicitly admit exact-pinned Git packages
   while rejecting registries, ranges, and a generalized solver;
   [`04-templates-and-modules.md`](../language/04-templates-and-modules.md) currently says there is no registry at all.
   Exact registry versions could be a coherent future design, but they are not an imported governing feature. For the
   first implementation, delete the registry case and retain bundled, exact Git, and editable local packages, or make a
   deliberate governance amendment before adding it.

### Low

1. **Case-collision equality needs a portable rule.** "Checks case collisions" is not yet a reproducibility law. A
   source tree containing both `A.musa` and `a.musa`, or canonically equivalent Unicode names, must receive the same
   accept/reject answer on case-sensitive and case-insensitive hosts. Define one language-level case and normalization
   policy over logical paths; do not inherit it from the host filesystem.

2. **The prototype adequacy claim is stronger than the evidence.** The T₂a–T₂f reviews established that the W/K examples
   are useful mechanism probes, not that two complete competing musical theories are expressible or that the source
   calculus avoids Western assumptions. Section 6 should say the calculus can host those probes and preserve competing
   abstract interfaces. Cultural and musical adequacy remains a separate gate.

3. **Persistent caches are a new architecture commitment.** Roadmap §10.7 defers an incremental dependency model until
   profiling demonstrates need. T₂g need not use Salsa, and exact cache artifacts can be simpler than an incremental
   query engine, but "should build now" still needs a prompt/governance change and measurements that justify the scope.
   This does not affect the mathematical key separation.

## Claims that survived

### Dependency-closed execution identity closes the T₂f bug

T₂f allowed a dependency body to change while its dependent package retained the same `ImplementationKey`. T₂g's
dependency-first `ExecutionKey` repairs that exact failure. If `D.tone` changes from `0` to `1`, `CheckedKey(D)` changes
because `D`'s exact source changes, hence `ExecutionKey(D)` changes. The parent key contains
`(dependency alias, ExecutionKey(D))`, so `ExecutionKey(P)` also changes even when `InterfaceKey(D)` and `CheckedKey(P)`
remain equal. Diamonds may share physical graph nodes without changing exact rooted-graph equality.

Corollary 5.3 therefore survives once the slot and resource repairs above are made: a dependency body-only edit can
preserve dependent **checking**, but not dependent **execution**.

### Trusted roots close the loose-path circularity

T₂g supplies the source root before import resolution: manifest directory for a package, project-file directory for a
project, and a caller-provided virtual root/map for loose or editor input. This closes review 56's counterexample in
which resolving `../../secrets/private.musa` first allowed the later common-prefix calculation to bless the escape.
Finite module maps, duplicate-name rejection, normalized path checks, forbidden-link checks, and an acyclic package
graph are the right formation boundary. The current implementation does not yet provide all those checks.

### Build-local nominal identity is sufficient for source checking

Within one accepted build, a unique slot plus logical module and declaration path can name one declaration, and a fresh
`TypeId` makes nominal equality constant-time while preserving abstraction. Two versions of the same package are
distinct when their slots are distinct. Equal field schemas do not collapse distinct declarations. Hidden constructors
remain hidden by sealing. The defect is only the use of those build-local slots in purportedly cross-build symbolic
artifacts without a relocation rule.

### Cache-result admission is the right boundary

T₂g correctly separates cache-key completeness from value serializability. A complete key does not authorize storing a
closure, a build-local nominal value, or a value containing `TypeId`. Compiler-owned structural data may cross builds
only under its own reviewed exact codec. It also correctly refuses to derive R1 or audio/render cache safety from the
source execution key: prepared rendering still needs complete musical input, bindings, seed, options, and versions.

### Parse and interface/body separation are natural

Exact source bytes plus parser/language versions are sufficient for a pure lossless parser and its canonical structured
diagnostics. Omitting dependency-private bodies from the public interface is safe for dependent type checking in this
non-dependent source type system; execution correctly uses the stronger dependency closure. Exact encoded data rather
than a bare hash as the equality witness avoids collision-based unsoundness.

## Current implementation distinction

- Current compiler imports are lexical string resolution over a caller-supplied finite `ImportSources` map. The compiler
  performs no I/O. The project layer follows paths, but it does not yet enforce T₂g's trusted-root, symlink,
  portable-case, package-lock, or exact-tree rules.
- Current package code implements the bundled standard library's declared `lib.musa`/`mod.musa` tree. It has no resolved
  multi-package graph, simultaneous version slots, interface files, package fetcher, or lockfile reader.
- Current internal declaration handles are transient implementation identities. The public compiler comments already
  warn that such handles must not be serialized. There is no implemented user-nominal `TypeId` or cross-build nominal
  artifact to validate T₂g directly.
- Current `SourceDocument` stores source text and a display name, confirming the diagnostic-path distinction above.
- Current `WorkMeter` implements the governing fixed limits and is threaded through checking/evaluation inside a
  compilation. There is no package cache or cached logical-cost certificate.
- Current `ProjectSession` recompiles from source; it has no persistent parse, interface, checked-program, execution, or
  value cache implementing these keys.

I ran the compiler package-tree tests, all 13 import-law tests, all 15 module-law tests, the core-law and resource-
validation suites, the template-law suite, all 22 project-file laws, and all 18 project laws. All passed. They establish
the present compatibility surface; they do not test T₂g's unimplemented package graph or caches.

## Verified, judged, and not checked

### Verified

- The literal fields and proofs in T₂g, including equality at every displayed cache-key layer.
- The full T₂a–T₂f repair/review chain relevant to nominal identity, packages, active code, stored values, and the T₂f
  dependency-body counterexample.
- Governing package/module shape, exact-pinned Git design, no-registry/no-solver decision, deterministic resource meter,
  and incrementality deferral.
- Current import resolution, package-module traversal, source-document diagnostics input, transient internal identities,
  resource meter, project compilation path, and the focused tests listed above.

### Judged

- The slot-renaming and cache-warmth counterexamples.
- The relocatable-`Self` repair, dependency-alias key requirement, and evaluator-equivariance obligation.
- The sufficiency of build-local nominal atoms for the retained source-core preservation, progress, determinism, and
  normalization arguments.
- The cache-codec boundary and the surviving exact execution-closure argument.

### Not checked

- Any executable T₂g resolver, interface serializer, checked-program loader, execution graph, logical-cost certificate,
  or persistent cache, because none exists.
- A registry service or generalized dependency solver; both are currently forbidden rather than implemented.
- Remote package fetching and lockfile behavior planned in the later language prompts.
- Cultural adequacy of the W/K probes or of the proposed package calculus for non-Western musical theories.

## Required closure

1. Replace absolute build-local slots in stored checked artifacts by relocatable `Self` and imported-interface
   parameters, or define and key a stable slot law that survives private body edits and unrelated graph changes.
2. Define `CheckedKey` over exact `(alias, resolved module, InterfaceKey)` dependencies and prove Theorem 5.1 for the
   resulting relocatable artifact.
3. Specify deterministic logical cost certificates and charge them identically on cache hit and miss; state whether
   resource acceptance is per package or whole build.
4. Add nominal-renaming equivariance and an exact versioned cache-codec law to Theorem 5.2.
5. Enumerate precisely which T₂b–T₂f metatheorems survive, which nominal-identity theorems are replaced, and which
   persistent-value claims are deleted.
6. Keep the first package implementation to the governing bundled/local/exact-Git scope unless the governing package
   decision is deliberately amended.

With those changes, T₂g is a materially better basis than T₂d–T₂f: it promises only the identities Musa presently needs,
and its checking/execution split matches the actual semantic dependency boundary.
