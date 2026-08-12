# Proof review: T₂c source closure corrigendum

**Status: independent closure review of the frozen T₂c draft; governs nothing.** I reviewed
[49](49-t2c-source-closure.md) against [47](47-t2b-minimal-source-closure.md), the regression obligations in
[48](48-proof-review-t2b.md), the governing total core in `docs/language/02-core-calculus.md`, the module/project rules
in `docs/language/04-templates-and-modules.md`, and the package closure in `docs/language/09-assets-and-packages.md`. I
treated the semantic calculus, project elaboration, persistent identity, domain adequacy, and current implementation as
separate claims.

## Findings

### High

1. **The complete value graph still does not order nominal field dependencies.**
   - **Location:** [49 §2](49-t2c-source-closure.md), especially header collection in step 2 and free-value edges in
     steps 4–5; [49 Theorem 7.1](49-t2c-source-closure.md).
   - **Type:** missing formation judgment / proof gap.
   - **Problem:** T₂c collects every nominal stamp before checking structures, but its one complete graph contains only
     free **value-definition** references. The imported T₂a metatheory requires every nominal occurring in a field to
     have strictly smaller declaration rank. T₂c does not say whether a stamp collected from another not-yet-checked
     structure counts as an "existing data type," nor does it construct or check a project-wide nominal dependency
     graph. Under the natural header-visible reading, this project passes the displayed value-graph check:

     ```text
     signature One { data type T; }

     structure A : One {
       data T { AZero; AStep(B.T) }
     }

     structure B : One {
       data T { BZero; BStep(A.T) }
     }
     ```

     Step 2 has made `A.T` and `B.T` known. There are no value definitions and hence no value-graph cycle. But field
     formation gives both `A.T < B.T` and `B.T < A.T`; the lexicographic nominal-rank interpretation imported by
     T₂b/T₂c cannot be defined. Strict positivity might support a different recursive-data proof, but T₂c expressly
     has not admitted recursive data or supplied that proof.
   - **Why it matters:** Theorem 7.1 applies T₂b Theorem 6.1 to every project accepted by the new project judgment. That
     dependency requires a finite strict rank for the whole nominal family, not merely an acyclic value graph. Header
     collection has reopened exactly the premise on which nominal reducibility and canonical field induction rely.
   - **Suggested repair:** define a second finite graph whose vertices are all nominal stamps and whose edge `μ→ν` means
     a field of `μ` contains `ν`, including through product, option, list, and result. Existing compiler-owned leaves
     add no edge. Reject cycles and assign a canonical dependency-first rank before body checking. Alternatively, state
     that structure-local checking receives only previously sealed nominal stamps and that collected future stamps are
     usable in value signatures but not fields. The latter is simpler but makes source declaration order semantically
     relevant. Do not leave "existing" to implementation interpretation.

2. **`OwnerPackageRef` is not total on the governing current project model.**
   - **Location:** [49 §4 and Lemma 4.1](49-t2c-source-closure.md) versus current project/package contracts.
   - **Type:** cross-document inconsistency / missing case.
   - **Problem:** `LocalRoot` requires a source-controlled `stable_project_id` and `canonical_package_name` in the root
     `musa.toml`. Governing Musa deliberately permits a loose `.musa` document with no manifest, and its current
     directory-project manifest is metadata which a piece does not need to compile. Existing `[project]` manifests have
     an optional display name, not a canonical package name or stable id. The bundled standard-library manifest has a
     `[package]` name, but ordinary local projects need not.
   - **Exact counterexamples:** a standalone material file can contain a structure but has no `musa.toml` from which to
     construct `LocalRoot`; `examples/album/musa.toml` has neither required field. Both are current accepted project
     shapes. Lemma 4.1's proof says closure resolution classifies the root, but the `LocalRoot` constructor is undefined
     for both.
   - **Why it matters:** silently rejecting such roots would make `musa.toml` compilation authority, reversing the
     governing "one file until it needs a project" and "metadata and nothing else" contracts. Silently deriving an id
     from path breaks path irrelevance; deriving it from changing content breaks the stated local stamp stability.
   - **Suggested repair:** decide the scope explicitly. The cleanest option is to permit exported persistent nominal
     declarations only in an identity-bearing **package** manifest, while loose pieces remain compilable consumers and
     may use only non-persistable/document-local nominal identity. Otherwise amend the governing project contract and
     give standalone files a defined canonical owner. In either case specify the exact manifest field grammar,
     diagnostics, and migration; do not describe the new required field as already present.

