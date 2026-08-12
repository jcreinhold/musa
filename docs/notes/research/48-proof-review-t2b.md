# Proof review: T₂b minimal source closure

**Status: independent review of the frozen T₂b draft; governs nothing.** I reviewed
[47](47-t2b-minimal-source-closure.md) against its parent calculus in [37](37-calculus-of-theory-modules.md), the T₂a
review in [46](46-proof-review-t2a.md), the total-core contracts in `docs/rules/language/02-core-calculus.md` §§5.5–5.8,
and the static module contracts in `docs/rules/language/04-templates-and-modules.md`. I checked the two §7 programs
against the forms and rules T₂b actually displays, not against the richer paper prototypes.

## Verdict

- **Decision: Incomplete.** The semantic extensions by finite `Text`, structural `Result`, and rank-ordered nominal data
  are type safe and strongly normalizing. I found no well-typed closed term that gets stuck, changes type, reduces
  nondeterministically, or diverges.
- **Why not Correct:** Lemma 3.1 proves only local structure/sibling checking, but Theorem 6.1 uses it to conclude that
  all source definitions are acyclic. Cross-structure value cycles remain unless the theorem explicitly imports the
  governing whole-project dependency check. Lemma 5.1 also relies on canonical-data contracts which T₂b never gives its
  two new data constructors. Finally, the displayed `Ok`/`Err` introduction rules leave one type component undetermined,
  while the claimed checking algorithm does not state the bidirectional rule that the examples need.
- **Programs:** after inserting the constructor-owner annotations which §7 claims are present, both W and K typecheck.
  Literally as printed, neither is a term of T₂a/T₂b's displayed `C_μ` syntax because every nominal constructor is
  written as an unqualified short tag. This is a small notation failure, not a domain counterexample.
- **Adequacy:** W and K are honest mechanism probes, not music-theory adequacy witnesses. `Text` and `Result` are
  reasonable minimal structural additions, but the probes do not exercise `text_equal` or `Result` elimination and K's
  musical representations are deliberately placeholders.
- **Severity:** no fatal semantic flaw; one high proof/integration gap; three medium exactness gaps; four low wording or
  coverage defects.

## Findings

### Fatal

**None.** The standard preservation, progress, determinism, and reducibility arguments survive the extensions under
their stated purity, finiteness, positivity, and rank restrictions.

### High

1. **Lemma 3.1 does not establish the acyclicity used by Theorem 6.1.**
   - **Location:** [47 §§3 and 6](47-t2b-minimal-source-closure.md).
   - **Problem:** the inherited T₂a structure algorithm records edges for sibling value references. Functional names
     make that local graph unambiguous, but they do not make a graph acyclic, and they do not account for qualified
     references to other ordinary structures. Consider the finite graph

     ```text
     signature S { let x : Nat; }

     structure A : S { let x : Nat = B.x; }
     structure B : S { let x : Nat = A.x; }
     ```

     Each structure has one locally unique member and no sibling cycle. Both qualified paths can be collected before
     bodies are checked, as governing core checking already does for forward references. If T₂b acceptance consists of
     Lemma 3.1's per-structure procedure, both bodies typecheck and flatten to a recursive pair with no first value.
     Thus the proof sentence "source definitions remain finite and acyclic by Lemma 3.1" is false.
   - **Qualification:** the governing core independently records every free named declaration and rejects cycles in the
     complete declaration graph. If "accepted" in Theorem 6.1 silently includes that pass, the counterexample is
     rejected and the theorem's intended conclusion is true. The defect is a missing premise and a wrong dependency in
     the proof, not evidence that recursive structures should be admitted.
   - **Repair:** state a project judgment which unions import, structure/member, generated-module, and ordinary
     free-name edges after flattening; require that finite graph to be acyclic; and replace the appeal to Lemma 3.1 by
     the governing topological-let argument. This restores [37 Corollary 5.4](37-calculus-of-theory-modules.md), which
     already had an explicit "acyclic project and sibling dependency graph" premise.

