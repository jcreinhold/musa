//! Written pitch (roadmap §6.3, §8.1): the spelled identity of a note. D♯
//! and E♭ are different values even though 12-TET maps them to one
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
    pub fn from_char(ch: char) -> Option<Self> {
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
    pub fn natural_semitone(self) -> i8 {
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

/// A pitch alteration in semitones: `ss`=+2, `s`=+1, none=0, `f`=-1,
/// `ff`=-2.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Accidental(pub i8);

impl Accidental {
    /// Double sharp (𝄪).
    pub const DOUBLE_SHARP: Self = Self(2);
    /// Sharp (♯).
    pub const SHARP: Self = Self(1);
    /// Natural.
    pub const NATURAL: Self = Self(0);
    /// Flat (♭).
    pub const FLAT: Self = Self(-1);
    /// Double flat (𝄫).
    pub const DOUBLE_FLAT: Self = Self(-2);
}

/// A written pitch: letter, accidental, octave (middle C = `c4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WrittenPitch {
    /// The diatonic letter.
    pub letter: Letter,
    /// The accidental.
    pub accidental: Accidental,
    /// The octave (scientific pitch notation).
    pub octave: i8,
}

impl WrittenPitch {
    /// Parse source text like `gs4`, `bff2`, `a-1`, `en5` (the lexer's
    /// pitch-literal shape). Returns `None` for malformed text.
    pub fn parse(text: &str) -> Option<Self> {
        let mut chars = text.chars();
        let letter = Letter::from_char(chars.next()?)?;
        let rest = chars.as_str();
        // Accidental is the longest matching prefix of s/ss/f/ff/n.
        let (accidental, octave_text) = if let Some(octave) = rest.strip_prefix("ss") {
            (Accidental::DOUBLE_SHARP, octave)
        } else if let Some(octave) = rest.strip_prefix("ff") {
            (Accidental::DOUBLE_FLAT, octave)
        } else if let Some(octave) = rest.strip_prefix('s') {
            (Accidental::SHARP, octave)
        } else if let Some(octave) = rest.strip_prefix('f') {
            (Accidental::FLAT, octave)
        } else if let Some(octave) = rest.strip_prefix('n') {
            (Accidental::NATURAL, octave)
        } else {
            (Accidental::NATURAL, rest)
        };
        let octave = octave_text.parse::<i8>().ok()?;
        Some(Self {
            letter,
            accidental,
            octave,
        })
    }

    /// Semitones above C within the octave, accounting for the accidental.
    pub fn semitone(self) -> i8 {
        self.letter.natural_semitone().saturating_add(self.accidental.0)
    }

    /// Transpose by an interval, keeping the result spellable (roadmap
    /// §5.4): the letter moves by the interval's diatonic steps and the
    /// accidental absorbs whatever semitone difference remains. Returns
    /// `None` when the result needs more than a double accidental.
    pub fn transpose(self, interval: Interval) -> Option<Self> {
        let steps = i32::from(self.letter.steps()).saturating_add(i32::from(interval.diatonic_steps));
        let letter = Letter::from_steps(i8::try_from(steps.rem_euclid(7)).ok()?)?;
        let octave_shift = steps.div_euclid(7);
        let current = i32::from(self.octave)
            .saturating_mul(12)
            .saturating_add(i32::from(self.semitone()));
        let moved = current.saturating_add(i32::from(interval.semitones));
        let octave_i32 = i32::from(self.octave).saturating_add(octave_shift);
        let natural = octave_i32
            .saturating_mul(12)
            .saturating_add(i32::from(letter.natural_semitone()));
        let accidental = moved.saturating_sub(natural);
        if !(-2..=2).contains(&accidental) {
            return None;
        }
        Some(Self {
            letter,
            accidental: Accidental(i8::try_from(accidental).ok()?),
            octave: i8::try_from(octave_i32).ok()?,
        })
    }
}

impl std::fmt::Display for WrittenPitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let accidental = match self.accidental.0 {
            2 => "ss",
            1 => "s",
            0 => "",
            -1 => "f",
            -2 => "ff",
            _ => "?",
        };
        write!(f, "{}{}{}", self.letter.as_char(), accidental, self.octave)
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

impl PitchClass {
    /// Parse a key tonic like `a`, `gs`, `bf`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut chars = text.chars();
        let letter = Letter::from_char(chars.next()?)?;
        let accidental = match chars.as_str() {
            "" => Accidental::NATURAL,
            "s" => Accidental::SHARP,
            "ss" => Accidental::DOUBLE_SHARP,
            "f" => Accidental::FLAT,
            "ff" => Accidental::DOUBLE_FLAT,
            _ => return None,
        };
        Some(Self { letter, accidental })
    }
}
