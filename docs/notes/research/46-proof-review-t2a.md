# Proof review: T₂a nominal theory modules

**Status: independent review of the frozen T₂a draft; governs nothing.** I reviewed
[37](37-calculus-of-theory-modules.md) against the motivating candidate [31](31-candidate-theory-modules.md), the paper
prototypes [36](36-theory-module-paper-prototypes.md), the governing total core in
`docs/rules/language/02-core-calculus.md`, and the existing value-only structure/template contract in
`docs/rules/language/04-templates-and-modules.md`. I treated internal type safety, module elaboration, separate
compilation, prototype adequacy, and current implementation status as distinct questions.

## Verdict

- **Decision**: **Incomplete.**
- **What survived**: the non-recursive ranked nominal-data calculus is type safe and strongly normalizing. Abstract
  members, ordinary structure sealing, private constructors, and distinct named structure stamps are coherent. The
  checking problem is decidable once all declaration namespaces and stamp metadata are made functional.
- **Why not correct**: the seven-phase structure algorithm does not reject or define resolution for duplicate private
  value members; the elaboration proof needs a separate hidden core data-signature environment after source sealing; and
  stable stamp identity/canonical encoding are contracts rather than exact separate-compilation definitions.
- **Adequacy result**: the exact prototypes in [36](36-theory-module-paper-prototypes.md) are not T₂a programs. They
  rely on non-core `Text`, generic `Result`, manifest type aliases/reuse, private helper data outside the signature,
  and—in §3.1—the expressly deferred abstract-member structure-template calculus. Smaller ordinary-structure probes can
  be rewritten into T₂a, but that rewrite has not been shown and would not preserve the paper APIs unchanged.
- **Severity**: no fatal metatheory counterexample; one high adequacy failure; three medium specification/proof gaps;
  three low exactness issues.

## Findings

### Fatal

**None.** I found no well-typed T₂a term which gets stuck, reduces nondeterministically, changes type, or fails to
normalize under the stated rank and foreign-operation restrictions.

### High

1. **The two paper prototypes do not establish a caller for the exact T₂a language.**
   - **Location**: [37 §§1.3, 6, and 9](37-calculus-of-theory-modules.md) and all executable-looking blocks in
     [36](36-theory-module-paper-prototypes.md).
   - **Type**: encoding–intended-example mismatch / adequacy claim not discharged.
   - **Exact obstacles**:
     - `Text` is used for practice names, lineage, vocabulary, claims, evidence, gestures, transcription, and errors.
       Governing `docs/rules/language/01-surface.md` explicitly says strings are not core values, and K₃ᴱ/T₂a does not
       add a `Text` base type.
     - Every materially partial operation is written with `Result A E`. T₂a has no `Result`, user type parameters, or
       general sum. [36 §1](36-theory-module-paper-prototypes.md) acknowledges this as paper notation.
     - The implementations declare `XRep` and then write `data type X = private XRep`. T₂a has neither manifest aliases
       nor representation equality, and it rejects extra data declarations in an ascribed structure.
     - CommonPractice claims it may privately reuse `ExistingNoteName`, `ExistingPitch`, and other compiler types by
       manifest realization. T₂a only permits a fresh nominal declaration for every abstract member.
     - `BoundUnit` is an extra private data declaration, which T₂a rejects.
     - `InContext` consumes structures with abstract members, aliases parameter members into its result, and creates a
       fresh result member. [37 §6](37-calculus-of-theory-modules.md) explicitly defers exactly this template calculus.
     - Connection examples also use generic `Derived`, `Timeline` type applications, and generic results which are not
       K₃ᴱ/T₂a value types.

     Ellipses additionally mean the operation bodies do not test whether the constructor/match-only interface is
     usable for the proposed algorithms.
   - **What can be salvaged**: replace each `Result A E` with a distinct nominal outcome member declared after `A` and
     `E`; export eliminators because its constructors seal; replace each `data XRep`/alias pair by a direct `data X`;
     wrap an existing compiler base value in a fresh constructor and explicitly unwrap it inside the owner; promote or
     inline helper data; and omit `InContext`. Those are legitimate T₂a encodings. They substantially change the public
     signatures and still do not supply arbitrary textual vocabulary.
   - **Why it matters**: T₂a's core purpose is domain adequacy, not merely another safe ADT calculus. The final
     recommendation to spike “the two paper packages” cannot be executed against the frozen language as written.
   - **Repair**: publish two exact, ellipsis-free programs in the T₂a abstract syntax or intended concrete surface. One
     should use only existing base domains; the other should explicitly justify and admit a finite `Text`/symbol domain
     through the governing D1–D4 boundary if arbitrary names are truly required. Keep the deferred `InContext` example
     as a falsifier for a later template calculus, not part of T₂a's acceptance evidence.

