//! Every `musa-dsp` integration test, linked as one binary.
//!
//! One test target per file means one more link of the whole workspace on every
//! build, and one more set of object files that cargo never reclaims from
//! `target/debug/deps`. See `docs/notes/toolchain/slow-test-suite.md`.

mod audio;
mod conformance_programs;
mod dsp_laws;
mod effects_laws;
mod elaboration_compatibility;
mod machine;
mod rt;
mod schedule;
mod studio;
mod support;
mod synth;