### Medium

1. **`Result` introduction is declaratively admissible but the functional checker is underspecified.**
   - **Location:** [47 §2](47-t2b-minimal-source-closure.md), Lemma 3.1, and K's `bind`.
   - **Problem:** in

     ```text
     Γ⊢e:A
     ────────────
     Γ⊢Ok(e):Result A E
     ```

     `E` is absent from the premise; dually, `A` is absent for `Err`. Consequently `Ok(0)` synthesizes no unique
     monomorphic type: for every well-formed `E`, the displayed declarative rules derive `Result Nat E`. This does not
     damage preservation or strong normalization, but exact type equality alone is not a syntax-directed synthesis
     algorithm and Lemma 3.1 says no phase searches among accepted results.
   - **Exact ambiguous source term:** an annotated outer result does not always resolve the omission:

     ```text
     let unwrap : Nat =
       match Ok(0) with { Ok(x) => x; Err(y) => 0; };
     ```

     The whole expression has type `Nat` for every choice of the hidden error type `E`; the scrutinee is in a synthesis
     position, and neither branch constrains `E`. Unlike governing `none_τ` and `[]_τ`, the syntax carries no missing
     component. A functional checker must reject this term as ambiguous or require an explicit `E`.
   - **Why K can still pass:** `bind` has the declared result `Result Phrase Error`. A bidirectional checker can check
     both match branches against that expected type, checking the `Ok` child at `Phrase` and the `Err` child at `Error`.
     The current governing checker already propagates expected types through lists, options, products, matches, and
     annotated definitions. T₂b has not stated the corresponding rules for `Result`.
   - **Repair:** make `Ok` and `Err` check-only forms,

     ```text
     Γ⊢e⇐A
     ─────────────────────────────
     Γ⊢Ok(e)⇐Result A E
     ```

     with the dual rule for `Err`, and define expected-type propagation through every annotated binding, application,
     constructor field, container, and match arm. Reject an `Ok`/`Err` in a synthesis position, or add explicit
     monomorphic type arguments. No polymorphic inference is needed.

2. **Lemma 5.1 assumes, but does not define, canonical encodings for the new constructors.**
   - **Location:** [47 §§5](47-t2b-minimal-source-closure.md).
   - **Problem:** the proof says to use each compiler-owned child's `CanonicalData` contract. T₂b defines neither a
     versioned canonical scalar-sequence encoding for `Text` nor tag/component/schema framing for `Result A E`. This is
     load-bearing: K's `SvaraIntent`, `GamakaIntent`, and `Error` contain `Text`, and the field grammar permits nominal
     members containing `Result F F`. K₃.3's generic record contract does not instantiate itself for these new types.
   - **Exact failure:** `encode_nominal(SvaraValue(name))` contains `enc(name)`, but the candidate supplies no function
     for that occurrence. Lemma 5.1 is therefore conditional rather than proved for the displayed K value.
   - **Repair:** define `CanonicalData Text` as a versioned, count-framed sequence of canonical scalar encodings (UTF-8
     is usable because scalar sequences have a unique valid UTF-8 encoding), and define `CanonicalData (Result A E)`
     with a type-schema header containing both child schemas, a distinct `Ok`/`Err` tag, and one length-framed child.
     Then prove the encoding/equality equivalence structurally.

