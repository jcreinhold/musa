# Proof review: T₂f active packages

**Status: independent review of the frozen T₂f draft; governs nothing.** I reviewed [55](55-t2f-active-packages.md)
against T₂b–T₂e and reviews [46](46-proof-review-t2a.md), [48](48-proof-review-t2b.md), [50](50-proof-review-t2c.md),
[52](52-proof-review-t2d.md), and [54](54-proof-review-t2e.md). I also compared its package, import, module, and
source-core claims with the governing language documents and current compiler/project code. I treated type identity,
active code selection, checking caches, execution caches, stored values, and current implementation as different
questions.

## Findings

### High

1. **`ImplementationKey` does not identify the implementation closure used by a build.**
   - **Location:** [55 §1, especially lines 21–33 and 69–70](55-t2f-active-packages.md).
   - **Type:** false statement / unsound cache key.
   - **Problem:** the key contains the current package's exact source but records an external dependency only by its
     `OwnerKey`. A stable local dependency deliberately keeps the same owner across a compatible body edit. Its public
     interface can also remain unchanged. The dependent package's displayed `ImplementationKey` then remains exactly
     equal although the code executed by the build has changed. This refutes both “which code did this build use?” and
     “a dependency edit changes it.”
   - **Exact counterexample:** let stable package `D` have owner `O_D` and public interface `tone : Nat`. Build zero
     contains

     ```text
     let tone : Nat = 0
     ```

     while build one changes only that body to `1`. Let stable package `P` contain unchanged source

     ```text
     let answer : Nat = D.tone
     ```

     `D`'s own implementation key changes, but its owner and public interface do not. Every displayed component of
     `ImplementationKey(P)` is equal in the two builds: compiler versions, `P`'s source-unit manifest, the external
     owner list containing `O_D`, and `P`'s public interface. An execution-result cache for `P.answer` under that key may
     therefore return `0` in build one, whose fresh evaluation returns `1`.
   - **Qualification:** the immutable `ActiveOwnerTable` still makes a fresh build select the new body of `D`. The
     counterexample does **not** refute one-build active selection or type safety. It refutes the stronger cache and
     exact-code claim assigned to `ImplementationKey`.
   - **A second uncovered input:** the displayed key does not unconditionally contain its own `OwnerKey`. If “exact
     public interface” is the ordinary member/type descriptor rather than an owner-qualified artifact descriptor, two
     `PinnedTree` owners with different remotes but the same exact admitted tree, package metadata, dependencies, and a
     base-only public signature get one implementation key. Private nominal stamps in their checked bodies nevertheless
     have different owners. T₂f never defines the public-interface descriptor strongly enough to make own-owner coverage
     an imported fact.
   - **Why it matters:** type-checking a dependent package may safely reuse work after a dependency body edit when the
     imported interface is unchanged. Linking, evaluating an exported value, or caching its result may not. One key
     cannot honestly serve both purposes with the displayed fields.
   - **Suggested repair:** define two keys in dependency-first order:

     ```text
     CheckedArtifactKey(P) = encode {
       OwnerKey(P), compiler versions, exact source,
       exact imported active interfaces, exact public interface
     }

     ExecutionClosureKey(P) = encode {
       CheckedArtifactKey(P),
       ordered (resolved dependency edge, ExecutionClosureKey(dependency)) list
     }
     ```

     The first permits the intended interface-preserving checking reuse. The second is a rooted DAG key for linkage,
     evaluated exports, and execution results. Exact comparison, not a bare digest, remains authoritative. A global
     whole-build execution key formed from the exact active implementation graph is an equivalent repair.

### Medium

1. **The loose-entry construction infers a root after resolution, so it cannot perform the imported root-escape check.**
   - **Location:** [55 §3](55-t2f-active-packages.md), importing T₂e §2.1's escape and symlink conditions.
   - **Type:** circular formation rule / cross-document consistency gap.
   - **Problem:** step 1 says to resolve using the governing path checks, including escape and symlink rejection. Step 2
     only then chooses the smallest containing directory. Root escape cannot be decided before a trusted root is known;
     if the post-resolution common directory is treated as the root, every reached path is inside it by construction.
   - **Exact hostile path:** `/work/pieces/main.musa` imports `../../secrets/private.musa`. If the latter is a readable
     Musa library, the common directory may become `/`; the generated logical names simply include both paths. No path
     escapes that inferred root, although it escaped the intended `/work` tree. The same circularity affects deciding
     whether a symlink target is external.
   - **Current compatibility:** current `resolve_import` is deliberately a pure lexical string function. The project
     layer reads the resulting path, but does not canonicalize it or enforce a supplied package/project root, case-fold
     policy, or symlink boundary. Thus “governing path checks” is not an implemented dependency that fills this hole.
   - **What survives:** once a finite set of admitted paths has already been fixed, taking their lexical common prefix
     does solve the `pieces/main.musa` versus `library/forms.musa` naming problem, and uniform relocation preserves the
     relative names.
   - **Suggested repair:** make a trusted source root an explicit resolver input. A declared package uses its package
     root; a metadata project uses the manifest directory; an in-memory/loose compilation receives a virtual logical
     root and finite source map from its caller. Resolve and reject escapes against that root first. The
     common-directory calculation may then remove a redundant prefix for identity, but it must not define the security
     boundary. If truly rootless loose files intentionally allow arbitrary parents, say so and drop the escape claim for
     that case.

