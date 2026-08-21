//! A fresh-variable minter for rank-1 signature instantiation.
//!
//! The registry's rank-1 signatures (`crate::phase::Type` operations like
//! `SyntaxOp::instantiate`) take fresh type variables per instantiation site.
//! Unification, generalization, and schemes left with the old checker;
//! `musa-calculus` owns typed elaboration now.
//!
//! Nothing here is public.

use crate::phase::Type;

/// A type variable, named by the [`Minter`] that made it.
pub(crate) type TypeVar = u32;

/// The minter: one counter, one fresh [`Type::Var`] per instantiation site.
///
/// One minter serves the whole piece, so a variable minted at one
/// instantiation site means the same thing wherever it is read.
#[derive(Default)]
pub(crate) struct Minter {
    next: TypeVar,
}

impl Minter {
    /// A variable nothing has said anything about yet.
    pub(crate) fn fresh(&mut self) -> Type {
        let variable = self.next;
        self.next = self.next.saturating_add(1);
        Type::Var(variable)
    }
}