### Medium

1. **Duplicate private value members make the structure algorithm non-functional.**
   - **Location**: [37 §§2–4.1](37-calculus-of-theory-modules.md).
   - **Type**: missing formation condition / algorithm ambiguity.
   - **Problem**: phase 1 rejects a duplicate **required** member and extra data, but extra value declarations are
     deliberately private and permitted. No phase requires all structure value names to be distinct. Consider

     ```text
     signature S {
       data type T;
       let x : T;
     }

     structure M : S {
       data T { C }
       let x : T = C;
       let helper : Nat = 0;
       let helper : Nat = 1;
     }
     ```

     Phase 4 collects both `helper` signatures. Phase 5's internal environment and sibling-reference graph no longer
     give one meaning to `helper`; phase 6 would generate the same qualified definition path twice. Choosing first or
     last silently changes checking and evaluation. Theorem 4.1 therefore does not yet describe a total deterministic
     checking algorithm for every displayed structure syntax.
   - **Repair**: require all value declaration names in one structure—required and extra—to be distinct before type
     collection. Likewise state uniqueness for structure names, signature names, type member names, and constructor
     owner-qualified paths in the enclosing static environment. Then every lookup and graph vertex is functional.

2. **Sealing needs two environments in the elaboration theorem.**
   - **Location**: [37 §§3.2 and 4.2–5.4](37-calculus-of-theory-modules.md).
   - **Type**: proof gap at the source/core boundary.
   - **Problem**: phase 7 correctly removes constructor paths from the external source environment `Σ'`. But the
     generated core definition still contains private constructor and match terms. In the minimal example

     ```text
     signature S { data type T; let x : T; }
     structure M : S { data T { C } let x : T = C; }
     ```

     `M.x=C` is typed under the internal constructor environment, while `C` is intentionally absent from `Σ'`.
     Lemma 4.3 proves the definition under the internal environment, and Corollary 5.4 later flattens it, but no target
     judgment states that the private nominal declaration/signatures remain in a core metadata environment while source
     name resolution is sealed. If “hide” erased that metadata, the final core would not typecheck; if it retains it,
     that retained object needs to be part of elaboration's result.
   - **Repair**: distinguish

     ```text
     Σ_public                         source paths clients may name
     Δ_data                           all nominal stamps and constructor signatures
     Δ_data ; Γ_core ⊢ definitions    sealed extended-core program.
     ```

     Make structure elaboration return `(Σ_public',Δ_data',definitions)` and prove constructor opacity as absence from
     `Σ_public'`, not erasure from `Δ_data'`. Then restate Lemma 4.3 and Corollary 5.4 over that target.

