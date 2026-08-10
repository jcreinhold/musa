//! Scales, degrees, and register frames (`docs/language/03-musical-domains.md`
//! §2).
//!
//! Four things musical practice keeps apart, and which one type would
//! silently merge:
//!
//! - a **collection** is an ordered cyclic pattern of spelled offsets — what
//!   "dorian" names, with no note in it yet;
//! - a **scale** is a collection rooted on a spelled tonic class — what
//!   `scale c dorian` names, still with no register;
//! - a **key** is a notational fact (tonic, mode, signature) that *suggests* a
//!   collection without claiming every pitch of the music belongs to it;
//! - a **frame** is a scale plus the absolute pitch of its first degree — the
//!   only thing that can answer "which C" and therefore the only thing that
//!   can turn a degree into a written pitch.
//!
//! Every value here is total: a collection is a closed finite table, so no
//! constructor can fail on a representation invariant. The two partial
//! operations are partial for musical reasons rather than representational
//! ones — a pitch may not belong to a collection ([`Frame::locate`]), and a
//! register may not belong to a tonic class ([`Frame::new`]).

use crate::origin::Interval;
use crate::pitch::{PitchClass, WrittenPitch};
use crate::score::{Key, Mode};

/// The written octave: seven staff steps and twelve semitones.
///
/// Every collection named here repeats at the octave. The period is carried
/// explicitly all the same, because "the pattern repeats at the octave" is a
/// fact about these collections rather than about scales.
const OCTAVE: Interval = Interval {
    diatonic_steps: 7,
    semitones: 12,
};

/// One spelled offset, written as the staff/semitone pair the rest of the
/// compiler uses.
///
/// Named for brevity: the table below is read as a grid of intervals, and a
/// longer name would push every row onto four lines and hide the shape.
#[expect(non_snake_case, reason = "one table-local abbreviation, read as a column heading")]
const fn O(diatonic_steps: i64, semitones: i64) -> Interval {
    Interval {
        diatonic_steps,
        semitones,
    }
}

/// A named ordered collection: the step pattern, before any tonic is chosen.
///
/// Collections that share a pattern but not a musical role stay distinct
/// values. [`Self::DescendingMelodicMinor`] spells exactly what
/// [`Self::NaturalMinor`] spells and is a different collection all the same,
/// because melodic minor is a *choice* between two forms and a language that
/// collapsed them would have no way to write which one a passage descends
/// through (OMT `014-minor-scales-scale-degrees-and-key-signatures.md`).
/// Collections that share a pattern *and* a role — ionian and major, aeolian
/// and natural minor — are one value under two spellings, because no theory
/// distinguishes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Collection {
    /// Major, and its modal spelling ionian (OMT `013`, `105`).
    Major,
    /// Dorian.
    Dorian,
    /// Phrygian.
    Phrygian,
    /// Lydian.
    Lydian,
    /// Mixolydian.
    Mixolydian,
    /// Natural minor, and its modal spelling aeolian (OMT `014`, `105`).
    NaturalMinor,
    /// Locrian.
    Locrian,
    /// Harmonic minor: natural minor with a raised seventh.
    HarmonicMinor,
    /// Melodic minor as it ascends: raised sixth and seventh.
    MelodicMinor,
    /// Melodic minor as it descends, which spells the natural form.
    DescendingMelodicMinor,
    /// The major pentatonic (OMT `106-collections.md`).
    MajorPentatonic,
    /// The minor pentatonic.
    MinorPentatonic,
    /// The whole-tone collection.
    WholeTone,
    /// The octatonic collection entered semitone-first.
    OctatonicHalfWhole,
    /// The octatonic collection entered tone-first.
    OctatonicWholeHalf,
    /// The hexatonic (augmented) collection.
    Hexatonic,
    /// The acoustic collection: a major scale with raised fourth and lowered
    /// seventh.
    Acoustic,
}

