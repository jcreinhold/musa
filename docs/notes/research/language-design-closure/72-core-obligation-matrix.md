# 72. The dependent core obligation matrix

**Status: governs nothing.** This is prompt 169's conformance record for the implementation of
`docs/rules/language/02-core-calculus.md`. The governing account of proof strength remains
`docs/rules/across-stages/05-metatheory.md` §1a: none of this metatheory is mechanized. Each row therefore separates its
mathematical argument from an executable control that can falsify the implementation claim.

`scripts/check-core-language-conformance.sh` runs every `K` selection below. A named test is evidence only for the
failure in its last column. In particular, the re-checker in K20 is not evidence for K1–K19: it shares the evaluator and
quotation machinery whose correctness those rows examine.

## 1. Matrix

| Statement | Implementation owner | Executable evidence | Failure the evidence catches |
| --- | --- | --- | --- |
| **K1 NbE soundness.** Reading back evaluation produces a term definitionally equal to the input. | `kernel/eval.rs`, `kernel/quote.rs`, and the value forms in `kernel/value.rs` | `normalization_laws::{a_normal_form_contains_no_redex,normal_forms_are_eta_long,definitions_are_unfolded}` and `conversion_laws::{conversion_decides_each_rule_the_specification_names,conversion_agrees_with_the_naive_oracle}` | A β/η/δ/ι rule changes meaning, leaves a redex, or disagrees with the deliberately direct reducer. |
| **K2 NbE completeness.** Definitionally equal terms read back to α-equal η-long normal forms. | `kernel/eval.rs`, `kernel/quote.rs`, `elaboration/convert/` | `normalization_laws::{alpha_equivalent_terms_normalize_to_the_same_term,normalization_is_idempotent}` and `conversion_laws::conversion_agrees_with_normalization` | Equal terms normalize differently, binder spelling leaks into equality, or η-long reading is unstable. |
| **K3 Conversion is decidable and an equivalence.** A well-formed question returns yes, no, or exhaustion; exhaustion is no answer. | `elaboration/convert/`, `kernel/budget.rs`, facade `convertible*` | `conversion_laws::{conversion_is_reflexive,conversion_is_symmetric,conversion_is_transitive,nothing_is_accepted_where_conversion_says_no,a_malformed_term_is_reported_rather_than_aborting,exhaustion_is_not_a_negative_answer}` | Non-equivalence, a false acceptance, a panic, or resource exhaustion collapsed into `false`. |
| **K4 Elaboration preserves types.** Every rule emits the core introduction or elimination at the type its bidirectional judgment states. | `elaboration/elab/`, `elaboration/declare*`, `elaboration/case/` | `elaboration_laws::{a_program_with_no_implicits_elaborates_to_itself,an_accepted_terms_metas_are_all_solved}`, `coverage_laws::{a_match_is_the_recursor_it_compiles_to,a_branch_is_checked_at_the_motive_instantiated_at_its_pattern}`, and K1's independent reducer oracle | A rule changes an explicit term, leaks a meta, emits the wrong recursor, or checks a branch at another branch's goal. This row intentionally does not cite K20. |
| **K5 Closed storable canonicity.** A closed value of a storable type reads back as its declared constructor, compact numeral, or owner-admitted base literal; never as a closure or stuck eliminator. | `kernel/quote.rs`, `kernel/family/`, `kernel/base.rs`, `elaboration/storable.rs` | `base_laws::{a_literal_and_a_base_type_are_their_own_normal_forms,a_saturated_constructor_reads_back_without_its_parameters,what_is_not_canonical_data_reads_back_as_nothing}`, `numeral_laws::a_numeral_and_the_tower_it_stands_for_are_one_program`, and `storable_laws::a_port_that_carries_a_function_is_refused_and_names_the_type` | A noncanonical closed datum crosses, a constructor reads with the wrong spine, numeral representation splits, or a function is admitted at any stored depth. |
| **K6 Strong normalization.** Every accepted core term has a normal form in the unmetered mathematical relation. | `kernel/eval.rs`, generated recursors in `kernel/family/`, and structural recursion admission in `kernel/terminate.rs` | `termination_laws::{a_call_on_a_smaller_argument_is_admitted_and_computes,a_recursion_the_measure_cannot_see_is_refused}`, `normalization_laws::a_normal_form_contains_no_redex`, and `budget_laws::a_definition_recursing_far_past_the_nesting_limit_is_accepted` | A non-descending definition is admitted, an admitted fold fails to compute, or host nesting is confused with source termination. Finite budgets may still exhaust; that is K19, not a counterexample. |
| **K7 Strict positivity implies consistency.** A closed inhabitant of a constructorless family cannot exist. | positivity in `elaboration/declare/`; canonical forms in `kernel/family/` and `kernel/quote.rs` | `family_laws::{a_declaration_is_refused_for_the_reason_it_is_wrong,an_empty_family_has_no_constructor_to_name}` and `nesting_laws::{an_occurrence_may_sit_at_any_depth_of_positive_parameters,an_occurrence_outside_a_positive_position_is_still_refused}` | A negative occurrence is admitted, a positive nested occurrence is rejected, or declaration invents an inhabitant for an empty family. |
| **K8 Coverage completeness.** An accepted match covers every reachable constructor and no impossible branch is required. | `elaboration/case/`, `kernel/case_tree.rs`, generated recursors | `coverage_laws::{a_match_is_the_recursor_it_compiles_to,a_match_is_refused_for_the_reason_it_is_wrong,a_missing_constructor_is_named_by_the_declaration_group}` | A missing reachable constructor passes, a complete matrix fails, or compiled matching diverges from recursor meaning. |
| **K9 Termination soundness.** Every accepted recursive definition is structurally descending and denotes a total function. | `elaboration/rec/` and `kernel/terminate.rs` | `termination_laws::{a_recursion_the_measure_cannot_see_is_refused,a_recursion_that_accumulates_carries_the_argument_it_changed,a_match_on_two_subjects_descends_in_one_of_its_columns,a_recursion_may_change_the_arguments_it_does_not_descend_on}` | A non-descending call passes, a descending multi-column call fails, or descent is attached to the wrong argument. |
| **K10 Unification soundness.** A committed solution makes the constrained terms convertible and never captures an out-of-scope variable. | `elaboration/convert/unify.rs`, metavariable scope data | `unification_laws::{a_metavariable_determined_twice_must_be_determined_the_same_way,a_solution_that_would_escape_its_scope_is_refused_rather_than_captured}` plus `conversion_laws::conversion_agrees_with_normalization` | Inconsistent assignments, scope escape, or a solution that conversion rejects. |
| **K11 Unification determinacy.** The pattern fragment has one solution; outside it the elaborator postpones or refuses and never defaults. | `elaboration/convert/`, `elaboration/elab/metas.rs` | `unification_laws::{an_inserted_implicit_is_solved_to_the_argument_that_determines_it,a_solution_that_is_itself_a_metavariable_is_followed_to_the_end,a_binder_nothing_determines_is_refused_rather_than_defaulted}` and `elaboration_laws::elaboration_is_deterministic` | Order-dependent solutions, unresolved aliases, or invented defaults. |
| **K12 Coverage under a dependent motive.** Each branch is checked at the motive instantiated by its own pattern; index-refuted branches are unreachable. | `elaboration/case/`, family recursors, index unification | `coverage_laws::{a_match_whose_goal_is_a_metavariable_elaborates,a_branch_is_checked_at_the_motive_instantiated_at_its_pattern,a_branch_that_answers_another_branchs_goal_is_refused}` and `family_laws::a_match_refines_the_goal_from_the_index_it_learned` | A branch answers the wrong indexed goal, a reachable indexed case disappears, or an impossible case is treated as reachable. |
| **K13 Storability is faithful.** The computed predicate accepts exactly finite data with no Π at any depth; owner-admitted base types carry the same promise. | `elaboration/storable.rs`, `Base::storable` | `storable_laws::{a_port_that_carries_a_function_is_refused_and_names_the_type,a_port_that_carries_a_declared_family_is_answered_without_being_written}` and `base_laws::{a_base_type_indexed_by_a_literal_is_finite_data,a_literal_is_data_only_where_a_base_type_indexes_on_it}` | A nested function is admitted, a recursive data family needs a source instance, or opaque base data is classified without its owner guarantee. |
| **K14 Track construction is safe.** A checked track expression closes to a finite, well-formed event term with exact duration, count, placement, payload, scope, and origin. | compiler `registry/{notation,track}/`, lowering, and `musa-events::{term,track}` | `events::{follow_translates_and_adds_durations,together_takes_max_duration_and_keeps_multiplicity,map_payloads_preserves_support}`, `terms::{every_well_formed_term_evaluates,ill_formed_terms_are_rejected}`, compiler tests `a_fragment_inhabits_the_track_type_it_was_promised`, `two_statements_are_one_follow_the_core_accepts`, `a_piece_reads_back_as_the_track_it_folded`, `following_adds_the_durations_and_places_the_second_after_the_first`, `together_is_the_events_stacking`, and `play_gives_every_fact_it_makes_the_scope_placement_and_origin_it_was_given` | Wrong exact duration/count, lost multiplicity, moved support, malformed closure, lost provenance, or lowering that does not inhabit the promised track type. |
| **K15 Musical domains conservatively extend the core.** Source δ operations are uniquely classified, data-only, deterministic, explicitly partial, and finite. | compiler `registry/`, calculus `kernel/base.rs` | compiler tests `the_compilers_own_context_builds`, `every_delta_spelling_is_registered_exactly_once`, `the_operations_past_both_tables_are_named_and_in_neither`, `every_owned_operation_names_its_hidden_information`, `the_unregistered_rows_are_the_families_they_are_said_to_be`, `every_partial_exact_time_operation_states_its_own_refusal`; calculus `base_laws::{a_destructuring_pattern_at_a_base_type_is_refused,a_rule_that_refuses_refuses_the_program,a_rule_that_answers_nothing_is_still_a_compiler_defect,two_registries_with_the_same_vocabulary_reduce_alike}` | An ill-typed or non-finite signature, duplicate/unowned or misclassified operation, unstated hidden information, undeclared partiality, stuck rule, base destructuring, or registry-order meaning. |
| **K16 Expansion is conservative, finite, hygienic, deterministic, and path-complete, including law 11.** | compiler expansion, phase registry, quotation and syntax recursor | The twelve executable selections in `scripts/check-syntax-adapter-conformance.sh`, frozen by note 67 | Phase vocabulary leaks, descent is unsealed, evaluation differs, hygiene/path/provenance fails, or an output lacks a derivation. |
| **K17 Numerals conservatively represent counting towers.** | numeral recognition in family declarations; evaluator and recursor numeral cases | `numeral_laws::{a_numeral_and_the_tower_it_stands_for_are_one_program,a_numeral_of_fifty_thousand_neither_overflows_nor_deepens,any_family_of_the_counting_shape_takes_a_numeral}` | Numeral and tower differ, `Nat` is privileged, or compact representation changes elimination. |
| **K18 Evaluation and elaboration are deterministic.** Equal context, input, registry, and budget produce equal values, terms, refusals, and charges. | `kernel/eval.rs`, `elaboration/elab/`, ordered registries and case trees | `normalization_laws::normalization_is_deterministic`, `elaboration_laws::elaboration_is_deterministic`, `base_laws::two_registries_with_the_same_vocabulary_reduce_alike`, and note 67 E3 | The same input changes with traversal, registration order, or another run, including a different semantic charge. |
| **K19 Budgets are independent of answers.** Narrowing a budget can only preserve an answer or return exhaustion; the three outcomes never collapse. | `kernel/budget.rs`, facade error types, compiler phase budgets | `budget_laws::{a_narrower_budget_exhausts_or_agrees,exhaustion_is_monotone_in_the_budget,the_language_budget_answers_every_question_in_the_corpus}` and `elaboration_laws::a_narrow_budget_exhausts_rather_than_refusing` | A budget changes yes to no or vice versa, exhaustion becomes refusal, or the published budget cannot answer the corpus. |
| **K20 Elaboration emits only kernel-accepted programs.** No meta, bad index, mistyped branch, uncovered case, or non-descending call crosses the trusted boundary. | `kernel/recheck.rs`, `Checked`, `recheck_program` | `recheck_laws`' per-construct positive and negative controls, `generated_laws::the_kernel_accepts_whatever_elaboration_produced`, and compiler `document::laws::{the_kernel_rechecks_the_standard_library,the_kernel_rechecks_every_example}` | The elaborator emits an ill-typed, uncovered, non-descending, scope-corrupt, or meta-bearing term. It does **not** detect a shared evaluator defect. |