3. **`owner_package_ref` is not total over the governing package universe as written.**
   - **Location:** [47 §5](47-t2b-minimal-source-closure.md) versus `docs/rules/language/04-templates-and-modules.md`
     and `docs/rules/language/09-assets-and-packages.md`.
   - **Problem:** T₂b says the key contains an immutable reference accepted by a "locked package registry." The
     governing language explicitly has no registry: local and bundled packages are resolved directly, and remote
     packages use exact pins in an offline lock graph. The draft neither gives a canonical reference for a local root
     package nor distinguishes bundled, local-project, and exact-pinned remote identities. Thus it has listed a key's
     fields without defining one field on every nominal declaration it claims to stamp.
   - **Repair:** replace the registry phrase by a tagged `OwnerPackageRef` grammar for all governing package classes.
     State its exact canonical bytes and conflict law. For a remote package this can include URL, hash algorithm, full
     commit object id, and verified manifest/tree identity; for bundled and local packages the design must choose and
     document an equally collision-safe project/package identity. Keep full-key confirmation after digest lookup.

### Low

1. **The two programs omit the constructor owners which their introduction says are explicit.** T₂a terms use `C_μ`; §7
   prints `WrittenValue(p)`, `LearnedGesture(name,contour)`, and so on. Bare tags are not generally functional because
   two nominal owners may both declare `Wrap`. Either print the owner on every nominal introduction and match arm or
   specify an expected/scrutinee-directed short-name elaboration. The latter is surface sugar, not the promised explicit
   core notation.

2. **The `Text` normalization sentence promises an unavailable package operation.** With only literals and `text_equal`,
   a package cannot inspect scalar values and cannot implement general Unicode normalization "before construction." It
   can accept an already-normalized literal or finitely map known literal names, but it cannot normalize arbitrary
   `Text`. Say that normalization is absent and belongs to a future separately admitted primitive, or admit one now with
   D1–D4.

3. **Lemma 4.1 overstates opacity as inability to "eliminate" a private member.** Governing flat matching permits a
   binding or wildcard over any type, so a client may bind, pass, or discard a `K.Phrase`. What it cannot do is apply a
   private **constructor eliminator**, distinguish constructors, or access fields. The proof establishes the latter,
   representation-opacity claim. Restating it that way avoids a false literal reading without changing the calculus.

4. **The probes leave two new mechanisms partly untested.** K constructs `Ok` and `Err`, but neither program matches a
   `Result`; both store `Text`, but neither forms a text literal nor calls `text_equal`. This does not invalidate their
   typing, but it makes "complete mechanism probes" too strong. Add one private or client function for exhaustive result
   elimination and one equality use before treating the spike as coverage of T₂b.

## Lemma-by-lemma audit

### Lemma 1.1: Correct for type safety, with one non-theorem prose defect

Finite Unicode-scalar sequences form an inert base domain. Exact equality is total, pure, deterministic, and bounded by
the finite input sizes; it returns `Bool` and introduces no eliminator that reveals sequence structure. Theorem 5's new
base candidate therefore applies. Literal formation must be a finite compiler judgment, as the draft says. The missing
normalizer is an API/wording issue, not a counterexample to preservation, progress, determinism, or normalization.

### Lemma 2.1: Correct metatheory; incomplete algorithmic presentation

`Result` is the ordinary strictly positive binary sum. Constructor progress, canonical forms, principal match reduction,
simultaneous substitution, left-to-right determinism, and the reducibility case all go through. The displayed candidate
should quantify over every reachable `Ok(v)` and `Err(w)`, but that is the evident reading. The missing expected-type
rule affects decidable elaboration, not the semantic proof.

### Lemma 3.1: Correct locally after a bidirectional `Result` rule; insufficient globally

The six namespace conditions repair T₂a's duplicate-private-value counterexample. All finite lookups, rank checks, exact
type comparisons, sibling-edge collection, cycle checks, and a canonically tie-broken topological sort are decidable. A
deterministic diagnostic precedence can make failure functional. The lemma neither states nor proves the whole-project
acyclicity later attributed to it.

### Lemma 4.1: Correct representation-opacity argument

Separate `Σ_public`, `Δ_data`, and `Γ_core` close the target-typing hole from review 46. Private constructor paths are
absent only from client source lookup while their signatures remain available to validate flattened owner code. A client
can transport an opaque nominal value but cannot introduce or inspect its private representation. This is the right
sealing theorem after the wording correction above.

