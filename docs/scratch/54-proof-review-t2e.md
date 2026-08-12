# Proof review: T₂e source-unit ownership and stable public types

**Status: independent closure review of the frozen T₂e draft; governs nothing.** I reviewed
[53](53-t2e-source-closure.md) against the complete T₂a–T₂e chain and independent reviews [46](46-proof-review-t2a.md),
[48](48-proof-review-t2b.md), [50](50-proof-review-t2c.md), and [52](52-proof-review-t2d.md). I also checked
compatibility with governing/current package, project, import, module, and total-core behavior. I treated semantic type
safety, owner/registry formation, separate compilation, stable API identity, domain adequacy, and implementation absence
separately.

## Findings

### High

1. **Stable owner equality does not select one active public artifact.**
   - **Location:** [53 §§2.2–3 and Theorem 5.1](53-t2e-source-closure.md).
   - **Type:** missing well-formedness judgment / separate-compilation failure.
   - **Problem:** `LocalPackage(StablePackageId,CanonicalPackageName)` deliberately omits source bytes. The registry
     permits different schemas at the same owner/path to coexist because the complete schema lies in each `StampKey`.
     That is correct for stored historical values, but source name resolution still needs one current answer for
     `(OwnerKey,module path,structure path,member name)` and for every exported value path. T₂e defines no active public
     interface/artifact descriptor and no equal-owner conflict rule for current definitions.
   - **Exact counterexample:** two local directories intentionally copy the same package id/name and are both reached in
     one build. They contain

     ```text
     // artifact A
     structure M : S { data T { Item(Nat) } }

     // artifact B
     structure M : S { data T { Item(Text) } }
     ```

     Their owners and symbolic paths are equal. Their complete `StampKey`s differ, so `NominalRegistry` condition 5
     does not reject them and its coexistence paragraph expressly admits both. But a client reference to `M.T` has two
     current stamps. If one is chosen by import order, checking is non-functional and alias/order dependent; if both are
     retained as overloads, exact monomorphic lookup no longer has one result.

     Even equal nominal schemas do not close the hole. If A exports `let zero:T=Item(0)` and B exports
     `let zero:T=Item(1)`, every displayed owner/stamp condition passes while `M.zero` has two bodies. The nominal
     registry does not store or compare exported value definitions.
   - **Why it matters:** Theorem 5.1(1), (5), and (6) require functional checking, total owner formation, and
     alias-independent paths. Those claims need a unique active interface and implementation, not merely exact
     historical schema keys. Separate compilation cannot link the displayed artifacts deterministically.
   - **Suggested repair:** distinguish the historical `NominalRegistry` from an immutable `ActiveOwnerTable`. Each
     `OwnerKey` in one build must resolve to exactly one complete canonical active artifact descriptor containing its
     module map, current path→stamp map, exported value signatures, and an identity/version or exact implementation
     manifest. Equal owner keys coalesce only when this complete active descriptor is byte-identical; otherwise owner
     merge is a conflict. Historical stamps may coexist outside the active map for decoding/migration. Copying a stable
     package remains identity preserving, but two divergent copies cannot both be active without a new owner identity.

2. **`StablePublic` omits the governing `Music` type.**
   - **Location:** [53 §4 and Theorem 5.1(7)](53-t2e-source-closure.md) versus `docs/language/02-core-calculus.md` and
     `docs/language/04-templates-and-modules.md`.
   - **Type:** broken case split / source-compatibility failure.
   - **Problem:** the displayed judgment covers compiler-owned `Base b`, product, option, list, result, arrow, and user
     nominal. Governing core syntax treats `music` as a separate abstract type, not a base `b`; signatures and
     structures explicitly export `Music` values and arrows. T₂e requires `StablePublic` of **every** stable public
     signature but supplies no derivation for `Music`.
   - **Exact counterexample:** the governing module example

     ```text
     signature CanonMaterial {
       let subject : Music;
       let answer : Music → Music;
     }
     ```

     cannot pass the literal stable-public judgment in a stable local, bundled, or pinned package. `StablePublic(Music)`
     is not a rule, and the arrow premise therefore also fails. This is not hypothetical: current modules and standard
     library functions use `Music` pervasively.
   - **Why it matters:** the theorem claims compatibility with stable bundled/public signatures. Omitting the stage-
     boundary value type would make the new identity discipline reject one of the language's central existing APIs.
   - **Suggested repair:** add `StablePublic(Music)` under the governing contextual-`Music` version contract, or explain
     why public `Music` is deliberately non-persistable and split `StablePublicAPI` from canonical serialization. The
     existing design strongly supports the first: `Music` is an abstract compiler-owned, versioned core type whose
     opacity and totality already have their own theorem.

