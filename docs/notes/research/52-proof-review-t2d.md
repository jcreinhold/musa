# Proof review: T₂d exact source closure

**Status: independent final-closure review of the frozen T₂d draft; governs nothing.** I reviewed
[51](51-t2d-source-closure.md) against T₂b/T₂c and reviews [48](48-proof-review-t2b.md) and
[50](50-proof-review-t2c.md), plus the governing/current project, package, import, module, and total-core behavior. I
treated in-memory type safety, persistent nominal identity, source compatibility, adequacy, and current implementation
as different questions.

## Findings

### High

1. **Leaf nominal rank zero breaks the imported reducibility measure.**
   - **Location:** [51 §1.1, Lemma 1.1, and Theorem 7.1](51-t2d-source-closure.md) against
     [47 §6](47-t2b-minimal-source-closure.md).
   - **Type:** cross-document inconsistency / proof gap.
   - **Problem:** T₂d assigns rank zero to a nominal with no nominal field dependency. The T₂b proof it invokes defines
     `r(A)` as the greatest nominal rank in `A`, reserves zero for a type containing **no** nominal stamp, and
     explicitly says no nominal stamp has rank zero. That distinction is load-bearing in the lexicographic candidate
     order.
   - **Exact counterexample to the proof:** consider

     ```text
     data Words { WordsValue(List Text) }
     ```

     T₂d assigns `rank(Words)=0`. The nominal reducibility clause `R_Words` recursively uses
     `R_(List Text)`. Under T₂b's measure, both types have `r=0`, while `size(List Text)>size(Words)` because a nominal
     stamp is atomic. The purported recursive call therefore goes from `(0,1)` to a larger `(0,2)` pair. The displayed
     lexicographic definition is not well founded. `Option (List Text)`, a product of bases, or `Result Text Text` gives
     the same defect.
   - **Why it matters:** Theorem 7.1 says the reviewed reducibility proof applies. It literally does not under T₂d's
     rank convention. This is not a divergence example—the finite calculus is still strongly normalizing—but a
     load-bearing proof dependency has been invalidated.
   - **Suggested repair:** assign dependency-free nominals rank one, with no-nominal structural/base types at sentinel
     rank zero; then assign `1+max` to every consumer. Equivalently use sentinel `-1` for types containing no nominal
     and leaf nominal rank zero, but restate `r` and the candidate induction everywhere. The first repair exactly
     restores the already reviewed T₂b proof.

2. **An existing local `[package]` without `identity` has no owner class.**
   - **Location:** [51 §§3.1–3.2 and Lemma 4.1](51-t2d-source-closure.md) against
     `docs/rules/language/04-templates-and-modules.md`.
   - **Type:** missing case / source-compatibility failure.
   - **Problem:** governing packages already have `[package] name=...` and a declared module tree. T₂d classifies a
     loose file or metadata-only `[project]` as `Snapshot`, and classifies a local package with the new id as
     `LocalPackage`. It never classifies an ordinary local `[package]` manifest which has a name but no new `identity`
     field.
   - **Exact edge case:** this existing manifest shape is neither case:

     ```toml
     [package]
     name = "practice"
     language_version = 1

     [build]
     source = "src"
     ```

     It is not loose/`[project]`, has no `stable_package_id`, and is not bundled or pinned. The claim in Lemma 4.1 that
     root classification is total therefore does not establish compatibility with the governing local-package
     universe. Rejecting it merely because it contains a nominal would turn package identity from an opt-in for
     persistence into a new compilation requirement.
   - **Why it matters:** T₂d is presented as preserving current source behavior while adding an opt-in stable identity.
     The formation judgment has a fifth current case with no output.
   - **Suggested repair:** state that an identity-less local package receives a snapshot-scoped owner and may use
     nominal abstraction but cannot publish persistent nominal types. Define whether its snapshot is package-local or
     build-root-local; the next finding shows why that choice needs one more component. A package attempting a
     persistent export without `identity` should receive the targeted opt-in diagnostic.

