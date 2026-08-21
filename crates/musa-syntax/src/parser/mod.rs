//! Hand-written recursive-descent parser producing a lossless Rowan tree.
//!
//! Architecture (roadmap §10.2–§10.4): the parser emits a flat event stream
//! (`StartNode` / `Token` / `FinishNode`); a second pass replays the events
//! into a `rowan::GreenNodeBuilder`. Parser code never touches Rowan. The
//! grammar is small enough that statements dispatch on their first token. A
//! postfix call inserts its parent start event at the primary's checkpoint;
//! this is the one place expression precedence needs a forward parent.
//!
//! The grammar is split vertically by concern; every submodule parses with
//! the same cursor from [`engine`] and shares this module's re-exports:
//!
//! - [`engine`] — the token cursor, event emission, recovery, and helpers
//! - [`documents`] — file shapes: document roots, imports, front matter
//! - [`declarations`] — signature, data, record, enum, trait, impl, structure
//! - [`functions`] — `let`, `fn`, trait methods, signatures, parameters
//! - [`types`] — type expressions and respellings
//! - [`expressions`] — the expression grammar: precedence, atoms, records
//! - [`pitches`] — written-pitch operators, scale and key expressions
//! - [`patterns`] — match patterns and constructor bindings
//! - [`quotation`] — quotes, splices, and kernel holes
//! - [`notes`] — note items: notes, rests, chords, stacks, ties, durations
//! - [`voice`] — voice-item statements beyond notes
//! - [`score`] — score scaffolding: parts, voices, motif/fragment
//! - [`harmony`] — harmonic analysis
//! - [`studio`] — the studio graph and performance profiles
//!
//! Recovery: an unexpected token records an error, then tokens are wrapped in
//! an `ERROR` node until a recovery point (`;`, `}`, or the next declaration
//! keyword), and parsing continues. The parser always returns a tree.

mod declarations;
mod documents;
mod engine;
mod expressions;
mod functions;
mod harmony;
mod notes;
mod patterns;
mod pitches;
mod quotation;
mod score;
mod studio;
mod types;
mod voice;

use crate::language::SyntaxNode;
use crate::{LexErrorKind, SyntaxError, lex};
use engine::Parser;

pub(crate) use documents::MODULE_NAME;

/// The result of parsing a source string.
///
/// Always contains a tree, even for invalid input (roadmap §10.3). Check
/// [`ParsedDocument::errors`] for diagnostics.
pub struct ParsedDocument {
    node: SyntaxNode,
    source: Box<str>,
    errors: Vec<SyntaxError>,
}

impl ParsedDocument {
    /// The root of the lossless concrete syntax tree. Cloning the node is
    /// cheap (reference-counted).
    ///
    /// Invariant: the tree's text equals the parsed source exactly.
    pub fn syntax(&self) -> SyntaxNode {
        self.node.clone()
    }

    /// The text this document was parsed from.
    ///
    /// Kept because the tree is lossless but not *contiguous*: rendering it
    /// back to a `String` walks every leaf and concatenates it, and a pass
    /// that wants to look at the source by byte offset — a lint reading the
    /// line around a diagnostic, say — would pay that walk once per question.
    /// The parser was handed this text, so it hands it on rather than making
    /// the next stage reconstruct it.
    pub fn text(&self) -> &str {
        &self.source
    }

    /// Lexical and parse errors with source spans, in source order.
    pub fn errors(&self) -> &[SyntaxError] {
        &self.errors
    }
}

/// Parse `source` into a lossless tree plus diagnostics.
pub fn parse(source: &str) -> ParsedDocument {
    let lexed = lex(source);
    let mut errors: Vec<SyntaxError> = lexed
        .errors()
        .iter()
        .map(|error| {
            let (message, label, help) = match error.kind() {
                LexErrorKind::InvalidToken => (
                    "musa does not read this",
                    "not part of the language",
                    "check for a stray character, or a word from another notation",
                ),
                LexErrorKind::UnterminatedString => (
                    "this text has no closing quote",
                    "the line ends here",
                    "a title, a name, or a line of front matter is one quoted line",
                ),
                LexErrorKind::UnterminatedBlockComment => (
                    "this comment is never closed",
                    "opened here",
                    "close it with `*/`, or use `//` for one line",
                ),
            };
            SyntaxError::new(error.range(), message, label).with_help(help)
        })
        .collect();
    let node = Parser::new(source, &lexed).run();
    errors.extend(node.1);
    ParsedDocument {
        node: node.0,
        source: source.into(),
        errors,
    }
}
