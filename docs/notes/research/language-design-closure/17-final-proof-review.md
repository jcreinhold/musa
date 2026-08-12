# Final proof review: retained source translation

## Findings

### High

1. **A retained prefix call can have a different type after translation when a default precedes a later required
   parameter.**

   - **Location:** `04-source-calculus.md` §4; `04a-formal-rules.md` §§4 and 12; `09-metatheory.md` Lemma 10.1 and
     Theorem 10.3; `16-retained-translation-repair.md` §"Exact call rule"; current `core.rs` application checking and
     `apply_closure`.
   - **Type:** false statement.
   - **Problem:** the repaired rule accepts every call whose supplied slots are a parameter prefix, but says that such a
     call inserts no default after the prefix. Current Musa instead excludes defaulted parameters from the residual
     function type and evaluates their defaults immediately, even when a later required parameter remains missing.
   - **Exact counterexample:** current Musa accepts this closed program:

     ```musa
     piece "prefix default" {
         fn choose(first: Nat, middle: Bool = true, last: Nat) -> Nat { first }
         let waiting: Nat -> Nat = choose(1);
         let result: Nat = waiting(2);
         score { part p { voice v { c4/1 } } }
     }
     ```

     The current checker reports `choose(1) : Nat -> Nat`: `middle` has a default, so only `last` is missing.
     `apply_closure` evaluates `middle` to `true` and returns a closure over `last`.

     The displayed repaired translation classifies `choose(1)` as a prefix call with `k = 1` and expands it only to

     ```text
     choose_new(1)
     ```

     where `choose_new : Nat -> Bool -> Nat -> Nat`. The result therefore has type `Bool -> Nat -> Nat`, not
     `Nat -> Nat`. Nevertheless `TranslatePrefixCall` concludes the translation judgment at the type returned by
     `current_check`, namely `Nat -> Nat`.
   - **Why it matters:** this directly refutes Lemma 10.1. The term is in `Old_retained` by the stated prefix rule, so
     defining the theorem domain by derivability does not remove the counterexample. The closed-program theorem and the
     claim that every accepted prefix call has the curried type of the current residual closure are false.
   - **Smallest honest repair:** either make prefix elaboration reproduce current default consumption, or exclude a
     prefix call whenever an omitted default lies before a remaining required parameter. State the resulting additional
     source break and audit it. Merely saying that no default is inserted after the prefix cannot preserve the current
     type.

2. **The related-environment premise does not relate a closure's captured default environment to the declaration
   bindings exposed at the call site.**

   - **Location:** `04a-formal-rules.md` §12; `09-metatheory.md` definition of `rho_old ~_Gamma rho_new` and Lemma 10.2;
     `16-retained-translation-repair.md` §"Correct proof shape".
   - **Type:** false lemma / missing invariant.
   - **Problem:** a current closure evaluates a default in the lexical values captured when its declaration was
     evaluated. The translation instead copies the default expression to the call expansion and evaluates it from the
     declaration binding identities placed separately in `rho_new`. Componentwise related environments do not say that
     the value stored inside the old closure equals the separately bound value with that identity.
   - **Exact counterexample:** use the checked declaration shape

     ```musa
     let captured: Nat = 1;
     fn choose(value: Nat = captured) -> Nat { value }
     let result: Nat = choose();
     ```

     and consider the open lemma at a context containing the resolved identities of `captured` and `choose`. Let
     `rho_old(choose)` be a current closure whose default environment captured `captured = 1`, but let
     `rho_old(captured) = 2`. Let `rho_new(choose)` be the translated function `lambda(value:Nat) => value`, and let
     `rho_new(captured) = 2`.

     These environments satisfy the definition printed in §10. The two `captured` bindings are equal. The two function
     values are related at `Nat -> Nat`, because every explicit related argument is returned unchanged. Nothing in the
     function relation observes a zero-argument default call or the old closure's stored default environment.

     Old evaluation of `choose()` uses the captured default and returns `1`. The target expansion is

     ```text
     let y = captured; choose_new(y)
     ```

     and returns `2`. Thus the conclusion of Lemma 10.2 is false under its stated hypotheses.
   - **Why it matters:** the open lemma is the load-bearing repair for functions, callbacks, defaults, and captured
     values. Resolved identities prevent accidental name capture, but they do not establish this semantic coherence
     invariant. The closed theorem may still be provable for environments generated together from one declaration graph,
     but it does not follow from the false open lemma.
   - **Smallest honest repair:** strengthen the relation to environments produced by related declaration evaluations,
     with an invariant connecting every closure's captured default environment to the corresponding declaration
     bindings. Alternatively, retain translated default closures with the function value. Then restate and prove the
     open lemma under that stronger relation. No new source-language `world` construct is needed; this is a proof-level
     environment invariant.

