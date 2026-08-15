//! Universe levels.
//!
//! `docs/rules/language/02-core-calculus.md` §1 gives levels their own sort,
//! `0 | succ l | max l l | ℓ`, where `ℓ` is a level metavariable the elaborator
//! solves. Prompt 133 has no metavariables, so every level a term can hold is
//! closed, and a closed level built from `0`, `succ`, and `max` *is* a natural
//! number — `max` computes. The representation is therefore that number.
//!
//! When prompt 135 adds level metavariables — alongside the level-polymorphic
//! families that are the first declarations unable to write their own levels —
//! this becomes a sum, and `succ` and `max` stop computing on the variable arms.
//! Keeping the three operations as the only way to build a level is what makes
//! that change local: no caller writes a numeral, so no caller has to learn
//! about the arms it grows.

/// A universe level.
///
/// Ordered, and the order is the `l ≤ l'` the elaborator will check level
/// constraints with. Note that this order is **not** a subtyping relation:
/// universes are predicative and *not* cumulative (§1), so `Type 0` is not a
/// `Type 1` and conversion compares levels for equality, never for inclusion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level(u32);

impl Level {
    /// The lowest universe, `Type 0` — where every type a program writes lives.
    pub const ZERO: Self = Self(0);

    /// `succ l`, the level of the universe that `Type l` itself inhabits.
    ///
    /// Saturating rather than wrapping: a level past `u32::MAX` is not a
    /// program anybody wrote, and wrapping would make `Type (succ l)` inhabit
    /// `Type 0`, which is `Type : Type` reached by arithmetic. Saturation keeps
    /// the hierarchy monotone at the cost of collapsing two levels no source
    /// can name.
    #[must_use]
    pub const fn succ(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// `max l l'`, the level a formation rule assigns when it combines two.
    #[must_use]
    pub const fn max(self, other: Self) -> Self {
        if self.0 < other.0 { other } else { self }
    }

    /// The level as a number, for a diagnostic that has to print it.
    #[must_use]
    pub const fn depth(self) -> u32 {
        self.0
    }
}

impl core::fmt::Display for Level {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Level;

    #[test]
    fn max_is_the_join_of_two_levels() {
        let zero = Level::ZERO;
        let one = zero.succ();
        assert_eq!(zero.max(one), one);
        assert_eq!(one.max(zero), one);
        assert_eq!(one.max(one), one);
    }

    #[test]
    fn the_hierarchy_is_strictly_increasing_and_never_wraps() {
        let mut level = Level::ZERO;
        for _ in 0..8 {
            let next = level.succ();
            assert!(level < next, "succ must climb");
            level = next;
        }
    }
}
