//! Written pitch (roadmap §6.3, §8.1): the spelled identity of a note.
//!
//! D♯ and E♭ are different values even though 12-TET maps them to one
//! frequency. MIDI numbers and frequencies are derived at the
//! performance/render edge, never stored here.

use serde::{Deserialize, Serialize};

use crate::origin::Interval;

/// The diatonic letter of a written pitch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Letter {
    /// C
    C,
    /// D
    D,
    /// E
    E,
    /// F
    F,
    /// G
    G,
    /// A
    A,
    /// B
    B,
}

impl Letter {
    /// Parse a letter character (`c`..`g`, `a`, `b`).
    pub(crate) fn from_char(ch: char) -> Option<Self> {
        match ch {
            'c' => Some(Self::C),
            'd' => Some(Self::D),
            'e' => Some(Self::E),
            'f' => Some(Self::F),
            'g' => Some(Self::G),
            'a' => Some(Self::A),
            'b' => Some(Self::B),
            _ => None,
        }
    }

    /// The letter as its lowercase character.
    pub fn as_char(self) -> char {
        match self {
            Self::C => 'c',
            Self::D => 'd',
            Self::E => 'e',
            Self::F => 'f',
            Self::G => 'g',
            Self::A => 'a',
            Self::B => 'b',
        }
    }

    /// Diatonic steps above C (C=0, D=1, …, B=6).
    pub fn steps(self) -> i8 {
        match self {
            Self::C => 0,
            Self::D => 1,
            Self::E => 2,
            Self::F => 3,
            Self::G => 4,
            Self::A => 5,
            Self::B => 6,
        }
    }

    /// The letter at a diatonic step above C (0..=6).
    pub fn from_steps(steps: i8) -> Option<Self> {
        match steps {
            0 => Some(Self::C),
            1 => Some(Self::D),
            2 => Some(Self::E),
            3 => Some(Self::F),
            4 => Some(Self::G),
            5 => Some(Self::A),
            6 => Some(Self::B),
            _ => None,
        }
    }

    /// Semitones above C of the natural (unaltered) letter.
    pub(crate) fn natural_semitone(self) -> i8 {
        match self {
            Self::C => 0,
            Self::D => 2,
            Self::E => 4,
            Self::F => 5,
            Self::G => 7,
            Self::A => 9,
            Self::B => 11,
        }
    }
}

/// A pitch alteration in semitones.
///
/// This is an integer coordinate, not a fixed accidental vocabulary. A
/// renderer may have a smaller notational capability, but the semantic value
/// remains exact and may be triply or more deeply altered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Accidental(pub i32);

impl Accidental {
    /// Double sharp (𝄪).
    pub(crate) const DOUBLE_SHARP: Self = Self(2);
    /// Sharp (♯).
    pub(crate) const SHARP: Self = Self(1);
    /// Natural.
    pub const NATURAL: Self = Self(0);
    /// Flat (♭).
    pub(crate) const FLAT: Self = Self(-1);
    /// Double flat (𝄫).
    pub(crate) const DOUBLE_FLAT: Self = Self(-2);
}

/// A written pitch: letter, accidental, octave (middle C = `c4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WrittenPitch {
    /// The diatonic letter.
    pub letter: Letter,
    /// The accidental.
    pub accidental: Accidental,
    /// The octave (scientific pitch notation).
    pub octave: i32,
}

impl WrittenPitch {
    /// Parse source text like `g#4`, `bbb2`, `a-1`, `en5` (the lexer's
    /// pitch-literal shape). Returns `None` for malformed text.
    pub fn parse(text: &str) -> Option<Self> {
        let mut chars = text.chars();
        let letter = Letter::from_char(chars.next()?)?;
        let rest = chars.as_str();
        // Accidental is the longest matching run of sharps or flats. The
        // letter is already off the front, so the `b` here is only ever a
        // flat: `bb2` arrives as letter `b` and rest `b2`.
        let (accidental, octave_text) = if let Some(octave) = rest.strip_prefix('n') {
            (Accidental::NATURAL, octave)
        } else if rest.starts_with('#') {
            let count = rest.bytes().take_while(|byte| *byte == b'#').count();
            (Accidental(i32::try_from(count).ok()?), rest.get(count..)?)
        } else if rest.starts_with('b') {
            let count = rest.bytes().take_while(|byte| *byte == b'b').count();
            (
                Accidental(i32::try_from(count).ok()?.checked_neg()?),
                rest.get(count..)?,
            )
        } else {
            (Accidental::NATURAL, rest)
        };
        let octave = octave_text.parse::<i32>().ok()?;
        Some(Self {
            letter,
            accidental,
            octave,
        })
    }

    /// Semitones above C within the octave, accounting for the accidental.
    pub fn semitone(self) -> i64 {
        i64::from(self.letter.natural_semitone()).saturating_add(i64::from(self.accidental.0))
    }

    /// Absolute staff coordinate above C0.
    pub(crate) fn diatonic_height(self) -> i64 {
        i64::from(self.octave)
            .saturating_mul(7)
            .saturating_add(i64::from(self.letter.steps()))
    }

    /// Absolute twelve-semitone coordinate above C0.
    pub(crate) fn chromatic_height(self) -> i64 {
        i64::from(self.octave)
            .saturating_mul(12)
            .saturating_add(self.semitone())
    }