3. **Separate-compilation stamp identity is specified intensionally, not yet as an exact collision-safe contract.**
   - **Location**: [37 §§1.3 and 6](37-calculus-of-theory-modules.md), with governing package identity in
     `docs/rules/language/09-assets-and-packages.md` and existing generated identity in
     `docs/rules/language/04-templates-and-modules.md` §2.
   - **Type**: missing identity formation/equality definition.
   - **Problem**: `stable_stamp(μ)` is said to derive from locked package identity, qualified declaration path, and
     schema version, but none of those is fixed here as the exact `StampKey`. In particular, “qualified path” must mean
     the declaration's owner-canonical package/module/structure/member path, not a local import alias, or importing the
     same locked declaration under two aliases yields two stamps contrary to §6. “Locked package identity” must bind
     every source/schema input that can change a public data declaration. If the external stamp is a finite digest,
     equality must confirm the complete key rather than identify a collision.
   - **Repair**: define a versioned, framed

     ```text
     StampKey = (owner package content identity,
                 owner-canonical declaration path,
                 nominal-schema version)
     ```

     and make nominal equality exact `StampKey` equality. A digest may index metadata only after full-key collision
     confirmation. Export that key or an exact descriptor with separately compiled signatures. State that import aliases
     resolve to the same owner key and that changed public constructor/field schemas require a new schema version or
     package content identity.

### Low

1. **The combined reducibility order should be written explicitly.** The phrase “simultaneously by type structure and
   nominal rank” is correct but underexplained. Let `r(A)` be the maximum nominal declaration rank in `A` and use the
   lexicographic measure `(r(A),size(A))`, treating no nominal stamp as below rank zero. Structural clauses for product,
   option, list, and arrow recurse to a strict subtype: rank does not increase and size decreases. The `R_μ` clause
   recurses to field types whose nominal ranks are all strictly below `rank(μ)`, regardless of their wrapper size. This
   proves well-foundedness, including `List ν` and `Option ν` fields and arrows whose domain/result contain `μ`.

2. **Nominal canonical bytes should import K₃.3's `CanonicalData` framing.** The tuple-like equation

   ```text
   encode(stamp,version,ordinal,enc(fields)...)
   ```

   is mathematically adequate if `encode` is an injective structural encoder, but it is not an executable byte grammar.
   Stamps, constructor ordinals, field count, and every variable-size child need tags/length framing and schema identity.
   This is separate from type safety and currently unimplemented.

3. **The smart-constructor sentence uses an unavailable type.** Section 1.3 says a quotient type uses a smart
   constructor returning `Result`, although T₂a has no `Result`. Say `Option` or require a specifically declared
   monomorphic outcome type. This local prose error is one instance of the larger prototype adequacy issue, not a core
   counterexample.

## Metatheory attack

### Syntax and decidability

The abstract constructor and flat exhaustive-match syntax is sufficient for syntax-directed checking. Constructor owner
qualification and the scrutinee stamp disambiguate equal short constructor names. Monomorphic exact type equality is
decidable after abstract members are substituted by rigid stamps; no unification, subtype search, existential inference,
or runtime type equality is needed. Field rank checking, required-member matching, free-sibling graph construction,
cycle checking, and topological sorting are finite.

Thus Theorem 4.1 survives after adding the all-value-name uniqueness condition. Concrete lexer/parser productions remain
implementation work, but no additional type construct is forced by the abstract syntax.

### Preservation, progress, and determinism

**Pass.** Constructor introduction has exactly the declared field types. Principal match reduction substitutes values
for binders at those same types, so simultaneous substitution proves preservation. A constructor either advances its
leftmost non-value field or is a value. A match either advances its scrutinee or canonical forms exposes one
constructor; exact exhaustive coverage supplies one branch. Owner-qualified tags and left-to-right contexts make the
redex unique. Sealing changes source visibility only and adds no reduction.

No counterexample arises from an abstract external stamp: evaluation may return or pass a privately constructed value
without external source syntax being able to inspect it. The core metatheory retains the nominal declaration in
`Δ_data`; only client name resolution is opaque.

### Strong normalization and finite values

**Pass, with the explicit lexicographic measure above.** Constructor fields are finite strongly normalizing terms;
constructor formation adds no recursion. Nominal match reduces once after its strongly normalizing scrutinee and enters
a branch already reducible under reducible field substitutions. Candidate expansion for `R_μ` is valid: an immediate
reduct reaching a constructor supplies the same lower-rank field obligations.

