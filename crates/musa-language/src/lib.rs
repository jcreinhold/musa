//! Language frontend for `.musa` source.
//!
//! Owns: tokens, the `logos`-based lexer, the hand-written recursive-descent
//! parser, the lossless `rowan` concrete syntax tree, typed syntax wrappers,
//! the formatter, syntax diagnostics, and text-edit utilities (design roadmap
//! §15.2).
//!
//! Must never expose: Logos token iterators or Rowan green-node construction
//! details beyond an intentional syntax API; any resolved names, computed
//! pitches, expanded motifs, or audio objects (those belong to
//! `musa-compiler`).
//!
//! Intended facade (roadmap §15.2), to be implemented by prompts 02–04:
//!
//! ```text
//! pub fn parse(source: &str) -> ParsedDocument;
//! pub fn format(document: &ParsedDocument) -> FormattedSource;
//! pub fn apply_edits(source: &str, edits: &[TextEdit]) -> String;
//! ```
//!
//! Invariants: parsing always returns a tree, even for invalid input; every
//! token, comment, and whitespace span is retained so the tree is lossless
//! (`tree.text() == source`); the formatter preserves semantics
//! (`semantic(parse(format(parse(s)))) == semantic(parse(s))`).