3. **Local stamp stability depends on an unformed `member_schema_version`.**
   - **Location:** [49 §4](49-t2c-source-closure.md), imported `StampKey`, and Theorem 7.1(5).
   - **Type:** undefined identity formation / theorem gap.
   - **Problem:** the point of `LocalRoot` is that ordinary source edits leave the owner reference unchanged. Therefore
     every public nominal schema change is distinguished only by `member_schema_version`. But the T2a/T2b/T2c
     `data T { … }` syntax and project algorithm do not say where that version comes from, how a clean checkout computes
     it, or how acceptance enforces a bump. The sentence "a public nominal schema edit changes `member_schema_version`"
     is an obligation, not an algorithm or premise.
   - **Exact counterexample:** compile these two revisions under the same local stable id, package name, module,
     structure, and member path:

     ```text
     // revision 1
     data T { Item(Nat) }

     // revision 2
     data T { Item(Text) }
     ```

     With no written or derived version rule, both may receive the same default version and hence the same `StampKey`,
     while their constructor signatures and value byte grammars disagree. Each revision can be accepted alone. A
     persisted value or separately compiled signature can then use one nominal identity for two incompatible schemas.
   - **Why it matters:** this refutes the unconditional persistent-identity part of Theorem 7.1, even though in-memory
     type safety of either individual build remains intact.
   - **Suggested repair:** define a complete `NominalSchemaDescriptor` containing ordered constructors, field type
     schemas, visibility/encoding version, and every equality-relevant choice. Either make the source carry an explicit
     monotonically managed schema version and reject reuse against stored metadata, or derive the key from the exact
     canonical descriptor. In both designs, equal `StampKey`s loaded together must be accompanied by byte-identical full
     descriptors; otherwise merge is a hard conflict. A digest may index the descriptor but cannot replace that
     comparison.

### Medium

1. **The pinned/bundled collision claim is asserted more strongly than the displayed references support.**
   - **Location:** [49 §4](49-t2c-source-closure.md), especially `PinnedGit` and the paragraph beginning "A digest may
     locate."
   - **Type:** missing hypothesis / encoding–intended-theorem mismatch.
   - **Problem:** `PinnedGit` stores a commit object id and a verified tree **digest**, not canonical commit/tree bytes.
     Suppose distinct canonical trees `m≠n` have equal values under the named digest algorithm, with all other reference
     fields equal. The displayed `OwnerPackageRef`s and therefore their `StampKey`s are equal. Verifying each tree
     against that digest separately does not distinguish them. `Bundled` has the same issue if "manifest identity" is
     only a digest. The prose promises that a collision never identifies them, while Lemma 4.1 merely cites an "exact
     conflict check" which no well-formedness judgment states.
   - **Qualification:** this is repaired if owner identity is explicitly relative to one well-formed merged owner table
     which retains canonical descriptor/manifest bytes and rejects a second equal reference unless those complete bytes
     and all nominal schema descriptors agree. That is the K₃.3 pattern, but T₂c has not copied its admission clauses.
     Alternatively the theorem can be conditional on collision resistance, but then it is not exact collision safety.
   - **Suggested repair:** define `OwnerTableWellFormed` and artifact merge. State unique keys, exact canonical
     descriptor bytes, equal-key/full-descriptor confirmation, nominal-schema confirmation, and immutable lookup during
     checking. Add that premise to Lemma 4.1 and Theorem 7.1. Keep fetched canonical tree manifests or an exact object
     representation for comparison; hashes remain lookup/verification indices.