### Medium

1. **The loose-entry root and import grammar conflict with governing relative imports.**
   - **Location:** [53 §§2 and 2.1](53-t2e-source-closure.md) versus current `resolve_import` and project import
     closure.
   - **Type:** cross-document inconsistency / undefined logical-path formation.
   - **Problem:** T₂e calls the entry `$entry`, lexically normalizes relative imports from that logical path, and
     rejects imports escaping the entry root. Current Musa resolves imports from the actual document path and commonly
     permits a piece below `pieces/` to import `../library/motifs.musa` inside its project/package root. If `$entry` has
     no logical parent, the first `..` escapes and is rejected.
   - **Exact current fixture:** both `examples/album/pieces/01-opening.musa` and `02-waltz.musa` import
     `../library/motifs.musa` and `../library/patches.musa`; the governing project law requires the imports to compile.
     Calling each piece one entry unit is compatible, but `$entry` alone cannot reproduce their sibling-library address.
   - **Suggested repair:** define a synthetic relocation-invariant unit root plus an entry logical path relative to it,
     preserving the minimal common/project package root needed by all reached local imports. For this fixture the map
     could contain `$root/pieces/01-opening.musa` and `$root/library/motifs.musa`. Keep absolute paths out, reject
     escape above `$root`, and prove relocation preserves the normalized map. Alternatively key imported modules by a
     deterministic traversal identity independent of written `..`, while retaining the written source site separately.

2. **Snapshot manifests recursively embed full external owner keys and can grow superlinearly.**
   - **Location:** [53 §2.1 and Theorem 5.1(5)](53-t2e-source-closure.md).
   - **Type:** resource/finiteness dependency omitted.
   - **Problem:** every external import stores `imported_OwnerKey`; a snapshot owner key is its complete source-unit
     manifest, whose external imports store their complete dependency owner keys, recursively. The acyclic graph makes
     the mathematical tree finite, so this is not a circular definition, but a diamond dependency duplicates complete
     transitive manifests in every incoming edge. A chain/branching graph can make the canonical owner key exponentially
     larger than the source closure it describes.
   - **Why it matters:** decidable formation survives for finite input, but Theorem 5.1 assumes the deterministic
     resource contract without saying owner-key expansion is metered. Exact identity need not require materializing a
     recursively duplicated tree.
   - **Suggested repair:** define owner manifests as a canonical finite DAG/table: external imports contain a framed
     reference to an earlier owner-table index together with exact table entries, or use a collision-checked digest plus
     full descriptor lookup. Define equality over the table structure and charge its construction. This is an
     implementation-shape issue unless unbounded expanded keys are part of the persistence format.

3. **Stable local active implementation changes have no identity/version rule.**
   - **Location:** [53 §2.2–3](53-t2e-source-closure.md).
   - **Type:** incomplete persistent artifact identity.
   - **Problem:** complete nominal schemas correctly change type stamps, but a stable package can change exported
     function/value bodies without changing owner or signature/stamp. That is often the desired meaning of API
     compatibility, yet separate compilation and caches still need to know which implementation artifact is active. The
     draft's `SourceUnitManifest` solves this for snapshots; `LocalPackage` retains no corresponding implementation
     manifest.
   - **Suggested repair:** keep nominal owner/type identity stable across compatible body edits, but give the active
     artifact descriptor a separate exact implementation/content identity. Linkage and evaluation caches select it; type
     equality does not. This is part of the `ActiveOwnerTable` repair, not a reason to put all source bytes back in
     `StampKey`.

### Low

1. **Theorem 5.1 miscounts source situations.** It says "six listed source situations" but item 5 lists loose entries,
   metadata projects, identity-less packages, identity-bearing packages, bundled packages, and pinned packages—six,
   while the proof says the "three snapshot situations" share a constructor. This is internally consistent if loose and
   metadata projects are treated as different situations. The wording is harmless, but an explicit six-row
   classification table would make totality easier to verify.

