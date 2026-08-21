//! See `ast` module docs; the items parsed in this family.

use super::AstNode;
use super::span_of;
use super::wrapper;
use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode};

/// `kernel EventTrack[WrittenTime, ScoreFact] { ... }` — a quoted composition
/// expression.
///
/// The interior is the kernel's grammar and this crate does not read it
/// (`docs/rules/language/01-surface.md` §7). What it offers is what a *host* needs:
/// which coordinate and payload type the quote claims, where its body is, and
/// where the holes are — so the compiler can cut the body into text and typed
/// antiquotations and hand the text to the one crate that owns the grammar.
pub struct KernelQuote(SyntaxNode);
wrapper!(KernelQuote, SyntaxKind::KernelQuote);

impl KernelQuote {
    /// The type constructor as written, with its span. `EventTrack`, or the
    /// mistake in its place.
    pub fn constructor(&self) -> Option<(String, (u32, u32))> {
        self.identifiers().next()
    }

    /// The coordinate as written, with its span — the first of the two type
    /// arguments, and the one that says which time the quote is written in.
    pub fn coordinate(&self) -> Option<(String, (u32, u32))> {
        self.identifiers().nth(1)
    }

    /// The payload type as written, with its span.
    pub fn payload_type(&self) -> Option<(String, (u32, u32))> {
        self.identifiers().nth(2)
    }

    /// The body's byte range: everything strictly inside the braces.
    ///
    /// `None` for a quote whose braces the parser never found, which is a
    /// quote that has already been reported as a syntax error.
    pub fn body_span(&self) -> Option<(u32, u32)> {
        let open = self
            .0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::LBrace)?;
        let close = self
            .0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| token.kind() == SyntaxKind::RBrace)
            .last()?;
        Some((span_of(&open).1, span_of(&close).0))
    }

    /// The antiquotations, in source order.
    pub fn holes(&self) -> Vec<KernelHole> {
        self.0.children().filter_map(KernelHole::cast).collect()
    }

    fn identifiers(&self) -> impl Iterator<Item = (String, (u32, u32))> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| token.kind() == SyntaxKind::Identifier)
            .map(|token| (token.text().to_string(), span_of(&token)))
    }
}

/// `${ expr }` — one typed antiquotation.
pub struct KernelHole(SyntaxNode);
wrapper!(KernelHole, SyntaxKind::KernelHole);

impl KernelHole {
    /// The host expression spliced here.
    pub fn expr(&self) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() != SyntaxKind::KernelHole)
    }

    /// The hole's own byte range, `$` through `}`.
    pub fn span(&self) -> (u32, u32) {
        let range = self.0.text_range();
        (u32::from(range.start()), u32::from(range.end()))
    }
}

/// `quote at here { … }` — a syntax quotation
/// (`docs/rules/language/11-quotation.md` §2).
///
/// Two parts and no third: the anchor a caller evaluates, and the body a
/// caller *reads as a tree*. There is deliberately no accessor for "the
/// splices", the way [`KernelQuote::holes`] has one — a splice's meaning
/// depends on where in the body it stands, so a flat list of them would be a
/// list with the one fact about each of them removed.
pub struct QuoteExpr(SyntaxNode);
wrapper!(QuoteExpr, SyntaxKind::QuoteExpr);

impl QuoteExpr {
    /// The expression after `at`: the node this quote's output is derived
    /// from.
    ///
    /// By position rather than by kind, because a quote has exactly two child
    /// nodes and they are these two. Asking "which kinds are expressions"
    /// would be a second list of the grammar's expression forms, kept beside
    /// the parser's and drifting from it.
    pub fn anchor(&self) -> Option<SyntaxNode> {
        self.0.children().next()
    }

    /// The quoted expression itself, between the braces.
    pub fn body(&self) -> Option<SyntaxNode> {
        self.0.children().nth(1)
    }
}

/// `quote { … }` in a pattern — the inverse form
/// (`docs/rules/language/11-quotation.md` §4).
///
/// One part, where [`QuoteExpr`] has two. The anchor is missing because a
/// pattern derives nothing: there is no node to be the origin of, so there is
/// no place for one to be named.
pub struct QuotePattern(SyntaxNode);
wrapper!(QuotePattern, SyntaxKind::QuotePattern);

impl QuotePattern {
    /// The quoted shape itself, between the braces.
    pub fn body(&self) -> Option<SyntaxNode> {
        self.0.children().next()
    }
}

/// `$x` or `${ e }` — one value spliced where one node stands.
pub struct Splice(SyntaxNode);
wrapper!(Splice, SyntaxKind::Splice);

impl Splice {
    /// The host expression whose value is spliced.
    ///
    /// The same accessor for both spellings, because `$x` *is* `${ x }` with
    /// the braces left off: the shorthand parses a [`SyntaxKind::NameExpr`]
    /// here, so nothing downstream has to know which was written.
    pub fn expr(&self) -> Option<SyntaxNode> {
        self.0.children().next()
    }
}

/// `$..xs` — a list of values spliced where a sequence stands.
pub struct SequenceSplice(SyntaxNode);
wrapper!(SequenceSplice, SyntaxKind::SequenceSplice);

impl SequenceSplice {
    /// The name of the list spliced here.
    pub fn name(&self) -> Option<String> {
        self.0
            .descendants_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .map(|token| token.text().to_string())
    }
}
