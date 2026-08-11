//! Where the barlines fall, and what each measure is numbered.
//!
//! Measure numbering is a function of the meters in force, not a division by
//! one number. Before this module every measure:beat conversion in the
//! workspace divided by `Meter::measure_len()` — nine functions in
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
//!   its bar checks several answers. A `meter` statement is therefore legal
//!   only among a voice's own items, and `elaborate` refuses one written
//!   anywhere else.

// Rational arithmetic on `Ratio<i64>` is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use num_rational::Ratio;

use crate::score::Meter;
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
    /// Where it ends. For an unmeasured stretch, where the next meter
    /// begins — or, at the last measure of a piece, where the music stops.
    pub end: MusicalTime,
    /// The meter in force across it.
    pub meter: Meter,
}

impl Measure {
    /// How long it is. For an unmeasured stretch, how long the passage
    /// turned out to be rather than how long the meter said it would be.
    pub fn length(self) -> MusicalDuration {
        self.end - self.start
    }
}

/// One run of measures under a single meter.
#[derive(Clone, Copy, Debug)]
struct Stretch {
    start: MusicalTime,
    first: u32,
    meter: Meter,
}

impl Stretch {
    fn is_measured(self) -> bool {
        self.meter.is_measured()
    }

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
    /// One meter, from the beginning. Where every piece starts, and where a
    /// piece that never changes meter stays.
    pub fn uniform(meter: Meter) -> Self {
        Self {
            first: Stretch {
                start: MusicalTime::ZERO,
                first: 1,
                meter,
            },
            rest: Vec::new(),
        }
    }

    /// Change meter at `at`, and report whether that was possible.
    ///
    /// Returns `false` — leaving the barlines exactly as they were — when
    /// `at` is not a barline. That is the one thing that would make the
    /// coordinate system ill-formed: a measure that begins where the
    /// numbering says it is half over stops [`BarLines::at`] and
    /// [`BarLines::time_of`] being inverses, and the engraver would have to
    /// invent a bar that is neither length. Elaboration turns the `false`
    /// into a diagnostic; rendering, which must be total, drops the change
    /// and prints a piece the composer has already been told about.
    ///
    /// Changes arrive ascending. One stated where the last one is replaces
    /// it, because the meter in force at an instant is the last meter stated
    /// there — which is what makes the header's meter an ordinary first
    /// change rather than a special case.
    pub fn change(&mut self, at: MusicalTime, meter: Meter) -> bool {
        let last = self.rest.last().copied().unwrap_or(self.first);
        if at < last.start {
            return false;
        }
        if at == last.start {
            let replacement = Stretch {
                start: at,
                first: last.first,
                meter,
            };
            match self.rest.last_mut() {
                Some(slot) => *slot = replacement,
                None => self.first = replacement,
            }
            return true;
        }
        let here = self.measure_at(at);
        if !last.is_measured() {
            // Inside an unmeasured stretch every moment is a barline, because
            // there are none to be off. The passage counted as one measure,
            // so the measured music resumes with the next number — which is
            // how a conductor's score numbers the bar after a cadenza.
            self.rest.push(Stretch {
                start: at,
                first: here.number.saturating_add(1),
                meter,
            });
            return true;
        }
        if here.start != at {
            return false;
        }
        self.rest.push(Stretch {
            start: at,
            first: here.number,
            meter,
        });
        true
    }

    /// The meter in force at a moment.
    pub fn meter_at(&self, at: MusicalTime) -> Meter {
        self.stretch_at(at).meter
    }

    /// Which measure `at` falls in, and how far into it.
    pub fn at(&self, at: MusicalTime) -> BarBeat {
        let stretch = self.stretch_at(at);
        let len = stretch.measure_len();
        if len <= Ratio::ZERO {
            // An unmeasured stretch is one measure with no beats in it, so
            // the only honest answer to "which beat" is the first. How far
            // *in* is still a real quantity, and it is the one an engraver
            // spaces a cadenza by.
            return BarBeat {
                measure: stretch.first,
                beat: Ratio::ONE,
                into: MusicalDuration::new((at - stretch.start).as_ratio().max(Ratio::ZERO)),
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
            return self.unmeasured(stretch, at);
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
        (first..=last.max(first)).map(move |number| self.measure(number, end))
    }

    /// The measure with a given number, in a piece that stops at `end`.
    ///
    /// `end` matters only to an unmeasured stretch that nothing closes: it
    /// runs until the next meter, and where there is no next meter it runs
    /// until the music stops. A measured stretch already knows its own
    /// lengths and ignores it.
    fn measure(&self, number: u32, end: MusicalTime) -> Measure {
        let stretch = self.stretch_for(number);
        if !stretch.is_measured() {
            return self.unmeasured(stretch, end);
        }
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

    /// The one measure an unmeasured stretch is, reaching as far as it does.
    ///
    /// The stretch is one measure with no end until the next meter, so everything that walks measures keeps
    /// working and the measure number does not advance across a cadenza —
    /// which is correct, because a cadenza inside measure 42 is measure 42.
    fn unmeasured(&self, stretch: Stretch, bound: MusicalTime) -> Measure {
        let next = self
            .rest
            .iter()
            .find(|later| later.start > stretch.start)
            .map(|later| later.start);
        Measure {
            number: stretch.first,
            start: stretch.start,
            end: next.unwrap_or_else(|| bound.max(stretch.start)),
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