2. **`source_site` needs a canonical presentation-independent formation rule.** If it is a raw byte span, whitespace
   inserted before an external import changes the snapshot owner even when exact source bytes already change it, so no
   correctness issue arises. For relocatable canonical encoding it must nevertheless exclude absolute file addresses and
   be uniquely framed. A logical module plus source byte range is sufficient.

## Verdict

- **Decision:** **Incomplete.** T₂e repairs every principal T₂d defect: positive nominal ranks restore the reducibility
  proof; per-source-unit snapshots classify identity-less packages and separate equal internal module paths; pinned
  ownership honestly uses exact package-tree semantics; alias invariance is properly narrowed; owner-qualified module
  lookup is explicit; and recursive `StablePublic` closes snapshot leaks. The remaining high defects are functional
  active-artifact selection for stable owner copies and the missing `Music` case in the public-type judgment.
- **Basis:** I replayed the full T₂a–T₂d counterexample suite, then attacked equal-owner/different-schema and equal-
  schema/different-body artifact merges, all six owner situations, cross-owner nominal DAGs, snapshot alias edits,
  relocation and `..` imports, structural and nominal persistence, exact pinned-tree equality, result framing, and the
  governing public type grammar.
- **Limits:** The T₂b semantic metatheory, T₂c bidirectional checker, T₂d exact nominal/result encodings, and
  compiler-owned base canonical-data contracts are treated as previously reviewed named dependencies. T₂e is not
  implemented, so current tests establish compatibility constraints rather than its new theorems.

## Regression audit

| Prior obligation | T₂e result |
| --- | --- |
| Leaf nominal must rank above no-nominal structures | **Closed.** Leaf rank is one and T₂b's lexicographic proof applies literally. |
| Identity-less local package needs an owner | **Closed.** It receives a per-unit `Snapshot`. |
| Two packages with equal internal paths must not collide | **Closed for unequal units.** Their exact unit manifests differ; byte-identical units intentionally share applicative ownership. |
| Snapshot leakage through a stable wrapper/signature | **Closed.** `StablePublic` recursively traverses structural types and nominal representations. |
| Snapshot alias alpha-invariance overclaim | **Closed.** The law is restricted to one fixed owner table. |
| Relocation/path formation | **Partially closed.** Absolute paths are excluded and package paths are exact; loose-entry `..` imports still need a unit-root construction. |
| Pinned commit digest used as exact equality | **Closed.** Mathematical ownership is exact canonical package-tree content; commit id is only retrieval evidence. |
| Owner-qualified module membership | **Closed.** `OwnerWellFormed` checks `(OwnerKey,LogicalModulePath)` in that owner's map. |
| Result delimiter ambiguity | **Closed by imported exact T₂d record framing.** |
| Same stable owner in multiple active artifacts | **Open.** Historical stamps coexist, but no unique active-interface/artifact map is defined. |
| Governing `Music` in stable signatures | **Open.** `StablePublic` has no `Music` rule. |

## Claim audit

### Positive ranks and the two project DAGs — Correct

The no-nominal sentinel is zero, every user nominal rank is positive, and a nominal field dependency strictly lowers the
maximum rank. Structural candidate clauses retain rank and reduce type size. The former `Words(List Text)` failure is
repaired. The separately reviewed nominal and value graphs remain finite, acyclic, dependency-first, and decidable.

### Source-unit manifest and owner-key formation — well founded, with one path gap

Internal imports refer to keys in the same finite module map, so the manifest does not contain itself. External owners
are formed in import-DAG order, so embedding their exact keys is recursively well founded. Per-unit ownership fixes the
former same-internal-path collision for distinct units. Exact source and dependency bytes give deterministic snapshot
identity; stable local ownership deliberately does not depend on source content. Pinned-tree equality states exactly the
semantic object retained by the key rather than pretending a commit digest is injective.

The `$entry` rule does not yet construct the current sibling-import fixture's logical paths. This blocks the relocation
claim for that current source shape, not the abstract possibility of a path-independent source-unit manifest.

### OwnerWellFormed and `NominalRegistry` — exact historical schemas, incomplete active linkage

