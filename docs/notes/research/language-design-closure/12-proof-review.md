# Second proof review: repaired language-design closure

**Verdict: Incorrect.** This is the second and final independent review of the frozen draft at commit `a5d8759`. The
repair closes most findings from the first review, but it does not establish all ten required claims. One existing,
governing source program has no embedding in the proposed calculus, so the conservative-extension theorem is false. The
displayed `map` and `filter` reductions also omit value premises and therefore do not define the deterministic relation
used by the safety and termination proofs.

The first failure is a compatibility defect, not an argument against the proposed nominal data, sealing, termination
discipline, `Music` recipe boundary, or stage model. The natural repair is small in concept: preserve the current closed
controlled operations as function values, or give a complete eta-expanding translation for every partial application.
The frozen proof does neither. Under the closure plan, a High or Medium problem keeps the language in research.

## Findings

### High

1. **The conservative-extension theorem still excludes current partial controlled operations.**

   - **Location:** `04a-formal-rules.md` §§4, 6.8, and 12; `09-metatheory.md` Theorem 10.1; governing
     `docs/rules/language/00-semantics.md` §3; current `crates/musa-compiler/src/core.rs`.
   - **Type:** false theorem / incomplete embedding.
   - **Exact counterexample:** the governing language gives

     ```musa
     let octave_answer: Music -> Music = transpose(P8);
     ```

     This is not hypothetical. It occurs in `examples/canon-functions.musa`; the governing module specification also
     uses `let answer: Music -> Music = transpose(P5)`. Current evaluation represents the result as
     `Value::Builtin(BuiltinValue { builtin: Transpose, bound: [P8, None] })`. `apply_builtin` returns that function
     value while an argument remains unbound.

     The proposed calculus explicitly says that a compiler operation must receive every argument and cannot be
     partially applied. Its term grammar has only the saturated node
     `music_operation(name, e_1, ..., e_n) : Music`. Its values contain closed source functions and compiler-owned base
     or `Music` values, but no controlled-operation function value. Its reduction rules construct a recipe only after
     every argument is present. Consequently there is no proposed typing derivation for `transpose(P8) : Music ->
     Music`, no target value to which the old `BuiltinValue` can be related, and no simulation of applying that returned
     function later.

     The arrow-shaped table entry `Interval -> Music -> Music` does not repair this. The text says both that controlled
     operations check by repeated application and that compiler operations cannot be partially applied, while the only
     formal controlled-operation rule is the saturated `MusicOperation` rule. There is no term, value, typing rule, or
     reduction supporting the intermediate arrow value.

   - **Why it matters:** `Old` is defined as the complete current checked expression core. The counterexample checks,
     evaluates successfully, and is exercised by a passing repository law test. Therefore all three parts of Theorem
     10.1 fail to cover its stated domain: the embedding is not total on old typed terms, the step simulation omits
     `BuiltinValue`, and the final-value relation has no old controlled-function case. Required claim 8 is false.
   - **Natural repair:** keep this finite closed table of controlled operations as typed function values. Partial
     application should produce a closed value carrying the already supplied arguments; saturation should construct the
     finite recipe. This does not open a foreign higher-order registry and does not threaten source termination. An
     alternative is an explicit, type-directed eta-expanding embedding, for example translating `transpose(P8)` to
     `lambda(m:Music) => music_operation(transpose, P8, m)`. That alternative must cover every subset and ordering of
     the current named/default arguments and prove the application simulation. Merely listing saturated operation names
     is insufficient.

### Medium