3. **A closure-wide `Snapshot` does not namespace two local packages with equal internal paths.**
   - **Location:** [51 §§1 and 3.1](51-t2d-source-closure.md), plus `OwnerTableWellFormed` condition 5.
   - **Type:** incomplete owner/path formation.
   - **Problem:** `NominalPath` contains one `OwnerKey`, a package-internal module path, structure path, and member. If
     identity-less local packages in one build fall back to the one source-closure `Snapshot`, two packages with the
     same internal module/structure names have the same nominal path. Complete source bytes in the owner key do not
     help: that key is shared by the closure, and the document/package slot is absent from `NominalPath`.
   - **Exact edge case:** let local packages `left` and `right` both contain module `theory` and declaration
     `structure Practice { data T { … } }`. Then each symbolic path is

     ```text
     (Snapshot(the whole closure), theory, Practice, T).
     ```

     If condition 5 is enforced, an otherwise legal resolved closure is rejected. If the collector coalesces them,
     equal schemas collapse distinct nominal owners and unequal schemas make one path ambiguous. If the phrase
     "canonical module path" was meant to contain a closure/package qualifier, that qualifier and its alias/path laws
     are missing from the displayed path grammar.
   - **Why it matters:** this is the exact multi-package case which owner qualification exists to handle. It also blocks
     a proof of path irrelevance: using an absolute package path would distinguish the two but make relocation change
     identity.
   - **Suggested repair:** give every snapshot-owned local declaration an exact logical owner slot, for example

     ```text
     Snapshot(root_source_manifest, canonical_local_package_slot)
     ```

     where the slot is a normalized root-relative dependency/document address in the framed manifest, never an absolute
     path or source alias. Copies in different slots remain distinct; relocation of the complete root preserves slots.
     Define this before module paths are formed.

4. **The persistence rule checks the immediate owner, not snapshot dependencies.**
   - **Location:** [51 §3.1 and Theorem 7.1](51-t2d-source-closure.md).
   - **Type:** persistence loophole / missing recursive judgment.
   - **Problem:** the theorem says a persistent nominal API/value requires a non-`Snapshot` owner and rejects an export
     "from a snapshot owner." But the nominal DAG permits fields from other owners, and public value signatures can
     mention foreign nominal paths. A stable owner can therefore expose a snapshot stamp transitively.
   - **Exact counterexamples:** if `S.Temp` is snapshot-owned and `P` has a stable `LocalPackage` owner, the displayed
     rules do not reject either

     ```text
     data Box { BoxValue(S.Temp) }
     let inspect : S.Temp → Text;
     ```

     exported by `P`. The first outer type has a non-snapshot owner, but its schema embeds `StampKey(S.Temp)`; an edit
     anywhere in S's snapshot changes `P.Box`'s stamp. The second leaks the snapshot type directly without a nominal
     owner on the value signature. Structural wrappers such as `Result S.Temp P.Error` give the same escape.
   - **Why it matters:** rejecting only a nominal whose **outer** owner is `Snapshot` does not isolate snapshot identity
     from separately compiled or persistent APIs. Old values become undecodable after an unrelated snapshot edit, even
     though their advertised package owner is stable.
   - **Suggested repair:** define `PersistableType` recursively. Compiler-owned stable bases are persistable; structural
     types are persistable exactly when all children are; a nominal is persistable only when its owner is non-snapshot
     **and every nominal stamp reachable through its complete schema is persistable**. Require this judgment for every
     type in a persistent public signature and for every persisted value/cache schema. This also supplies the exact
     static diagnostic boundary.

### Medium

1. **Snapshot bytes and the blanket alias-erasure claim cannot both determine `OwnerKey`.**
   - **Location:** [51 §3.1, §4 after Lemma 4.1, and Theorem 7.1](51-t2d-source-closure.md).
   - **Type:** false nearby statement / scope ambiguity.
   - **Problem:** a `Snapshot` key contains exact source bytes. Import aliases are source bytes. Therefore changing only
     `import "lib.musa" as First` to `... as Second` changes every local snapshot owner/stamp, even after all lookups
     are resolved. The later sentence that aliases are erased **before `OwnerKey` formation** is literally false for
     this owner constructor.
   - **Qualification:** the narrower law survives: within one fixed accepted snapshot, two aliases which resolve to one
     already-owned declaration produce the same resolved nominal path and stamp. Bundled, pinned, and stable local
     owners are also invariant under an importing client's alias edit. What fails is alpha-renaming invariance of
     snapshot-owned declarations across source variants.
   - **Suggested repair:** state the theorem in that narrow fixed-owner form. Alternatively canonicalize a semantic
     source manifest which erases aliases and other presentation before forming `Snapshot`, but then it is no longer the
     exact-source snapshot currently defined and needs its own correctness theorem.

2. **Logical-name path irrelevance is asserted without a formation algorithm.**
   - **Location:** [51 §3.1](51-t2d-source-closure.md).
   - **Type:** undefined notation / proof gap for relocation stability.
   - **Problem:** "canonical logical document names" are not defined for a manifest-free root and its relative imports.
     Current import inputs and session compilation use resolved path strings, which may be absolute when a file was
     opened by an absolute path. Merely saying the snapshot excludes absolute paths does not show how
     `/work/a/main.musa` and `/tmp/a/main.musa` produce identical logical names while distinct sibling packages and
     documents remain distinct.
   - **Suggested repair:** choose a synthetic root name for the entry document, assign every imported document its
     lexically normalized root-relative resolved name, reject escape/case collisions as governing resolution already
     requires, and encode the sorted `(logical-name,source-bytes)` map. For multi-package roots include the package slot
     from High 3. Prove relocation preserves this map and that two simultaneously resolved documents never share a key.

