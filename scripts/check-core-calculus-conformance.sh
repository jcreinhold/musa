#!/bin/sh
set -eu

# Executable map for note 77. These selections are falsifiers and regression
# controls for the paper arguments; they are not a mechanized metatheory.
run() {
    label=$1
    expression=$2
    printf '%s\n' "$label"
    cargo nextest run --workspace -E "$expression"
}

./scripts/check-core-language-conformance.sh

run 'R1-R3 event bounds, coordinates, algebra, multiplicity, and equality' \
    'test(/construction_checks_bounds|a_negative_duration_never_reaches_a_track|support_is_half_open|the_coordinate_is_part_of_exact_identity|follow_associativity|follow_zero_identity|follow_duration_additivity|together_associativity|together_commutativity|together_fixed_duration_identity|together_not_idempotent|equal_occurrences_are_kept_apart|canonical_order_is_start_end_payload|framed_identity_covers_schema_and_multiplicity|arbitrary_payload_delimiters_preserve_equality_hash_agreement|framed_gesture_identity_does_not_parse_payload_delimiters/)'

run 'R4-R6 finite machine formation, total step, causality, feedback, and structural laws' \
    'test(/registry_names_and_kinds_are_unique_and_versioned|preparation_refuses_malformed_structure_and_stored_data|preparation_and_step_report_typed_port_failures|every_leaf_constructor_takes_its_exact_step|every_reference_primitive_runs_the_function_its_registration_names|one_description_is_deterministic_and_causal|feedback_has_a_first_output_and_commits_boolean_negation_afterward|connect_and_beside_follow_the_structural_equations|connect_reassociation_preserves_the_step_function|current_step_chain_defeats_the_old_whole_node_schedule/)'

run 'R7-R10 checked scheduling, decisions, handles, overlay, succession, and finished state' \
    'test(/every_positive_boundary_is_emitted_once_with_half_open_order|point_and_each_collapse_policy_are_explicit|decisions_are_exact_deterministic_and_tracks_remain_distinguishable|malformed_and_nonmonotone_maps_name_exact_values|duplicate_overlay_is_occurrence_local_and_merge_namespaces_handles|merge_requires_the_complete_policy_not_only_its_version|additive_follow_is_the_delayed_union_of_separate_tables|source_countdown_finishes_and_seek_reads_the_target_frame|ambient_silence_delays_finished_without_storing_empty_batches|adversarial_bounds_fail_before_building_an_unbounded_table|handle_debug_does_not_reveal_private_spelling/)'

run 'R11-R13 one-frame audio, partition equality, offline/live agreement, and RT instrumentation' \
    'test(/score_to_graph_to_wav_matches_the_migration_oracle|stochastic_primitives_obey_the_explicit_render_seed|production_preparation_refuses_layout_tuning_and_resource_violations|render_allocates_nothing|engine_and_offline_harnesses_share_one_step|the_callback_path_allocates_nothing|the_callback_module_contains_no_logging|replaced_plans_retire_to_the_control_side/)'

run 'R14 complete notation-led and audio-led witnesses' \
    'test(/tonal_construction_runs_through_exact_gestures_and_one_frame_audio|unmeasured_time_runs_without_manufacturing_a_meter|phrase_led_transcription_keeps_ambiguity_and_states_loss|ensemble_tuning_is_configuration_not_a_rewritten_pitch|a_finite_live_protocol_builds_a_machine_that_need_not_finish|audio_first_microphone_synth_and_effect_paths_compose_explicitly|every_note_a_pass_produced_reaches_the_source|two_uses_of_one_motif_agree_on_the_body_and_differ_in_the_site|grafting_is_associative|grafting_shares_a_common_input_and_keeps_every_parent/)'

if rg -n 'PerformancePlan|PerformanceEvent|PerformanceLane|PerformedNote|lower_performance|compile_graph|pub (struct|enum) (StudioGraphSpec|RenderPlan)' \
    crates/*/src; then
    printf '%s\n' 'legacy runtime path remains' >&2
    exit 1
fi

if rg -n 'compile_graph' AGENTS.md docs/book; then
    printf '%s\n' 'current handbook still advertises the deleted graph compiler' >&2
    exit 1
fi

if rg -n '(:|->|<|,)[[:space:]]*Music([[:space:],;=>}]|$)' stdlib/src examples editors/tree-sitter-musa/test/corpus; then
    printf '%s\n' 'legacy Music spelling remains in executable source' >&2
    exit 1
fi

printf '%s\n' 'core calculus conformance selections and legacy-path audit: ok'
