//! Every `musa-render` integration test, linked as one binary.
//!
//! One test target per file means one more link of the whole workspace on every
//! build, and one more set of object files that cargo never reclaims from
//! `target/debug/deps`. See `docs/notes/toolchain/slow-test-suite.md`.

mod annotations;
mod chords;
mod curves;
mod graces;
mod lilypond;
mod mei;
mod midi;
mod musicxml;
mod open_form;
mod pitch_capabilities;
mod plan;
mod polymeter;
mod ramps;
mod unmeasured;
