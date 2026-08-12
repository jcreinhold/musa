# Proof review: language-design closure through T₂ metatheory

**Status: independent review of the frozen research files at commit `967f632`; governs nothing.** I reviewed the source
calculus, complete rules, stage semantics, all five paper cases, proof prototype, outline, and metatheory against the
governing language, kernel, and across-stage specifications and the current compiler core. I treated trusted
compiler-operation and adapter contracts as assumptions when the theorem says that it does. I did not treat a contract
that merely repeats a conclusion as evidence that an implementation satisfies it.

## Findings

### High

1. **A well-typed annotation can get stuck, so progress and source termination are false as written.**
   - **Location:** `04a-formal-rules.md` §§8–9; `09-metatheory.md` Theorems 5.2 and 6.4.
   - **Type:** false statement / missing operational rule.
   - **Problem:** annotations are terms, and `(v:A) --> v` is a basic reduction, but the evaluation-context grammar has
     no annotation context `(E:A)`. Consequently the closed term

     ```text
     ((let x = true; x) : Bool)
     ```

     checks and is not a value. Its inner `let` is reducible, but no context permits that reduction. The whole term is
     not an annotation redex because its subject is not yet a value. It therefore cannot step. This also refutes Lemma
     5.3's claimed decomposition and the annotation case of the fundamental reducibility lemma: the annotation does not
     have the same operational behavior as its checked term.
   - **Why it matters:** required claims 4 and 6 fail for the literal calculus. Preservation and determinism alone do
     not rescue a stuck well-typed term.
   - **Suggested repair:** add `| (E:A)` to the evaluation contexts, state that annotations evaluate their subject
     before erasure, and add the annotation cases to the context-typing, unique-decomposition, progress, and
     reducibility proofs. The repair is local; the counterexample is not an argument against annotations.

2. **The claimed conservative extension is not an embedding of the governing old core.**
   - **Location:** `09-metatheory.md` Theorem 10.1; `04a-formal-rules.md` §6.8; governing
     `docs/rules/language/02-core-calculus.md` §§5.6–5.7; `crates/musa-compiler/src/core.rs`.
   - **Type:** false statement / wrong source calculus.
   - **Problem:** the proposed formal grammar and rules contain only `nat_fold`, `list_fold`, and `option_fold` as
     structural eliminators. The general primitive rule is expressly unavailable to structural folds and the bounded
     music traversal. No separate rules are supplied for the governing old operations `map`, `filter`, `range`,
     `repeat`, or `map_note_pitches`. Typed kernel quotation is also a current core form with a dedicated governing
     rule, but it has no proposed term or typing rule. Current `core.rs` confirms all seven structural eliminators,
     `MapNotePitches`, `Music`, and `KernelQuote` as real distinctions.

     For example, the accepted old expression

     ```text
     map(id, range(1))
     ```

     has no derivation in the displayed proposed rules: `map` contains an arrow argument and so cannot be an ordinary
     first-order entry in `P`, while neither `map` nor `range` has a structural rule. This is not repaired by saying the
     old entry is embedded unchanged; the target rule needed to type it is absent.

     There is a second exact mismatch. Old multi-argument beta reduction substitutes all arguments at once, while
     `04a` elaborates multi-argument functions to nested unary lambdas. Thus

     ```text
     (lambda(x, y). x)(1, 2)
     ```

     takes one simultaneous old beta step but more than one unary target step. Theorem 10.1(2), which says every old
     step is *the same* new step, is false even if the missing forms are added. The final-value claim could still hold
     by a multi-step simulation.
   - **Why it matters:** required claim 8 is about the exact accepted expression fragment, not a similar STLC subset.
     The current standard library and law suites use `map`, `range`, and `repeat`; this is not dead compatibility
     surface.
   - **Suggested repair:** define an explicit embedding for every governing old type and term form, including all seven
     eliminators, controlled music operations, and typed quotation. State and prove a forward simulation `e ->Old e'`
     implies `embed(e) -->* embed(e')`, plus preservation of final values, instead of step identity when currying
     changes the number of steps. A finite table mapping every current `ExprKind`, `Primitive` family, and music
     operation to its target rule would make the proof auditable.

