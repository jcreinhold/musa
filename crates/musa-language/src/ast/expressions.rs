//! See `ast` module docs; the items parsed in this family.

use super::FnParam;
use super::child;
use super::children;
use super::params_of;
use super::token_text;
use super::wrapper;
use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode};

/// `fn (x: τ, …) -> τ { e }` — an anonymous function in a value position.
///
/// It reads the same way `FnDecl` does, minus the name: the parts a reader
/// already knows from a declaration, standing where a value stands.
pub struct LambdaExpr(SyntaxNode);
wrapper!(LambdaExpr, SyntaxKind::LambdaExpr);

impl LambdaExpr {
    /// Its parameters in source order, annotated or not.
    pub fn params(&self) -> Vec<FnParam> {
        params_of(&self.0)
    }
}

/// A value reference.
pub struct NameExpr(SyntaxNode);
wrapper!(NameExpr, SyntaxKind::NameExpr);

/// A scalar literal expression.
pub struct LiteralExpr(SyntaxNode);
wrapper!(LiteralExpr, SyntaxKind::LiteralExpr);

/// A parenthesized expression.
pub struct ParenExpr(SyntaxNode);
wrapper!(ParenExpr, SyntaxKind::ParenExpr);

/// A product value.
pub struct ProductExpr(SyntaxNode);
wrapper!(ProductExpr, SyntaxKind::ProductExpr);

/// A finite list value.
pub struct ListExpr(SyntaxNode);
wrapper!(ListExpr, SyntaxKind::ListExpr);

/// `some(value)` or `none`.
pub struct OptionExpr(SyntaxNode);
wrapper!(OptionExpr, SyntaxKind::OptionExpr);

/// `Ok(value)` or `Err(reason)`.
pub struct ResultExpr(SyntaxNode);
wrapper!(ResultExpr, SyntaxKind::ResultExpr);

/// Ordinary function application.
pub struct ApplyExpr(SyntaxNode);
wrapper!(ApplyExpr, SyntaxKind::ApplyExpr);

/// The arguments of an ordinary application.
pub struct ExprArgList(SyntaxNode);
wrapper!(ExprArgList, SyntaxKind::ExprArgList);

/// One positional or named application argument.
pub struct ExprArg(SyntaxNode);
wrapper!(ExprArg, SyntaxKind::ExprArg);

/// Exhaustive finite case analysis.
pub struct MatchExpr(SyntaxNode);
wrapper!(MatchExpr, SyntaxKind::MatchExpr);

impl MatchExpr {
    /// The alternatives in source order.
    pub fn arms(&self) -> Vec<MatchArm> {
        children(&self.0)
    }
}

/// One match alternative.
pub struct MatchArm(SyntaxNode);
wrapper!(MatchArm, SyntaxKind::MatchArm);

/// A match pattern.
pub struct Pattern(SyntaxNode);
wrapper!(Pattern, SyntaxKind::Pattern);

impl Pattern {
    /// The record pattern this is, if it is one.
    ///
    /// A record pattern is the one shape that is *not* a case analysis — a
    /// record has one shape, so naming its fields opens them rather than
    /// choosing among alternatives — and a reader asking which arms split on a
    /// constructor needs to be able to tell.
    pub fn record(&self) -> Option<RecordPattern> {
        child(&self.0)
    }
}

/// `Pending { read = r, taken }` — a record pattern.
pub struct RecordPattern(SyntaxNode);
wrapper!(RecordPattern, SyntaxKind::RecordPattern);

impl RecordPattern {
    /// The fields it names, in written order. It says nothing about the rest.
    pub fn fields(&self) -> Vec<FieldPattern> {
        children(&self.0)
    }
}

/// `read = r`, or the `taken` shorthand — one field of a [`RecordPattern`].
pub struct FieldPattern(SyntaxNode);
wrapper!(FieldPattern, SyntaxKind::FieldPattern);

impl FieldPattern {
    /// The field's name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The pattern the field is matched against, or `None` for the shorthand
    /// that binds the field to its own name.
    pub fn pattern(&self) -> Option<Pattern> {
        child(&self.0)
    }
}

