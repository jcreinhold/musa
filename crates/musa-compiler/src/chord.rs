//! Chord classes, triads, and voicings (`docs/rules/language/03-musical-domains.md`
//! §3).
//!
//! Four things a lead sheet blurs together and this module keeps apart:
//!
//! - a **chord symbol** is a written annotation — `crate::harmony`, recorded
//!   and never interpreted;
//! - a **chord class** is rooted spelled content: a root class, a stack of
//!   spelled members, and an optional designated bass class. It has no
//!   register, spacing, doubling, or omission;
//! - a **triad** is the checked refinement whose content is a major or minor
//!   triad, which is the domain the transformational operations
//!   are defined over;
//! - a **voicing** is exact written pitches, which is the only thing that can
//!   sound.
//!
//! The forgetfulness lemma of §3 is why the last step needs a policy rather
//! than a coercion: every voicing projects onto one chord class, and one chord
//! class has as many voicings as there are registers, spacings, doublings, and
//! bass choices. So the maps in this module go one way for free and the other
//! way only through a named policy that can decline.
//!
//! Spelling follows the diatonic letter stack of OMT `017-triads.md` and
//! `018-seventh-chords.md`: a third is a third because it is two letters up,
//! never because it is three or four semitones. That is why the table below
//! holds spelled intervals rather than semitone counts — `c` to `d#` is an
//! augmented second, and no chord in this table contains one.

use crate::origin::Interval;
use crate::pitch::{PitchClass, WrittenPitch};

/// One spelled member interval above the root, written as the staff/semitone
/// pair the rest of the compiler uses.
///
/// Named for brevity, exactly as in `crate::scale`: the table reads as a grid
/// of intervals, and a longer name would hide its shape.
#[expect(non_snake_case, reason = "one table-local abbreviation, read as a column heading")]
const fn M(diatonic_steps: i64, semitones: i64) -> Interval {
    Interval {
        diatonic_steps,
        semitones,
    }
}

/// The written octave, used to lift a member into successive registers.
const OCTAVE: Interval = Interval {
    diatonic_steps: 7,
    semitones: 12,
};

/// A named chord type: the quality and extensions, before any root is chosen.
///
/// Types that share a member set but not a musical role stay distinct values,
/// for the reason `crate::scale::Collection` does: a language that spelled
/// them the same would have no way to write which one a passage means. Nothing
/// here is a symbol; `cmaj7` is what a musician writes above the staff, and
/// [`ChordType::Major7`] is what the notes of that chord are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ChordType {
    /// A major triad (OMT `017-triads.md`).
    Major,
    /// A minor triad.
    Minor,
    /// A diminished triad.
    Diminished,
    /// An augmented triad.
    Augmented,
    /// A triad whose third is replaced by a second.
    Suspended2,
    /// A triad whose third is replaced by a fourth.
    Suspended4,
    /// A major triad with an added sixth.
    Major6,
    /// A minor triad with an added sixth.
    Minor6,
    /// A major triad with a major seventh (OMT `018-seventh-chords.md`).
    Major7,
    /// A major triad with a minor seventh: the dominant seventh.
    Dominant7,
    /// A minor triad with a minor seventh.
    Minor7,
    /// A minor triad with a major seventh.
    MinorMajor7,
    /// A diminished triad with a minor seventh: the half-diminished seventh.
    HalfDiminished7,
    /// A diminished triad with a diminished seventh.
    Diminished7,
    /// A dominant seventh with a major ninth.
    Dominant9,
    /// A major seventh with a major ninth.
    Major9,
    /// A minor seventh with a major ninth.
    Minor9,
    /// A dominant ninth with a perfect eleventh.
    Dominant11,
    /// A dominant eleventh with a major thirteenth.
    Dominant13,
    Dominant7FlatFive,
    Dominant7SharpFive,
    Dominant7FlatNine,
    Dominant7SharpNine,
    ItalianSixth,
    FrenchSixth,
    GermanSixth,
}

