//! One concern of the enclosing module; see its module docs.

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
