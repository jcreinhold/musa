#!/bin/sh
set -eu

# Executable map for note 86. Each selection contains positive, negative, or
# differential evidence. This is a falsifier suite, not a mechanized proof.
run() {
    label=$1
    expression=$2
    printf '%s\n' "$label"
    cargo nextest run --workspace -E "$expression"
}

./scripts/check-core-calculus-conformance.sh

run 'A1 source ownership and exact checked projections' \
    'test(/source_quantities_keep_exact_dimensions_until_dsp_preparation|standard_studio_vocabulary_is_checked_source_data|the_production_studio_projection_retains_the_complete_checked_value|standard_performance_vocabulary_is_checked_source_data|standard_instruments_are_checked_source_data_with_private_machines|checked_source_is_the_complete_sample_map_authority|host_media_types_are_an_exact_projection_of_the_source_schema/)'

run 'A2 indexed controls use only pattern unification' \
    'test(/instrument_mapping_indices_use_the_general_unifier|a_control_kind_is_inferred_by_the_general_pattern_unifier|a_control_index_blocked_by_a_lambda_is_settled_by_later_arguments|disagreeing_control_indices_are_refused_without_a_sound_specific_fallback|an_unresolved_control_index_is_refused_instead_of_defaulted|a_control_index_cannot_escape_the_lambda_scope_that_names_it/)'

run 'A3 exact performance, tempo, curves, profiles, and bindings' \
    'test(/the_source_hairpin_law_has_exact_endpoints_and_midpoint|the_exact_time_at_a_tempo_change_is_the_sum_of_what_came_before|the_same_score_under_two_profiles_is_two_performances|settings_are_checked_by_name_range_and_unit|adjacent_gestures_share_one_exact_boundary|r1_equal_complete_arguments_have_equal_preparation_observations|presentation_lineage_is_separate_from_execution|every_execution_affecting_option_axis_is_explicit/)'

run 'A4 instrument instances, routing, sends, swaps, and reinstall' \
    'test(/private_instrument_policy_changes_exact_execution_identity|a_private_instrument_body_is_not_a_client_address|an_unrouted_part_cannot_sound_or_steal_another_parts_voices|equal_pitches_through_one_declaration_are_two_instrument_instances|a_send_begins_at_the_named_parts_instrument_output|reinstall_and_in_place_seek_keep_the_same_part_bindings|a_send_arrives_at_the_level_it_was_written_at/)'

run 'A5 exact assets, packages, invalidation, and offline closure' \
    'test(/a_locked_asset_exposes_metadata_and_never_its_bytes|changing_bytes_invalidates_only_the_locked_asset_identity|traversal_kind_and_size_are_rejected_before_locking|a_symlink_cannot_escape_the_project_root|fetch_materializes_exact_blobs_and_writes_a_deterministic_lock|a_corrupt_candidate_cache_cannot_substitute_for_the_locked_tree|a_locked_package_compiles_offline_and_its_definitions_are_read_only|locked_verification_rejects_manifest_drift_without_fetching|package_asset_addresses_resolve_to_verified_locked_metadata/)'

run 'A6 native, SFZ, and SoundFont adapters and refusals' \
    'test(/a_project_prepares_only_a_checked_map_from_the_exact_verified_read|sfz_inheritance_layers_sequences_and_release_regions_cross_one_checked_map|unsupported_sound_changing_sfz_opcodes_are_positioned_errors|sfz_directives_and_sample_paths_cannot_escape_the_strict_adapter_boundary|every_sfz_parser_resource_has_an_explicit_enforced_bound|strict_bank_adapts_to_exact_checked_source_and_prepared_pcm|unsupported_sound_changing_generator_and_bounds_are_named_refusals|session_resolves_the_fragment_only_after_the_verified_bank_identity/)'

run 'A7 sample selection and recorded-media semantics' \
    'test(/round_robin_is_per_instance_and_block_partition_cannot_change_sound|looping_release_pedal_and_voice_stealing_are_bounded_and_deterministic|stable_weighted_selection_is_seeded_by_realization_and_semantic_identity|clip_and_fixed_media_have_disjoint_written_support|every_declared_fit_policy_projects_exactly|fixed_media_retains_natural_physical_duration_and_mono_becomes_stereo|crop_loop_and_rate_have_distinct_finite_readings|prepared_audio_seek_reads_the_same_absolute_media_frame|media_render_is_independent_of_host_block_partition|repeated_fixed_cues_overlap_by_deterministic_addition|named_media_uses_the_authored_route_and_send_graph/)'

run 'A8 deterministic, finite, offline/live, and RT behavior' \
    'test(/two_renders_are_byte_equal|stochastic_primitives_obey_the_explicit_render_seed|adversarial_parameters_stay_finite|non_finite_parameter_is_rejected|engine_and_offline_harnesses_share_one_step|the_callback_path_allocates_nothing|recorded_media_frame_steps_allocate_nothing|replaced_plans_retire_to_the_control_side|regression_saturated_retirement_queue_never_drops_a_plan|underrun_without_a_plan_is_silence/)'

run 'A9 source-edit, tooling, navigation, and stale-state evidence' \
    'test(/the_facts_report_what_the_compiler_resolved|a_fader_rewrites_the_level_and_nothing_else|fixing_the_source_clears_the_diagnostics|bundled_names_keep_source_maps_docs_and_read_only_identity|hover_on_an_sfz_instrument_reports_the_imported_contract_and_support|hover_on_a_soundfont_preset_reports_the_strict_adapter_boundary|sound_completion_comes_from_checked_instrument_and_project_facts|exposed_control_help_retains_the_source_index_and_navigates_to_its_declaration|invalid_source_keeps_the_last_valid_score_and_says_so/)'

python3 - <<'PY'
import json
from pathlib import Path

root = Path('.')
expected = json.loads((root / 'tests/fixtures/elaboration-expected-changes.json').read_text())
if expected['entries']:
    raise SystemExit('audio expected-change ledger is not empty')
ledger = (root / 'docs/plan/clean-break-ledger.md').read_text()
if 'reassigned; live' in ledger or 'reassigned with' in ledger:
    raise SystemExit('clean-break ledger still has a live reassigned row')
events_manifest = (root / 'crates/musa-events/Cargo.toml').read_text()
for forbidden in ('musa-score', 'musa-compiler', 'musa-dsp', 'musa-project'):
    if forbidden in events_manifest:
        raise SystemExit(f'musa-events depends on {forbidden}')
for source in (
    'stdlib/src/performance/mod.musa',
    'stdlib/src/sound/catalogue.musa',
    'stdlib/src/sound/instrument.musa',
    'stdlib/src/sound/graph.musa',
    'stdlib/src/sound/sample.musa',
    'stdlib/src/sound/media.musa',
    'stdlib/src/sound/quantity.musa',
):
    if not (root / source).is_file():
        raise SystemExit(f'missing source-owned declaration module: {source}')
PY

if rg -n 'pub (struct|enum|type) (StudioSpec|InstrumentSpec|WrittenQuantity)|pub fn (compile_graph|lift)\b' crates/*/src; then
    printf '%s\n' 'authoritative or legacy public sound path remains' >&2
    exit 1
fi

printf '%s\n' 'audio language conformance selections, ledgers, ownership, and dependency audits: ok'
