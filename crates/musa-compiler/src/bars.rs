//! Where the barlines fall, and what each measure is numbered.
//!
//! Measure numbering is a function of the meters in force, not a division by
//! one number. Before this module every measure:beat conversion in the
//! workspace divided by `MeterMap::measure_len()` — nine functions in
//! `musa-render`'s planner threaded it as a parameter — so nothing in any
//! signature said "this piece has a single time signature", and nothing
//! objected when it did not.
//!
//! Two invariants live here rather than in the callers, because they are about
//! bars and the callers are about other things:
//!
//! - **A `BarLines` is built over one time coordinate.** Notation folds
//!   repeats (`musa-render`'s `Fold`), so a written measure and a sounding
//!   measure are different measures; performance and the fact index do not
//!   fold. Those are two `BarLines` values built at two call sites, not one
//!   value with a flag, so that neither can silently inherit the other's
//!   coordinate.
//! - **A context change belongs to a place in the piece, not to material.**
//!   A body elaborated once and referenced many times (`elaborate`'s `Share`)
//!   sits at several absolute times, so a meter written inside it would give
//!   its bar checks several answers. Meter statements inside motif and bar
//!   bodies are therefore forbidden. Nothing can violate this yet — there is
//!   one meter per piece — and the rule is written down so that the prompt
//!   which makes meter positional has something to enforce rather than
//!   something to discover.

// Rational arithmetic on `Ratio<i64>` is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use num_rational::Ratio;

use crate::score::MeterMap;
use crate::time::{MusicalDuration, MusicalTime};

/// A written position, as musicians count: both 1-based, `1:1` being the
/// downbeat of the first measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarBeat {
    /// The measure number.
    pub measure: u32,
    /// Which beat of it, in the meter's beat unit.
    pub beat: Ratio<i64>,
    /// How far into the measure, in whole notes — the same position said in
    /// the units durations are written in.
    pub into: MusicalDuration,
}

/// One measure: its number, its bounds, and the meter that gave it its length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measure {
    /// The 1-based measure number.
    pub number: u32,
    /// Where it begins.
    pub start: MusicalTime,
    /// Where it ends. Equal to `start` only when the piece is unmeasured.
    pub end: MusicalTime,
    /// The meter in force across it.
    pub meter: MeterMap,
}

impl Measure {
    /// How long it is. Zero only when the piece is unmeasured.
    pub fn length(self) -> MusicalDuration {
        self.end - self.start
    }
}

/// One run of measures under a single meter.
#[derive(Clone, Copy, Debug)]
struct Stretch {
    start: MusicalTime,
    first: u32,
    meter: MeterMap,
}

impl Stretch {
    fn measure_len(self) -> Ratio<i64> {
        self.meter.measure_len().as_ratio()
    }

    fn beat_len(self) -> Ratio<i64> {
        Ratio::new(1, i64::from(self.meter.denominator().max(1)))
    }
}

/// Where the barlines fall.
///
/// `at` and `time_of` are inverses at every barline; eight call sites depend
/// on that and none of them checked it before this type existed.
///
/// A degenerate meter — one whose measure length is not positive — makes the
/// piece *unmeasured*: everything is measure 1 and no division happens. That
/// case is handled once, here, rather than by the five scattered zero guards
/// it replaces.
#[derive(Clone, Debug)]
pub struct BarLines {
    /// The stretch every piece has: one meter, from time zero, measure one.
    /// Split out from the rest so that "non-empty" is a fact about the type
    /// rather than a fallback two lookups have to repeat.
    first: Stretch,
    /// Any later stretches, ascending by `start` and contiguous with `first`.
    rest: Vec<Stretch>,
}

impl BarLines {
    /// One meter for the whole piece.
    ///
    /// The only constructor there is a caller for. Mid-piece meter adds the
    /// other one in the commit that needs it.
    pub fn uniform(meter: MeterMap) -> Self {
        Self {
            first: Stretch {
                start: MusicalTime::ZERO,
                first: 1,
                meter,
            },
            rest: Vec::new(),
        }
    }

    /// Whether barlines fall anywhere at all.
    ///
    /// False for a meter with no length, which is the condition three callers
    /// check before asking a question that would have no answer.
    pub fn is_measured(&self) -> bool {
        std::iter::once(&self.first)
            .chain(&self.rest)
            .any(|stretch| stretch.measure_len() > Ratio::ZERO)
    }

    /// The meter in force at a moment.
    pub fn meter_at(&self, at: MusicalTime) -> MeterMap {
        self.stretch_at(at).meter
    }