2. **The graph encoding closes per-root exponential duplication but does not define build-wide sharing.**
   - **Location:** [55 §4](55-t2f-active-packages.md).
   - **Type:** resource overclaim / underspecified canonical representation.
   - **Problem:** there are two possible readings, and neither establishes every sentence as written.

     1. If each `Snapshot` key contains its own complete root-reachable node table, a dependency chain of `n` snapshot
        owners stores tables of sizes `1,2,…,n`. Every individual key is linear in its reachable graph and diamonds are
        shared within it, but the active build stores Θ(`n²`) node/source material for a graph of size Θ(`n`).
     2. If all roots include one build-wide table once, storage is linear, but putting that complete table into owner
        equality makes an unrelated added unit change an existing source unit's owner. That regresses from T₂e's
        per-source-unit ownership to T₂d's rejected whole-world identity.

   - **Further exactness gap:** T₂e's imported manifest grammar contains `imported_OwnerKey`. T₂f says a node refers to
     dependency indices but never gives the replacement edge grammar. On a literal import, “local source-unit fields”
     still contains the recursively embedded owner; on the intended reading, the exact fields preserving source site,
     imported logical module, edge order, and multiplicity are implicit.
   - **Why it matters:** Theorem 5.1 needs only finiteness, which survives. The stronger linear-storage and exact graph-
     equality claims have not yet selected one mathematical object and one physical representation.
   - **Suggested repair:** define one shared immutable owner-graph artifact whose node grammar replaces
     `imported_OwnerKey` by a framed earlier-node reference while retaining all other import-edge fields. Define a
     snapshot owner by the **root-reachable structural DAG**, independent of unreachable table entries. The artifact may
     intern nodes or use collision-checked digests for lookup, but equality must dereference and compare complete rooted
     descriptors. State separately that the canonical expanded encoding of one root is linear in its reachable DAG and
     that the shared artifact stores each build node once.

### Low

1. **Lemma 1.1 says every package “member” has a checked body.** A nominal type member has one current stamp and schema,
   not a value body. The proof is correct for resolved value members; for all public members, the conclusion should be a
   disjoint case: one current type/schema for a type member, or one checked type and body for a value member.

2. **“Finite canonical-data base type” is ambiguous.** `Nat` is not a finite type, but each admitted natural value has a
   finite canonical encoding. The `StorableValue` rule should say “compiler-owned base type with an exact finite
   encoding for every closed value.” This is wording, not a counterexample to Lemma 2.1.

3. **The relocation theorem needs the exact relocation relation.** The common-directory proof establishes invariance
   under one uniform prefix replacement preserving all path segments, source bytes, and resolved edges. It does not
   establish invariance under arbitrary file moves. That narrower relation is the evident intent and should be stated as
   a hypothesis.

## Verdict

- **Decision:** **Incorrect as written.** The central `ImplementationKey` statement and its linkage/execution-cache
  conclusion are false under an ordinary compatible body edit to a stable dependency. The repair is small and does not
  require changing nominal ownership, the term language, or active one-build selection.
- **Theorem 5.1 specifically:** items 2–7 survive the attack under the imported reviewed assumptions. Item 8 is correct
  for an already admitted finite path graph and uniform prefix relocation, but T₂f does not yet define the trusted root
  needed by its acceptance/path checks. Item 1 retains the previously reviewed source-core result once that formation
  input is made explicit. The theorem does not itself state cache coherence, so its semantic core is stronger than the
  document's overall verdict.
- **Basis:** I replayed the prior same-owner/different-schema and same-schema/different-body counterexamples, then
  tested body-only dependency edits across builds, different owners with equal source trees, direct and transitive
  snapshot leakage, functions and `Music` under both new judgments, parent imports, path escape, symlink-boundary
  formation, diamonds, chains, duplicate DAG nodes, nominal ranks, and the imported source-core proof dependencies.
- **Limits:** T₂f is not implemented. I treated the T₂b/T₂c operational metatheory and exact `Text`/`Result`/nominal
  encoders as previously reviewed dependencies. I did not test a remote package fetcher because prompt 162 remains
  pending and no such current implementation exists.

## Claim audit

### Active owner selection and Lemma 1.1 — Correct within one accepted build

