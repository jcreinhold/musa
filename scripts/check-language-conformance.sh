#!/bin/sh
set -eu

# Executable map for note 87. The selections are bounded falsifiers for the
# governing contracts, not a mechanized proof of their universal statements.
run() {
    label=$1
    expression=$2
    printf '%s\n' "$label"
    cargo nextest run --workspace -E "$expression"
}

./scripts/check-core-calculus-conformance.sh

run 'L1 surface elaborations preserve the one core reading' \
    'test(/a_conditional_lowers_to_the_two_armed_boolean_match|a_question_lowers_to_a_match_that_evaluates_its_subject_once|an_update_along_a_path_replaces_the_field_the_path_ends_at|the_literal_and_its_goal_must_name_one_family|a_core_refusal_is_restated_at_the_span_that_caused_it/)'

run 'L2 nested patterns compile only to the existing case tree' \
    'test(/a_nested_match_answers_what_the_two_level_match_answered|an_inexhaustive_nested_match_is_refused_where_it_always_was|a_nested_list_pattern_parses_and_lowers|a_nested_record_pattern_parses_and_lowers|a_match_refines_the_goal_from_the_index_it_learned/)'

run 'L3 sealed syntax traversal is structural, repeatable, and charged' \
    'test(/the_fold_rebuilds_the_region_the_way_the_old_evaluator_does|recursing_descends_the_leftmost_spine_the_way_the_old_evaluator_does|every_self_application_stands_at_a_smaller_node|law_9_the_derived_fold_is_the_recursor_at_a_context_nothing_reads|law_3_a_branch_is_read_under_exactly_the_context_it_was_run_with|law_6_a_step_may_be_omitted_or_run_more_than_once|law_2_a_step_carried_into_a_nested_recursor_still_runs_its_own_algebra|law_5_a_nested_recursor_over_the_original_subject_still_terminates|law_7_two_runs_of_one_transformer_agree_on_value_and_on_charge|law_10_capture_and_repetition_are_charged_for_what_they_cost|a_region_deeper_than_the_budget_allows_is_refused_rather_than_fatal/)'

run 'L4 musical domains remain source libraries with exact counterexamples' \
    'test(/a_scale_walks_its_own_reference_map|chromatic_motion_and_scale_stepping_do_not_commute|every_chord_type_spells_the_letter_stack_the_formula_names|a_chord_class_chooses_no_register_and_stacking_is_where_one_arrives|the_quotient_reduces_modulo_twelve|a_set_class_survives_transposition_and_inversion|a_row_has_twelve_order_positions|the_named_compositions_are_the_ones_omt_names|the_diatonic_triads_are_the_ones_the_collection_stacks|every_schema_bass_is_the_row_omt_prints|a_claim_that_holds_changes_nothing_about_the_music|a_reading_changes_nothing_and_repeats_exactly|every_rule_is_checked_and_exercised/)'

run 'L5 events documents and quotation preserve exact meaning and provenance' \
    'test(/every_events_file_is_a_musa_document|the_unified_route_preserves_the_events_meaning|formatting_a_events_document_is_the_events_printing|a_payload_the_compiler_does_not_know_is_refused|an_unknown_format_version_is_refused|printing_and_parsing_an_example_preserves_its_meaning|a_hairpin_shape_survives_as_exact_rationals|the_quotation_locus_follows_the_terms_structure|a_hole_cannot_be_captured_by_a_name_the_quote_binds|a_payload_this_build_does_not_own_is_refused|a_quote_may_not_carry_context_or_choose_a_voice|a_quote_agrees_with_the_term_written_in_the_surface|a_quotes_time_stays_exact/)'

run 'L6 modules, ownership, provenance, and compatibility close' \
    'test(/the_kernel_rechecks_the_standard_library|the_kernel_rechecks_every_example|the_schema_libraries_add_no_compiler_builtin|the_same_names_resolve_once_their_module_is_imported|every_delta_spelling_is_registered_exactly_once|every_owned_operation_names_its_hidden_information|every_note_a_pass_produced_reaches_the_source|the_expected_change_ledger_is_empty_after_audio_conformance|existing_language_behavior_matches_the_migration_oracle/)'

run 'L7 project, exports, playback, and editor protocol share one source' \
    'test(/wav_export_is_deterministic|midi_states_what_one_tempo_track_costs|invalid_source_keeps_the_last_valid_score_and_says_so|every_existing_backend_matches_the_migration_oracle|engine_and_offline_harnesses_share_one_step|every_start_end_object_is_a_source_span|every_example_and_every_broken_fixture_folds|bundled_names_keep_source_maps_docs_and_read_only_identity|the_lexers_tokens_are_the_grammars_test_data/)'

python3 - <<'PY'
import json
from pathlib import Path

root = Path('.')
if json.loads((root / 'tests/fixtures/elaboration-expected-changes.json').read_text())['entries']:
    raise SystemExit('expected-change ledger is not empty')
ledger = (root / 'docs/plan/clean-break-ledger.md').read_text()
if 'reassigned; live' in ledger or 'reassigned with' in ledger:
    raise SystemExit('clean-break ledger has a live row')
if '**Status: candidate' in (root / 'docs/rules/language/README.md').read_text():
    raise SystemExit('language specification has not been graduated')
PY

if rg -n 'PerformancePlan|PerformanceEvent|PerformanceLane|PerformedNote|lower_performance|pub (struct|enum|type) (StudioSpec|InstrumentSpec|WrittenQuantity)|pub fn (compile_graph|lift)\b' crates/*/src; then
    printf '%s\n' 'a stale or independently authoritative language path remains' >&2
    exit 1
fi

if rg -n '(:|->|<|,)[[:space:]]*Music([[:space:],;=>}]|$)' stdlib/src examples editors/tree-sitter-musa/test/corpus; then
    printf '%s\n' 'removed contextual Music syntax remains executable' >&2
    exit 1
fi

if rg -n '(^|[^[:alnum:]_])(overlay|sequence)[[:space:]]*\(' stdlib/src examples editors/tree-sitter-musa/test/corpus; then
    printf '%s\n' 'removed event operation spelling remains executable' >&2
    exit 1
fi

printf '%s\n' 'whole-language conformance selections, graduation, and alternate-path audit: ok'
