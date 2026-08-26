//! Every `musa-project` integration test, linked as one binary.
//!
//! One test target per file means one more link of the whole workspace on every
//! build, and one more set of object files that cargo never reclaims from
//! `target/debug/deps`. See `docs/notes/toolchain/slow-test-suite.md`.

mod analysis_session_laws;
mod assets_laws;
mod barline_laws;
mod editing_laws;
mod elaboration_backend_compatibility;
mod large_score_generators;
mod library_laws;
mod logging_laws;
mod package_laws;
mod project_files_laws;
mod project_laws;
mod provenance_laws;
mod realization_laws;
mod resource_session;
mod sampler_laws;
mod session_laws;
mod studio_laws;
mod transcription_trial_laws;
mod ui_fixtures_generators;
mod wire_laws;