3. **The full preservation, progress, determinism, and termination proofs use a music traversal that has no formal
   rule.**
   - **Location:** `04a-formal-rules.md` §§6.8 and 9; `09-metatheory.md` §§5–6.
   - **Type:** proof gap / undefined term and reduction.
   - **Problem:** `map_note_pitches` is not a term former, is forbidden from the ordinary primitive rule, and is said to
     have its “own” rule, but that static rule, its callback evaluation order, and its source small steps are never
     given. The metatheory nevertheless counts callback calls as ordinary reductions and invokes the operation in
     preservation, progress, determinism, and Lemma 6.2. The prose contract gives a finite traversal bound and is a
     credible basis for a rule, but it is not the rule the induction must inspect.
   - **Why it matters:** after repairing annotations, the four theorems would be sound for the displayed ordinary
     fragment, but they still would not cover the advertised full language or this existing higher-order operation. An
     opaque one-step host call and a sequence of visible callback steps have different evaluation contexts, meter
     events, and determinism obligations.
   - **Suggested repair:** choose one semantics. Either make the traversal one primitive delta step whose contract
     includes evaluation of the callback and a deterministic charge trace, or add a private traversal state and exact
     callback small-step contexts. Give its typing rule and prove that the callback result is `Pitch`, that each visit
     decreases the finite bound, and that no other higher-order entry can use the rule. Give equally explicit rules for
     `map` and `filter`.

4. **The sealed-constructor theorem forbids a match that the static rules accept.**
   - **Location:** `04a-formal-rules.md` §5; `09-metatheory.md` Lemma 8.1 and Theorem 8.2.
   - **Type:** false statement / statement-rule mismatch.
   - **Problem:** `_` and a bare binder are well-typed at every subject type and cover that whole type. Therefore a
     client can write

     ```text
     let ignore(value: Box.Item): Bool =
       match value {
         _ => true;
       };
     ```

     without resolving any private constructor. The formal rules accept it. Theorem 8.2 nevertheless says that no
     client-originated node can “directly match” a value of the abstract type, and its proof incorrectly says every
     direct nominal match requires the private constructor set.
   - **Why it matters:** required claim 7 is literally false, although the important representation-hiding property is
     not. The example discards an abstract value; it does not inspect its representation or forge a constructor.
   - **Suggested repair:** state the natural theorem: clients cannot name a private constructor, construct an abstract
     value with one, or use a constructor pattern to destructure it. Explicitly permit representation-independent
     wildcard and binder matches. If the stronger no-`match` syntax policy is truly intended, add it as a static
     premise; it would be an unnecessary restriction on clients.

5. **Typed path joining survives, but Theorem 12.1 adds premises and a loss algebra that the governing specification
   does not supply.**
   - **Location:** `09-metatheory.md` Theorem 12.1; `08-proof-outline.md` Theorem 10; governing
     `docs/rules/across-stages/02-derivation-diagrams.md` §4.
   - **Type:** missing hypothesis / unverified dependency / undefined operation.
   - **Problem:** the governing rule joins paths only at the exact same anchor, including its `PresentationRef`. The
     outline weakens this to the same representation type and semantic identity. Two separately stored, semantically
     equal `B` values can have anchors `b₁` and `b₂`; a path ending at `b₁` cannot be joined to one beginning at `b₂`.
     The final proof silently asserts exact anchor agreement instead of assuming it.

     The theorem also promises a “normalized combined loss record” and cites a governing loss operation and closure law.
     The governing document defines a list of losses and defines exact-anchor path concatenation, but it defines no loss
     normalization operation or closure theorem. The cited dependency does not exist there.
   - **Why it matters:** required claim 10 is established only for the origin-path component, under the stronger exact
     anchor premise. The full `PassResult` conclusion has not been proved.
   - **Suggested repair:** require that every consumed `Q` input anchor is literally the corresponding `P` output
     anchor. Then either define combined losses as ordered list concatenation, or add a versioned loss algebra with an
     explicit well-formedness and closure theorem. Keep semantic equality out of the joining premise unless a separate
     conversion step records the change of representation identity.

### Medium

1. **The `Music` closure theorem is correct only because its fourth assumption already states the successful half of the
   conclusion.**
   - **Location:** `09-metatheory.md` Theorem 11.1.
   - **Type:** theorem is correct under stated contracts / missing discharge.
   - **Problem:** totality of `instantiate` and `close` gives the two error-or-success case splits, while assumption 4
     says every successful close result is a closed, well-typed `Term<ScoreFact>`. The proof is valid, but it verifies
     no property of the proposed `Music` representation or its constructors. Finiteness is likewise an adapter
     invariant. The governing old calculus has a structural contextual-music closure proof; this research theorem does
     not transport or replace that proof for the proposed bridge operations.
   - **Why it matters:** required claim 9 survives as a conditional interface theorem, not as evidence that an adapter
     satisfies the interface. It cannot by itself justify promotion of the proposed source language.
   - **Suggested repair:** keep Theorem 11.1 as the client theorem, but add a separate adapter theorem. Define the
     private recipe invariant and prove every admitted `Music` constructor and transform preserves finite fragments,
     closed binding dependencies, payload admission, and the close precondition. The accepted kernel normalization
     theorem can then be imported honestly after closure succeeds.