/// One row per chord type: the value, the words that name it, the spelled
/// members above the root, and the sentence that describes it.
///
/// One table, so the parser, the language server, `std::harmony`, and the
/// voicing policies cannot disagree about which chords exist or what is in
/// them. The first spelling of each row is the one values print as, and the
/// unison is written explicitly because a chord class is normalized to contain
/// its own root (`03-musical-domains.md` §3) even when a voicing omits it.
const TYPES: [(ChordType, &[&str], &[Interval], &str); 26] = [
    (
        ChordType::Major,
        &["major", "maj"],
        &[M(0, 0), M(2, 4), M(4, 7)],
        "a major third and a perfect fifth",
    ),
    (
        ChordType::Minor,
        &["minor", "min"],
        &[M(0, 0), M(2, 3), M(4, 7)],
        "a minor third and a perfect fifth",
    ),
    (
        ChordType::Diminished,
        &["diminished", "dim"],
        &[M(0, 0), M(2, 3), M(4, 6)],
        "a minor third and a diminished fifth",
    ),
    (
        ChordType::Augmented,
        &["augmented", "aug"],
        &[M(0, 0), M(2, 4), M(4, 8)],
        "a major third and an augmented fifth",
    ),
    (
        ChordType::Suspended2,
        &["sus2"],
        &[M(0, 0), M(1, 2), M(4, 7)],
        "a major second where the third would be, and a perfect fifth",
    ),
    (
        ChordType::Suspended4,
        &["sus4"],
        &[M(0, 0), M(3, 5), M(4, 7)],
        "a perfect fourth where the third would be, and a perfect fifth",
    ),
    (
        ChordType::Major6,
        &["major6", "maj6"],
        &[M(0, 0), M(2, 4), M(4, 7), M(5, 9)],
        "a major triad with an added major sixth",
    ),
    (
        ChordType::Minor6,
        &["minor6", "min6"],
        &[M(0, 0), M(2, 3), M(4, 7), M(5, 9)],
        "a minor triad with an added major sixth",
    ),
    (
        ChordType::Major7,
        &["major7", "maj7"],
        &[M(0, 0), M(2, 4), M(4, 7), M(6, 11)],
        "a major triad with a major seventh",
    ),
    (
        ChordType::Dominant7,
        &["dominant7", "dom7"],
        &[M(0, 0), M(2, 4), M(4, 7), M(6, 10)],
        "a major triad with a minor seventh",
    ),
    (
        ChordType::Minor7,
        &["minor7", "min7"],
        &[M(0, 0), M(2, 3), M(4, 7), M(6, 10)],
        "a minor triad with a minor seventh",
    ),
    (
        ChordType::MinorMajor7,
        &["minor_major7"],
        &[M(0, 0), M(2, 3), M(4, 7), M(6, 11)],
        "a minor triad with a major seventh",
    ),
    (
        ChordType::HalfDiminished7,
        &["half_diminished7"],
        &[M(0, 0), M(2, 3), M(4, 6), M(6, 10)],
        "a diminished triad with a minor seventh",
    ),
    (
        ChordType::Diminished7,
        &["diminished7", "dim7"],
        &[M(0, 0), M(2, 3), M(4, 6), M(6, 9)],
        "a diminished triad with a diminished seventh",
    ),
    (
        ChordType::Dominant9,
        &["dominant9", "dom9"],
        &[M(0, 0), M(2, 4), M(4, 7), M(6, 10), M(8, 14)],
        "a dominant seventh with a major ninth",
    ),
    (
        ChordType::Major9,
        &["major9", "maj9"],
        &[M(0, 0), M(2, 4), M(4, 7), M(6, 11), M(8, 14)],
        "a major seventh with a major ninth",
    ),
    (
        ChordType::Minor9,
        &["minor9", "min9"],
        &[M(0, 0), M(2, 3), M(4, 7), M(6, 10), M(8, 14)],
        "a minor seventh with a major ninth",
    ),
    (
        ChordType::Dominant11,
        &["dominant11", "dom11"],
        &[M(0, 0), M(2, 4), M(4, 7), M(6, 10), M(8, 14), M(10, 17)],
        "a dominant ninth with a perfect eleventh",
    ),
    (
        ChordType::Dominant13,
        &["dominant13", "dom13"],
        &[M(0, 0), M(2, 4), M(4, 7), M(6, 10), M(8, 14), M(10, 17), M(12, 21)],
        "a dominant eleventh with a major thirteenth",
    ),
    (
        ChordType::Dominant7FlatFive,
        &["dominant7b5", "dom7b5"],
        &[M(0, 0), M(2, 4), M(4, 6), M(6, 10)],
        "a dominant seventh with a lowered fifth",
    ),
    (
        ChordType::Dominant7SharpFive,
        &["dominant7s5", "dom7s5"],
        &[M(0, 0), M(2, 4), M(4, 8), M(6, 10)],
        "a dominant seventh with a raised fifth",
    ),
    (
        ChordType::Dominant7FlatNine,
        &["dominant7b9", "dom7b9"],
        &[M(0, 0), M(2, 4), M(4, 7), M(6, 10), M(8, 13)],
        "a dominant seventh with a lowered ninth",
    ),
    (
        ChordType::Dominant7SharpNine,
        &["dominant7s9", "dom7s9"],
        &[M(0, 0), M(2, 4), M(4, 7), M(6, 10), M(8, 15)],
        "a dominant seventh with a raised ninth",
    ),
    (
        ChordType::ItalianSixth,
        &["italian6", "it6"],
        &[M(0, 0), M(2, 4), M(5, 10)],
        "a major third and an augmented sixth, with no fifth",
    ),
    (
        ChordType::FrenchSixth,
        &["french6", "fr6"],
        &[M(0, 0), M(2, 4), M(3, 6), M(5, 10)],
        "a major third, an augmented fourth, and an augmented sixth",
    ),
    (
        ChordType::GermanSixth,
        &["german6", "ger6"],
        &[M(0, 0), M(2, 4), M(4, 7), M(5, 10)],
        "a major third, a perfect fifth, and an augmented sixth",
    ),
];

