# Proof review: breaking-change repair

## Findings

### High

1. **The retained-fragment translation does not cover current named partial applications of ordinary functions.**

   - **Location:** `04a-formal-rules.md` §§4 and 12; `08-proof-outline.md` Theorem 8; `09-metatheory.md` Theorem 10.1;
     current `crates/musa-compiler/src/core.rs` application checking and `apply_closure`.
   - **Type:** false statement / incomplete translation.
   - **Problem:** `Old_complete` excludes partial compiler operations but explicitly retains ordinary source functions.
     Current Musa permits a call to supply a later named parameter while leaving an earlier required parameter missing.
     The evaluator captures the supplied value and returns a closure over the missing parameter. I checked this closed
     current program:

     ```musa
     piece "named partial" {
         fn choose(first: Nat, second: Bool) -> Nat { first }
         let waiting: Nat -> Nat = choose(second: true);
         let result: Nat = waiting(1);
         score { part p { voice v { c4/1 } } }
     }
     ```

     `musa check /dev/stdin` reports `ok`. The current checker records `second` in slot 1, reports `first` as the only
     missing parameter, and assigns `waiting` type `Nat -> Nat`. `apply_closure` captures `true` and returns a closure
     over `first`.

     This program belongs to `Old_complete`: it contains no compiler operation at all. The proposed translation turns
     `choose` into

     ```text
     lambda(first:Nat) => lambda(second:Bool) => first
     ```

     and says that named arguments are reordered and then applied as nested unary applications. There is no unary
     application that supplies `second` while skipping `first`. Applying the sole supplied value first would attempt
     to pass `true` where `Nat` is required. The table gives no eta-expanded term such as
     `lambda(first:Nat) => choose(first)(true)` and no other translation for the missing slot.
   - **Why it matters:** the table is not exhaustive over its stated domain, so Theorem 10.1 does not define a refined
     translation for every `e` it quantifies over. The type-preservation and related-result claims therefore fail as a
     universal theorem. This is independent of the deliberately rejected partial compiler operations and does not
     require translating `BuiltinValue`.
   - **Suggested repair:** either narrow the retained fragment to ordinary calls that supply a prefix of the parameter
     list, and audit that additional break, or define the exact translation of arbitrary current argument slots. An
     honest translation can evaluate supplied arguments into `let` bindings and eta-expand over the missing parameters
     in their declared order. Defaults that depend on earlier parameters need their own exact case. Then prove that the
     generated closure has the same curried type and behavior.

### Medium

1. **The final-result comparison proof needs an open, related-environment lemma that is not stated.**

   - **Location:** `09-metatheory.md` Theorem 10.1, especially the function case.
   - **Type:** proof gap.
   - **Problem:** the theorem is stated only for a closed expression. Its proof says that the induction hypothesis for a
     function body proves the function clause of `~`. A function body is open in its parameters and captured names,
     while that clause quantifies over every pair of related arguments. The stated closed theorem is not an induction
     hypothesis that can be applied to such a body. The same missing strengthening is needed for callbacks, defaults,
     and closures that capture earlier arguments.
   - **Why it matters:** even after the translation table is repaired, the proof as written does not establish related
     results at function types. Higher-order source functions are part of the retained fragment and are used by the
     musical examples, so this is load-bearing rather than an optional generalization.
   - **Suggested repair:** state and prove the standard open-term lemma: if the old and refined environments assign
     related values to every name in `Gamma`, then evaluating an old term and its translation yields related results.
     Prove it by induction on typing. The closed theorem follows from empty related environments. This is a routine
     repair, but it must be present for the function case to be valid.

2. **The repository audit omits a governing dynamic partial use and incorrectly implies that anonymous functions are
   needed to replace it.**

   - **Location:** `14-breaking-change-repair.md` §§“Why this is the right boundary” and “Repository migration audit”;
     `04a-formal-rules.md` §12; `09-metatheory.md` §10.2; governing `docs/rules/language/00-semantics.md` §3.
   - **Type:** incomplete audit / cross-document inconsistency.
   - **Problem:** the audit says it searched the governing language documents and found only fixed partial shapes. The
     governing semantics contains the explicitly parameterized law

     ```text
     transpose(i) : music -> music
     ```

     and says that an unsupplied required parameter produces a closure. This is not one of the four fixed shapes in the
     audit. It is a governing statement that must be deliberately rewritten when compiler operations cease to be
     values.

     The repair also says a run-time `transpose(interval)` would require anonymous functions or an interface change.
     The selected language already creates ordinary closures from named multi-argument functions. It can write:

     ```musa
     fn transposer(interval: Interval, music: Music) -> Music {
         transpose(interval, music)
     }

     let answer: Music -> Music = transposer(runtime_interval);
     ```

     Elaboration gives `lambda(interval).lambda(music).music_operation(...)`; the prefix application captures the
     run-time interval. No anonymous function or compiler-owned partial value is needed.
   - **Why it matters:** the audit is correctly described as non-universal, but it is not exact over the scope it names.
     Promotion would otherwise leave a governing semantic law silently inconsistent with the new operation boundary. The
     mistaken closure claim also obscures a natural migration route that supports dynamic musical parameters while
     preserving the simpler `op` design.
   - **Suggested repair:** add the governing `transpose(i)` law to the audit and replace it with a named generic wrapper
     law. List the concrete files or occurrences for each fixed wrapper migration. State clearly that named curried
     functions can capture run-time operation arguments; what is rejected is only using the compiler operation name
     itself as that function value.