### Lemma 5.1: Correct conditional framing argument, not discharged for T₂b

At one fixed schema, full `StampKey` comparison, unique outer framing, constructor ordinal/count, and recursively exact
field encodings give both directions of the stated equivalence. Digest collisions cannot identify values because full
bytes are confirmed. The missing `Text`/`Result` encoders and incomplete package-reference formation prevent the lemma
from applying to every value in the displayed calculus today.

### Theorem 6.1: Core theorem survives; exported-graph corollary needs the governing premise

The lexicographic `(maximum nominal rank, type size)` order is well founded. Structural type clauses recurse to strict
subtypes; nominal clauses recurse through fields to strictly lower-ranked stamps, even under product, option, list, or
result wrappers. Arrows may contain nominals outside stored fields and remain ordinary structurally smaller candidate
arguments. With no recursive data, arrow field, nominal-inspecting foreign primitive, fixed point, effect, or mutable
cell, the extended fundamental lemma proves strong normalization.

The final sentence about every exported value follows only after adding the governing finite whole-declaration graph
premise identified above. Resource rejection is external to reduction: for a graph admitted by that separate meter,
evaluation terminates and returns the unique value; strong normalization alone does not promise admission within the
budget.

## Exact typecheck of the §7 programs

The following checks assume only the mechanical repair of writing the nominal owner on each tag. No aliases, user
generics, recursive data, nested patterns, subtyping, or dependent types are used.

### W

- `WrittenValue : Pitch → W.Written` and `MotionValue : Interval → W.Motion` are well formed at their independent
  nominal ranks.
- The bodies of `written` and `motion` therefore have their declared result types.
- Matching a `W.Written` exposes exactly `p:Pitch`; matching a `W.Motion` exposes exactly `i:Interval`. Each one-arm
  match is exhaustive because its owner has one constructor, and the arm body has the declared observer result.
- The structure has exactly one declaration for every required signature member and no extra data member.

**Result:** W typechecks after owner qualification. It proves fresh-wrapper/sealing mechanics and nothing about the
musical suitability of compiler `Pitch` or `Interval`.

### K

The declarations are rank-correct in source order: `Phrase` mentions only earlier `Context`, `SvaraIntent`, and
`GamakaIntent`; the remaining fields contain only admitted base/list types.

- `context`, `svara`, and `gamaka` apply their owners' constructors to fields of exactly the declared types.
- In `bind`, matching `g:GamakaIntent` binds `name:Text` and `contour:List Ratio`. The governing list match is flat and
  exhaustive: the empty arm constructs `EmptyContour(name):Error`, while the cons arm binds `head:Ratio` and
  `tail:List Ratio` and constructs `BoundUnit(ctx,s,g):Phrase`. Checked against the annotated result, the arms are
  respectively `Err : Result Phrase Error` and `Ok : Result Phrase Error`.
- In `realize`, matching `phrase:Phrase` binds `g:GamakaIntent`; its sole constructor match binds `contour:List Ratio`,
  so `Trajectory(contour):Gesture`. Both nominal matches are exhaustive.
- In `explain`, the sole `Error` constructor binds `name:Text`, which is the declared result.

Unused `ctx`, `s`, `name`, `head`, and `tail` do not affect typing or reduction. Immutability preserves the nonempty
contour after `bind`. Because the only source operation returning `K.Phrase` is the successful arm of `bind` and its
constructor is private, every `K.Phrase` obtainable by a T₂b client has that invariant. This is a source-abstraction
statement; an eventual persistence/FFI decoder must separately validate that it cannot inject arbitrary private tags.

**Result:** K typechecks after owner qualification and explicit expected-type checking for `Result`. It is not literally
accepted by the displayed unqualified `C_μ` grammar, and it does not test result elimination.

## Are `Text` and `Result` natural or loopholes?

### `Text`