/// Every word that may follow `chord`, with the sentence that says what it
/// stacks.
///
/// Published for the same reason [`crate::scale::scale_collections`] is: the
/// language server must offer exactly the chord types this table holds. It is
/// a vocabulary, not a value.
pub fn chord_types() -> impl Iterator<Item = (&'static str, &'static str)> {
    TYPES
        .iter()
        .flat_map(|(_, spellings, _, doc)| spellings.iter().map(move |spelling| (*spelling, *doc)))
}

impl ChordType {
    /// The chord type a source word names, or `None` when no type is spelled
    /// that way.
    pub(crate) fn named(word: &str) -> Option<Self> {
        TYPES
            .iter()
            .find(|(_, spellings, _, _)| spellings.contains(&word))
            .map(|(kind, _, _, _)| *kind)
    }

    /// The spelling this type prints as.
    pub(crate) fn name(self) -> &'static str {
        TYPES
            .iter()
            .find(|(kind, _, _, _)| *kind == self)
            .and_then(|(_, spellings, _, _)| spellings.first().copied())
            .unwrap_or("major")
    }

    /// Every word that names a chord type, in declaration order.
    pub(crate) fn spellings() -> impl Iterator<Item = &'static str> {
        TYPES.iter().flat_map(|(_, spellings, _, _)| spellings.iter().copied())
    }

    /// The spelled members above the root, starting at the unison.
    pub(crate) fn members(self) -> &'static [Interval] {
        TYPES
            .iter()
            .find(|(kind, _, _, _)| *kind == self)
            .map_or(&[], |(_, _, members, _)| *members)
    }

    /// The type whose members are exactly these intervals, if the table names
    /// one.
    ///
    /// The inverse of [`ChordType::members`], and partial because the table is
    /// a vocabulary rather than a computation: a diatonic collection can stack
    /// thirds into a sonority conventional harmony has no word for, and this
    /// says so by being absent rather than by inventing a name. Matching is on
    /// the spelled intervals, so a diminished fourth is not a major third and
    /// the two never collapse into one answer.
    pub(crate) fn spelling(members: &[Interval]) -> Option<Self> {
        TYPES
            .iter()
            .find(|(_, _, table, _)| *table == members)
            .map(|(kind, _, _, _)| *kind)
    }
}

/// Rooted spelled content, with no register and no chosen bass unless one was
/// designated.
///
/// The bass is an `Option` and not a default because "no designation" and
/// "the root is in the bass" are different claims: the first says the chord
/// has not been inverted, and the second says a voicing that puts the third
/// lowest disagrees with the class. Only a voicing sounds, so the class never
/// has to answer which C.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ChordClass {
    root: PitchClass,
    kind: ChordType,
    bass: Option<PitchClass>,
}

/// Why a chord class could not be inverted onto a member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InversionError {
    /// The chord has fewer members than the requested position.
    NoSuchMember,
}

impl ChordClass {
    /// The chord type rooted on `root`, in no inversion. Total: every type is
    /// defined over every spelled root class.
    pub(crate) fn new(root: PitchClass, kind: ChordType) -> Self {
        Self { root, kind, bass: None }
    }

    /// The spelled root class.
    pub(crate) fn root(self) -> PitchClass {
        self.root
    }

    /// The chord type.
    pub(crate) fn kind(self) -> ChordType {
        self.kind
    }

    /// The designated bass class, when one was chosen.
    pub(crate) fn bass(self) -> Option<PitchClass> {
        self.bass
    }

    /// The same content rooted somewhere else, with any designated bass
    /// dropped — the bass of a C chord is not a fact about an F chord.
    pub(crate) fn rooted_at(self, root: PitchClass) -> Self {
        Self {
            root,
            kind: self.kind,
            bass: None,
        }
    }