2. **The `Result` value grammar does not unambiguously frame its child schema descriptors.**
   - **Location:** [49 §3.2 and Lemma 3.1](49-t2c-source-closure.md).
   - **Type:** undefined byte grammar / conditional proof gap.
   - **Problem:** the encoding explicitly frames the selected child value, but writes `schema(A),schema(E)` as adjacent
     components. If `frame(x,y,…)` means only "frame the whole record" as the explicit nested `frame(enc_A(a))`
     suggests, variable child schemas can admit the delimiter counterexample

     ```text
     enc_schema(A)  = "a"    enc_schema(E)  = "bc"
     enc_schema(A') = "ab"   enc_schema(E') = "c".
     ```

     The concatenated headers agree although the ordered schema pairs differ. Lemma 3.1's first decoding step then does
     not follow.
   - **Qualification:** if every schema descriptor is fixed-width or a self-delimiting `CanonicalData` value and `frame`
     is a typed record writer which length-frames each variable argument, the proof is correct. The notation needs to
     say so; prior scratch documents use explicit child framing, so this cannot safely be inferred.
   - **Suggested repair:** write `frame(enc_schema(A))` and `frame(enc_schema(E))`, or define the record-level `frame`
     grammar once as count plus a length for every variable child. Then Lemma 3.1 is the standard injectivity induction.

3. **The dependency-arrow orientation and returned order disagree in ordinary graph terminology.**
   - **Location:** [49 §2 steps 4 and 6](49-t2c-source-closure.md) and Lemma 2.1.
   - **Type:** algorithm ambiguity.
   - **Problem:** the draft records `x→y` when consumer `x` refers to dependency `y`. A conventional topological order
     places `x` before `y`, while the returned non-recursive `let` sequence needs `y` before `x`. The proof correctly
     asks for dependencies before consumers but calls this "a topological order" of the displayed orientation.
   - **Suggested repair:** orient edges `dependency→consumer`, or explicitly return the reverse topological order of
     `consumer→dependency`. Apply the owner-path tie-break at the ready-set of that specified algorithm. This is not a
     mathematical obstacle—every finite DAG has the required reverse order—but an implementation following the literal
     steps would emit ill-scoped lets.

### Low

1. **Bidirectional checking is sound but not complete for unannotated declaratively typable T₂b terms.**
   - **Location:** [49 §1 and Lemma 1.1](49-t2c-source-closure.md).
   - **Type:** unclaimed completeness / exposition issue.
   - **Exact example:** in the following term, the inner match is the outer result match's scrutinee and must
     synthesize. Its first arm is check-only, so T₂c rejects it although T₂b has a declarative typing derivation:

     ```text
     let read : Nat =
       match (match flag with {
         true  => Ok(0);
         false => Err("bad");
       }) with {
         Ok(n) => n;
         Err(message) => 0;
       };
     ```

     Adding `: Result Nat Text` around the inner match makes it check and then synthesize. This is ordinary
     bidirectional annotation discipline, not a soundness bug. Lemma 1.1 claims decidability and soundness, not
     completeness, and Theorem 7.1 needs no stronger result.
   - **Suggested repair:** explicitly state "complete up to type annotations" and prove the standard annotation
     insertion lemma if preservation of all declaratively typable programs matters. Otherwise keep the intentional
     rejection and include this case in diagnostics tests.

2. **Several identity atoms are called canonical without formation rules.** `stable_project_id`,
   `canonical_package_name`, `canonical_remote_url`, `distribution_identity`, manifest identity, algorithm names, and
   canonical subdirectories need concrete finite grammars and normalization/rejection rules before the checking and
   encoding algorithm is executable. This is downstream specification work if the owner-table and manifest decisions
   above are made, but those names alone do not prove cross-machine equality.

## Verdict

- **Decision:** **Incomplete.** T₂c repairs review 48's `Result` checking defect, value-definition cycle, opacity
  wording, missing `Text`/`Result` encoding shape, and owner qualification. Its pure semantic calculus remains safe and
  strongly normalizing. The closure theorem is not yet proved because project header collection lacks a global nominal
  rank judgment and persistent local owner identity lacks a formed/enforced schema version. Package-owner totality also
  conflicts with current manifest-free and metadata-only project shapes.
