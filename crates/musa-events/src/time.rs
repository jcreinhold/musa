//! Exact musical time: coordinates, positions, durations, and spans
//! (docs/rules/events/00-purpose.md, 03 D0, §4).
//!
//! Positions form the abelian group `(ℚ, +, 0)`; durations form the ordered
//! commutative monoid `(ℚ≥0, +, 0)`. Those are two structures, so they are two
//! types: [`Position<C>`] for *when* and [`Duration<C>`] for *how much*. One
//! type for both would make beat 3 and three beats addable, which is the one
//! arithmetic error a tagged rational exists to catch.
//!
//! Both are `Ratio<i64>` inside — floats never represent symbolic musical
//! time. Rational arithmetic on musa's magnitudes is total, so the workspace
//! arithmetic lint is allowed module-wide (the sanctioned pattern, see
//! musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use std::marker::PhantomData;

use num_rational::Ratio;

/// Whose time this is.
///
/// The coordinate is the *one* type index the core carries, and
/// `docs/rules/constitution.md` §8 says why it is the only one: every other
/// candidate — part, voice, metre, tuning — has a diagnostic elsewhere, and a
/// written beat added to a physical second has none until the sound is wrong.
///
/// There is no operation anywhere in this crate that changes coordinate.
/// Changing coordinate is a named conversion above the core
/// (`docs/rules/events/07-backend-contract.md`), and a named conversion leaves
/// a record; a silent one would not.
pub trait Coordinate: Copy + Clone + std::fmt::Debug + Eq + Ord + std::hash::Hash + Default + 'static {
    /// The name an events document writes for this coordinate
    /// (`docs/rules/events/01-grammar.md`).
    const NAME: &'static str;
}

/// Notated time: the coordinate a score is written in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WrittenTime;

/// Performed time: the coordinate a performance interpretation produces, after
/// swing, rubato, and grace-note policy have moved things.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PerformedTime;

/// Physical seconds: the coordinate a tempo map lands in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalTime;

impl Coordinate for WrittenTime {
    const NAME: &'static str = "WrittenTime";
}

impl Coordinate for PerformedTime {
    const NAME: &'static str = "PerformedTime";
}

impl Coordinate for PhysicalTime {
    const NAME: &'static str = "PhysicalTime";
}

/// A position in `C`'s time, exactly: *when* something happens.
#[derive(Debug)]
pub struct Position<C> {
    at: Ratio<i64>,
    coordinate: PhantomData<C>,
}

/// An amount of `C`'s time, exactly and nonnegatively: *how much* it takes.
#[derive(Debug)]
pub struct Duration<C> {
    amount: Ratio<i64>,
    coordinate: PhantomData<C>,
}

/// The value traits, written out rather than derived.
///
/// `derive` on a type with a `PhantomData<C>` field asks for `C: Clone`,
/// `C: PartialEq`, and so on — bounds on the *tag*, which is never stored and
/// never compared. Leaving them in would make every generic function that
/// merely holds a position repeat them. The tag's job is to keep two
/// coordinates apart in the type checker, and it does that whether or not `C`
/// is comparable.
macro_rules! tagged_rational_traits {
    ($tagged:ident, $field:ident) => {
        impl<C> Clone for $tagged<C> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<C> Copy for $tagged<C> {}

        impl<C> Default for $tagged<C> {
            fn default() -> Self {
                Self::ZERO
            }
        }

        impl<C> PartialEq for $tagged<C> {
            fn eq(&self, other: &Self) -> bool {
                self.$field == other.$field
            }
        }

        impl<C> Eq for $tagged<C> {}

        impl<C> PartialOrd for $tagged<C> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl<C> Ord for $tagged<C> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.$field.cmp(&other.$field)
            }
        }

        impl<C> std::hash::Hash for $tagged<C> {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.$field.hash(state);
            }
        }
    };
}

tagged_rational_traits!(Position, at);
tagged_rational_traits!(Duration, amount);

