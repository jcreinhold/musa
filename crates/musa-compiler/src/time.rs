//! Exact musical time (roadmap §5.1): nonnegative rationals where `1` is a
//! whole note. No floats in the compositional model.

// Rational arithmetic on `Ratio<i64>` is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use num_rational::Ratio;
use serde::{Deserialize, Serialize};

/// A point in musical time, in whole notes from the piece start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MusicalTime(Ratio<i64>);

impl MusicalTime {
    /// The start of the piece.
    pub const ZERO: Self = Self(Ratio::ZERO);

    /// Create a time from a whole-note ratio, clamped to be nonnegative.
    pub fn new(value: Ratio<i64>) -> Self {
        if value < Ratio::ZERO { Self::ZERO } else { Self(value) }
    }

    /// The value as a ratio of a whole note.
    pub fn as_ratio(self) -> Ratio<i64> {
        self.0
    }

    /// Read a time spelled the way the language spells a duration: `3`,
    /// `1/2`, `7/8` — whole notes from the piece start.
    ///
    /// `None` for anything outside that spelling, including a zero
    /// denominator and a negative value, so a caller reporting "that is not a
    /// position" never has to decide what a bad one meant.
    pub fn parse(text: &str) -> Option<Self> {
        let (numerator, denominator) = match text.split_once('/') {
            Some((numerator, denominator)) => (numerator, denominator.parse::<i64>().ok()?),
            None => (text, 1),
        };
        let numerator = numerator.parse::<i64>().ok()?;
        if denominator <= 0 || numerator < 0 {
            return None;
        }
        Some(Self(Ratio::new(numerator, denominator)))
    }
}

impl std::fmt::Display for MusicalTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add<MusicalDuration> for MusicalTime {
    type Output = Self;

    fn add(self, rhs: MusicalDuration) -> Self {
        Self::new(self.0 + rhs.as_ratio())
    }
}

impl std::ops::Sub for MusicalTime {
    type Output = MusicalDuration;

    fn sub(self, rhs: Self) -> MusicalDuration {
        MusicalDuration::new(self.0 - rhs.0)
    }
}

/// A length of musical time, in whole notes (roadmap §5.1's additive
/// monoid: `0` identity, `+` associative composition).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MusicalDuration(Ratio<i64>);

impl MusicalDuration {
    /// The identity duration.
    pub const ZERO: Self = Self(Ratio::ZERO);
    /// One whole note.
    pub const WHOLE: Self = Self(Ratio::new_raw(1, 1));

    /// Create a duration from a whole-note ratio, clamped to be nonnegative.
    pub fn new(value: Ratio<i64>) -> Self {
        if value < Ratio::ZERO { Self::ZERO } else { Self(value) }
    }

    /// The value as a ratio of a whole note.
    pub fn as_ratio(self) -> Ratio<i64> {
        self.0
    }

    /// Whether this duration is zero.
    pub fn is_zero(self) -> bool {
        self.0 == Ratio::ZERO
    }
}

impl std::fmt::Display for MusicalDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add for MusicalDuration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for MusicalDuration {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::iter::Sum for MusicalDuration {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |total, item| total + item)
    }
}