    /// Which measure `at` falls in, and how far into it.
    pub fn at(&self, at: MusicalTime) -> BarBeat {
        let stretch = self.stretch_at(at);
        let len = stretch.measure_len();
        if len <= Ratio::ZERO {
            return BarBeat {
                measure: stretch.first,
                beat: Ratio::ONE,
                into: MusicalDuration::ZERO,
            };
        }
        let elapsed = (at - stretch.start).as_ratio();
        let index = (elapsed / len).floor();
        let into = elapsed - index * len;
        BarBeat {
            measure: number(stretch.first, index.to_integer()),
            beat: into / stretch.beat_len() + Ratio::ONE,
            into: MusicalDuration::new(into),
        }
    }

    /// The measure an exclusive end closes: the one it is the last instant of.
    ///
    /// A span ending exactly on a barline closes the measure before it, which
    /// is the difference between this and [`BarLines::at`] and the reason both
    /// exist.
    pub fn closing(&self, at: MusicalTime) -> u32 {
        let stretch = self.stretch_at(at);
        let len = stretch.measure_len();
        if len <= Ratio::ZERO {
            return stretch.first;
        }
        // One less than `at` would give: a moment exactly on a barline is the
        // *end* of the measure before it, not the start of the one after.
        let index = ((at - stretch.start).as_ratio() / len).ceil().to_integer();
        number(stretch.first, index.saturating_sub(1)).max(stretch.first)
    }

    /// The measure containing `at`, with its bounds and its meter.
    pub fn measure_at(&self, at: MusicalTime) -> Measure {
        let stretch = self.stretch_at(at);
        let len = stretch.measure_len();
        if len <= Ratio::ZERO {
            return Measure {
                number: stretch.first,
                start: stretch.start,
                end: stretch.start,
                meter: stretch.meter,
            };
        }
        let index = ((at - stretch.start).as_ratio() / len).floor();
        let start = MusicalTime::new(stretch.start.as_ratio() + index * len);
        Measure {
            number: number(stretch.first, index.to_integer()),
            start,
            end: MusicalTime::new(start.as_ratio() + len),
            meter: stretch.meter,
        }
    }

    /// The time of a written position, or `None` when it is outside the
    /// coordinate system — measure zero, or a beat before the downbeat.
    ///
    /// Total past the end of the piece: a caller that cares whether the music
    /// reaches that far is asking a different question, and answers it with a
    /// diagnostic of its own.
    pub fn time_of(&self, measure: u32, beat: Ratio<i64>) -> Option<MusicalTime> {
        if measure == 0 || beat < Ratio::ONE {
            return None;
        }
        let stretch = self.stretch_for(measure);
        let index = Ratio::from_integer(i64::from(measure.saturating_sub(stretch.first)));
        Some(MusicalTime::new(
            stretch.start.as_ratio() + index * stretch.measure_len() + (beat - Ratio::ONE) * stretch.beat_len(),
        ))
    }

    /// Every measure needed to cover `span`, in order — at least one, so a
    /// piece with no notes still has a page.
    pub fn measures_through(&self, span: MusicalDuration) -> impl Iterator<Item = Measure> + '_ {
        let end = MusicalTime::ZERO + span;
        let last = self.closing(end);
        let first = self.first.first;
        (first..=last.max(first)).map(|number| self.measure(number))
    }

    /// The measure with a given number.
    fn measure(&self, number: u32) -> Measure {
        let stretch = self.stretch_for(number);
        let index = Ratio::from_integer(i64::from(number.saturating_sub(stretch.first)));
        let len = stretch.measure_len();
        let start = MusicalTime::new(stretch.start.as_ratio() + index * len);
        Measure {
            number,
            start,
            end: MusicalTime::new(start.as_ratio() + len),
            meter: stretch.meter,
        }
    }

    fn stretch_at(&self, at: MusicalTime) -> Stretch {
        let mut found = self.first;
        for stretch in &self.rest {
            if stretch.start <= at {
                found = *stretch;
            }
        }
        found
    }

    fn stretch_for(&self, measure: u32) -> Stretch {
        let mut found = self.first;
        for stretch in &self.rest {
            if stretch.first <= measure {
                found = *stretch;
            }
        }
        found
    }
}

/// A stretch-relative measure index turned into a measure number, saturating
/// rather than wrapping: a number is a label on a page, and a page has no
/// negative measures.
fn number(first: u32, index: i64) -> u32 {
    let index = u32::try_from(index.max(0)).unwrap_or(u32::MAX);
    first.saturating_add(index)
}