## 2. Why the mathematical rows follow

The proof strength is exactly the one stated in the governing §1a, not stronger. Define reducibility by induction on
types: a value at a Π maps every reducible argument to a reducible result; a value at a declared family is a finite
constructor spine whose recursive fields are reducible; a value at an inert base is a closed owner value; universes
contain reducible types. Strict positivity makes the family clause well founded. The fundamental lemma follows by
induction on the kernel typing derivation. The recursion case uses the case tree's structurally smaller field, and the
generated recursor case uses the same induction hypothesis. Acyclic definitions and total, finitely constructing δ rules
add no recursive semantic case.

Thus accepted terms normalize. Evaluation preserves the reducibility interpretation, and type-directed quotation returns
an η-long term of the same type, giving K1. At the identity environment every definitional equality rule maps to equal
semantic values, giving K2. Finite normal forms have decidable α-equality, giving K3. Induction over the bidirectional
rules gives K4 because every emitted elimination is checked against the value of its domain and every introduction is
checked at its expected value. Closed neutral heads have neither a local nor global name to block on, so the
canonical-forms induction gives K5. For a constructorless family that induction has no case; hence K7.

K8 and K12 are the standard pattern-matrix induction, with the dependent refinement made explicit: splitting a column
enumerates its declaration's constructors, unification removes only constructors whose indices cannot inhabit the
subject type, and each surviving row checks at the motive applied to that constructor. K9 is the recursion case of the
fundamental lemma. K10 follows by induction on the pattern-unification walk; assignments are permitted only at the
unknown's scope and the substitution makes that equation reflexive. Pattern heads determine the only possible
assignment, while all other heads postpone or refuse, giving K11. K13 is structural induction on the evaluated type; the
family cycle assumes only the family currently being inspected and every other field is inspected recursively.