3. **Pinned commit identity is still represented by a digest without retaining the exact commit object.**
   - **Location:** [51 §3.3–4 and Lemma 4.1](51-t2d-source-closure.md).
   - **Type:** exact-object mismatch.
   - **Problem:** T₂d correctly retains complete canonical **tree** manifests, so different trees under a colliding tree
     or commit id are rejected at merge. But `PinnedGit` still contains only the commit algorithm and object id. Two
     distinct commit objects can, in principle, have the same id and the same tree while differing in parents, author,
     or message. The retained tree manifest is then equal, so the exact check does not distinguish the commits even
     though §3.3 says commit digests are never equality substitutes.
   - **Suggested repair:** either retain and compare canonical verified commit-object bytes as part of the owner
     descriptor, or define owner semantics explicitly by canonical package tree content plus remote/subdirectory and
     remove the stronger exact-commit claim. Collision resistance may be an engineering premise, but it is not an exact
     equality proof.

4. **Owner-table module existence is not owner-qualified.**
   - **Location:** [51 §4, `OwnerTableWellFormed` condition 3](51-t2d-source-closure.md).
   - **Type:** incomplete well-formedness condition.
   - **Problem:** requiring that a logical module path exists somewhere in the resolved source closure does not prove
     that it belongs to the `OwnerKey` paired with it. A malformed separately compiled artifact can claim `(OwnerA,m)`
     when only OwnerB contains `m` and satisfy the literal global-existence condition.
   - **Suggested repair:** make every owner descriptor contain an exact finite map from its canonical logical module
     paths to source/artifact descriptors, and require lookup of `(OwnerKey,module_path)` in that map. The same map
     supplies the namespace needed by Snapshot owners.

### Low

1. **Lemma 2.1 silently fixes `nominal_format_version`.** At fixed owner/path but different nominal-format versions,
   equal schema descriptors yield unequal `StampKey`s, contradicting the literal "iff." In one accepted build the
   compiler version presumably fixes this component. Add it to the lemma's fixed parameters.

2. **The local identity grammar is deferred beyond an "exact closure" proof.** A fixed finite UUID/package-name grammar
   is plainly decidable, so this is not a mathematical obstacle, but `identity = "uuid:…"`, canonical package names,
   duplicate/conflict diagnostics, and canonical bytes should be specified before the implementation prompt rather than
   by it.

3. **Equal local identities need a conflict, not an unconditional duplicate error.** Copying a package's id/name is
   expressly an identity-preserving copy, and importing the same package twice through aliases must coalesce it.
   Therefore "duplicate identity is a diagnostic" should mean equal key with unequal complete owner descriptor, while
   byte-identical repeats coalesce. The owner-table merge text has the right rule; §3.2 should use it.

## Verdict

- **Decision:** **Incomplete.** T₂d closes the cross-structure nominal cycle, schema-version, result-framing, and
  digest-confirmation defects from review 50. Its nominal schema/stamp construction is acyclic and exact. The final
  theorem is not yet proved because the new zero-based nominal rank is incompatible with the imported reducibility
  measure, current identity-less local packages lack an owner case, snapshot ownership lacks a collision-free logical
  package slot, and persistence is not closed transitively over type schemas.
- **Basis:** I reconstructed owner-key formation before stamp formation, replayed self/mutual nominal cycles, leaf
  nominals with compound base fields, same-path schema edits, same-key artifact merges, digest collisions, alias edits,
  loose/metadata/package roots, two local packages with equal internal paths, and persistent wrappers around snapshot
  types. I also rechecked the exact `Result` grammar and the corrected W/K typing dependencies.
- **Limits:** I treated the existing compiler-owned base canonical-data contracts and the already reviewed T₂b semantic
  cases as named dependencies. No implementation exists for the new nominal calculus or owner model, so executable tests
  can verify only the governing behavior T₂d proposes to extend.

## Claim audit

### Lemma 1.1 — graph result correct; rank convention incompatible with its consumer

Both finite graph checks terminate. The type graph now includes nominal occurrences under every admitted structural
wrapper, and dependency-to-consumer orientation gives a valid topological order. Self and mutual nominal recursion are
rejected. The value graph orientation is also now exact and yields dependencies before consumers.

Induction over the type DAG proves every **nominal dependency** has a smaller natural rank. That fact is correct. What
does not follow is compatibility with T₂b's candidate order when a leaf nominal and a no-nominal structural field both
receive rank zero. Starting nominal ranks at one repairs the dependency without changing either graph.