`Text` is a natural minimal carrier for open, user-authored names which are values rather than source names, paths, or
quotations. Exact scalar-sequence equality is a defensible first equality; refusing implicit Unicode normalization
avoids ambient locale and normalization-version dependence. A compiler-owned `Text` also avoids pretending that every
practice's vocabulary is a closed Musa enumeration.

It is a possible **modeling escape hatch**, but not a semantic loophole. A library can encode all distinctions as text
tags and thereby discard nominal ownership; T₂b correctly says review should reject that design. The calculus itself
does not turn text into syntax, assets, kernel terms, or ambient paths, so the tag abuse does not breach staging or
totality. Real packages will probably require a small, explicitly admitted normalization or vocabulary-validation
operation; the current calculus cannot implement one generically.

### `Result`

`Result A E` is a natural structural sum for a pure total language: it distinguishes a returned success from an owned
finite failure without exceptions or effects and retains the failure payload which `Option A` discards. It has the same
claim to compiler ownership as `Option` and `List`. Requiring a fresh nominal outcome type for every partial library
operation would add sealing boilerplate without adding representation independence.

It should remain structural and monomorphic at each use. It must not become an implicit exception channel, a hidden
effect, or a reason to admit arbitrary host errors. T₂b does not make any of those moves. The checker and canonical
encoding omissions above are repair obligations, not evidence that `Result` is the wrong construct.

## Verified, judged, and not checked

### Verified

- I read the complete frozen T₂b and T₂a documents, review 46, governing core §§5.5–5.8, and the governing module
  specification, including its whole-project dependency and package-resolution rules.
- I checked every declaration, field rank, constructor application, match coverage/binding, and branch/result type in W
  and K against the displayed abstract calculus.
- I replayed duplicate namespaces, ambiguous `Result` introduction, private constructor access, cross-structure cycles,
  import-alias identity, missing child encodings, and the combined reducibility measure.
- I inspected current `crates/musa-compiler/src/core.rs`, `module.rs`, and `package.rs`. The current checker is
  expected-type-aware, but its private `Type`, `Value`, and `Pattern` surfaces have no `Text`, structural `Result`, or
  nominal-data cases. Current modules contain value members only.
- I ran `cargo test -p musa-compiler --test module_laws -q`; all 15 governing value-only module tests passed.

### Judged

- The safety and normalization verdict is a mathematical judgment about the frozen abstract calculus.
- The conclusion that `Text` and `Result` are minimal enough for a spike is a language-design judgment, not a proof that
  they suffice for real theory libraries.
- I treated constructor-owner omission literally because §7 expressly claims exact core notation. I did not turn that
  repairable presentation error into a semantic refutation.

### Not checked

- No current Rust parser/checker/evaluator implements T₂b nominal data, abstract members, `Text`, or structural
  `Result`, so the two programs cannot yet be run through Musa.
- No canonical `Text`, `Result`, nominal-value, or complete local-package identity encoder exists to property-test.
- K has not been reviewed by a Karnatak practitioner and is explicitly not a theory of Karnatak music.
- The paper prototypes remain outside T₂b by the exclusions T₂b now states honestly.

## Recommendation

Repair T₂b rather than replace it:

1. add the explicit governing whole-project dependency-graph premise and proof;
2. specify bidirectional checking for `Result` and print owner-qualified constructors in both probes;
3. define exact `CanonicalData Text`, `CanonicalData (Result A E)`, and a package-reference grammar covering local,
   bundled, and exact-pinned remote owners;
4. extend K with one `Result` match and one `text_equal` call; and
5. only then build the isolated parser/checker spike and replace one toy operation in W and K with a real reviewed
   algorithm.

These are small closure obligations around a sound core idea. They do not justify aliases, dependency, recursive data,
worlds, theta-links, or a larger kernel. They also do not establish domain adequacy: that remains the next empirical
test, exactly as §9 proposes.
