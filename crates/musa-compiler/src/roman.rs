//! Roman numerals: which degree of a tonal context a chord is built on, how
//! many members it stacks, and which member is in the bass.
//!
//! A numeral is a *description*, not a chord. It names no pitch class until a
//! collection is supplied, and it names no register ever. That is what makes
//! `V` in C major and `V` in E-flat minor the same value: the numeral is the
//! coordinate, and the scale is the chart.
//!
//! The numeral carries no quality. In a tonal context a diatonic chord's
//! quality is not chosen and then transposed onto a degree — it is whatever
//! the collection's own notes stack up to, which is why `ii` is minor in
//! major and `II` is major in Dorian without either being stipulated. Storing
//! a quality here would let a caller write a numeral whose quality disagreed
//! with the collection it is realized against, and there would be no honest
//! answer to which one wins. Chromatic sonorities — the Neapolitan, the
//! augmented sixths, an altered dominant — are exactly the chords whose
//! quality is *not* the collection's, so they are not numerals against that
//! collection at all: they name their altered degrees explicitly and are built
//! from `scale_class` and `chord_on` in source. Mixture needs no alteration
//! either, because borrowing from the parallel mode is realizing the same
//! numeral against the parallel collection, which is what the word means.

/// The lowest ordinal a numeral can name.
const FIRST: u64 = 1;

/// The highest ordinal a numeral can name.
///
/// Roman-numeral notation has exactly seven symbols, `I` through `vii`. A
/// numeral is not a general scale degree: `VIII` is `I` written badly, and an
/// octatonic collection's eighth degree is not something the notation has ever
/// been asked to spell. Realization against a collection of another size is a
/// separate partiality, and stays one.
const LAST: u64 = 7;

/// The fewest members a numeral can stack: a triad.
const SMALLEST: u64 = 3;

/// The most members a numeral can stack: a thirteenth chord, which is the
/// largest stack of thirds the chord vocabulary names.
const LARGEST: u64 = 13_u64.div_ceil(2);

/// A Roman numeral against an unnamed tonal context.
///
/// The refinement is the whole point: a bare product of three numbers is
/// constructible with a degree of nine or an inversion of five, and neither
/// describes anything. [`Roman::new`] is the only way in, so a value of this
/// type is evidence that the numeral can be written down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Roman {
    /// Which numeral, from one.
    ordinal: u64,
    /// How many members the stack of thirds holds.
    members: u64,
    /// Which member is in the bass, counted from zero.
    inversion: u64,
}

impl Roman {
    /// The numeral these three numbers describe, when they describe one.
    ///
    /// Absent when the ordinal is outside `I`–`vii`, when the stack is smaller
    /// than a triad or larger than a thirteenth, or when the inversion names a
    /// member the stack does not have. The last of these is what makes the
    /// realization in source total once the chord itself is named: a third
    /// inversion of a triad is unsayable rather than merely unrealizable.
    pub(crate) fn new(ordinal: u64, members: u64, inversion: u64) -> Option<Self> {
        ((FIRST..=LAST).contains(&ordinal)
            && (SMALLEST..=LARGEST).contains(&members)
            && inversion < members)
            .then_some(Self { ordinal, members, inversion })
    }

    /// The ordinal the numeral names, from one.
    ///
    /// A number and not a `degree`, because a numeral is never altered: the
    /// diatonic stack is indexed by the plain ordinal, and source that wants
    /// the altered degree a chromatic constructor spells builds it with
    /// `degree_of` and `lower`.
    pub(crate) fn ordinal(self) -> u64 {
        self.ordinal
    }

    /// How many members the numeral stacks: three is a triad, four a seventh.
    pub(crate) fn members(self) -> u64 {
        self.members
    }

    /// Which member the numeral puts in the bass, counted from zero, so that
    /// zero is root position and one is first inversion.
    pub(crate) fn inversion(self) -> u64 {
        self.inversion
    }
}

impl std::fmt::Display for Roman {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "roman {}/{} over {}", self.ordinal, self.members, self.inversion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three bounds, each violated on its own, so that a widened bound
    /// cannot pass by being masked by another.
    #[test]
    fn a_numeral_refuses_what_cannot_be_written() {
        assert!(Roman::new(1, 3, 0).is_some());
        assert!(Roman::new(7, 7, 6).is_some());
        assert_eq!(Roman::new(0, 3, 0), None, "there is no numeral zero");
        assert_eq!(Roman::new(8, 3, 0), None, "there is no numeral eight");
        assert_eq!(Roman::new(1, 2, 0), None, "two members is not a chord");
        assert_eq!(Roman::new(1, 8, 0), None, "a fifteenth is the octave again");
        assert_eq!(Roman::new(1, 3, 3), None, "a triad has no third inversion");
        assert!(Roman::new(1, 4, 3).is_some(), "a seventh chord has one");
    }

    /// What went in comes back out, which is what lets source read a numeral
    /// apart and rebuild it.
    #[test]
    fn a_numerals_parts_are_what_it_was_built_from() {
        for ordinal in FIRST..=LAST {
            for members in SMALLEST..=LARGEST {
                for inversion in 0..members {
                    let numeral = Roman::new(ordinal, members, inversion).expect("a numeral");
                    assert_eq!(numeral.ordinal(), ordinal);
                    assert_eq!(numeral.members(), members);
                    assert_eq!(numeral.inversion(), inversion);
                }
            }
        }
    }
}