### Lemma 2.1 — correct at fixed encoding version

The descriptor contains ordered constructor names, ordered and recursively framed field schemas, and the equality/
encoding version. Because a nominal field embeds only an already constructed dependency stamp, the descriptor/stamp
definition is well founded over the accepted DAG; there is no circular `StampKey` definition. At fixed owner, path, and
nominal-format version, unique framing gives the stated equivalence. `Item(Nat)` and `Item(Text)` necessarily receive
different stamps.

### Lemma 4.1 — collision argument passes for retained descriptors; owner totality does not

`Snapshot` compares complete manifest bytes. A stable local id is exact asserted ownership rather than a finite digest.
Bundled/pinned artifacts retain complete tree/manifest descriptors and equal-key unequal-descriptor merge is rejected.
Consequently a hash-table or tree-digest collision cannot silently merge **different retained descriptors**. The narrow
collision argument is sound.

The lemma's classification does not cover identity-less local packages, and exact commit-object identity requires the
additional bytes noted above. Snapshot owner/path formation also needs the logical package slot. Thus totality and the
strongest exact-object reading remain incomplete.

### Result encoding — Correct

`record` has a field count and length-frames every variable child. The child schema encodings are independently framed;
the two tags are fixed-width and distinct; the selected child value is length-framed. The former `("a","bc")` versus
`("ab","c")` delimiter counterexample is impossible. T₂c Lemma 3.1 now applies exactly.

### Theorem 7.1 — semantic conclusion remains plausible, closure dependencies do not all hold

After changing the rank base, accepted graphs elaborate to a finite rank-ordered nominal family and finite non-recursive
core lets. T₂b's preservation, progress, determinism, opacity, and strong-normalization arguments then apply. Exact
nominal encoding follows from exact child encodings, the acyclic descriptor construction, and full stamps.

The displayed project judgment still lacks total current-owner formation and a recursive persistence judgment. Its
blanket alias statement also needs narrowing for exact-source snapshots. These are integration and theorem-premise gaps,
not evidence for recursive data, dependent types, worlds, or a larger term language.

## Current implementation distinction

- Current `musa-compiler` has no `Text`, structural `Result`, nominal-data types, nominal DAG, stamp/schema descriptor,
  snapshot owner, stable package owner, or owner-table merge.
- Current package code reads the declared source/module tree and does not parse a package identity. Current bundled
  `stdlib/musa.toml` uses the existing `[package] name/language_version` shape with no identity; it is distinguished
  only because it is bundled.
- Current project manifests parse optional `[project]` metadata and accept loose files. They do not parse
  `stable_package_id`, and project metadata is not required for compilation.
- Current imports are keyed by resolved path strings; a future Snapshot implementation must add the canonical logical
  root/package naming that T₂d currently leaves abstract.
- I ran the five current package-tree unit tests, all 15 module-law tests, the focused project-metadata law, and all ten
  project laws; all passed. They validate current compatibility constraints, not T₂d.

## Adequacy

Nothing in this review changes the earlier domain verdict. W and K remain well-typed mechanism probes under the reviewed
bidirectional and owner-qualified rules. T₂d adds identity and project formation, not musical expressive power. It
proves no adequacy result for pitch, phrase, gesture, gamaka, or any cultural practice, and it supplies no evidence for
the still-excluded larger features.

## Clean passes

- The nominal dependency DAG detects self recursion, cross-structure cycles, and dependencies beneath all admitted
  structural wrappers.
- Schema/stamp construction is dependency-ordered rather than circular.
- Complete schema descriptors eliminate manual version-bump reliance and distinguish exact schema edits.
- Full-key confirmation prevents finite stamp-table digests from defining equality.
- The value DAG is dependency-first and closes review 48's cross-structure value cycle.
- Result schema/value encoding is unambiguously framed.
- Same-key unequal retained owner/schema descriptors are rejected at merge.
- Snapshot keys use complete source bytes rather than a content digest and can support exact snapshot-local decoding.
- The bidirectional checker boundary and owner-qualified W/K probes remain sound.

## Required repairs

1. Start user nominal ranks at one, preserving zero for no-nominal types.
2. Classify identity-less local packages as snapshot-scoped owners.
3. Add an exact logical package/document slot to snapshot ownership and prove relocation/path laws.
4. Define recursive `PersistableType` and apply it to every persistent public signature and stored value.
5. Narrow snapshot alias invariance, owner-qualify module membership, and retain exact commit bytes or weaken the commit
   identity claim.

These repairs are about formation and identity boundaries; none requires enlarging the source term calculus.
