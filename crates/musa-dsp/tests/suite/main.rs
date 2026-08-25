//! Every `musa-dsp` integration test, linked as one binary.
//!
//! One test target per file means one more link of the whole workspace on every
//! build, and one more set of object files that cargo never reclaims from
//! `target/debug/deps`. See `docs/notes/toolchain/slow-test-suite.md`.

mod audio_bridge_laws;
mod audio_conformance_programs;
mod audio_elaboration_compatibility;
mod audio_support;
mod checked_source_laws;
mod dsp_laws;
mod effects_laws;
mod expressive_control_laws;
mod machine;
mod routing_laws;
mod rt;
mod sampler_laws;
mod schedule;
mod studio_laws;
mod studio_lowering_laws;
mod synth_laws;