These are paper arguments paired with falsifying tests, not a mechanized proof. K20 is stronger implementation evidence
for its one finite-corpus statement, but cannot promote the preceding arguments.

## 3. Track-construction safety re-derived over the dependent core

Let a private fragment be `(t, B, d, n)` as §5.7 defines it. The induction is over the finite acyclic declaration graph
and, inside a definition, over its structurally descending case tree. A literal track is accepted only through
`musa-events::track`, so its occurrences lie in `[0,d]`; its registry signature fixes the coordinate and payload types,
and its payload owner supplies the exact finite encoding required by event admission.

For `follow`, translating the second fragment by the first duration preserves every span and places it within
`[0,d₁+d₂]`; duration and count become `d₁+d₂` and `n₁+n₂`. For `together`, no span moves, the ambient duration becomes
`max(d₁,d₂)`, and multiset union makes the count `n₁+n₂`. A mark is a total payload map, so it preserves support and
count. Introducing a completed binding adds one backward edge to `B`; replacing its body by a reference changes neither
denotation, duration, nor count because δ opens that acyclic definition.

Every registry application is elaborated at its dependent Π signature. K14's lowering controls compare explicit expected
core forms and their event denotations; they do not appeal to the rechecker. Induction therefore preserves the exact
coordinate, payload, placement, scope, and origin parameters rather than erasing them into an old rank-1 class. Closing
the acyclic bindings removes all free names, and structural termination makes the result finite. The resulting event
term evaluates to exactly `n` occurrences by the constructor cases above.

