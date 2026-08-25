#!/bin/sh
set -eu

# Each selection is named in note 67. Keep this executable map narrower than
# the workspace suite: its purpose is to say which evidence freezes which rule.
run() {
    label=$1
    expression=$2
    printf '%s\n' "$label"
    cargo nextest run --workspace -E "$expression"
}

run 'E1 F1 phase separation and storable crossing' \
    'test(/a_piece_cannot_write_a_quote_at_all|the_phase_vocabulary_is_readable_only_in_a_phase_source|a_port_that_carries_a_function_is_refused_and_names_the_type/)'
run 'E2 F2 finite expansion and deterministic refusal' \
    'test(/a_region_deeper_than_the_budget_allows_is_refused_rather_than_fatal|a_region_nested_deeper_than_anyone_writes_still_expands/)'
run 'E3 F3 deterministic value, expansion, normalization, and charge' \
    'test(/expansion_is_deterministic|law_7_two_runs_of_one_transformer_agree_on_value_and_on_charge|normalization_is_deterministic|elaboration_is_deterministic/)'
run 'E4 F4 quotation hygiene and substitution' \
    'test(/a_spliced_value_arrives_where_the_splice_stood|a_quoted_binder_does_not_capture_a_spliced_name|a_quoted_binder_and_a_quoted_use_of_it_are_one_name/)'
run 'E5 F5 derived identity and unique paths' \
    'test(/two_literal_positions_in_one_quote_are_two_nodes|two_quotes_at_one_anchor_are_two_sites|one_quote_used_twice_at_one_anchor_is_one_site|a_derivation_is_its_three_components_and_nothing_else|a_derived_output_path_never_addresses_an_input_node/)'
run 'E6 F6 source attribution and anchors' \
    'test(/a_derived_node_and_a_source_node_match_alike|an_authored_note_is_supported_by_the_place_it_is_written|two_uses_of_one_motif_agree_on_the_body_and_differ_in_the_site/)'
run 'E7 F7 edit locality' \
    'test(/an_edit_changes_the_pitch_it_names_and_no_other_byte|setting_mix_replaces_only_its_exact_value|a_command_the_staff_does_not_serve_is_refused_by_name/)'
run 'E8 F8 value-level print round trips and named loss' \
    'test(/a_printed_page_says_what_the_value_said|a_page_the_staff_cannot_spell_is_a_stated_loss|a_printed_graph_reads_back_to_the_same_valid_description/)'
run 'E9 F9 syntax patterns through ordinary coverage and case trees' \
    'test(/match_after_build_binds_what_the_quote_spliced|a_comment_between_two_elements_does_not_defeat_a_match|a_match_of_shapes_still_needs_the_arm_that_says_what_this_reads|a_pattern_decides_a_known_shape/)'
run 'E10 F10 derivation coverage and associative grafting' \
    'test(/every_note_a_pass_produced_reaches_the_source|grafting_covers_every_anchor|grafting_is_associative|grafting_shares_a_common_input_and_keeps_every_parent|an_uncovered_result_is_reported_rather_than_hidden/)'
run 'E11 F2 F11 all sealed-step laws and budget accounting' \
    'test(/law_2_a_step|law_3_a_branch|law_5_a_nested_recursor|law_6_a_step|law_7_two_runs|law_9_the_derived_fold|law_10_capture|nineteen_phase_rows_are_registered_and_one_is_defined|a_private_case_is_refused_outside_its_module_by_name|deeper_than_the_budget/)'
run 'E12 F12 both unprivileged adapters' \
    'test(/staff_expansion_laws|staff_writing_laws|graph_adapter_laws/)'