    /// The spelled members above the root, starting at the unison.
    pub(crate) fn members(self) -> &'static [Interval] {
        self.kind.members()
    }

    /// How many members the chord has.
    pub(crate) fn size(self) -> usize {
        self.members().len()
    }

    /// The spelled class of the member at `position`, counting the root as
    /// zero.
    pub(crate) fn member_class(self, position: usize) -> Option<PitchClass> {
        let member = *self.members().get(position)?;
        Some(written(self.root)?.transpose(member)?.pitch_class())
    }

    /// Which member a spelled class is, when it is one.
    ///
    /// Membership is by spelling: a chord containing `a#` does not contain
    /// `bb`, and a caller that meant the other spelling has to write it.
    pub(crate) fn position_of(self, class: PitchClass) -> Option<usize> {
        (0..self.size()).find(|position| self.member_class(*position) == Some(class))
    }

    /// The inversion that puts member `position` in the bass (OMT
    /// `019-inversion.md`).
    ///
    /// This is a true inversion: the designated bass is required to be a
    /// member, which is what the `position` argument guarantees. A bass the
    /// chord does not contain is [`Self::over`] instead, and the two are
    /// deliberately different constructions rather than one lenient one.
    pub(crate) fn inverted(self, position: usize) -> Result<Self, InversionError> {
        let bass = self.member_class(position).ok_or(InversionError::NoSuchMember)?;
        Ok(Self {
            bass: Some(bass),
            ..self
        })
    }

    /// The slash-bass construction: this content over a stated bass class.
    ///
    /// Total, and deliberately so. `c/d` is an ordinary chord with a bass
    /// outside its content, and refusing to write it would be the language
    /// having an opinion the notation does not.
    pub(crate) fn over(self, bass: PitchClass) -> Self {
        Self {
            bass: Some(bass),
            ..self
        }
    }

    /// Whether the designated bass, if any, is a member of the content.
    fn bass_is_member(self) -> bool {
        self.bass.is_none_or(|bass| self.position_of(bass).is_some())
    }
}

/// A written representative of a spelled class, in the register middle C sits
/// in.
///
/// Chord arithmetic is about classes, and a class has no register; this picks
/// one so the spelled interval action can do the work, and every caller takes
/// the class back off the result.
fn written(class: PitchClass) -> Option<WrittenPitch> {
    WrittenPitch::from_heights(
        i64::from(class.letter.steps()).checked_add(28)?,
        i64::from(class.letter.natural_semitone())
            .checked_add(i64::from(class.accidental.0))?
            .checked_add(48)?,
    )
}

/// The lowest pitch strictly above `floor` that spells `class`.
fn next_above(class: PitchClass, floor: WrittenPitch) -> Option<WrittenPitch> {
    let letter = i64::from(class.letter.steps());
    let octaves = floor.diatonic_height().checked_sub(letter)?.div_euclid(7);
    let base = WrittenPitch::from_heights(
        octaves.checked_mul(7)?.checked_add(letter)?,
        octaves
            .checked_mul(12)?
            .checked_add(i64::from(class.letter.natural_semitone()))?
            .checked_add(i64::from(class.accidental.0))?,
    )?;
    if base.diatonic_height() > floor.diatonic_height() {
        return Some(base);
    }
    base.transpose(OCTAVE)
}

impl std::fmt::Display for ChordClass {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "chord {} {}", self.root, self.kind.name())?;
        match self.bass {
            Some(bass) => write!(out, " over {bass}"),
            None => Ok(()),
        }
    }
}

/// A chord class whose content is a major or minor triad.
///
/// The refinement carries no data of its own: it is the evidence that the
/// content passed the check, which is what lets the transformations
/// be total on their domain instead of returning an option at every step.
/// Three members is not enough — a suspended chord has three and no third —
/// and neither is a triadic quality with a seventh stacked on top.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Triad(ChordClass);

impl Triad {
    /// The triad this class is, when it is one.
    pub(crate) fn of(class: ChordClass) -> Option<Self> {
        matches!(class.kind(), ChordType::Major | ChordType::Minor).then_some(Self(class))
    }

    /// The chord class, with the evidence forgotten.
    pub(crate) fn class(self) -> ChordClass {
        self.0
    }

    /// Whether this is the major triad rather than the minor one.
    ///
    /// A `bool` and not a partial answer: the refinement admitted exactly two
    /// chord classes, so "not major" is "minor" here and nowhere else.
    pub(crate) fn is_major(self) -> bool {
        matches!(self.0.kind(), ChordType::Major)
    }
}

impl std::fmt::Display for Triad {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "triad {} {}", self.0.root, self.0.kind.name())
    }
}