The new `Storable` boundary makes the crossing argument simpler: it computes “no Π at any depth” over the actual
instantiated type. It is not an inferred `d` class and has no source instance table. Event payload admission asks the
next, distinct question—whether that finite value has a versioned exact canonical encoding. Both must hold; neither is a
second implementation of the other.

## 4. Musical-domain conservativity

An old core term contains no newly registered base or operation name. Extending the registry therefore leaves every
lookup, evaluation step, quoted form, and conversion of that term byte-for-byte unchanged. A new inert base contributes
one canonical literal case and no eliminator. A new δ rule fires only at its unique registered head, is a pure function
of closed arguments, returns a declared value or refusal, and charges its finite construction before publishing it.
Consequently it adds no overlap to β/η/ι, no new observation of old values, no stuck closed term, and no infinite
reduction. The reducibility and canonical-forms arguments above gain exactly the base-literal and total-rewrite cases.
K15 tests each implementation premise; the musical-domain suites separately test whether the returned value is the right
musical fact.

## 5. Expansion re-derived over the dependent core

Note 67 proves the expansion theorem over the same dependent core and freezes all twelve premises. Its §3.1 extends the
reducibility interpretation with sealed `SyntaxStep C A`; §3.2 proves deterministic hygienic substitution and identity;
§3.3 proves matching, provenance, edit, print, and derivation laws; and §3.4 proves phase conservativity. Because the
phase environment only adds names while ordinary resolution never installs it, dependent families, indices, motives,
conversion, and normal forms of old source terms are unchanged. `scripts/check-syntax-adapter-conformance.sh` is the
executable map, including law 11's association and local-decrease controls. That is the §5.9 re-derivation; duplicating
it here would create the second proof path this audit forbids.

## 6. Boundary with prompt 174

This matrix owns the source dependent calculus, bidirectional elaboration, case trees and source recursion, source δ
registries, `Storable`, track-expression construction, and syntax expansion. Prompt 174 imports K1–K20 and note 67; it
does not restate them. It owns event-track runtime algebra beyond construction, the machine value and step relation,
scheduling, one-frame audio, end-to-end traces through those stages, and deletion of legacy runtime paths. Thus K14
proves that language construction reaches a valid event term; prompt 174 proves what the runtime does with that term.
