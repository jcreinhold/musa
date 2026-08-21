//! Universe sorts: the two there are.
//!
//! `02-core-calculus.md` §1: "Two universes, fixed. `Type 0 : Type 1`, and there
//! is no `Type : Type`, no `Type 2`, and no level a program can write or a
//! checker can solve." This module is that sentence as a type.
//!
//! The previous representation — a normalized `max(k, ℓ₁ + k₁, …)` with level
//! metavariables — existed for universe-polymorphic declarations, and the course
//! correction's audit found none. What remains is an ordering with two points,
//! and the only arithmetic is the two rules a formation judgment uses: `succ`
//! for the universe a `Type l` inhabits, and `max` for the join a record type or
//! enumeration takes of its parts.

/// A universe: `Type 0` or `Type 1`, fixed.
///
/// Named `Sort` and not `Level` because [`crate::term::Level`] is a position in
/// an environment, and one crate cannot spell two unrelated numberings the same
/// way and expect the mix-up to be caught. Prompt 152 gives this type its
/// contents; what it has today is the two points §1 fixes.
///
/// Ordered `Zero < One`. Conversion compares sorts with `==`, never with `<`:
/// universes are predicative and not cumulative, so `Type 0` is not a `Type 1`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Sort {
    /// `Type 0` — where every type a program writes lives.
    Zero,
    /// `Type 1` — where a type of types stands: a record type constructor over
    /// `Type`, an enumeration's own signature.
    One,
}

impl Sort {
    /// The lowest universe, `Type 0`.
    pub const ZERO: Self = Self::Zero;

    /// `succ l`: the universe that `Type l` itself inhabits.
    ///
    /// `None` at `One`: there is no `Type 2`, and the caller turns that into the
    /// refusal a declaration that needs it gets.
    #[must_use]
    pub const fn succ(&self) -> Option<Self> {
        match self {
            Self::Zero => Some(Self::One),
            Self::One => None,
        }
    }

    /// `max l l'`, the level a formation rule assigns when it combines two.
    #[must_use]
    pub const fn max(self, other: Self) -> Self {
        match (self, other) {
            (Self::One, _) | (_, Self::One) => Self::One,
            (Self::Zero, Self::Zero) => Self::Zero,
        }
    }
}

impl core::fmt::Display for Sort {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.write_str(match self {
            Self::Zero => "0",
            Self::One => "1",
        })
    }
}