/// Exact written pitches, associated with the chord class they voice.
///
/// The only value here that sounds. Its pitches are strictly ascending, so the
/// bass is the first of them; a doubling is two members an octave or more
/// apart and is an ordinary voicing, while the same pitch written twice is
/// not a chord and is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Voicing {
    class: ChordClass,
    /// The lowest sounding pitch, held apart from the rest so that "a voicing
    /// has a bass" is a fact of the type rather than of a length check.
    bass: WrittenPitch,
    /// The pitches above the bass, strictly ascending.
    above: Vec<WrittenPitch>,
}

/// Why a list of pitches is not a voicing of a chord class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VoicingError {
    /// A chord with no notes is a rest, not a voicing.
    Empty,
    /// The pitches were not written low to high, or one was written twice.
    NotAscending,
    /// A pitch spells a class the chord does not contain.
    NotAMember,
    /// The bass sounds a class the chord class designated against.
    WrongBass,
    /// The written coordinates left the range exact arithmetic covers.
    OutOfRange,
}

impl Voicing {
    /// The voicing of `class` sounding exactly `pitches`.
    ///
    /// Checks what a voicing is and nothing more: ascending distinct pitches,
    /// every one of them a member of the class, and a bass agreeing with any
    /// the class designated. Omission is not checked, because omission is the
    /// point of a rootless voicing; spacing is not checked, because spacing is
    /// what a policy chooses.
    pub(crate) fn new(class: ChordClass, pitches: Vec<WrittenPitch>) -> Result<Self, VoicingError> {
        let mut sounding = pitches.into_iter();
        let Some(bass) = sounding.next() else {
            return Err(VoicingError::Empty);
        };
        let above: Vec<WrittenPitch> = sounding.collect();
        let mut previous = bass;
        for pitch in &above {
            if previous.diatonic_height() >= pitch.diatonic_height() {
                return Err(VoicingError::NotAscending);
            }
            previous = *pitch;
        }
        for pitch in std::iter::once(bass).chain(above.iter().copied()) {
            let sounded = pitch.pitch_class();
            if class.position_of(sounded).is_none() && class.bass() != Some(sounded) {
                return Err(VoicingError::NotAMember);
            }
        }
        if let Some(designated) = class.bass()
            && bass.pitch_class() != designated
        {
            return Err(VoicingError::WrongBass);
        }
        Ok(Self { class, bass, above })
    }

    /// The chord class this voicing sounds.
    pub(crate) fn class(&self) -> ChordClass {
        self.class
    }