2. **The paper source terms largely typecheck, but the advertised complete stage executions silently unwrap `Result`.**
   - **Location:** `06-paper-programs.md` §§3.3 and 5.1–5.3.
   - **Type:** incomplete program / displayed type mismatch.
   - **Problem:** the score program declares

     ```text
     cadence: Result<Music, PieceError>
     ```

     but the stage trace begins with `cadence Music` and passes it to `instantiate`. No displayed term or host rule
     handles `Err` and extracts the `Music` from `Ok`. The phrase case similarly defines only
     `phrase_result: Result<PhraseModel.Phrase, PhraseError>` and then describes `PhraseModel.perform` as if a private
     `Phrase` had already been extracted. These are ordinary, repairable `Result` cases, but the claim that the complete
     examples contain no omitted convenience makes them material.
   - **Why it matters:** the examples do demonstrate that the package algorithms can return typed results. They do not
     literally demonstrate the full successful source-to-stage path they claim.
   - **Suggested repair:** show the explicit source or host case split at each exported `Result`. On `Err`, stop with
     the stated stage error; on `Ok(music)`, pass exactly that value to instantiation. Do the same before calling
     `perform` or `transcribe` on an abstract phrase.

3. **The complete score trace omits inputs required by the governing performance boundary.**
   - **Location:** `06-paper-programs.md` §5.3; `05-stage-semantics.md` §3.6; governing
     `docs/rules/across-stages/01-stage-judgments.md` §4.
   - **Type:** cross-document inconsistency.
   - **Problem:** the trace writes only `interpret("measured-keyboard.v1")`. The governing operation consumes the score,
     a performance profile, and a realization seed; the research stage semantics additionally calls for an explicit
     performance context and a versioned interpretation method. The trace's seed is passed later to `prepare`, which
     does not show that the interpretation seed or performance context was supplied.
   - **Why it matters:** this is precisely the kind of hidden context the closure says it is removing.
   - **Suggested repair:** display a complete typed call such as
     `interpret(score, profile, performance_context, realization_seed)` and distinguish that seed from any preparation
     seed unless the governing operation explicitly makes them one input.

### Low

1. **The source-to-core elaboration used by the paper programs should be one explicit table.** The examples consistently
   use the explanatory spelling `fold_list(items, initial, step)`, while the complete core rules use
   `list_fold(initial, step, items)` and merely say surface argument order may differ. They also use multi-argument
   calls against a unary core grammar. The intended translations are evident and preserve typing, so this is not another
   counterexample, but a “complete paper program” should cite exact desugarings rather than ask the reader to reconcile
   two spellings.

2. **Unique-decomposition wording is slightly non-exclusive.** The context grammar contains `hole`, while Lemma 5.3 says
   a term is either a basic redex or uniquely `E[r]`. A redex is also `hole[r]`. Requiring `E` to be nonempty in the
   second alternative, or stating that every non-value uniquely decomposes as `E[r]` including the empty context,
   removes the ambiguity. This is expositional once the annotation context is added.

## Verdict

- **Decision:** **Incorrect.** The displayed calculus has a closed, well-typed stuck term, so progress and source
  termination are false. The sealing theorem is also literally false, and the old-fragment theorem is about a target
  calculus missing real old forms and uses the wrong step relation for currying. Typed path concatenation is sound, but
  the full stage theorem lacks an exact-anchor premise and cites an undefined loss operation.
- **Basis:** I reconstructed the syntax, typing, reductions, logical relation, module sealing, adapter boundary, all
  five paper programs, and all ten required claims. I compared them with the governing source core, contextual-music
  theorem, temporal kernel, exact-anchor derivation law, current `core.rs` forms, standard-library uses, and focused law
  suites.
- **Limits:** the proposed `Text`, `Result`, nominal data, abstract type members, and private constructors are not
  implemented, so there is no parser or evaluator with which to run the paper programs. Compiler-operation totality and
  the proposed notation adapter were reviewed as mathematical contracts, not independently verified implementations.

## Disposition of the ten required claims

