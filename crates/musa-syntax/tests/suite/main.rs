//! Every `musa-syntax` integration test, linked as one binary.
//!
//! One test target per file means one more link of the whole workspace on every
//! build, and one more set of object files that cargo never reclaims from
//! `target/debug/deps`. See `docs/notes/toolchain/slow-test-suite.md`.

mod adapter_region_laws;
mod editing_laws;
mod expression_syntax_laws;
mod formatter;
mod highlight_laws;
mod literal_parts_laws;
mod parser;
mod quotation_syntax_laws;
mod record_syntax_laws;
mod root_recovery_laws;
mod text_encoding_laws;
mod tree_sitter_fixtures;
mod visibility_syntax_laws;
