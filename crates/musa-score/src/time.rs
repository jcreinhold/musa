//! Exact musical time (roadmap §5.1): nonnegative rationals where `1` is a
//! whole note. No floats in the compositional model.
//!
//! The exactness is the module's subject, so the arithmetic that keeps a
//! rational exact lives here too — [`exact_arithmetic`] and the [`exact_ratio`]
//! it reduces through — as does [`written_rational`], the printer for the
//! literal grid [`MusicalTime::parse`] reads. Both are reached from more than
//! one caller: a duration sum, a position shift, and a plain `Ratio` sum are
//! the same exact operation, and the compiler answers all three from one place
//! rather than from whichever pass happens to be evaluating.

// Rational arithmetic on `Ratio<i64>` is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it. It
// does apply to the wide integer arithmetic the exact operations below reduce
// through, which is why each of those re-arms the lint for itself rather than
// inheriting this.
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

/// The greatest common divisor of two magnitudes, by Euclid.
///
/// Written out because reduction happens in `i128` here: a sum of two
/// representable rationals need not be representable, so the arithmetic is
/// done wide, reduced, and only then asked whether it fits.
#[deny(clippy::arithmetic_side_effects)]
pub(crate) fn greatest_common_divisor(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left.checked_rem(right).unwrap_or(0);
        left = right;
        right = remainder;
    }
    left
}

/// A wide numerator and denominator as an exact `Ratio<i64>`, or nothing.
///
/// Nothing means the reduced value does not fit, which every caller turns into
/// a stated failure rather than a stuck term: D2 forbids partiality anywhere
/// but the result type.
#[deny(clippy::arithmetic_side_effects)]
pub fn exact_ratio(numerator: i128, denominator: i128) -> Option<Ratio<i64>> {
    if denominator == 0 {
        return None;
    }
    let divisor = i128::try_from(greatest_common_divisor(
        numerator.unsigned_abs(),
        denominator.unsigned_abs(),
    ))
    .ok()?;
    let divisor = if divisor == 0 { 1 } else { divisor };
    let (numerator, denominator) = (numerator.checked_div(divisor)?, denominator.checked_div(divisor)?);
    let (numerator, denominator) = if denominator < 0 {
        (numerator.checked_neg()?, denominator.checked_neg()?)
    } else {
        (numerator, denominator)
    };
    Some(Ratio::new(
        i64::try_from(numerator).ok()?,
        i64::try_from(denominator).ok()?,
    ))
}

/// The four exact operations, named apart from the builtins that offer them.
///
/// Separate because the same four are reached from six builtins — a duration
/// sum is a rational sum, a position shift is one too — and because a match on
/// the builtin at the call site would have to name every operation that is
/// *not* one of these four.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Exact {
    Add,
    Sub,
    Mul,
    Div,
}

/// `left · right`, computed wide and reduced before it is asked whether it
/// fits.
#[deny(clippy::arithmetic_side_effects)]
pub fn exact_arithmetic(left: Ratio<i64>, right: Ratio<i64>, operation: Exact) -> Option<Ratio<i64>> {
    let (a, b) = (i128::from(*left.numer()), i128::from(*left.denom()));
    let (c, d) = (i128::from(*right.numer()), i128::from(*right.denom()));
    match operation {
        Exact::Add => exact_ratio(a.checked_mul(d)?.checked_add(c.checked_mul(b)?)?, b.checked_mul(d)?),
        Exact::Sub => exact_ratio(a.checked_mul(d)?.checked_sub(c.checked_mul(b)?)?, b.checked_mul(d)?),
        Exact::Mul => exact_ratio(a.checked_mul(c)?, b.checked_mul(d)?),
        Exact::Div => exact_ratio(a.checked_mul(d)?, b.checked_mul(c)?),
    }
}

/// The source literal that names an exact rational, or nothing.
///
/// `p/q`, and `p` where the denominator is one, which is how the reader writes
/// a whole note — the same grid [`MusicalTime::parse`] reads. Nothing below
/// zero: the grammar has no negative numeric literal, so a `-7/6` spelled here
/// would be text the reader would not read back, and this family's one law is
/// that it does.
pub fn written_rational(value: Ratio<i64>) -> Option<String> {
    if value < Ratio::ZERO {
        return None;
    }
    Some(if *value.denom() == 1 {
        value.numer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    })
}
