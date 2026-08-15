//! Every `musa-compiler` integration test, linked as one binary.
//!
//! One test target per file would be ~50 separate links of the whole workspace
//! on every build, and ~50 fresh sets of object files left in `target/debug/deps`
//! that cargo never reclaims. See `docs/notes/toolchain/slow-test-suite.md`.

mod analysis_laws;
mod annotation_laws;
mod assertion_laws;
mod bars;
mod chord_construction_laws;
mod compiler;
mod complete_call_laws;
mod core_laws;
mod core_validation;
mod curve_laws;
mod derivation_laws;
mod elaboration;
mod elaboration_compatibility;
mod elaboration_fixture_generators;
mod expression_stage;
mod finite_data_laws;
mod graces;
mod groove;
mod higher_order_music_laws;
mod import_laws;
mod inference_laws;
mod inferred_core_laws;
mod kernel_interop;
mod kernel_quote_laws;
mod kernel_subset_laws;
mod key_and_clef_changes;
mod lint_laws;
mod literal_pattern_laws;
mod machine_laws;
mod marks;
mod meter_changes;
mod module_laws;
mod music_compatibility;
mod music_laws;
mod notation_details_laws;
mod notation_marks;
mod open_form;
mod pc12_laws;
mod performance;
mod pitch_action_laws;
mod polymeter;
mod profile_laws;
mod ramps;
mod realize;
mod resource_validation;
mod scale_context_laws;
mod schema_generation_laws;
mod serial_laws;
mod sharing_laws;
mod staff_expansion_laws;
mod staff_package_laws;
mod staff_writing_laws;
mod studio_laws;
mod syntax_category_laws;
mod template_laws;
mod tonal_analysis_validation;
mod tonal_harmony_construction_laws;
mod transform_laws;
mod transformational_harmony_laws;
mod unmeasured;
mod voice_leading_validation;