| Claim | Review result |
| --- | --- |
| Decidable resolution and checking | **Survives for the displayed finite calculus.** The resolved graph is an input; every lookup, graph traversal, equality, pattern check, and signature comparison is finite and syntax-directed. The missing special operation rules must be added before this covers the intended full language. |
| Substitution | **Survives.** Capture avoidance, flat-pattern binders, and unchanged nominal tables give the standard induction. No dependent type or effect invalidates it. |
| Preservation | **Survives for every reduction rule actually displayed.** The full-language claim remains incomplete until the structural operations and music traversal have exact rules. |
| Progress | **False.** A non-value annotation subject is stuck because `(E:A)` is missing. |
| Determinism | **Survives for the displayed steps.** Left-to-right contexts, first-arm matching, and functional primitive contracts select at most one result. It must be rechecked for the missing traversal semantics. |
| Source termination | **False as written.** The stuck annotation does not reach a value. After that local repair, the nominal-rank/type-size logical relation is well founded; the function clause refers only to smaller component types and is not circular. |
| Sealed-constructor unforgeability | **The representation-hiding core survives, but the theorem is false as stated.** Private constructor construction and constructor-pattern inspection are excluded; wildcard matching is accepted. |
| Conservative extension of the old expression fragment | **False/incomplete.** Required old operations and quotation are absent, and unary currying gives a simulation rather than identical steps. |
| `Music` closure | **Correct conditional on a contract that contains the success conclusion.** The adapter invariant itself is unproved for the proposed language. |
| Typed stage composition | **Correct only for exact-anchor path concatenation.** The weaker semantic-identity wording and normalized-loss conclusion are unsupported. |

## Clean passes

- The nominal dependency scan looks through products, `Option`, `List`, and `Result`, so self-reference hidden under a
  container is rejected. The rank-one leaf convention makes nominal field types decrease the lexicographic measure.
- The logical relation is not circular. For `A -> B`, both `A` and `B` are smaller written types; for a nominal, every
  field has lower maximum nominal rank. The lambda, application, list-fold, and nominal-constructor cases follow after
  the operational gaps above are repaired.
- Flat patterns give a finite exact coverage algorithm. Source-order arm selection makes overlapping literal/catch-all
  arms deterministic. The paper programs use only flat patterns and cover their transparent data constructors.
- The first-order compiler-operation restriction blocks the arbitrary higher-order foreign-loop counterexample. The
  special music traversal contract is narrow enough in principle; it needs formal rules, not a wider registry.
- The five package examples use no recursive data, recursive value definition, hidden global context, dependent index,
  source effect, or infinite source object. Their data declarations are acyclic, their `Ok`/`Err` forms receive expected
  types, and their fold accumulators typecheck under the intended surface-to-core elaboration.
- The tonal example correctly keeps scale degree and harmonic function distinct. The phrase and ensemble examples do not
  force their values through Western staff notation before performance; their optional transcription losses are
  explicit.
- Theorem 11.1 is a valid two-case argument under its four adapter assumptions, and the accepted temporal kernel does
  normalize any successful closed, well-typed term to a finite timeline.
- Governing derivation paths compose associatively by exact-anchor list concatenation and retain intermediate anchors,
  generated roots, and sites. No `World` or `Link` source term is needed for that fact.

## Verified, judged, and not checked

### Verified

- Every definition, lemma, theorem, and displayed proof in research files 04, 04a, and 05–09.
- Every data dependency, match coverage obligation, `Result` expectation, fold accumulator, and public/private boundary
  in the five paper programs, after the surface desugarings those files state or clearly imply.
- The governing seven eliminators, contextual-music boundary, kernel closure/normalization theorem, and exact-anchor
  path-joining rule.
- Current compiler forms for all seven eliminators, music operations, and kernel quotation, plus current
  standard-library and test uses of `map`, `range`, and `repeat`.
- Focused current suites: five core laws, four finite-data laws, fifteen module laws, and fifteen kernel-term laws all
  passed. They verify the current implementation, not the unimplemented proposal.

### Judged

- The exact stuck-annotation, wildcard-abstract-match, currying-simulation, old-form, result-unwrapping, and anchor
  counterexamples.
- Well-foundedness of the repaired logical relation and validity of the standard substitution, canonical-forms,
  preservation, and determinism arguments for the fully specified fragment.
- The minimum repairs needed to turn the conditional `Music` and stage-path results into useful closure theorems.

### Not checked

- An executable implementation of the proposed data, `Text`, `Result`, sealing, or bridge semantics, because none
  exists.
- Totality of every future compiler operation over all well-typed inputs, or the proposed adapter invariant over every
  `Music` recipe.
- Musical adequacy beyond the five selected cases. Type safety cannot establish that the theory packages are sufficient
  for any practice.

## Required closure before a second proof review

1. Repair annotation evaluation and rerun the safety and reducibility proofs.
2. Freeze exact rules for all seven structural operations and `map_note_pitches`, including their meter-visible
   evaluation order.
3. Replace old-step identity by a complete old-form embedding and multi-step simulation.
4. Narrow unforgeability to constructor construction and representation inspection, or change the wildcard rule.
5. Discharge the `Music` adapter invariant separately from the client-facing error-or-typed-term theorem.
6. Require exact intermediate anchors and define the loss-combination operation used by stage composition.
7. Add explicit `Result` case splits and complete performance inputs to the paper stage traces.

These repairs do not call for a more elaborate kernel language. They make the selected small source language say exactly
what its proofs already almost establish.