### Medium

1. **The `Music` value relation does not define how a stored callback argument is compared.**

   - **Location:** `09-metatheory.md` §10, definition of `~_A` and the `map_note_pitches` case of Lemma 10.2.
   - **Type:** undefined relation / proof gap.
   - **Problem:** finite recipes are said to agree when a graph isomorphism preserves every constructor and "argument."
     The same paragraph says function values are compared behaviorally rather than by closure layout. A `MapPitches`
     recipe has a function as an argument. Old and refined callback closures need not be identical objects; they are
     only related at `Pitch -> Pitch`.

     If "preserves every argument" means exact equality, the `map_note_pitches` proof case is false for callbacks whose
     closure representations differ. If it means the type-indexed relation, that label relation has not been defined.
     The later sentence that the recipe stores related callbacks assumes the latter interpretation without stating it.
   - **Why it matters:** the exact `Music`/provenance relation is part of the theorem's conclusion, not an
     implementation detail. Graph isomorphism alone does not determine equality of higher-order node labels.
   - **Smallest honest repair:** define recipe-node label agreement explicitly: ordinary scalar arguments agree at their
     displayed type, the `MapPitches` callback agrees under `~_(Pitch -> Pitch)`, child recipes follow the graph
     isomorphism, and source anchors plus derivation records are equal. The private node identifier may still be renamed
     consistently.

2. **The calculus summary still states the broader compatibility obligation that the repair deliberately rejected.**

   - **Location:** `04-source-calculus.md` §13, compared with `01-current-language-audit.md` §§3 and 7,
     `04a-formal-rules.md` §12, `08-proof-outline.md` Theorem 8, `09-metatheory.md` §10, and
     `16-retained-translation-repair.md`.
   - **Type:** cross-document inconsistency.
   - **Problem:** §13 still requires that "current expressions with complete compiler calls keep their old meaning."
     That domain includes the current non-prefix ordinary call `choose(second: true)`, because it contains no compiler
     operation at all. The repaired theorem deliberately rejects that call and defines a narrower derivation-indexed
     fragment. The same checklist mentions repository rewrites for rejected compiler calls but omits rejected non-prefix
     ordinary calls.
   - **Why it matters:** this is the top-level list of claims the calculus says must be proved. It disagrees with the
     theorem that would be promoted, so a reader cannot tell which compatibility promise governs the design.
   - **Smallest honest repair:** replace those two checklist bullets with the exact `Old_retained` theorem and both
     rejected-call audits already stated in the plan and later proof files.

### Low

1. **The translation rules hide their recursive premises inside `expand_complete` and `expand_prefix`.**

   - **Location:** `04a-formal-rules.md` §12.
   - **Type:** exposition issue only.
   - **Problem:** the two displayed call rules show an expansion-function equality but not the translation premises for
     the callee declaration, supplied arguments, and used defaults. The prose supplies those premises and the acyclic
     declaration order plus strict subterm recursion gives a well-founded implementation, so I found no necessary
     circularity after reconstructing that intended definition. It is nevertheless not literally the complete
     syntax-directed rule set claimed by the heading.
   - **Suggested repair:** display the declaration-translation judgment and the argument/default translation premises,
     or define the expansion functions by lexicographic recursion on declaration rank and term size before using them.

## Verdict