    /// Build the unique written pitch at two compatible integer coordinates.
    ///
    /// Every pair is compatible: the diatonic coordinate chooses a letter
    /// and octave, while the difference from that natural staff position is
    /// retained as an unbounded accidental.
    pub(crate) fn from_heights(diatonic: i64, chromatic: i64) -> Option<Self> {
        let letter = Letter::from_steps(i8::try_from(diatonic.rem_euclid(7)).ok()?)?;
        let octave = i32::try_from(diatonic.div_euclid(7)).ok()?;
        let natural = i64::from(octave)
            .checked_mul(12)?
            .checked_add(i64::from(letter.natural_semitone()))?;
        Some(Self {
            letter,
            accidental: Accidental(i32::try_from(chromatic.checked_sub(natural)?).ok()?),
            octave,
        })
    }

    /// Transpose by a written interval without respelling.
    ///
    /// Failure means only that the fixed machine integer was exceeded; no
    /// accidental magnitude is rejected.
    pub fn transpose(self, interval: Interval) -> Option<Self> {
        Self::from_heights(
            self.diatonic_height().checked_add(interval.diatonic_steps)?,
            self.chromatic_height().checked_add(interval.semitones)?,
        )
    }

    /// The written interval that carries this pitch to `other`.
    ///
    /// The inverse of [`Self::transpose`] in the sense that matters: for any two
    /// written pitches, `a.transpose(a.between(b)) == Some(b)`. That is the
    /// cancellation half of `docs/rules/language/03-musical-domains.md` §1 —
    /// intervals act on pitches simply and transitively, so the interval between
    /// two of them exists and is unique — and it is what makes `Pitch` and
    /// `Interval` a torsor rather than merely a carrier with an action.
    ///
    /// Written, not sounding: `c4` to `e4` is `M3` and `c4` to `fb4` is `d4`,
    /// because the two answers differ in the letter they count to even where
    /// they agree in semitones. An implementation that subtracted only
    /// [`Self::semitone`] would return one interval for both and lose the
    /// spelling this type exists to keep.
    ///
    /// Failure means only that the fixed machine integer was exceeded, as in
    /// [`Self::transpose`].
    pub fn between(self, other: Self) -> Option<Interval> {
        Some(Interval {
            diatonic_steps: other.diatonic_height().checked_sub(self.diatonic_height())?,
            semitones: other.chromatic_height().checked_sub(self.chromatic_height())?,
        })
    }

    /// Mirror this pitch about `axis` (roadmap §5.4).
    ///
    /// Diatonic, not chromatic: the letter reflects through the axis's
    /// letter and the accidental absorbs the semitone difference, so a
    /// third above the axis comes back a third below it and the result is
    /// still a note an engraver would write. Chromatic mirroring would put
    /// `eb4` a *diminished* somewhere and spell the answer by pitch class,
    /// which is how a spelling-preserving language loses its spelling.
    ///
    /// No accidental magnitude is rejected.
    pub fn invert(self, axis: Self) -> Option<Self> {
        Self::from_heights(
            axis.diatonic_height()
                .checked_mul(2)?
                .checked_sub(self.diatonic_height())?,
            axis.chromatic_height()
                .checked_mul(2)?
                .checked_sub(self.chromatic_height())?,
        )
    }

    /// The pitch class this pitch spells, without its octave.
    pub fn pitch_class(self) -> PitchClass {
        PitchClass {
            letter: self.letter,
            accidental: self.accidental,
        }
    }
}

impl std::fmt::Display for WrittenPitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.pitch_class(), self.octave)
    }
}

/// A pitch class (no octave): the tonic of a key signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PitchClass {
    /// The diatonic letter.
    pub letter: Letter,
    /// The accidental.
    pub accidental: Accidental,
}

impl std::fmt::Display for PitchClass {
    /// As the language spells it: `g`, `bb`, `f#`. One place spells a pitch
    /// class, so a written pitch and a key tonic can never disagree.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = usize::try_from(self.accidental.0.unsigned_abs()).map_err(|_| std::fmt::Error)?;
        let accidental = match self.accidental.0.cmp(&0) {
            std::cmp::Ordering::Greater => "#".repeat(count),
            std::cmp::Ordering::Less => "b".repeat(count),
            std::cmp::Ordering::Equal => String::new(),
        };
        write!(f, "{}{accidental}", self.letter.as_char())
    }
}

impl PitchClass {
    /// Parse a key tonic like `a`, `g#`, `bb`.
    ///
    /// The letter comes off first, so the `b` that remains is a flat and
    /// never the note B: `bb` is B flat, and `b` on its own is B.
    pub fn parse(text: &str) -> Option<Self> {
        let mut chars = text.chars();
        let letter = Letter::from_char(chars.next()?)?;
        let rest = chars.as_str();
        let accidental = if rest.is_empty() {
            Accidental::NATURAL
        } else if rest.bytes().all(|byte| byte == b'#') {
            Accidental(i32::try_from(rest.len()).ok()?)
        } else if rest.bytes().all(|byte| byte == b'b') {
            Accidental(i32::try_from(rest.len()).ok()?.checked_neg()?)
        } else {
            return None;
        };
        Some(Self { letter, accidental })
    }

    /// Transpose by a written interval, without respelling and without an
    /// octave.
    ///
    /// A written interval acts on a pitch class exactly as it acts on a
    /// pitch — the letter moves by the generic size and the accidental
    /// absorbs the rest — so this is that action with the octave dropped
    /// afterwards rather than a second rule. Any octave would give the same
    /// answer; one in the middle of the range is used so that the fixed
    /// machine integer is never what decides.
    ///
    /// Failure means only that a diatonic or chromatic height overflowed,
    /// which no interval a score can write comes near.
    pub fn transpose(self, interval: Interval) -> Option<Self> {
        WrittenPitch {
            letter: self.letter,
            accidental: self.accidental,
            octave: 4,
        }
        .transpose(interval)
        .map(WrittenPitch::pitch_class)
    }
}