impl<C> Position<C> {
    /// The origin.
    pub const ZERO: Self = Self {
        at: Ratio::new_raw(0, 1),
        coordinate: PhantomData,
    };

    /// A position from an exact rational.
    pub const fn new(ratio: Ratio<i64>) -> Self {
        Self {
            at: ratio,
            coordinate: PhantomData,
        }
    }

    /// An integral position.
    pub fn from_integer(units: i64) -> Self {
        Self::new(Ratio::from_integer(units))
    }

    /// The exact rational value.
    pub fn as_ratio(self) -> Ratio<i64> {
        self.at
    }

    /// Whether this position is the origin.
    pub fn is_zero(self) -> bool {
        self.at == Ratio::ZERO
    }

    /// This position moved forward by `amount`.
    ///
    /// Exact and total: musa's magnitudes are far from `i64`'s edges, which is
    /// why this module — and only this module — carries the arithmetic lint
    /// allowance. Callers elsewhere go through here rather than repeating it.
    #[must_use]
    pub fn plus(self, amount: Duration<C>) -> Self {
        Self::new(self.at + amount.amount)
    }

    /// This position with its distance from the origin multiplied.
    pub(crate) fn times(self, factor: Ratio<i64>) -> Self {
        Self::new(self.at * factor)
    }

    /// The amount of time from `earlier` to here.
    ///
    /// # Errors
    /// [`EventsError::InvalidSpan`](crate::EventsError::InvalidSpan) when
    /// `earlier` is later: two positions subtract to a duration only when the
    /// difference is nonnegative, because a duration is nonnegative.
    pub fn since(self, earlier: Self) -> Result<Duration<C>, crate::EventsError> {
        if earlier > self {
            return Err(crate::EventsError::InvalidSpan {
                start: earlier.to_string(),
                end: self.to_string(),
            });
        }
        Ok(Duration::new_raw(self.at - earlier.at))
    }
}

impl<C> Duration<C> {
    /// No time at all — the identity of `follow` (L2).
    pub const ZERO: Self = Self {
        amount: Ratio::new_raw(0, 1),
        coordinate: PhantomData,
    };

    /// A duration from an exact rational.
    ///
    /// # Errors
    /// [`EventsError::NegativeDuration`](crate::EventsError::NegativeDuration)
    /// when `ratio` is negative. This is the one place the monoid's `≥ 0` is
    /// enforced, so every `Duration` in the crate is nonnegative by
    /// construction and nothing downstream re-checks it.
    pub fn new(ratio: Ratio<i64>) -> Result<Self, crate::EventsError> {
        if ratio < Ratio::ZERO {
            return Err(crate::EventsError::NegativeDuration {
                amount: format!("{ratio}"),
            });
        }
        Ok(Self::new_raw(ratio))
    }

    /// An integral duration, for the amounts a caller writes as literals.
    ///
    /// # Errors
    /// As [`Self::new`].
    pub fn from_integer(units: i64) -> Result<Self, crate::EventsError> {
        Self::new(Ratio::from_integer(units))
    }

    /// A duration whose nonnegativity the caller has already established —
    /// a difference of ordered positions, a product with a positive factor, a
    /// sum of durations.
    pub(crate) const fn new_raw(ratio: Ratio<i64>) -> Self {
        Self {
            amount: ratio,
            coordinate: PhantomData,
        }
    }

    /// The exact rational value.
    pub fn as_ratio(self) -> Ratio<i64> {
        self.amount
    }

    /// Whether this is no time at all.
    pub fn is_zero(self) -> bool {
        self.amount == Ratio::ZERO
    }

    /// Two durations added — the monoid operation.
    #[must_use]
    pub fn plus(self, other: Self) -> Self {
        Self::new_raw(self.amount + other.amount)
    }

    /// This duration multiplied by a positive factor (D5).
    pub(crate) fn times(self, factor: Ratio<i64>) -> Self {
        Self::new_raw(self.amount * factor)
    }

