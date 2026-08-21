//! Where a core term came from.
//!
//! `docs/rules/language/02-core-calculus.md` §7:
//!
//! > Every core term records the surface node it was elaborated from. This is a
//! > property of the representation, not a debugging convenience.
//!
//! It is normative for three reasons, and the first is the one this crate feels:
//! a dependent checker reports failures in terms the author never wrote. A
//! conversion failure is between two normal forms produced by `quote`, and a
//! normal form is not source. Without an origin on the terms involved, the best
//! available diagnostic is two unfamiliar expressions and no place to point.
//!
//! **An origin is opaque here.** `musa-calculus` is a leaf: it does not know what a
//! file is, what a span is, or what a syntax node is, and it must not learn.
//! What it carries is a number the caller assigned, and the only thing it does
//! with that number is keep it attached to the right term.
//!
//! Origins are **not** part of conversion (§7), and that exclusion is enforced
//! by [`crate::Term`]'s hand-written `PartialEq` rather than remembered at each
//! comparison site. Otherwise provenance would change what a program means, and
//! a compiler that type-checked differently after a file was moved would be the
//! result.

/// The surface node a term was elaborated from.
///
/// `Copy` and one word wide: every term holds one, and a term that had to
/// allocate to say where it came from would make provenance a thing worth
/// switching off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Origin(u32);

impl Origin {
    /// A term nobody wrote.
    ///
    /// The variable quotation invents at a binder, the shape a fresh context
    /// extension stands for. This crate never fabricates a plausible-looking
    /// origin in their place: a wrong origin points a reader at code that is not
    /// the cause, which is worse than pointing nowhere.
    pub const UNKNOWN: Self = Self(0);

    /// The origin naming the caller's node `node`.
    ///
    /// Numbering is the caller's, and `node(0)` is the caller's first node
    /// rather than "unknown" — the two are different answers and a scheme that
    /// spelled them the same would lose one of them at the boundary.
    #[must_use]
    pub const fn node(node: u32) -> Self {
        Self(node.saturating_add(1))
    }

    /// The caller's node number, or `None` for [`Self::UNKNOWN`].
    #[must_use]
    pub const fn node_number(self) -> Option<u32> {
        self.0.checked_sub(1)
    }

    /// Whether this origin names a node.
    #[must_use]
    pub const fn is_known(self) -> bool {
        self.node_number().is_some()
    }
}

impl Default for Origin {
    fn default() -> Self {
        Self::UNKNOWN
    }
}

#[cfg(test)]
mod tests {
    use super::Origin;

    #[test]
    fn the_callers_first_node_is_not_unknown() {
        assert_eq!(Origin::UNKNOWN.node_number(), None);
        assert!(!Origin::UNKNOWN.is_known());
        assert_eq!(Origin::node(0).node_number(), Some(0));
        assert!(Origin::node(0).is_known());
        assert_ne!(Origin::node(0), Origin::UNKNOWN);
    }

    #[test]
    fn distinct_nodes_have_distinct_origins() {
        assert_ne!(Origin::node(1), Origin::node(2));
        assert_eq!(Origin::node(7), Origin::node(7));
    }
}