/// One row per collection: the value, the words that name it, the pattern it
/// is, and the sentence that describes it.
///
/// One table, so that the parser, the language server's completion, the
/// formatter, `std::collections`, and the step arithmetic cannot disagree
/// about which collections exist or what they contain. The first spelling of
/// each row is the one values print as.
///
/// Spellings retain the letters a musician writes rather than the letters a
/// seven-note assumption would force: the octatonic and hexatonic collections
/// have more members than there are letters, so a letter necessarily repeats,
/// and these are the practical spellings rather than the multiply-flatted
/// theoretical ones.
const COLLECTIONS: [(Collection, &[&str], &[Interval], &str); 17] = [
    (
        Collection::Major,
        &["major", "ionian"],
        &[O(0, 0), O(1, 2), O(2, 4), O(3, 5), O(4, 7), O(5, 9), O(6, 11)],
        "the major collection, W–W–H–W–W–W–H",
    ),
    (
        Collection::Dorian,
        &["dorian"],
        &[O(0, 0), O(1, 2), O(2, 3), O(3, 5), O(4, 7), O(5, 9), O(6, 10)],
        "the second rotation of the diatonic collection: minor with a raised sixth",
    ),
    (
        Collection::Phrygian,
        &["phrygian"],
        &[O(0, 0), O(1, 1), O(2, 3), O(3, 5), O(4, 7), O(5, 8), O(6, 10)],
        "the third rotation: minor with a lowered second",
    ),
    (
        Collection::Lydian,
        &["lydian"],
        &[O(0, 0), O(1, 2), O(2, 4), O(3, 6), O(4, 7), O(5, 9), O(6, 11)],
        "the fourth rotation: major with a raised fourth",
    ),
    (
        Collection::Mixolydian,
        &["mixolydian"],
        &[O(0, 0), O(1, 2), O(2, 4), O(3, 5), O(4, 7), O(5, 9), O(6, 10)],
        "the fifth rotation: major with a lowered seventh",
    ),
    (
        Collection::NaturalMinor,
        &["natural_minor", "aeolian", "minor"],
        &[O(0, 0), O(1, 2), O(2, 3), O(3, 5), O(4, 7), O(5, 8), O(6, 10)],
        "the natural minor collection, which is also the sixth rotation",
    ),
    (
        Collection::Locrian,
        &["locrian"],
        &[O(0, 0), O(1, 1), O(2, 3), O(3, 5), O(4, 6), O(5, 8), O(6, 10)],
        "the seventh rotation: a diminished fifth above the tonic",
    ),
    (
        Collection::HarmonicMinor,
        &["harmonic_minor"],
        &[O(0, 0), O(1, 2), O(2, 3), O(3, 5), O(4, 7), O(5, 8), O(6, 11)],
        "natural minor with a raised seventh, which supplies the leading tone",
    ),
    (
        Collection::MelodicMinor,
        &["melodic_minor"],
        &[O(0, 0), O(1, 2), O(2, 3), O(3, 5), O(4, 7), O(5, 9), O(6, 11)],
        "melodic minor as it ascends: raised sixth and seventh",
    ),
    (
        Collection::DescendingMelodicMinor,
        &["descending_melodic_minor"],
        &[O(0, 0), O(1, 2), O(2, 3), O(3, 5), O(4, 7), O(5, 8), O(6, 10)],
        "melodic minor as it descends, spelled like the natural form",
    ),
    (
        Collection::MajorPentatonic,
        &["major_pentatonic"],
        &[O(0, 0), O(1, 2), O(2, 4), O(4, 7), O(5, 9)],
        "the five-note major pentatonic collection",
    ),
    (
        Collection::MinorPentatonic,
        &["minor_pentatonic"],
        &[O(0, 0), O(2, 3), O(3, 5), O(4, 7), O(6, 10)],
        "the five-note minor pentatonic collection",
    ),
    (
        Collection::WholeTone,
        &["whole_tone"],
        &[O(0, 0), O(1, 2), O(2, 4), O(3, 6), O(4, 8), O(5, 10)],
        "the six-note whole-tone collection",
    ),
    // C D♭ E♭ E F♯ G A B♭: eight members over seven letters, so E carries
    // both the lowered and the natural third.
    (
        Collection::OctatonicHalfWhole,
        &["octatonic_half_whole"],
        &[O(0, 0), O(1, 1), O(2, 3), O(2, 4), O(3, 6), O(4, 7), O(5, 9), O(6, 10)],
        "the eight-note octatonic collection, entered semitone-first",
    ),
    // C D E♭ F G♭ A♭ A B: here A carries both.
    (
        Collection::OctatonicWholeHalf,
        &["octatonic_whole_half"],
        &[O(0, 0), O(1, 2), O(2, 3), O(3, 5), O(4, 6), O(5, 8), O(5, 9), O(6, 11)],
        "the eight-note octatonic collection, entered tone-first",
    ),
    // C E♭ E G A♭ B.
    (
        Collection::Hexatonic,
        &["hexatonic"],
        &[O(0, 0), O(2, 3), O(2, 4), O(4, 7), O(5, 8), O(6, 11)],
        "the six-note hexatonic collection, alternating minor thirds and semitones",
    ),
    (
        Collection::Acoustic,
        &["acoustic"],
        &[O(0, 0), O(1, 2), O(2, 4), O(3, 6), O(4, 7), O(5, 9), O(6, 10)],
        "major with a raised fourth and a lowered seventh",
    ),
];