### Low

1. **One introductory compatibility sentence still states the abandoned universal claim.**

   - **Location:** `04-source-calculus.md` §2.
   - **Type:** cross-document inconsistency / exposition issue only.
   - **Problem:** the bridge-type paragraph says the migration set lets “old checked programs still embed in the new
     core.” §§8 and 13 and the formal documents now claim preservation only for `Old_complete`. The broad sentence is
     stale.
   - **Why it matters:** the exact theorem is narrowed later, so this does not create an additional proof failure, but
     it makes the document contradict its own breaking-change decision.
   - **Suggested repair:** say that the bridge set covers the retained current fragment, as `04a` already does.

## Verdict

- **Decision:** **Incorrect**.
- **Basis:** the retained-fragment theorem is false as a statement about the displayed translation table. A closed
  current program with a non-prefix named partial application of an ordinary function lies in the stated domain but has
  no refined translation under that table. The final-result proof also omits the related-environment strengthening
  needed at function types, and the claimed repository audit misses an in-scope governing dynamic partial use.
- **Limits:** the proposed source language is not implemented, so I checked its literal rules and proofs rather than an
  executable refined compiler. The finite old/refined operation comparison, every atom and transform recipe contract,
  and the accepted kernel and cross-stage theorems remain named inputs rather than results re-proved here.

The verdict does not reject the breaking-change choice. Treating an operation name as a non-term label for one complete
call is coherent and simpler than retaining `BuiltinValue`. The failure is in the theorem about which current ordinary
function calls are translated, plus the incomplete migration record.

## Verified, judged, and unverified

### Verified

- I read the frozen `04-source-calculus.md`, `04a-formal-rules.md`, `07-proof-prototype.md`, `08-proof-outline.md`,
  `09-metatheory.md`, `12-proof-review.md`, and `14-breaking-change-repair.md`, and checked their copied claims against
  the governing current source rules and `core.rs`.
- Current `application` accepts arbitrary named argument slots, computes the function type from the missing required
  slots, and `apply_closure` captures supplied values while retaining missing parameters.
- The exact non-prefix program above passed current `musa check` through `/dev/stdin`.
- The migration search confirmed the fixed partial source shapes listed in the repair: fixed `transpose`, `stretch`,
  `invert`, and bare `retrograde` uses occur in examples, teaching material, generated fixtures, and higher-order law
  tests. It also found the omitted governing `transpose(i)` statement.
- Focused current tests passed: all 8 higher-order music laws, 4 finite-data laws, and 5 core laws. They verify the
  present implementation behavior used by the counterexamples, not the unimplemented refined calculus.

### Judged

- The non-prefix call cannot be represented by merely reordering supplied arguments before unary application.
- An eta-expanding slot translation can repair that case without adding compiler-operation values.
- Theorem 10.1 requires an open logical relation over related environments to justify its function case.
- A named curried wrapper is sufficient to capture a run-time interval and preserve the dynamic musical use while
  keeping `transpose` itself out of the term language.

### Unverified

- The old/refined complete-operation comparison contract is declared but has no refined implementation or finite-table
  proof artifact to inspect.
- The atom and transform obligations used by the `Music` closure theorem were treated as explicit contracts, not as
  independently discharged implementation theorems.
- I did not re-prove the accepted temporal-kernel and exact-anchor stage results; I checked that this repair does not
  strengthen their use.

## Open questions or assumptions

- Will the retained theorem preserve all current ordinary partial applications, including named non-prefix arguments and
  defaults, or deliberately break some of them? The theorem and migration audit must choose the same domain.
- Is related recipe equality intended to include complete Origin data or only musical recipe structure? The finite
  operation comparison contract should say so before it is checked entry by entry.
- Promotion still depends on the named operation-comparison and recipe-adapter contracts. Their eventual implementation
  witnesses should be recorded beside the governing rules rather than inferred from passing current tests.

## Clean passes

- **The `op` boundary is coherent.** An operation signature belongs to the finite operation table, not the source type
  grammar or `Gamma`. A complete surface call elaborates to `primitive`, `music_operation`, or the dedicated
  `map_note_pitches` core term. Substitution acts only on its argument terms; evaluation waits for argument values;
  progress uses totality; termination uses the first-order or bounded-recipe contract; and denotation interprets the
  complete call directly. No operation function value is needed.
- **The previous nondeterminism is repaired.** Fold equations require value accumulators, steps, and canonical
  structural inputs. `map` and `filter` enter private states only from value arguments; state formation keeps every
  non-pending field a value; `map_wait` completes only on a value; and `filter_wait` completes only on the disjoint
  canonical Booleans. `range` and `repeat` operate on canonical natural values. Complete compiler operations, quotation,
  and `map_note_pitches` reduce only after their displayed or stated value premises. I found no competing basic and
  contextual reductions in this repaired relation.
- **Private `BuiltinValue` steps are not needed for a final-result theorem.** Once its source translation is total and
  the complete-operation comparison contract is discharged, an extensional logical relation may compare final values
  without simulating either evaluator's private administrative states or resource schedule.
- **The migration claim is honestly non-universal in intent.** It explicitly rejects a theorem for arbitrary partial
  compiler operations and requires another audit before implementation. The problem is the missed in-scope governing
  case, not an attempt to hide universal compatibility.
- The annotation, sealing, `Music` recipe, exact-anchor stage composition, and finite-audio-prefix repairs from the
  earlier review remain intact. This breaking repair does not introduce a new problem in those results.