1. **The displayed `map` and `filter` rules are nondeterministic because their basic reductions lack value premises.**

   - **Location:** `04a-formal-rules.md` §§8–9; `09-metatheory.md` Lemma 5.3 and Theorems 5.4, 9.1.
   - **Type:** false operational theorem as written / missing rule premises.
   - **Exact counterexample:** let `id = lambda(x:Nat) => x` and consider the well-typed closed term

     ```text
     map(id, ((lambda(z:Nat) => z)(0)) :: [])
     ```

     The context `map(v,E)` permits the inner beta step. But the displayed basic rule is

     ```text
     map(function, items) --> map_state(function, items, [])
     ```

     with no requirement that `items` be a value. It therefore also permits the whole term to enter `map_state`
     immediately. These are distinct next terms. The same overlap exists for `filter`.

     There is a second overlap inside the traversal. `map_wait(f, xs, completed, y)` records arbitrary `y`, while the
     context `map_wait(v, values, values, E)` steps a non-value pending expression. Thus

     ```text
     map_wait(id, [], [], (lambda(z:Nat) => z)(0))
     ```

     may either beta-reduce its pending callback or record the unreduced callback expression. Besides refuting unique
     decomposition and determinism, the latter path can evaluate later callbacks before the earlier one, contradicting
     the claimed source-order callback and charge trace.

   - **Why it matters:** this is the compact reference's claimed exact reduction relation. The proof of determinism says
     that a basic redex and a contextual redex cannot compete, but these rules make them compete. Required claim 5 and
     the repeatable-meter theorem do not hold literally. The intended design is nevertheless clear and repairable.
   - **Natural repair:** require value arguments on the entry rules and a value callback result on `map_wait`, using the
     same `v` notation and side conditions used elsewhere. State corresponding value premises for the private-state list
     spine and completed prefix. Then redo unique decomposition, determinism, and the source-order metering case.

## Disposition of the first review

The repair genuinely closes the other High and Medium findings from `10-proof-review.md`:

- `(E:A)` now evaluates an annotation subject before erasure. The former closed stuck term steps, and the annotation
  cases have been added to preservation, progress, decomposition, and reducibility.
- All seven structural operations, `map_note_pitches`, and checked kernel quotation now have term forms, checking rules,
  evaluation behavior, and metatheory cases. The two missing value side conditions above are a new exactness bug, not a
  return to the former absence of rules.
- The sealing theorem now states the correct representation-hiding property. It permits wildcard and whole-value binder
  matches while excluding private constructor names, construction, and constructor-pattern inspection.
- `Music` closure is no longer obtained by assuming the successful conclusion. The draft defines a finite recipe
  invariant and separates source construction, instantiation, and closing.
- Stage paths now join at the exact stored full anchor. Loss records combine by ordered concatenation, without citing
  the nonexistent normalization algebra.
- The phrase and score examples now handle `Result` explicitly. The score trace supplies musical context, performance
  profile and context, a realization seed, and a distinct preparation seed.

## The ten required claims

| Claim | Result of the second review |
| --- | --- |
| Decidable resolution and checking | **Survives under the stated finite resolved-graph and finite-operation-table assumptions.** The rules are syntax directed; nominal ranking, exact equality, flat coverage, signature comparison, and quote checking are finite. |
| Substitution | **Survives.** Capture-avoiding substitution leaves nominal and module identities fixed. The new term forms are congruence cases, and callback closures use the ordinary function case. |
| Preservation | **Survives for the displayed steps.** Even the prematurely applicable `map` rules preserve types; their defect is nondeterminism and callback order. Compiler operations remain conditional on their declared type-preservation contracts. |
| Progress | **Survives.** Annotation evaluation is repaired, coverage supplies a match arm, and every saturated closed operation has a step. |
| Determinism | **False as written.** `map` and `filter` can take an outer administrative step while an argument can take the context-selected step; `map_wait` has the same competition for its pending callback. |
| Source termination | **Survives for the intended value-restricted relation.** The nominal-rank and written-type-size relation is well founded, including under product, arrow, `Option`, `List`, and `Result`. The displayed nondeterministic relation still appears terminating, but the proof invokes the false unique-decomposition theorem and does not prove all its possible administrative schedules. |
| Sealed-constructor unforgeability | **Survives.** Public resolution cannot name private constructors; wildcard and binder matches do not inspect representation. |
| Conservative extension of the old expression fragment | **False.** Existing partial `BuiltinValue` terms have no target typing, value, reduction, or value-relation case. |
| `Music` closure | **Survives under the stated finite atom and transform contracts.** The proof now exposes those contracts and derives the client theorem by recipe induction, instantiation, and checked closing. |
| Typed stage composition | **Survives.** Exact full-anchor equality is an explicit premise and ordered loss-list concatenation needs no invented algebra. The finite-prefix qualification for audio is honest. |

## Literal paper-program audit

I checked the five programs against the displayed declaration, type, pattern, match, fold, `Text`, `Result`, sealing,
and surface-elaboration rules rather than against the current parser.

- Every declared nominal dependency is acyclic. The flat constructor patterns have legal binder-only fields, and the
  displayed matches cover their transparent subject types or use a catch-all.