    /// The pitches, low to high.
    pub(crate) fn pitches(&self) -> impl Iterator<Item = WrittenPitch> + '_ {
        std::iter::once(self.bass).chain(self.above.iter().copied())
    }

    /// How many pitches sound.
    pub(crate) fn size(&self) -> usize {
        self.above.len().saturating_add(1)
    }

    /// The lowest sounding pitch.
    pub(crate) fn bass(&self) -> WrittenPitch {
        self.bass
    }

    /// Which member is in the bass, counting the root as zero (OMT
    /// `019-inversion.md`).
    ///
    /// `None` for a slash bass: the chord sounds over a note it does not
    /// contain, which is a bass choice and not an inversion.
    pub(crate) fn inversion(&self) -> Option<usize> {
        self.class.position_of(self.bass.pitch_class())
    }

    /// The close-position voicing of `class` from `bass` upward.
    ///
    /// One member per chord member, each the lowest pitch above the last that
    /// spells it, starting at the bass. This is the documented policy the
    /// `stack` sugar desugars to. It declines when `bass` does not spell a
    /// member — a close-position C major that starts on D is not a spacing
    /// choice, it is a different chord.
    pub(crate) fn close_position(class: ChordClass, bass: WrittenPitch) -> Result<Self, VoicingError> {
        if !class.bass_is_member() {
            // A designated slash bass is a member of the sound and not of the
            // content, so the ordinary member walk cannot place it.
            return Self::slash_close_position(class, bass);
        }
        let Some(start) = class.position_of(bass.pitch_class()) else {
            return Err(VoicingError::NotAMember);
        };
        let mut pitches = vec![bass];
        for step in 1..class.size() {
            let position = start
                .checked_add(step)
                .and_then(|reached| reached.checked_rem(class.size()))
                .ok_or(VoicingError::OutOfRange)?;
            let member = class.member_class(position).ok_or(VoicingError::NotAMember)?;
            let floor = pitches.last().copied().unwrap_or(bass);
            let next = next_above(member, floor).ok_or(VoicingError::OutOfRange)?;
            pitches.push(next);
        }
        Self::new(class, pitches)
    }

    /// Close position under a bass the content does not contain: the stated
    /// bass, then the whole chord above it from its own root.
    fn slash_close_position(class: ChordClass, bass: WrittenPitch) -> Result<Self, VoicingError> {
        if class.bass() != Some(bass.pitch_class()) {
            return Err(VoicingError::WrongBass);
        }
        let mut pitches = vec![bass];
        for position in 0..class.size() {
            let member = class.member_class(position).ok_or(VoicingError::NotAMember)?;
            let floor = pitches.last().copied().unwrap_or(bass);
            pitches.push(next_above(member, floor).ok_or(VoicingError::OutOfRange)?);
        }
        Self::new(class, pitches)
    }

    /// The drop-`n` voicing: close position with the `n`th voice from the top
    /// moved down an octave (OMT `076-jazz-voicings.md`).
    ///
    /// Drop-2 and drop-3 are the two a guitarist or arranger writes. `n` is
    /// counted from the top because that is how the name reads, and a drop
    /// that would reach past the top voice is refused rather than silently
    /// clamped to drop-1, which is not a voicing anyone means.
    pub(crate) fn dropped(class: ChordClass, bass: WrittenPitch, n: usize) -> Result<Self, VoicingError> {
        let close = Self::close_position(class, bass)?;
        let count = close.size();
        if n < 2 || n > count {
            return Err(VoicingError::NotAscending);
        }
        let index = count.checked_sub(n).ok_or(VoicingError::OutOfRange)?;
        let mut pitches: Vec<WrittenPitch> = close.pitches().collect();
        if index >= pitches.len() {
            return Err(VoicingError::OutOfRange);
        }
        let moved = pitches
            .remove(index)
            .transpose(Interval {
                diatonic_steps: -OCTAVE.diatonic_steps,
                semitones: -OCTAVE.semitones,
            })
            .ok_or(VoicingError::OutOfRange)?;
        pitches.push(moved);
        pitches.sort_by_key(|pitch| (pitch.diatonic_height(), pitch.chromatic_height()));
        Self::new(class, pitches)
    }

    /// The same voicing with member `position` left out.
    ///
    /// The rootless voicings of `076-jazz-voicings.md` are this with
    /// `position` zero, and naming the omission is the point: a chord missing
    /// its root is a choice a player made, not a chord class that lost a note.
    pub(crate) fn omitting(&self, position: usize) -> Result<Self, VoicingError> {
        let Some(omitted) = self.class.member_class(position) else {
            return Err(VoicingError::NotAMember);
        };
        let pitches: Vec<WrittenPitch> = self.pitches().filter(|pitch| pitch.pitch_class() != omitted).collect();
        Self::new(self.class, pitches)
    }
}