/// Every word that may follow `scale`, with the sentence that says what it
/// collects.
///
/// Published because the language server's completion and hover must offer
/// exactly the collections this table holds. It is a vocabulary, not a value:
/// no scale, degree, or frame crosses the crate boundary with it.
pub fn scale_collections() -> impl Iterator<Item = (&'static str, &'static str)> {
    COLLECTIONS
        .iter()
        .flat_map(|(_, spellings, _, doc)| spellings.iter().map(move |spelling| (*spelling, *doc)))
}

impl Collection {
    /// The collection a source word names, or `None` when no collection is
    /// spelled that way.
    pub(crate) fn named(word: &str) -> Option<Self> {
        COLLECTIONS
            .iter()
            .find(|(_, spellings, _, _)| spellings.contains(&word))
            .map(|(collection, _, _, _)| *collection)
    }

    /// The spelling this collection prints as.
    pub(crate) fn name(self) -> &'static str {
        COLLECTIONS
            .iter()
            .find(|(collection, _, _, _)| *collection == self)
            .and_then(|(_, spellings, _, _)| spellings.first().copied())
            .unwrap_or("major")
    }

    /// Every word that names a collection, in declaration order, including
    /// the alternative spellings.
    pub(crate) fn spellings() -> impl Iterator<Item = &'static str> {
        COLLECTIONS
            .iter()
            .flat_map(|(_, spellings, _, _)| spellings.iter().copied())
    }

    /// The ordered spelled offsets above the tonic, starting at the unison.
    pub(crate) fn offsets(self) -> &'static [Interval] {
        COLLECTIONS
            .iter()
            .find(|(collection, _, _, _)| *collection == self)
            .map_or(&[], |(_, _, offsets, _)| *offsets)
    }
}

/// A collection rooted on a spelled tonic class: `scale c dorian`.
///
/// A scale has no register. It answers "which notes", never "which C", which
/// is why turning a degree into a pitch needs a [`Frame`] and not this.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Scale {
    tonic: PitchClass,
    collection: Collection,
}

impl Scale {
    /// The scale rooted on `tonic`. Total: every collection is defined over
    /// every spelled tonic class.
    pub(crate) fn new(tonic: PitchClass, collection: Collection) -> Self {
        Self { tonic, collection }
    }

    /// The spelled tonic class.
    pub(crate) fn tonic(self) -> PitchClass {
        self.tonic
    }