`ActiveOwnerTable` is the right object missing from T₂e. It is finite and immutable; one owner has one active entry;
equal-owner unequal implementations conflict; active nominal paths have one current stamp; and all checked source names
are constrained to that implementation. Combined with T₂b's duplicate/path rejection, a resolved value path has one
signature and checked body. The historical registry can retain old schemas without participating in current source
resolution. The old equal-schema/different-body counterexample is closed.

This result should not be weakened to make the cache key pass. Active selection and historical decoding really are
different jobs. The missing third job is execution-closure identity.

### `StableInterface` and `StorableValue` — Correct

The split is both minimal and semantically necessary.

- `StableInterface` now includes the governing `Music` leaf and arrows, so current signatures such as `subject : Music`
  and `answer : Music -> Music` are admitted.
- Its nominal case recursively follows the positive-rank schema, so direct, structurally wrapped, and transitively
  hidden snapshot stamps are rejected.
- `StorableValue` excludes functions and presently excludes `Music`. It admits only compiler-owned canonical bases,
  canonical structural children, and exact-schema nominals whose fields recursively pass.
- Snapshot nominals can therefore be exact snapshot-scoped cache values without becoming stable public types.
- Both decisions terminate by the stated rank/size induction. The source-core reducibility proof is unchanged because
  neither judgment adds a term, type former, value, or reduction rule.

No counterexample survived for function closure storage, `Music` storage, a snapshot hidden under `Result`/`List`, or a
non-snapshot nominal whose field transitively reaches a snapshot.

### Snapshot DAG equality — Correct per rooted table, storage scope incomplete

For one explicitly framed root-reachable table, dependency-first indices and deterministic ready-node ordering give a
canonical finite representation. Exact duplicate nodes may coalesce because T₂e deliberately chose applicative equality
for byte-identical snapshot units with equal dependencies. Edge lists preserve repeated imports if multiplicity is part
of the eventual exact edge grammar. Full table and root comparison is exact; a digest need only locate candidates.

The unresolved question is where that table lives when every node is itself an owner. A per-root table fixes an
exponential diamond inside one key but repeats prefixes across root keys; a whole-build table threatens per-unit
identity. The shared artifact/rooted equality distinction above closes both without changing the mathematical owner.

### Source-core safety — Survives

The repaired positive nominal ranks make every nominal representation recurse to a lower rank. Structural recursion at
fixed rank decreases type size. The two finite project DAGs still yield dependency-first nominal descriptors and
non-recursive value lets. `StableInterface`, `StorableValue`, active lookup, and graph serialization are static artifact
checks; none changes call-by-value reduction. Preservation, progress, determinism, representation opacity, and strong
normalization therefore continue to follow from the reviewed T₂b–T₂e premises.

## Current implementation distinction

- Current local imports are resolved lexically by `resolve_import`; `.` and `..` are handled without filesystem
  canonicalization. The compiler receives a finite `ImportSources` map and performs no I/O.
- The project layer reads the transitive paths but currently supplies no trusted logical root, symlink-escape check, or
  case-collision policy to the compiler.
- Current package code implements the declared `lib.musa`/`mod.musa` tree for the bundled standard library. Stable local
  package identity, pinned remote packages, owner/stamp registries, and active implementation tables are not present.
- Current signatures and structures already expose `Music` and `Music -> Music`, confirming the compatibility case that
  T₂f repairs.
- Current code has no user nominal data, `Text`, structural `Result`, `StableInterface`, or `StorableValue` judgment.
- I ran the five package-tree tests, all 13 import-law tests, all 15 module-law tests, all 18 project-file laws, and all
  ten project laws. They passed. These tests establish the current compatibility surface, not T₂f's unimplemented
  theorems.

## Verified, judged, and not checked

### Verified

- The literal T₂f definitions and proof against every imported T₂b–T₂e repair and independent review.
- Governing source-core `Music`, finite-data, primitive, strong-normalization, module, package-tree, and import rules.
- Current lexical import resolution, project import closure, declared package-module traversal, and structure behavior.
- The focused current tests listed above.

### Judged

- The dependency-body counterexample to exact implementation/cache identity.
- Termination and transitive-closure arguments for the two new type judgments.
- Canonical rooted-DAG equality, its resource scope, and the source-core proof transport.

### Not checked

- Any executable T₂f checker, serializer, owner table, or cache, because none exists.
- Remote fetch/lock/cache behavior planned by prompt 162.
- Cultural adequacy of the W/K mechanism probes; T₂f adds no new musical evidence.

## Required closure

1. Split interface-checking identity from dependency-closed execution identity, and include the implementation's own
   owner explicitly.
2. Give loose/project resolution a trusted logical-root input before applying escape, symlink, and case checks.
3. Define the exact snapshot edge grammar and distinguish rooted structural owner equality from one shared physical
   owner-graph artifact.
4. Narrow Lemma 1.1's member conclusion and state the uniform-prefix relocation relation.

None of these repairs calls for dependent types, recursive data, first-class modules, worlds, or a larger kernel.