impl std::fmt::Display for Voicing {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("voicing")?;
        for pitch in self.pitches() {
            write!(out, " {pitch}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn class(text: &str) -> PitchClass {
        PitchClass::parse(text).expect("a pitch class")
    }

    fn pitch(text: &str) -> WrittenPitch {
        WrittenPitch::parse(text).expect("a pitch literal")
    }

    fn members_of(chord: ChordClass) -> Vec<String> {
        (0..chord.size())
            .filter_map(|position| chord.member_class(position))
            .map(|member| member.to_string())
            .collect()
    }

    /// The reference formula of OMT `017-triads.md` and `018-seventh-chords.md`:
    /// a chord is a stack of thirds, so its members climb the letters two at a
    /// time. Every type in the table has to satisfy that, which is what stops
    /// a semitone-counting spelling from writing a third as a second.
    #[test]
    fn every_chord_type_stacks_thirds_from_its_root() {
        for (kind, spellings, members, _) in TYPES {
            assert_eq!(members.first(), Some(&M(0, 0)), "{spellings:?} omits its root");
            let suspended = matches!(kind, ChordType::Suspended2 | ChordType::Suspended4);
            let sixth = matches!(kind, ChordType::Major6 | ChordType::Minor6);
            let augmented_sixth = matches!(
                kind,
                ChordType::ItalianSixth | ChordType::FrenchSixth | ChordType::GermanSixth
            );
            for (index, member) in members.iter().enumerate() {
                let steps = i64::try_from(index).expect("a small chord");
                if suspended || (sixth && index == 3) || augmented_sixth {
                    // A suspension replaces the third with its neighbour and a
                    // sixth is added above the fifth; both are named exactly
                    // because they are not the third stacked on the third. An
                    // augmented sixth is not a stack of thirds at all — the
                    // interval it is named for is the whole point of it, and
                    // spelling that interval as a seventh would make a German
                    // sixth into the dominant seventh it only sounds like.
                    continue;
                }
                assert_eq!(
                    member.diatonic_steps,
                    steps.checked_mul(2).expect("a small chord"),
                    "{spellings:?} member {index} is not a stacked third"
                );
            }
        }
    }

    /// The members of a chord are distinct spelled classes, in one ascending
    /// order — the representation invariant a voicing's membership check reads.
    #[test]
    fn every_chord_type_has_distinct_ascending_members() {
        for (kind, spellings, members, _) in TYPES {
            for window in members.windows(2) {
                let [previous, next] = window else { continue };
                assert!(
                    previous.diatonic_steps < next.diatonic_steps && previous.semitones < next.semitones,
                    "{spellings:?} is not ascending at {next:?}"
                );
            }
            let chord = ChordClass::new(class("c"), kind);
            let written = members_of(chord);
            let mut seen = written.clone();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), written.len(), "{spellings:?} repeats a spelled class");
        }
    }

    /// Spelling is by letter stack across every root: a C sharp chord spells
    /// E sharp, never F, because the third of C sharp is a kind of E.
    #[test]
    fn chord_spelling_follows_the_letters_across_roots() {
        assert_eq!(
            members_of(ChordClass::new(class("c#"), ChordType::Major)),
            ["c#", "e#", "g#"]
        );
        assert_eq!(
            members_of(ChordClass::new(class("eb"), ChordType::Minor7)),
            ["eb", "gb", "bb", "db"]
        );
        assert_eq!(
            members_of(ChordClass::new(class("b"), ChordType::Diminished7)),
            ["b", "d", "f", "ab"]
        );
        assert_eq!(
            members_of(ChordClass::new(class("f"), ChordType::Major7)),
            ["f", "a", "c", "e"]
        );
    }

    /// Three notes are not a triad. The refinement asks what the notes are.
    #[test]
    fn the_triad_refinement_admits_only_major_and_minor() {
        assert!(Triad::of(ChordClass::new(class("c"), ChordType::Major)).is_some());
        assert!(Triad::of(ChordClass::new(class("c"), ChordType::Minor)).is_some());
        // Three members, no third: `{c, d, g}` is a suspension.
        assert_eq!(Triad::of(ChordClass::new(class("c"), ChordType::Suspended2)), None);
        assert_eq!(Triad::of(ChordClass::new(class("c"), ChordType::Diminished)), None);
        assert_eq!(Triad::of(ChordClass::new(class("c"), ChordType::Major7)), None);
        let major = ChordClass::new(class("g"), ChordType::Major);
        assert_eq!(Triad::of(major).map(Triad::class), Some(major));
    }

    /// An inversion designates a member; a slash bass states a note the chord
    /// does not contain. They are different constructions and stay so.
    #[test]
    fn inversion_designates_a_member_and_a_slash_bass_does_not() {
        let major7 = ChordClass::new(class("c"), ChordType::Major7);
        assert_eq!(major7.bass(), None);
        assert_eq!(major7.inverted(1).map(ChordClass::bass), Ok(Some(class("e"))));
        assert_eq!(major7.inverted(3).map(ChordClass::bass), Ok(Some(class("b"))));
        assert_eq!(major7.inverted(4), Err(InversionError::NoSuchMember));
        // `cmaj7/d` is an ordinary chord and is not a fourth inversion.
        let slash = major7.over(class("d"));
        assert_eq!(slash.bass(), Some(class("d")));
        assert_eq!(slash.position_of(class("d")), None);
    }

    /// The root is not the bass. A first-inversion chord is the same content
    /// with a different lowest note, and the class says so.
    #[test]
    fn the_root_is_not_the_bass() {
        let first = ChordClass::new(class("c"), ChordType::Major)
            .inverted(1)
            .expect("a third");
        assert_eq!(first.root(), class("c"));
        assert_eq!(first.bass(), Some(class("e")));
        let voicing = Voicing::close_position(first, pitch("e4")).expect("a close voicing");
        assert_eq!(voicing.bass(), pitch("e4"));
        assert_eq!(voicing.inversion(), Some(1));
        assert_eq!(
            Voicing::close_position(first, pitch("c4")),
            Err(VoicingError::WrongBass)
        );
    }

    /// Close position walks the members upward from the bass, and the pitches
    /// it writes are the ones a keyboard player's right hand holds.
    #[test]
    fn close_position_stacks_from_the_bass() {
        let major7 = ChordClass::new(class("c"), ChordType::Major7);
        let close = Voicing::close_position(major7, pitch("c4")).expect("a close voicing");
        assert_eq!(
            close.pitches().collect::<Vec<_>>(),
            [pitch("c4"), pitch("e4"), pitch("g4"), pitch("b4")]
        );
        let second = major7.inverted(2).expect("a fifth");
        let from_fifth = Voicing::close_position(second, pitch("g3")).expect("a close voicing");
        assert_eq!(
            from_fifth.pitches().collect::<Vec<_>>(),
            [pitch("g3"), pitch("b3"), pitch("c4"), pitch("e4")]
        );
        assert_eq!(from_fifth.inversion(), Some(2));
    }

    /// Drop-2 and close position are different voicings of one chord class —
    /// the forgetfulness lemma's point, written as a counterexample.
    #[test]
    fn a_drop_voicing_is_not_its_close_position() {
        let major7 = ChordClass::new(class("c"), ChordType::Major7);
        let close = Voicing::close_position(major7, pitch("c4")).expect("a close voicing");
        let drop2 = Voicing::dropped(major7, pitch("c4"), 2).expect("a drop-2 voicing");
        assert_eq!(
            drop2.pitches().collect::<Vec<_>>(),
            [pitch("g3"), pitch("c4"), pitch("e4"), pitch("b4")]
        );
        assert_ne!(drop2, close);
        assert_eq!(drop2.class(), close.class());
        // Dropping the top voice is not a voicing name anyone writes.
        assert_eq!(
            Voicing::dropped(major7, pitch("c4"), 1),
            Err(VoicingError::NotAscending)
        );
        assert_eq!(
            Voicing::dropped(major7, pitch("c4"), 5),
            Err(VoicingError::NotAscending)
        );
    }

    /// A rootless voicing states which member it omits, and remains a voicing
    /// of the chord it omits it from.
    #[test]
    fn an_omission_keeps_the_class_it_omits_from() {
        let ninth = ChordClass::new(class("c"), ChordType::Dominant9);
        let close = Voicing::close_position(ninth, pitch("c3")).expect("a close voicing");
        let rootless = close.omitting(0).expect("a rootless voicing");
        assert_eq!(
            rootless.pitches().collect::<Vec<_>>(),
            [pitch("e3"), pitch("g3"), pitch("bb3"), pitch("d4")]
        );
        assert_eq!(rootless.class(), ninth);
        // The bass is now the third, so the voicing reads as an inversion.
        assert_eq!(rootless.inversion(), Some(1));
    }

    /// A voicing is exact pitches, and the checks are the ones that make it
    /// one: nonempty, ascending, and drawn from the chord.
    #[test]
    fn a_voicing_checks_what_a_voicing_is() {
        let major = ChordClass::new(class("c"), ChordType::Major);
        assert_eq!(Voicing::new(major, Vec::new()), Err(VoicingError::Empty));
        assert_eq!(
            Voicing::new(major, vec![pitch("e4"), pitch("c4")]),
            Err(VoicingError::NotAscending)
        );
        assert_eq!(
            Voicing::new(major, vec![pitch("c4"), pitch("c4")]),
            Err(VoicingError::NotAscending)
        );
        assert_eq!(
            Voicing::new(major, vec![pitch("c4"), pitch("d4")]),
            Err(VoicingError::NotAMember)
        );
        // A doubling is two members an octave apart, and is an ordinary
        // voicing.
        let doubled =
            Voicing::new(major, vec![pitch("c3"), pitch("g3"), pitch("c4"), pitch("e4")]).expect("a doubled voicing");
        assert_eq!(doubled.inversion(), Some(0));
    }

    /// A slash bass sounds under the whole chord, and is the one class a
    /// voicing may hold that the content does not.
    #[test]
    fn a_slash_bass_sounds_under_the_content() {
        let slash = ChordClass::new(class("c"), ChordType::Major).over(class("d"));
        let voicing = Voicing::close_position(slash, pitch("d3")).expect("a slash voicing");
        assert_eq!(
            voicing.pitches().collect::<Vec<_>>(),
            [pitch("d3"), pitch("c4"), pitch("e4"), pitch("g4")]
        );
        assert_eq!(voicing.inversion(), None);
        assert_eq!(
            Voicing::close_position(slash, pitch("c3")),
            Err(VoicingError::WrongBass)
        );
    }

    /// Every spelling names a type and every type prints as a spelling that
    /// parses back to it.
    #[test]
    fn spellings_round_trip() {
        for spelling in ChordType::spellings() {
            let kind = ChordType::named(spelling).expect("a named type");
            assert_eq!(ChordType::named(kind.name()), Some(kind));
        }
        assert_eq!(ChordType::named("majr7"), None);
    }
}