Existing `Option`, `List`, product, and arrow candidates remain parametric in their element/component candidates. A
compiler-owned `list_fold` at `List μ`, for example, decreases the finite list length and applies a callback reducible
at its concrete monomorphic arrow type. It does not recurse through `μ` unless the checked callback explicitly matches
one finite value. Higher-order user functions over `μ` are ordinary STLC arrows; no arrow occurs inside a nominal field
and no nominal declaration is recursive. The foreign δ registry cannot inspect `μ`. I found no cyclic candidate
definition or non-normalizing term.

Theorem 5.3 is therefore sound for a finite rank-ordered family, and Corollary 5.4 follows once the elaboration target
retains `Δ_data` and all dependency namespaces are functional.

## Stamps, sealing, and templates

- **Distinct ordinary structures**: sound. `A.T` and `B.T` receive distinct owner keys, so matching signatures does not
  identify their carriers.
- **Repeated import of one locked declaration**: sound only when stamps use the owner-canonical key specified above, not
  the importing alias.
- **Private constructors**: sound with the two-environment elaboration. Clients lack source lookup; compiled owner code
  retains constructor metadata.
- **Runtime generativity**: correctly unclaimed. Ordinary named structures are applicative across the same locked
  declaration, not freshly generated by evaluation.
- **Value-only structure templates**: legitimately imported unchanged from the governing language.
- **Templates over abstract members**: correctly excluded. No result-type path alias, parameter-member substitution, or
  fresh result-member theorem is smuggled into T₂a. Consequently the motivating admitted-repertoire prototype is not an
  adequacy witness for T₂a.

## Verified, judged, and not checked

### Verified

- I read the frozen T₂a draft, both motivating prototype documents, and the governing core and module contracts.
- I checked every type/form used by the paper prototypes against the admitted T₂a forms and the governing base types.
- I replayed nominal mixing, hidden-constructor, lower-rank list/option fields, higher-order arrows over nominals,
  exhaustive match, forward field references, sibling cycles, duplicate private values, repeated imports, and deferred
  template-result identity.
- I ran `cargo test -p musa-compiler --test module_laws -q`; all 15 governing value-only module tests passed.

### Judged

- The safety/normalization result is a mathematical judgment about the fixed abstract calculus, not an implementation
  claim.
- I classify T₂a as **Incomplete** because its core extension is sound but its module algorithm/elaboration identity is
  not fully fixed and its motivating examples do not inhabit the claimed language.

### Not checked

- T₂a nominal data, abstract data members, stamp metadata, constructor sealing, and their parser/checker do not exist in
  current Rust. Existing tests cover only governing value-member structures/templates.
- The ellipsis bodies in the paper prototypes provide no algorithms to typecheck or evaluate, and the Karnatak-facing
  vocabulary has not had practitioner review.
- I did not test a concrete canonical nominal encoder or separately compiled metadata format because neither is
  specified or implemented.

## Recommendation

Do not reject the T₂a core and do not schedule it from the current paper examples. First close the small formal gaps:

1. require globally functional declaration namespaces, beginning with duplicate private values;
2. make elaboration return both sealed source exports and the retained private core data signature;
3. define exact owner-canonical `StampKey` identity and K₃.3-framed nominal encodings; and
4. write two exact ordinary-structure probes with no `Result`, aliases, extra data, abstract-member templates, generic
   wrappers, or ellipses.

The second probe must make an explicit decision about textual vocabulary. If it needs arbitrary `Text`, admit that
finite base domain and its operations honestly; if it does not, demonstrate the resulting finite nominal vocabulary.
Only then can measured syntax and diagnostics decide whether T₂a is the right minimal extension. The proof does not call
for dependent types, first-class modules, recursive data, or another kernel redesign.
