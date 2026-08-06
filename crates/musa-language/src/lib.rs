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
//! Intended facade (roadmap §15.2). `lex` is implemented (prompt 02);
//! `parse`, `format`, and `apply_edits` arrive with prompts 03–04:
//!
//! ```text
//! pub fn lex(source: &str) -> Lexed;
//! pub fn parse(source: &str) -> ParsedDocument;
//! pub fn format(document: &ParsedDocument) -> FormattedSource;
//! pub fn apply_edits(source: &str, edits: &[TextEdit]) -> String;
//! ```
//!
//! Invariants: parsing always returns a tree, even for invalid input; every
//! token, comment, and whitespace span is retained so the tree is lossless
//! (`tree.text() == source`); the formatter preserves semantics
//! (`semantic(parse(format(parse(s)))) == semantic(parse(s))`).

pub mod ast;
mod error;
mod language;
mod lexer;
mod parser;
mod syntax_kind;

pub use crate::error::SyntaxError;
pub use crate::language::{MusaLanguage, SyntaxElement, SyntaxNode, SyntaxToken};
pub use crate::lexer::{LexError, LexErrorKind, Lexed, Token, lex};
pub use crate::parser::{ParsedDocument, parse};
pub use crate::syntax_kind::SyntaxKind;
