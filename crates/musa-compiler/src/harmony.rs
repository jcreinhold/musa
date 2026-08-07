//! Chord symbols (roadmap §8.2): what a lead sheet writes above the staff.
//!
//! A chord symbol is **recorded, never interpreted**. Nothing in musa derives
//! notes from one, checks that the notes under it agree with it, or has an
//! opinion about voice leading — that restraint is the feature, and later
//! theory libraries are algorithms over this model rather than parts of it.
//!
//! What the core does owe such a library is a *structured* symbol: `fmaj7` is
//! an F, major, with a major seventh, not the four letters `fmaj7`. Parsing
//! it here means a consumer never re-implements the spelling rules, and a
//! symbol musa cannot read is a diagnostic at compile time rather than a
//! surprise in a renderer.

use serde::{Deserialize, Serialize};

use crate::pitch::{Accidental, Letter};

/// The triad a symbol names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChordQuality {
    /// `c`, `cmaj`
    Major,
    /// `cm`, `cmin`
    Minor,
    /// `cdim`
    Diminished,
    /// `caug`
    Augmented,
    /// `csus2`
    Suspended2,
    /// `csus4`
    Suspended4,
}

/// The seventh a symbol adds, when it adds one.
///
/// The distinction is the whole reason a symbol is parsed rather than stored:
/// `c7` and `cmaj7` share a triad and differ in one note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Seventh {
    /// A minor seventh: `c7`, `cm7`.
    Minor,
    /// A major seventh: `cmaj7`, `cmmaj7`.
    Major,
    /// A diminished seventh: `cdim7`.
    Diminished,
}

/// A chord symbol as written above the staff.
///
/// `text` is what the composer typed, kept verbatim: a renderer prints the
/// symbol the way it was written, and only a consumer that wants the notes
/// reads the parts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChordSymbol {
    /// The root's letter.
    pub letter: Letter,
    /// The root's accidental.
    pub accidental: Accidental,
    /// The triad.
    pub quality: ChordQuality,
    /// The seventh, when the symbol names one.
    pub seventh: Option<Seventh>,
    /// The highest extension named (`9`, `11`, `13`), when there is one. An
    /// extension implies the seventh below it, which is why `c9` parses with
    /// a minor seventh.
    pub extension: Option<u8>,
    /// The symbol exactly as written (`fmaj7`).
    pub text: String,
}

impl ChordSymbol {
    /// Read a chord symbol as the language spells it: a root (letter plus
    /// optional `s`/`ss`/`f`/`ff`), a quality, and an optional number.
    ///
    /// Returns `None` for anything outside that grammar, which the compiler
    /// turns into a diagnostic naming the symbol. The grammar is deliberately
    /// small — it covers what a lead sheet writes, and extending it is a
    /// language change with a fixture, not a silent regex.
    pub fn parse(text: &str) -> Option<Self> {
        let mut chars = text.chars();
        let letter = Letter::from_char(chars.next()?)?;
        let rest = chars.as_str();
        let (accidental, rest) = if let Some(rest) = rest.strip_prefix("ss") {
            (Accidental::DOUBLE_SHARP, rest)
        } else if let Some(rest) = rest.strip_prefix("ff") {
            (Accidental::DOUBLE_FLAT, rest)
        } else if let Some(rest) = rest.strip_prefix('s').filter(|_| !rest.starts_with("sus")) {
            // `ds` is D sharp and `dsus4` is D suspended: the `s` of `sus`
            // belongs to the quality, not to the root.
            (Accidental::SHARP, rest)
        } else if let Some(rest) = rest.strip_prefix('f') {
            (Accidental::FLAT, rest)
        } else {
            (Accidental::NATURAL, rest)
        };

        // Quality, longest spelling first so `min` is not read as `m` with a
        // leftover `in`. `maj` here marks the seventh as major if a seventh
        // follows, and names a plain major triad if none does.
        let (quality, mut major_seventh, rest) = if let Some(rest) = rest.strip_prefix("maj") {
            (ChordQuality::Major, true, rest)
        } else if let Some(rest) = rest.strip_prefix("min") {
            (ChordQuality::Minor, false, rest)
        } else if let Some(rest) = rest.strip_prefix("dim") {
            (ChordQuality::Diminished, false, rest)
        } else if let Some(rest) = rest.strip_prefix("aug") {
            (ChordQuality::Augmented, false, rest)
        } else if let Some(rest) = rest.strip_prefix("sus2") {
            (ChordQuality::Suspended2, false, rest)
        } else if let Some(rest) = rest.strip_prefix("sus4") {
            (ChordQuality::Suspended4, false, rest)
        } else if let Some(rest) = rest.strip_prefix('m') {
            (ChordQuality::Minor, false, rest)
        } else {
            (ChordQuality::Major, false, rest)
        };

        // A minor chord with a major seventh is written `cmmaj7`: the quality
        // reader has taken the `m`, and `maj` is what is left.
        let rest = match rest.strip_prefix("maj") {
            Some(rest) => {
                major_seventh = true;
                rest
            }
            None => rest,
        };

        let (seventh, extension) = match rest {
            "" => (None, None),
            "6" => (None, Some(6)),
            "7" => (Some(seventh_for(quality, major_seventh)), None),
            "9" | "11" | "13" => (
                Some(seventh_for(quality, major_seventh)),
                Some(rest.parse::<u8>().ok()?),
            ),
            _ => return None,
        };
        Some(Self {
            letter,
            accidental,
            quality,
            seventh,
            extension,
            text: text.to_owned(),
        })
    }
}

/// Which seventh a quality takes when the symbol does not say `maj`.
fn seventh_for(quality: ChordQuality, major_seventh: bool) -> Seventh {
    if major_seventh {
        return Seventh::Major;
    }
    match quality {
        ChordQuality::Diminished => Seventh::Diminished,
        ChordQuality::Major | ChordQuality::Minor | ChordQuality::Augmented => Seventh::Minor,
        ChordQuality::Suspended2 | ChordQuality::Suspended4 => Seventh::Minor,
    }
}
