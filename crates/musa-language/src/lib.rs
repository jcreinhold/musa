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
//! Public facade (roadmap §15.2):
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
mod edits;
mod error;
mod formatter;
mod highlight;
mod keywords;
mod language;
mod lexer;
mod meter;
mod parser;
mod syntax_kind;
mod types;

pub use crate::edits::{
    Anchor, EditError, EditIntent, HeaderField, Statement, TextEdit, apply_edits, compute_edits, read_header,
    spell_duration,
};
pub use crate::error::SyntaxError;
pub use crate::formatter::{BarSpacing, FormattedSource, format};
pub use crate::highlight::{MODULE_NAME_KEYWORDS, SPELLINGS, TokenClass, classify};
pub use crate::keywords::{KeywordDoc, builtin_doc, keyword_doc};
pub use crate::language::{MusaLanguage, SyntaxElement, SyntaxNode, SyntaxToken};
pub use crate::lexer::{LexError, LexErrorKind, Lexed, Token, lex};
pub use crate::meter::beat_groups;
pub use crate::parser::{ParsedDocument, parse};
pub use crate::syntax_kind::SyntaxKind;
pub use crate::types::{PRIMITIVE_TYPES, RESPELLED_TYPES, respelled_type};