    /// The collection.
    pub(crate) fn collection(self) -> Collection {
        self.collection
    }

    /// The same collection rooted somewhere else.
    pub(crate) fn rooted_at(self, tonic: PitchClass) -> Self {
        Self { tonic, ..self }
    }

    /// How many degrees the pattern has before it repeats.
    pub(crate) fn size(self) -> usize {
        self.collection.offsets().len()
    }

    /// The interval after which the pattern repeats.
    ///
    /// Every collection this table holds repeats at the written octave. It is
    /// a method rather than a constant because the period belongs to the
    /// scale: a collection with another one would answer differently here,
    /// and every caller already asks the scale.
    #[expect(
        clippy::unused_self,
        reason = "the period is the scale's, and a caller that read a constant instead would have to be rewritten"
    )]
    pub(crate) fn period(self) -> Interval {
        OCTAVE
    }
}

impl std::fmt::Display for Scale {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "scale {} {}", self.tonic, self.collection.name())
    }
}

/// The collection a key signature supplies as a default coordinate system.
///
/// This is a *default*, not a claim: a minor key routinely sounds its raised
/// sixth and seventh, and the natural collection this returns does not say
/// those notes are outside the key. Choosing harmonic or melodic minor for a
/// passage is what `in scale` is for (OMT
/// `014-minor-scales-scale-degrees-and-key-signatures.md`).
pub(crate) fn signature_scale(key: Key) -> Scale {
    Scale::new(
        key.tonic(),
        match key.mode() {
            Mode::Major => Collection::Major,
            Mode::Minor => Collection::NaturalMinor,
        },
    )
}

/// A signed ordinal in some scale, plus a chromatic alteration.
///
/// Deliberately scale-independent: the same degree 5 means the dominant of
/// whichever scale it is read against, and reading it needs a scale supplied
/// at that moment rather than one frozen into the value. The ordinal is
/// unbounded and signed, so degree 8 is the tonic one period up and degree 0
/// is the seventh below — which is what makes stepping additive rather than
/// wrapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Degree {
    ordinal: i64,
    alteration: i64,
}

impl Degree {
    /// The unaltered degree at `ordinal`; `1` is the tonic.
    pub(crate) fn new(ordinal: i64) -> Self {
        Self { ordinal, alteration: 0 }
    }

    /// The ordinal, unaltered by the chromatic alteration.
    pub(crate) fn ordinal(self) -> i64 {
        self.ordinal
    }

    /// The chromatic alteration in semitones.
    pub(crate) fn alteration(self) -> i64 {
        self.alteration
    }

    /// Move `by` scale steps, keeping any alteration.
    ///
    /// The alteration rides along because it is part of the coordinate being
    /// moved, which is what makes stepping additive:
    /// `d.step(a).step(b) = d.step(a + b)`. A raised fourth stepped up is a
    /// raised fifth; write the unaltered degree when that is not what is
    /// meant.
    pub(crate) fn step(self, by: i64) -> Option<Self> {
        Some(Self {
            ordinal: self.ordinal.checked_add(by)?,
            alteration: self.alteration,
        })
    }

    /// Raise by a chromatic semitone.
    pub(crate) fn raised(self) -> Option<Self> {
        Some(Self {
            alteration: self.alteration.checked_add(1)?,
            ..self
        })
    }

    /// Lower by a chromatic semitone.
    pub(crate) fn lowered(self) -> Option<Self> {
        Some(Self {
            alteration: self.alteration.checked_sub(1)?,
            ..self
        })
    }
}

impl std::fmt::Display for Degree {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let alteration = match self.alteration.cmp(&0) {
            std::cmp::Ordering::Greater => "#".repeat(usize::try_from(self.alteration).unwrap_or(0)),
            std::cmp::Ordering::Less => "b".repeat(
                self.alteration
                    .checked_neg()
                    .and_then(|count| usize::try_from(count).ok())
                    .unwrap_or(0),
            ),
            std::cmp::Ordering::Equal => String::new(),
        };
        write!(out, "degree {alteration}{}", self.ordinal)
    }
}