- **Decision:** Incorrect.
- **Basis:** I checked the frozen commit `f023e97`, reconstructed the retained-call expansions against the current
  checker and evaluator, and attacked the repaired typing and open-result lemmas. The accepted prefix/default program
  above gives a closed counterexample to Lemma 10.1. The independently constructed related environments give a direct
  counterexample to Lemma 10.2. Either High finding is enough to stop promotion; together they show that the two main
  repairs are not yet theorems.
- **Limits:** the refined calculus is not implemented. The complete-operation comparison table and recipe-adapter
  contracts remain stated implementation obligations rather than executable evidence. I did not re-prove the accepted
  temporal-kernel and exact-anchor stage theorems, because this draft uses them without strengthening their conclusions.

## Verified, judged, and not checked

### Verified

- Commit `f023e97` was checked out on `main`. I read the repaired `01`, `03`, `04`, `04a`, `06`--`09`, `14`, and `16`
  files, the earlier review `15`, the closure plan, and the current governing source semantics and core-calculus rules.
- In `core.rs`, `ExprKind::Apply` evaluates supplied `CallArgument` expressions in written order. The checker assigns
  them to parameter slots, excludes defaulted slots from `missing`, and forms the residual function type from missing
  required slots only. `apply_closure` then visits parameter slots in declaration order, evaluates omitted defaults in
  the closure's captured environment, and retains only parameters that still lack values.
- Both displayed current programs in the High findings passed `target/debug/musa check /dev/stdin`.
- Focused current tests passed: 5 `core_laws`, 8 `higher_order_music_laws`, and 4 `finite_data_laws`. These establish
  the present behavior used by the counterexamples; they do not implement the proposed language.
- The five paper programs use complete operation calls and no defaulted or non-prefix partial call. The repaired call
  issue is therefore a metatheory failure, not a newly found type error in those programs.

### Judged

- The prefix/default program has a target type different from the current checked type under the literal expansion.
- Componentwise related environments are too weak to move a declaration-scope default out of a closure while preserving
  its value.
- The intended recursive translation is well founded once declaration dependency order is made an explicit measure; the
  presentation problem by itself is not a circularity counterexample.
- Exact recipe provenance can coexist with behavioral callback agreement, but only after the node-label relation says so
  explicitly.

### Not checked

- No refined compiler exists with which to run the paper programs or inspect generated closure and recipe values.
- I did not discharge every old/refined operation comparison or every atom/transform invariant entry by entry.
- This review does not judge the cultural adequacy of the five domain cases; the final gate in scope was the literal
  language and proof repair.

## Open questions or assumptions

- Will a partial call consume every currently available default immediately, as current Musa does, or will defaults be
  inserted only for complete direct calls? The language may choose either rule, but its retained domain and type theorem
  must choose the same one.
- Is the compatibility theorem intended only for environments generated jointly from translated declaration graphs? If
  so, that generation/coherence condition belongs in Lemma 10.2 rather than being recovered informally in the closed
  corollary.

## Clean passes

- The previous non-prefix counterexample is now honestly classified as a source break. The repaired rules no longer
  claim to translate `choose(second: true)`.
- Complete compiler operations remain coherent as non-term `op` labels. They have checking, evaluation, substitution,
  progress, termination, and denotational cases without reintroducing private partial `BuiltinValue` states.
- The value premises added to folds, `map`, `filter`, complete operations, quotation, and `map_note_pitches` still give
  unique decomposition and deterministic charging. I found no return of the earlier administrative-step overlap.
- Reordering complete named-call arguments from written order to parameter order does not refute the stated final-result
  theorem. The source fragment is pure and total, all arguments are evaluated exactly once, and the theorem explicitly
  does not compare budgets, charge traces, diagnostics, or private steps across language versions. It would be false to
  claim equal metered behavior under the same budget, but the frozen theorem does not make that claim.
- The direct-call expansion uses resolved binding identities and substitutes earlier fresh parameter values into
  defaults. This prevents caller spelling from capturing a declaration default. The remaining open-environment failure
  is semantic capture coherence, not name resolution.
- Indirect applications have one argument and no named/default processing; non-prefix partial direct calls have no rule.
  Apart from defaults that cross a retained prefix, this division is coherent.
- The termination, sealing, finite `Music` closure, and typed stage-composition arguments that survived the previous
  review were not weakened by the retained-translation repair.