/// `if condition { consequent } else { alternative }`.
///
/// Surface syntax the compiler elaborates to the two-arm boolean match, so
/// nothing downstream of elaboration has an `if`. What a *reader* of the tree
/// needs is which of the three children is which, and the answer is positional:
/// the condition is the first expression, the consequent the second, the
/// alternative the third — a nested `if` in that last position being an `else
/// if` rung.
pub struct IfExpr(SyntaxNode);
wrapper!(IfExpr, SyntaxKind::IfExpr);

/// `subject with { read.refusal = why, ... }`.
///
/// Surface syntax the compiler elaborates to one `let` and one record literal
/// per path segment, so nothing downstream of elaboration has an update. The
/// subject is the first expression child; the [`FieldUpdate`]s follow it in
/// written order, which is the order their diagnostics come in.
pub struct RecordUpdateExpr(SyntaxNode);
wrapper!(RecordUpdateExpr, SyntaxKind::RecordUpdateExpr);

/// `read.refusal = why` — one replaced place of a [`RecordUpdateExpr`].
pub struct FieldUpdate(SyntaxNode);
wrapper!(FieldUpdate, SyntaxKind::FieldUpdate);

impl FieldUpdate {
    /// The path this replaces, outermost field first.
    ///
    /// Handed over rather than left to be re-derived: the parser has already
    /// decided which identifiers before the `=` are the path, and a consumer
    /// that filtered tokens itself would have to know that the ones after it
    /// are not.
    pub fn path(&self) -> Vec<String> {
        child::<FieldPath>(&self.0)
            .map(|path| path.segments())
            .unwrap_or_default()
    }

    /// The expression the path's field is replaced by.
    pub fn value(&self) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() != SyntaxKind::FieldPath)
    }
}

/// `read.refusal` — the path one [`FieldUpdate`] replaces.
pub struct FieldPath(SyntaxNode);
wrapper!(FieldPath, SyntaxKind::FieldPath);

impl FieldPath {
    /// The field names, outermost first. Never empty in a tree that parsed.
    pub fn segments(&self) -> Vec<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| token.kind() == SyntaxKind::Identifier)
            .map(|token| token.text().to_string())
            .collect()
    }
}

/// `Pending { read = r, dots = d }` — a record literal.
///
/// There is no positional form: construction names every field, in any order,
/// and the declaration's order is the evaluation order (`01-surface.md` §1.2).
pub struct RecordLiteralExpr(SyntaxNode);
wrapper!(RecordLiteralExpr, SyntaxKind::RecordLiteralExpr);

impl RecordLiteralExpr {
    /// The written head: the type name, or the qualified case of an enum.
    pub fn head(&self) -> Option<SyntaxNode> {
        self.0
            .children()
            .find(|child| matches!(child.kind(), SyntaxKind::NameExpr | SyntaxKind::PathExpr))
    }

    /// Its fields, in written order.
    pub fn fields(&self) -> Vec<FieldInit> {
        children(&self.0)
    }
}

/// `read = r` — one field of a [`RecordLiteralExpr`].
pub struct FieldInit(SyntaxNode);
wrapper!(FieldInit, SyntaxKind::FieldInit);

impl FieldInit {
    /// The field's name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The expression it is given.
    pub fn value(&self) -> Option<SyntaxNode> {
        self.0.children().next()
    }
}

/// `Tying::Untied` — a name in a type's namespace.
pub struct PathExpr(SyntaxNode);
wrapper!(PathExpr, SyntaxKind::PathExpr);

impl PathExpr {
    /// The segments, left to right. §1.5's capitalization rule is what decides
    /// where the module prefix ends; the parser counted nothing.
    pub fn segments(&self) -> Vec<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| !token.kind().is_trivia() && token.kind() != SyntaxKind::Colon)
            .map(|token| token.text().to_string())
            .collect()
    }
}

/// `subject?` — propagate a `Result`'s failure out of the function.
///
/// Surface syntax the compiler elaborates to the exhaustive `Result` match,
/// so nothing downstream of elaboration has a question. The subject is the one
/// expression child; the `?` itself is the last token.
pub struct QuestionExpr(SyntaxNode);
wrapper!(QuestionExpr, SyntaxKind::QuestionExpr);