/// A scale with a register: the absolute written pitch its first degree
/// sounds.
///
/// This is the only value that maps degrees to pitches, and it exists so that
/// the register is something a musician writes rather than something the
/// compiler assumes. `degree 5` of C major is a coordinate; `g4` is a note,
/// and nothing but a stated tonic register turns the one into the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Frame {
    scale: Scale,
    tonic: WrittenPitch,
}

impl Frame {
    /// The frame of `scale` whose first degree is `tonic`.
    ///
    /// `None` when `tonic` does not spell the scale's tonic class: a C major
    /// frame rooted on `g4` is not a transposition, it is a mistake, and
    /// answering it with a G major scale would be the compiler guessing.
    pub(crate) fn new(scale: Scale, tonic: WrittenPitch) -> Option<Self> {
        (tonic.pitch_class() == scale.tonic()).then_some(Self { scale, tonic })
    }

    /// The frame of `scale` in the register `near` sits in.
    ///
    /// The tonic chosen is the highest one not above `near`, so a frame built
    /// around a note contains that note's own octave rather than the one
    /// below it. Used where the register is implied by a note already written
    /// — stepping away from it — and never where a degree arrives without one.
    pub(crate) fn around(scale: Scale, near: WrittenPitch) -> Option<Self> {
        let tonic_letter = i64::from(scale.tonic().letter.steps());
        let octaves = near.diatonic_height().checked_sub(tonic_letter)?.div_euclid(7);
        let tonic = WrittenPitch::from_heights(
            octaves.checked_mul(7)?.checked_add(tonic_letter)?,
            octaves
                .checked_mul(12)?
                .checked_add(i64::from(scale.tonic().letter.natural_semitone()))?
                .checked_add(i64::from(scale.tonic().accidental.0))?,
        )?;
        Self::new(scale, tonic)
    }

    /// The scale this frame registers.
    pub(crate) fn scale(self) -> Scale {
        self.scale
    }

    /// The pitch of the first degree.
    pub(crate) fn tonic(self) -> WrittenPitch {
        self.tonic
    }

    /// The written pitch a degree names in this frame.
    ///
    /// Total up to machine integers: the ordinal is divided by the collection
    /// size into a period count and a position, so every signed ordinal names
    /// a pitch, and the alteration moves the semitone coordinate without
    /// touching the letter.
    pub(crate) fn pitch(self, degree: Degree) -> Option<WrittenPitch> {
        let size = i64::try_from(self.scale.size()).ok()?;
        let position = degree.ordinal().checked_sub(1)?;
        let periods = position.div_euclid(size);
        let within = usize::try_from(position.rem_euclid(size)).ok()?;
        let step = self.scale.collection().offsets().get(within)?;
        let period = self.scale.period();
        self.tonic.transpose(Interval {
            diatonic_steps: periods
                .checked_mul(period.diatonic_steps)?
                .checked_add(step.diatonic_steps)?,
            semitones: periods
                .checked_mul(period.semitones)?
                .checked_add(step.semitones)?
                .checked_add(degree.alteration())?,
        })
    }