- **Basis:** I reconstructed every new lemma, replayed the review-48 counterexamples, searched for cross-structure type
  as well as value cycles, checked both directions of the new encodings, tested digest-collision and schema-edit cases,
  and hand-typechecked every corrected probe fragment against the displayed bidirectional calculus.
- **Limits:** The verdict treats existing compiler-owned canonical-data contracts as named inputs except where T₂c newly
  defines `Text`, `Result`, and owner/stamp data. Those contracts are architectural specifications, not current
  implementations. I did not assume cryptographic hashes are injective.

## Lemma and theorem audit

### Lemma 1.1 — Correct within its stated soundness scope

Synthesis has at most one result because every lookup is functional, annotations expose one exact type, applications
synthesize their domains, and a synthesizing result match commits to the first arm's unique synthesized type. Checking
is driven by one expected type. `Ok` and `Err` select the corresponding known child, so no metavariable or search is
introduced. Erasing the bidirectional mode yields the declarative introduction/match derivations; the annotation rule's
erasure is exact conversion. Coverage and duplicate-arm checks are inherited.

The proof should mention the annotation and checking-lambda cases explicitly, but they are routine. The checker is
intentionally incomplete without annotations, as the low finding records.

### Lemma 2.1 — Correct for value definitions, conditional on exact graph scope/order

Review 48's `A.x→B.x→A.x` project is now rejected. Recording every free reference after flattening covers qualified
cross-structure calls, imported value calls, private owner helpers, captured higher-order dependencies, and generated
value-instance bodies. With all ordinary and generated definitions included and the dependency-first order made
explicit, the governing topological-let proof applies unchanged.

This lemma is about definitions, not nominal type formation. It does not repair the separate rank cycle in High 1.

### Lemma 3.1 — `Text` passes; `Result` passes after exact schema framing

For `Text`, canonical scalar count and byte count delimit one valid UTF-8 byte sequence. Valid UTF-8 is injective on
Unicode scalar sequences; count validation rejects malformed, overlong, surrogate, and trailing encodings. Equal scalar
sequences deterministically re-encode to equal bytes. No normalization equality is smuggled back in.

For `Result`, distinct tags and the selected child's canonical-data equivalence give exactly the desired sum equality.
The argument is correct once the two child schema descriptors themselves have a uniquely decodable ordered encoding.

### Lemma 4.1 — Alias independence passes; totality does not

After resolution, two source aliases of one local, bundled, or pinned owner can indeed map to the same owner reference
and canonical module path. Conversely, distinct tagged constructors cannot compare equal. That proves the
alias-independence half under a functional owner table.

The totality half fails on current loose/metadata-only roots and lacks the exact well-formed owner-table/conflict
premise needed for collision claims. Those are formation problems, not problems with alias erasure itself.

### Theorem 7.1 — Semantic clauses pass; identity/project closure remains open

Items 2–4 follow once the project produces a finite well-typed non-recursive let nest: T₂b's reducibility measure,
constructor/match cases, contextual-`Music` premise, and deterministic resource boundary remain valid. Corrected
representation opacity permits transport/discard while forbidding private constructor inspection. Item 1 follows for the
displayed term checker and finite graphs once all formation judgments are fixed.

Items 5–6 require exact owner/schema admission. Alias invariance itself is sound, but exact canonical identity is not
established across local schema revisions or adversarial digest collisions. Moreover, applying T₂b's strong
normalization theorem requires the missing global nominal rank premise from High 1. The theorem therefore remains
incomplete rather than false in its semantic core.

## Literal probe typecheck

### W corrected bodies

- `WrittenValue_{W.Written}` has signature `Pitch → W.Written`; under the declared arrow, `p` checks at `Pitch`, so
  `written` checks.
- `MotionValue_{W.Motion}` analogously checks at `Interval → W.Motion`.
- A scrutinee `w:W.Written` selects exactly the qualified `WrittenValue_{W.Written}` branch and binds `p:Pitch`; the
  observer arm checks at `Pitch`. The motion observer is identical at `Interval`.
