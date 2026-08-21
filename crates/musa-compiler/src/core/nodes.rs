//! One concern of the enclosing module; see its module docs.

use musa_syntax::{SyntaxKind, SyntaxNode};

use super::Type;

/// The type a written name denotes in an adapter module, and nowhere else.
///
/// Deliberately absent from `musa-syntax`'s `BASE_TYPES`: these are not
/// spellings the parser offers, the language server completes, or a composer
/// can write. They are read only where [`crate::data::TypeScope::in_phase`]
/// holds, which is the same boundary [`Reading::Expansion`] draws for the
/// phase's operations — one line between the two languages rather than two.
///
/// The four here take no arguments. `Syntax<Cat>` and `SyntaxStep<C, A>` do, so
/// they are read where the other applied forms are, under the same `in_phase`
/// gate.
///
/// `Syntax` is deliberately absent from the bare list. It takes a category, so
/// the bare word names no type — the same reason `Duration` is absent from
/// [`named_type`], and for the same reason an adapter that writes it is told
/// what to write instead rather than being handed one category by default.
pub(crate) fn phase_type(text: &str) -> Option<Type> {
    match text {
        "NodePath" => Some(Type::NodePath),
        "BindingPath" => Some(Type::BindingPath),
        "TokenKind" => Some(Type::TokenKind),
        "Delimiter" => Some(Type::Delimiter),
        _ => None,
    }
}

pub(crate) fn child_of(node: &SyntaxNode, predicate: fn(SyntaxKind) -> bool) -> Option<SyntaxNode> {
    node.children().find(|child| predicate(child.kind()))
}

pub(crate) fn is_type_node(kind: SyntaxKind) -> bool {
    musa_syntax::ast::is_type(kind)
}

/// The type a declaration or parameter annotates, as a node.
pub(crate) fn type_node_of(node: &SyntaxNode) -> Option<SyntaxNode> {
    child_of(node, is_type_node)
}

/// The expression a wrapper node holds, as a node.
pub(crate) fn expr_node_of(node: &SyntaxNode) -> Option<SyntaxNode> {
    child_of(node, is_expr_node)
}

pub(crate) fn is_expr_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::NameExpr
            | SyntaxKind::LiteralExpr
            | SyntaxKind::ParenExpr
            | SyntaxKind::BlockExpr
            | SyntaxKind::ProductExpr
            | SyntaxKind::ListExpr
            | SyntaxKind::OptionExpr
            | SyntaxKind::ResultExpr
            | SyntaxKind::ApplyExpr
            | SyntaxKind::LambdaExpr
            | SyntaxKind::PitchExpr
            | SyntaxKind::ChordExpr
            | SyntaxKind::ScaleExpr
            | SyntaxKind::KeyExpr
            | SyntaxKind::StepExpr
            | SyntaxKind::MatchExpr
            | SyntaxKind::IfExpr
            | SyntaxKind::RecordUpdateExpr
            | SyntaxKind::QuestionExpr
            | SyntaxKind::MusicExpr
            | SyntaxKind::EventsQuote
            | SyntaxKind::QuoteExpr
    )
}