- Empty lists, `Ok`, and `Err` occur where a declared result or accumulator supplies the needed expected type.
- Fold callbacks and accumulators have the displayed curried types after the stated argument-order elaboration.
- The phrase program unwraps `Result<Phrase, PhraseError>` before calling the exported operation. Notation and
  arithmetic adapter results are matched before their successful values are used. The score host unwraps `Result<Music,
  PieceError>` before instantiation.
- The performance-led phrase and finite live-protocol cases do not require notation, source effects, recursion, or a
  source stream. The host-owned repeated protocol is correctly outside the source-termination theorem.

The programs are paper programs: the bridge names are assumed finite entries in the closed adapter table, and the stage
arrows are host judgments rather than source terms. Under those stated assumptions, I found no remaining literal type
error or undefined convenience in them. They do not exercise the old partial-controlled-operation case that refutes
Theorem 10.1.

## Clean passes

- The reducibility definition is not circular. Components of structural and arrow types have smaller written size;
  nominal fields contain only lower-ranked nominals. Leaf nominals at rank 1 keep nominal unfolding below the enclosing
  type even when a field contains a structural wrapper.
- First-order ordinary compiler operations cannot hide an unbounded callback. `map_note_pitches` stores its one admitted
  callback in a finite recipe and invokes it only over a pre-bounded finite set of pitch-bearing facts during
  instantiation.
- The narrowed sealing theorem matches the static rules and proves the intended abstraction property rather than
  forbidding harmless wildcard use.
- The recipe proof separates source termination from temporal instantiation. Sequence, overlay, transforms, quotation,
  sounded voicing, and acyclic references have explicit finite cases. Successful closing delegates only the final
  syntactic fact to the accepted kernel checker.
- The stage theorem no longer substitutes a semantically equal representation for the exact stored intermediate. It
  retains full anchors, intermediate steps, generated sites, and loss order.
- The stage account keeps source evaluation, temporal normalization, one process step, and an unbounded host run
  distinct. It does not claim normalization of infinite audio.

## Verified, judged, and not checked

### Verified

- The frozen files `04-source-calculus.md`, `04a-formal-rules.md`, `05-stage-semantics.md`, `06-paper-programs.md`,
  `07-proof-prototype.md`, `08-proof-outline.md`, `09-metatheory.md`, the first review, and the repair record.
- The governing source semantics for partial application, the governing core and module contracts, the temporal-kernel
  and exact-anchor stage contracts used by the proofs.
- The current `ExprKind`, `Value::Builtin`, `BuiltinValue`, controlled-operation table, and `apply_builtin` behavior in
  `crates/musa-compiler/src/core.rs`.
- The shipped partial-application example and its focused law test. The test
  `higher_order_music_laws::a_delayed_canon_accepts_a_partially_applied_answer_and_has_maximum_extent` passed.
- Focused current suites also passed: 5 core laws, 4 finite-data laws, 15 module laws, and 12 kernel-quotation laws.
  These tests establish the current behavior used by the compatibility counterexample; they do not implement the
  proposed calculus.

### Judged

- The two exact counterexamples and their theorem consequences.
- The well-foundedness of the reducibility relation and the standard substitution, preservation, progress, and sealing
  arguments after reading every displayed rule.
- Literal typing of the five paper programs under their stated surface-to-core translations and adapter assumptions.
- Sufficiency of value premises to repair the administrative traversal, and of controlled function values or an explicit
  eta translation to repair old-program embedding.

### Not checked

- There is no implementation of the proposed `Text`, `Result`, fresh nominal data, signature sealing, or repaired
  evaluator against which to run the paper programs.
- I did not independently implement or exhaustively test every atom and transform adapter contract. The theorem is
  correct under the explicitly stated finite table obligations; an implementation must discharge them entry by entry.
- This review proves no cultural or musical adequacy beyond the five stated pressure tests. That limit is correctly not
  one of the metatheory claims.

## Final decision

The repaired source calculus is close to a coherent small language, and most of the first review's objections were fixed
rather than renamed. It cannot yet govern Musa under its own closure gate. The old-fragment theorem makes an exhaustive
compatibility claim contradicted by a current, governing, tested program, and the exact structural-operation rules
refute deterministic evaluation. Record the design as **Incorrect at the final proof gate** and retain the two natural
repairs above as requirements for any later, deliberately reopened language decision.