- Each one-constructor match is flat, exhaustive, and owner-unambiguous.

**Result:** the four W fragments literally typecheck.

### K corrected fragments

Systematic replacement by the six displayed owner-qualified tags makes every original K constructor introduction and
nominal match unambiguous. The `bind` body remains checked against `Result K.Phrase K.Error`, so its bare `Err` and `Ok`
arms are in checking positions and select `Error` and `Phrase` respectively.

For the additions:

- the declared type of `same_name` checks `left` and `right` at `Text`; `text_equal(left,right)` synthesizes `Bool`;
- `result` synthesizes `Result K.Phrase K.Error`, so the outer match binds `phrase:K.Phrase` and `error:K.Error`;
- the declared result propagates expected `Text` to both outer arms, so `"accepted"` checks as `Text`;
- the inner `K.Error` match selects `EmptyContour_{K.Error}`, binds `name:Text`, and its sole arm checks as `Text`; and
- both result and nominal matches are flat and exhaustive. Unused `phrase` is permitted.

**Result:** the corrected K substitutions and both added definitions literally typecheck. They now exercise result
introduction/elimination and text literal/equality as claimed.

## Semantic correctness, adequacy, and implementation

### Semantic correctness

The additions do not refute the accepted T₂b semantic result. Exact `Text`, a strictly positive structural `Result`,
ranked finite nominals, flat exhaustive matches, pure total primitives, and a finite non-recursive value program form a
sound deterministic strongly normalizing calculus. The remaining defects are integration/identity formation premises.

### Adequacy

The corrected probes remain honest mechanism probes. W proves wrapper ownership; K proves that open text names, owned
finite failure, private phrase construction, and a separate gesture result can inhabit the fragment. Neither proves a
musically adequate pitch, phrase, gamaka, context, or gesture theory. Nothing in the corrigendum justifies aliases,
dependent indices, recursive data, or abstract-member templates without a concrete failed real algorithm.

### Current implementation absence

- Current `musa-compiler` `Type`, `Value`, and `Pattern` have no `Text`, structural `Result`, or nominal-data cases.
- Current modules implement value members/templates only; there is no abstract type member, nominal constructor sealing,
  project-wide nominal rank graph, `StampKey`, or owner table.
- Current project manifests do not parse `stable_project_id`; loose files compile without any manifest, and directory
  manifests treat `[project]` as optional metadata.
- Current package code implements declared local/standard module trees, not pinned remote owner references or exact
  artifact merge.
- I ran `cargo test -p musa-compiler --test module_laws -q`; all 15 current value-module laws passed. I also ran the
  focused current project-manifest metadata law and all 10 `project_laws`; they passed. These tests verify the governing
  behavior which the proposed required local id would amend, not T₂c itself.

## Clean passes

- Review 48's cross-structure **value** cycle is closed by one flattened free-reference graph.
- `Ok`/`Err` no longer require inference of their absent parameter in the accepted fragments.
- Bidirectional checking is decidable, functional, and sound for accepted terms.
- The `Text` equality/encoding theorem is exact and does not promise unavailable normalization.
- Structural `Result` equality uses distinct tags and the correct selected child equality.
- The representation-opacity correction says exactly what sealing proves.
- Owner qualification is literal in W and systematic/unambiguous in K.
- Both corrected mechanism probes typecheck and cover the mechanisms they claim to cover.
- Import aliases are absent from the proposed owner/stamp key after canonical resolution.

## Required closure before implementation scheduling

1. Add the global nominal field-dependency/rank judgment beside the value-definition graph.
2. Decide whether persistent nominal declarations require a package identity and amend the loose-file/project-manifest
   contract explicitly; define every owner class on the resulting source universe.
3. Define and enforce full nominal schema descriptors and equal-key conflict checking, especially for stable local
   owners.
4. State `OwnerTableWellFormed` and exact merge/collision behavior for bundled and pinned artifacts.
5. Make result schema framing and dependency-order orientation executable byte/graph grammars.

After those repairs, Theorem 7.1 should close without enlarging the term language.
