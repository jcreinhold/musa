//! Exact musical time: positions and spans (docs/rules/kernel/03 D0, §4).
//!
//! Positions form the abelian group `(ℚ, +, 0)`; durations form the ordered
//! commutative monoid `(ℚ≥0, +, 0)`. Both are `Ratio<i64>` — floats never
//! represent symbolic musical time. Rational arithmetic on musa's magnitudes
//! is total, so the workspace arithmetic lint is allowed module-wide (the
//! sanctioned pattern, see musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use num_rational::Ratio;

/// A position in musical time, in beats, exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Beat(Ratio<i64>);

impl Beat {
    /// The origin.
    pub const ZERO: Self = Self(Ratio::new_raw(0, 1));

    /// A position from an exact rational.
    pub const fn new(ratio: Ratio<i64>) -> Self {
        Self(ratio)
    }

    /// An integral position.
    pub fn from_integer(beats: i64) -> Self {
        Self(Ratio::from_integer(beats))
    }

    /// The exact rational value.
    pub fn as_ratio(self) -> Ratio<i64> {
        self.0
    }

    /// Whether this position is the origin.
    pub fn is_zero(self) -> bool {
        self.0 == Ratio::ZERO
    }

    /// This position moved forward by `other`.
    ///
    /// Exact and total: musa's magnitudes are far from `i64`'s edges, which
    /// is why this module — and only this module — carries the arithmetic
    /// lint allowance. Callers elsewhere go through here rather than
    /// repeating the allowance.
    pub(crate) fn plus(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }

    /// This position with its distance from the origin multiplied.
    pub(crate) fn times(self, factor: Ratio<i64>) -> Self {
        Self(self.0 * factor)
    }
}

impl std::fmt::Display for Beat {
    /// Reduced `p/q` form; integers print without `/1` (docs/rules/kernel/05 N5).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self.0.denom() == 1 {
            write!(f, "{}", self.0.numer())
        } else {
            write!(f, "{}/{}", self.0.numer(), self.0.denom())
        }
    }
}

/// A half-open temporal support `[start, end)` with `0 ≤ start ≤ end`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    start: Beat,
    end: Beat,
}

impl Span {
    /// The empty span at the origin.
    pub const ZERO: Self = Self {
        start: Beat::ZERO,
        end: Beat::ZERO,
    };

    /// A span from two positions.
    ///
    /// # Errors
    /// [`KernelError::InvalidSpan`](crate::KernelError::InvalidSpan) when
    /// `start > end` or either is negative.
    pub fn new(start: Beat, end: Beat) -> Result<Self, crate::KernelError> {
        if start > end || start.as_ratio() < Ratio::ZERO {
            return Err(crate::KernelError::InvalidSpan { start, end });
        }
        Ok(Self { start, end })
    }

    /// The start, inclusive.
    pub fn start(self) -> Beat {
        self.start
    }

    /// The end, exclusive.
    pub fn end(self) -> Beat {
        self.end
    }

    /// The duration `end - start`.
    pub fn duration(self) -> Ratio<i64> {
        self.end.as_ratio() - self.start.as_ratio()
    }

    /// The span translated by `offset` beats.
    #[must_use]
    pub fn translate(self, offset: Beat) -> Self {
        Self {
            start: Beat::new(self.start.as_ratio() + offset.as_ratio()),
            end: Beat::new(self.end.as_ratio() + offset.as_ratio()),
        }
    }

    /// The span with both endpoints scaled by a positive factor.
    #[must_use]
    pub fn scale(self, factor: Ratio<i64>) -> Self {
        Self {
            start: Beat::new(self.start.as_ratio() * factor),
            end: Beat::new(self.end.as_ratio() * factor),
        }
    }

    /// Whether this span is observable through `window` (docs/rules/kernel/03 D6).
    ///
    /// Non-degenerate spans are visible when `[s, e) ∩ [i, j) ≠ ∅`; degenerate
    /// (point) spans are visible when `s ∈ [i, j)` — otherwise point
    /// occurrences could never be observed, which D6's interval-intersection
    /// phrasing does not intend for them. An empty window observes nothing.
    pub fn visible_through(self, window: Self) -> bool {
        if window.start == window.end {
            return false;
        }
        if self.start < self.end {
            self.start < window.end && self.end > window.start
        } else {
            (window.start..window.end).contains(&self.start)
        }
    }

    /// Whether `at` lies inside this span (docs/rules/kernel/03 D8).
    ///
    /// Support is half-open: `s ≤ at < e`. The end instant belongs to
    /// whatever comes next, which is what makes `sequence` unambiguous — the
    /// second timeline's first instant is the first one's end, and one
    /// instant must not be inside both.
    ///
    /// A **point** span (`s = e`) contains its own instant, for D6's reason:
    /// the half-open reading alone would make a point unobservable
    /// everywhere, which is a defect rather than a definition.
    pub fn contains(self, at: Beat) -> bool {
        if self.start == self.end {
            self.start == at
        } else {
            self.start <= at && at < self.end
        }
    }

    /// The intersection `self ∩ window`, for visible spans.
    #[must_use]
    pub fn clip(self, window: Self) -> Self {
        Self {
            start: self.start.max(window.start),
            end: self.end.min(window.end),
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {})", self.start, self.end)
    }
}