Owner-qualified module lookup, unique nominal paths, resolved external owner dependencies, full stamps, and full schema
comparison eliminate the prior collision and wrong-owner lookup gaps. Digest lookup is only an accelerator. Recursive
dependency descriptors make a stored nominal value self-identifying and prevent reinterpretation under a different
snapshot.

Those facts define a historical type registry. They do not decide which of two divergent active local-package copies is
the implementation a source import denotes. A complete active artifact/interface table is a separate necessary object.

### Lemma 4.1 — Correct for the displayed type grammar; grammar is incomplete

On base, product, option, list, result, arrow, and ranked nominal types, the decision procedure terminates by the same
lexicographic rank/size measure. The nominal closure property follows because every stored field is recursively checked;
structural rules expose all children. `Stable.Box(Snapshot.Temp)`, direct snapshot arrows, and wrapped variants are all
rejected.

The proof has no case for governing `Music`. Adding a versioned compiler-owned `Music` leaf yields the intended theorem
without affecting termination or transitive snapshot closure.

### Theorem 5.1 — semantic clauses pass; functional owner/linkage closure remains open

Items 2–4 and 7 follow after adding the `Music` leaf: the finite positive-rank family and non-recursive core program
meet the reviewed semantic premises; exact canonical identity follows by DAG induction; representation opacity is
unchanged; recursive stable-public checking excludes snapshot dependencies. Item 6 is sound for packages and for a fixed
snapshot owner, subject to the loose-entry path repair.

Item 1 and the active reading of item 5 fail without unique current artifact selection for equal stable owners. The
owner key gives applicative type ownership, not an implementation revision. The two must be represented separately.

## Current implementation distinction

- Current `musa-compiler` has no `Text`, structural `Result`, nominal data, nominal DAG, exact nominal schemas/stamps,
  source-unit owner, nominal registry, active owner table, or `StablePublic` checker.
- Current `[package]` manifests do not parse `identity`; identity-less packages and bundled stdlib use the existing
  package/module-tree behavior.
- Current loose/project import closure resolves from actual document paths and supports sibling imports using `..`; it
  does not construct T₂e's relocation-independent logical module map.
- Current modules are value-only and already expose `Music`; any stable-public implementation must preserve that case.
- I ran all five package-tree unit tests, all 15 module-law tests, the focused metadata-project import law, and all ten
  project laws. They passed. These are evidence for current compatibility obligations, not T₂e's unimplemented owner
  calculus.

## Adequacy

T₂e changes identity/formation only. It adds no expressive power beyond the previously reviewed T₂b/T₂c term calculus. W
and K remain mechanism probes; no common-practice or non-Western theory algorithm is added, and no cultural adequacy
follows. None of the findings justifies aliases, recursive data, dependent types, worlds, or theta-links.

## Clean passes

- Positive nominal ranks restore the exact reviewed reducibility measure.
- Both project DAGs and dependency-ordered schema construction remain non-circular.
- Per-unit snapshots classify loose, metadata-project, and identity-less package units.
- Distinct snapshot unit manifests namespace equal internal module/structure/member paths.
- Exact snapshot manifests make snapshot-local serialization deterministic and self-identifying.
- Stable package identity has an exact finite grammar, and equal keys are not treated as probabilistic hash equality.
- Pinned ownership is exact admitted tree identity rather than commit-hash identity.
- Owner-qualified module lookup and full descriptor comparison close adversarial digest/wrong-owner cases.
- Recursive `StablePublic` rejects direct and transitive snapshot nominal leaks for every displayed case.
- Snapshot alias invariance is narrowed honestly to a fixed owner table.
- The imported exact `Text`, `Result`, nominal encoding, opacity, and bidirectional results survive unchanged.

## Required closure

1. Add an exact immutable active-artifact/interface table, distinct from the historical nominal registry, and reject
   divergent active artifacts under one stable owner key.
2. Add the governing `Music` case to `StablePublic` (or split API stability from serialization with an explicit
   alternative contract).
3. Define the loose-entry logical unit root so current `../library/...` imports remain valid and relocation invariant.
4. Store stable implementation/content identity separately from nominal owner/type identity and meter canonical owner-
   DAG formation.

These are linkage and path-formation repairs. They do not require changing the term/type calculus.
