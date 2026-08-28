//! One test binary for the crate (`AGENTS.md`).
//!
//! Exercising a C ABI means writing C calls, so this suite carries the same
//! documented exception the crate root does; `../../TRUST.md` says what it
//! buys and what holds it up.

#![allow(
    unsafe_code,
    reason = "these tests call the C ABI they are testing, which has no safe spelling"
)]
// The same allowances the other crates' suites take, for the same reasons.
// A helper that `expect`s a statically-valid input is asserting; a test that
// indexes a buffer it just filled is reading what it wrote; and comparing two
// renders bit for bit is the point of a differential, not a float mistake.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::float_cmp)]

mod allocation;
mod boundary;
mod controls;
mod differential;
mod header;