    /// The position this much time from the origin reaches.
    ///
    /// A track's ambient region is `[0, d]`, so this is how a bound on
    /// occurrences is stated: an occurrence ends at or before `d.reach()`. It
    /// is the only bridge between the two types, and it is one-way — a
    /// position does not become a duration without saying what it is measured
    /// from ([`Position::since`]).
    pub fn reach(self) -> Position<C> {
        Position::new(self.amount)
    }
}

impl<C> std::fmt::Display for Position<C> {
    /// Reduced `p/q` form; integers print without `/1` (docs/rules/events/05 N5).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_ratio(f, self.at)
    }
}

impl<C> std::fmt::Display for Duration<C> {
    /// As [`Position`]: the same lexical form, since a file writes both as
    /// rationals and the declaration says which coordinate they are in.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_ratio(f, self.amount)
    }
}

fn write_ratio(f: &mut std::fmt::Formatter<'_>, value: Ratio<i64>) -> std::fmt::Result {
    if *value.denom() == 1 {
        write!(f, "{}", value.numer())
    } else {
        write!(f, "{}/{}", value.numer(), value.denom())
    }
}

/// A half-open temporal support `[start, end)` with `0 ≤ start ≤ end`.
#[derive(Debug)]
pub struct Span<C> {
    start: Position<C>,
    end: Position<C>,
}

// The same reason as the tagged rationals above: no bound on the tag.
impl<C> Clone for Span<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for Span<C> {}

impl<C> PartialEq for Span<C> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl<C> Eq for Span<C> {}

impl<C> std::hash::Hash for Span<C> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);
    }
}

impl<C> Span<C> {
    /// The empty span at the origin.
    pub const ZERO: Self = Self {
        start: Position::ZERO,
        end: Position::ZERO,
    };

    /// A span from two positions.
    ///
    /// # Errors
    /// [`EventsError::InvalidSpan`](crate::EventsError::InvalidSpan) when
    /// `start > end` or either is negative.
    pub fn new(start: Position<C>, end: Position<C>) -> Result<Self, crate::EventsError> {
        if start > end || start.as_ratio() < Ratio::ZERO {
            return Err(crate::EventsError::InvalidSpan {
                start: start.to_string(),
                end: end.to_string(),
            });
        }
        Ok(Self { start, end })
    }

    /// The start, inclusive.
    pub fn start(self) -> Position<C> {
        self.start
    }

    /// The end, exclusive.
    pub fn end(self) -> Position<C> {
        self.end
    }

    /// How much time this span takes: `end - start`, nonnegative because the
    /// span is ordered by construction.
    pub fn duration(self) -> Duration<C> {
        Duration::new_raw(self.end.as_ratio() - self.start.as_ratio())
    }

    /// The span translated later by `offset`.
    #[must_use]
    pub fn translate(self, offset: Duration<C>) -> Self {
        Self {
            start: self.start.plus(offset),
            end: self.end.plus(offset),
        }
    }

    /// The span with both endpoints scaled by a positive factor.
    #[must_use]
    pub fn scale(self, factor: Ratio<i64>) -> Self {
        Self {
            start: self.start.times(factor),
            end: self.end.times(factor),
        }
    }

    /// Whether this span is observable through `window` (docs/rules/events/03 D6).
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

    /// Whether `at` lies inside this span (docs/rules/events/03 D8).
    ///
    /// Support is half-open: `s ≤ at < e`. The end instant belongs to whatever
    /// comes next, which is what makes `follow` unambiguous — the second
    /// track's first instant is the first one's end, and one instant must not
    /// be inside both.
    ///
    /// A **point** span (`s = e`) contains its own instant, for D6's reason:
    /// the half-open reading alone would make a point unobservable everywhere,
    /// which is a defect rather than a definition.
    pub fn contains(self, at: Position<C>) -> bool {
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

impl<C> std::fmt::Display for Span<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {})", self.start, self.end)
    }
}