    /// The degree a written pitch occupies in this frame, or `None` when the
    /// pitch is not a member of the collection.
    ///
    /// Membership is by spelling, not by sound: in C major `f#4` is not
    /// degree 4 raised and not degree 5 lowered, it is simply not in the
    /// collection, and the caller has to say what it meant. The answer is
    /// always unaltered, which is what makes it the canonical representative
    /// the round trip returns.
    pub(crate) fn locate(self, pitch: WrittenPitch) -> Option<Degree> {
        let diatonic = pitch.diatonic_height().checked_sub(self.tonic.diatonic_height())?;
        let chromatic = pitch.chromatic_height().checked_sub(self.tonic.chromatic_height())?;
        let period = self.scale.period();
        let size = i64::try_from(self.scale.size()).ok()?;
        for (index, step) in self.scale.collection().offsets().iter().enumerate() {
            let staff = diatonic.checked_sub(step.diatonic_steps)?;
            let semitones = chromatic.checked_sub(step.semitones)?;
            if staff.rem_euclid(period.diatonic_steps) != 0 || semitones.rem_euclid(period.semitones) != 0 {
                continue;
            }
            let periods = staff.div_euclid(period.diatonic_steps);
            if periods != semitones.div_euclid(period.semitones) {
                continue;
            }
            return Some(Degree::new(
                periods
                    .checked_mul(size)?
                    .checked_add(i64::try_from(index).ok()?)?
                    .checked_add(1)?,
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn pitch(text: &str) -> WrittenPitch {
        WrittenPitch::parse(text).expect("a pitch literal")
    }

    fn class(text: &str) -> PitchClass {
        PitchClass::parse(text).expect("a pitch class")
    }

    fn frame(tonic: &str, collection: Collection) -> Frame {
        let root = pitch(tonic);
        Frame::new(Scale::new(root.pitch_class(), collection), root).expect("a matching tonic")
    }

    /// Every collection starts on its tonic, has distinct members, and stays
    /// inside one period — the representation invariant the table has to
    /// satisfy for degrees to divide cleanly.
    #[test]
    fn every_collection_is_an_ordered_cycle_within_its_period() {
        for (collection, spellings, _, _) in COLLECTIONS {
            let offsets = collection.offsets();
            assert!(!offsets.is_empty(), "{spellings:?} is empty");
            assert_eq!(offsets.first(), Some(&O(0, 0)), "{spellings:?} skips its tonic");
            for window in offsets.windows(2) {
                let [previous, next] = window else { continue };
                assert!(
                    previous.semitones < next.semitones,
                    "{spellings:?} is not ascending at {next:?}"
                );
            }
            let last = offsets.last().copied().unwrap_or(O(0, 0));
            assert!(
                last.semitones < OCTAVE.semitones && last.diatonic_steps < OCTAVE.diatonic_steps,
                "{spellings:?} reaches past its period"
            );
        }
    }

    /// The round trip of `03-musical-domains.md` §2: locating a realized
    /// degree returns the degree it realized, for every collection and every
    /// ordinal across several periods.
    #[test]
    fn locating_a_realized_degree_returns_it() {
        for (collection, spellings, _, _) in COLLECTIONS {
            let frame = frame("c4", collection);
            for ordinal in -9..=17 {
                let degree = Degree::new(ordinal);
                let pitch = frame.pitch(degree).expect("a total realization");
                assert_eq!(frame.locate(pitch), Some(degree), "{spellings:?} at {ordinal}");
            }
        }
    }

    /// A degree one period higher is the same note one period higher — the
    /// periodicity that makes the ordinal a coordinate rather than a name.
    #[test]
    fn a_degree_a_period_up_is_the_pitch_a_period_up() {
        for (collection, spellings, _, _) in COLLECTIONS {
            let frame = frame("d4", collection);
            let size = i64::try_from(frame.scale().size()).expect("a small collection");
            for ordinal in 1..=9 {
                let here = frame.pitch(Degree::new(ordinal)).expect("a pitch");
                let above = frame
                    .pitch(Degree::new(ordinal).step(size).expect("a degree"))
                    .expect("a pitch");
                assert_eq!(
                    Some(above),
                    here.transpose(frame.scale().period()),
                    "{spellings:?} at {ordinal}"
                );
            }
        }
    }

    /// Stepping is additive, which is the law that lets a phrase be moved by
    /// one step twice and by two steps once and mean the same thing.
    #[test]
    fn stepping_is_additive() {
        let degree = Degree::new(3).raised().expect("a raised degree");
        for first in -4..=4_i64 {
            for second in -4..=4_i64 {
                assert_eq!(
                    degree.step(first).and_then(|moved| moved.step(second)),
                    degree.step(first + second),
                    "{first} then {second}"
                );
            }
        }
    }

    /// A frame needs a tonic in the scale's own class. Rooting C major on G
    /// is not a G major scale; it is a question the compiler declines to
    /// answer for the composer.
    #[test]
    fn a_frame_refuses_a_tonic_from_another_class() {
        let major = Scale::new(class("c"), Collection::Major);
        assert!(Frame::new(major, pitch("c4")).is_some());
        assert_eq!(Frame::new(major, pitch("g4")), None);
        assert_eq!(Frame::new(major, pitch("c#4")), None);
    }

    /// The three minor collections are three values, and they disagree about
    /// exactly the notes the theory says they disagree about.
    #[test]
    fn the_minor_collections_are_distinct() {
        let natural = frame("a3", Collection::NaturalMinor);
        let harmonic = frame("a3", Collection::HarmonicMinor);
        let melodic = frame("a3", Collection::MelodicMinor);
        let descending = frame("a3", Collection::DescendingMelodicMinor);
        let sixth = Degree::new(6);
        let seventh = Degree::new(7);
        assert_eq!(natural.pitch(seventh), Some(pitch("g4")));
        assert_eq!(harmonic.pitch(seventh), Some(pitch("g#4")));
        assert_eq!(harmonic.pitch(sixth), Some(pitch("f4")));
        assert_eq!(melodic.pitch(sixth), Some(pitch("f#4")));
        assert_eq!(melodic.pitch(seventh), Some(pitch("g#4")));
        // The descending form spells the natural one and is not it.
        assert_eq!(descending.pitch(seventh), natural.pitch(seventh));
        assert_ne!(descending.scale(), natural.scale());
    }

    /// Membership is spelled, so an enharmonic neighbour is not a member.
    #[test]
    fn membership_keeps_its_spelling() {
        let major = frame("c4", Collection::Major);
        assert_eq!(major.locate(pitch("e4")), Some(Degree::new(3)));
        assert_eq!(major.locate(pitch("f#4")), None);
        assert_eq!(major.locate(pitch("gb4")), None);
        // The lydian fourth is spelled F sharp and is a member there.
        assert_eq!(
            frame("c4", Collection::Lydian).locate(pitch("f#4")),
            Some(Degree::new(4))
        );
    }

    /// A frame built around a note contains that note's own octave.
    #[test]
    fn a_frame_around_a_note_holds_it() {
        let major = Scale::new(class("c"), Collection::Major);
        assert_eq!(Frame::around(major, pitch("e4")).map(Frame::tonic), Some(pitch("c4")));
        assert_eq!(Frame::around(major, pitch("c4")).map(Frame::tonic), Some(pitch("c4")));
        assert_eq!(Frame::around(major, pitch("b3")).map(Frame::tonic), Some(pitch("c3")));
        // An altered tonic keeps its spelling in every register.
        let flat = Scale::new(class("bb"), Collection::Major);
        assert_eq!(Frame::around(flat, pitch("d4")).map(Frame::tonic), Some(pitch("bb3")));
    }

    /// A key supplies a default collection and does not thereby claim it.
    #[test]
    fn a_key_supplies_a_default_collection() {
        assert_eq!(
            signature_scale(Key::new(class("eb"), Mode::Major)),
            Scale::new(class("eb"), Collection::Major)
        );
        assert_eq!(
            signature_scale(Key::new(class("c"), Mode::Minor)),
            Scale::new(class("c"), Collection::NaturalMinor)
        );
    }

    /// Every spelling names a collection and every collection prints as a
    /// spelling that parses back to it.
    #[test]
    fn spellings_round_trip() {
        for spelling in Collection::spellings() {
            let collection = Collection::named(spelling).expect("a named collection");
            assert_eq!(Collection::named(collection.name()), Some(collection));
        }
        assert_eq!(Collection::named("phyrgian"), None);
    }
}
