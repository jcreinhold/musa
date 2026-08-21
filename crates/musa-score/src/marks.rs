//! The notation vocabulary, as a table.
//!
//! Musa has no fermata. The reason was never that a fermata is hard — the
//! parser has accepted `g4/4 fermata` and validates
//! nothing — but that the compiler's `ArticulationMark` was a closed enum of
//! five, so a sixth mark meant editing four files: the enum, its name list,
//! its parser and its printer in `score.rs`, plus one `match` arm in each of
//! MEI, `LilyPond`, and `MusicXML`.
//!
//! The repository already contains the decision this module applies. Studio
//! processor names are deliberately not keywords (`musa-language`'s
//! `syntax_kind`), "so the studio vocabulary can grow without lexer changes".
//! Notation gets the same treatment: a mark is a row, and the backends read
//! the row rather than each carrying their own five-way `match` that could
//! drift from the others.
//!
//! **What must never appear here is performance.** A `gate`, an `amplitude`,
//! a `steal`, or anything else describing what a mark *does* belongs to a
//! profile (roadmap §2: an articulation as written is not a gate multiplier).
//! If a column starts to look like an interpretation, it is in the wrong file.

use std::hash::{Hash, Hasher};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Where a mark attaches in *time*.
///
/// This is the question the grammar asks: a note-anchored mark trails a note's
/// duration, a point is written where the cursor stands, and a span wraps the
/// music it covers in a block. The table answering it is why `mark` is one
/// keyword rather than one keyword per shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// On the note it trails: `g4/4 staccato`.
    Note(Slot),
    /// At the instant it is written: `mark breath;`.
    Point,
    /// Over the music inside its block: `mark pedal { … }`.
    Span,
}

/// Where a note-anchored mark prints in the *notation*.
///
/// Four values, and they are not musa's invention: they are the children
/// `MusicXML` puts under `<notations>`, and the same division MEI makes
/// between `@artic` and the ornament control events. Each backend reads this
/// once, in a four-arm `match` that does not grow when a row is added — which
/// is the property the table exists for. Putting the slot in the row instead
/// would only move the four arms into fifteen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// How the note is attacked and released: staccato, tenuto, accent.
    Articulation,
    /// A figure played instead of the note: trill, mordent, turn.
    Ornament,
    /// How the player's hands do it: bowing, a harmonic.
    Technical,
    /// A held note. Its own slot in every format, because it is the one mark
    /// that changes the note's duration rather than its sound.
    Fermata,
}

/// What a mark is written with, when it is written with anything.
///
/// One argument at most. A second would need names to tell them apart
/// (`mark sample "kick" gain 0.8`), and `gain` is the studio's word for a
/// thing the studio decides — the page prints the sample's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Argument {
    /// Nothing follows the name.
    None,
    /// A quoted string: `mark text "sul ponticello";`.
    Text,
    /// A whole number, which may be negative: `mark ottava -1 { … }`.
    Number,
}

/// One entry in the notation vocabulary: what a mark is called here, where it
/// attaches, what it is written with, and what each backend calls it.
///
/// A mark's spelling in three formats is one fact about the mark. Splitting it
/// three ways is what produced three `match`es that could disagree.
///
/// A backend column is `None` when the format has no way to say this mark.
/// That is a fact about the format, so it becomes an export warning rather
/// than a silent omission (`docs/rules/kernel/07-backend-contract.md`). What a column
/// *means* follows the anchor: for [`Anchor::Note`] it is the slot's spelling
/// (MEI `@artic` or ornament element, the `MusicXML` child, the `LilyPond`
/// script), and for a point or a span it is the control element MEI writes,
/// the `<direction-type>` child `MusicXML` writes, and the `LilyPond` command.
#[derive(Clone, Copy, Debug)]
pub struct MarkDef {
    /// The name the composer writes.
    pub name: &'static str,
    /// Where it attaches in time, and — on a note — in the notation.
    pub anchor: Anchor,
    /// What is written after the name.
    pub takes: Argument,
    /// The mark notation writes for it, when notation writes one — `>` for an
    /// accent, `^` for a marcato — or `None` when the word is the only
    /// spelling. A shorthand is one token, so it can sit on a note with no
    /// space in front of it.
    pub shorthand: Option<&'static str>,
    /// MEI's spelling, or `None` when MEI cannot say it.
    pub mei: Option<&'static str>,
    /// `MusicXML`'s spelling, or `None` when `MusicXML` cannot say it.
    pub musicxml: Option<&'static str>,
    /// `LilyPond`'s spelling, or `None` when `LilyPond` cannot say it. For an
    /// articulation this is the script suffix; the `^`/`_` side character is
    /// the engraver's decision and is not part of the mark.
    pub lilypond: Option<&'static str>,
}

/// Every mark musa reads.
///
/// Rows are added by the change that has a piece needing them, never in
/// advance. Every row below the five articulations
/// has a piece in `examples/` that writes it.
pub const VOCABULARY: &[MarkDef] = &[
    MarkDef {
        name: "staccato",
        shorthand: None,
        anchor: Anchor::Note(Slot::Articulation),
        takes: Argument::None,
        mei: Some("stacc"),
        musicxml: Some("staccato"),
        lilypond: Some("."),
    },
    MarkDef {
        name: "staccatissimo",
        shorthand: None,
        anchor: Anchor::Note(Slot::Articulation),
        takes: Argument::None,
        mei: Some("stacciss"),
        musicxml: Some("staccatissimo"),
        lilypond: Some("!"),
    },
    MarkDef {
        name: "tenuto",
        shorthand: None,
        anchor: Anchor::Note(Slot::Articulation),
        takes: Argument::None,
        mei: Some("ten"),
        musicxml: Some("tenuto"),
        lilypond: Some("-"),
    },
    MarkDef {
        name: "accent",
        shorthand: Some(">"),
        anchor: Anchor::Note(Slot::Articulation),
        takes: Argument::None,
        mei: Some("acc"),
        musicxml: Some("accent"),
        lilypond: Some(">"),
    },
    MarkDef {
        name: "marcato",
        shorthand: Some("^"),
        anchor: Anchor::Note(Slot::Articulation),
        takes: Argument::None,
        mei: Some("marc"),
        musicxml: Some("strong-accent"),
        lilypond: Some("^"),
    },
    // A fermata holds; how long is the profile's answer, not the page's
    // (roadmap §2). Nothing here says how long.
    MarkDef {
        name: "fermata",
        shorthand: None,
        anchor: Anchor::Note(Slot::Fermata),
        takes: Argument::None,
        mei: Some("fermata"),
        musicxml: Some("fermata"),
        lilypond: Some("\\fermata"),
    },
    MarkDef {
        name: "trill",
        shorthand: None,
        anchor: Anchor::Note(Slot::Ornament),
        takes: Argument::None,
        mei: Some("trill"),
        musicxml: Some("trill-mark"),
        lilypond: Some("\\trill"),
    },
    MarkDef {
        name: "mordent",
        shorthand: None,
        anchor: Anchor::Note(Slot::Ornament),
        takes: Argument::None,
        mei: Some("mordent"),
        musicxml: Some("mordent"),
        lilypond: Some("\\mordent"),
    },
    MarkDef {
        name: "turn",
        shorthand: None,
        anchor: Anchor::Note(Slot::Ornament),
        takes: Argument::None,
        mei: Some("turn"),
        musicxml: Some("turn"),
        lilypond: Some("\\turn"),
    },
    MarkDef {
        name: "harmonic",
        shorthand: None,
        anchor: Anchor::Note(Slot::Technical),
        takes: Argument::None,
        mei: Some("harm"),
        musicxml: Some("harmonic"),
        lilypond: Some("\\flageolet"),
    },
    MarkDef {
        name: "upbow",
        shorthand: None,
        anchor: Anchor::Note(Slot::Technical),
        takes: Argument::None,
        mei: Some("upbow"),
        musicxml: Some("up-bow"),
        lilypond: Some("\\upbow"),
    },
    MarkDef {
        name: "downbow",
        shorthand: None,
        anchor: Anchor::Note(Slot::Technical),
        takes: Argument::None,
        mei: Some("dnbow"),
        musicxml: Some("down-bow"),
        lilypond: Some("\\downbow"),
    },
    // A breath falls *between* notes, so it is a point rather than something
    // on a note. `MusicXML` disagrees — it files both of these under a note's
    // articulations — so both columns are `None` and the export says so.
    MarkDef {
        name: "breath",
        shorthand: None,
        anchor: Anchor::Point,
        takes: Argument::None,
        mei: Some("breath"),
        musicxml: None,
        lilypond: Some("\\breathe"),
    },
    MarkDef {
        name: "caesura",
        shorthand: None,
        anchor: Anchor::Point,
        takes: Argument::None,
        mei: Some("caesura"),
        musicxml: None,
        lilypond: Some("\\breathe"),
    },
    MarkDef {
        name: "text",
        shorthand: None,
        anchor: Anchor::Point,
        takes: Argument::Text,
        mei: Some("dir"),
        musicxml: Some("words"),
        lilypond: Some("\\markup"),
    },
    MarkDef {
        name: "rehearsal",
        shorthand: None,
        anchor: Anchor::Point,
        takes: Argument::Text,
        mei: Some("reh"),
        musicxml: Some("rehearsal"),
        lilypond: Some("\\mark"),
    },
    // A drum chart prints its sample's name above the staff exactly like a
    // text direction, and every format writes it as one. Triggering it is the
    // studio's job and is not notation.
    MarkDef {
        name: "sample",
        shorthand: None,
        anchor: Anchor::Point,
        takes: Argument::Text,
        mei: Some("dir"),
        musicxml: Some("words"),
        lilypond: Some("\\markup"),
    },
    MarkDef {
        name: "cue",
        shorthand: None,
        anchor: Anchor::Point,
        takes: Argument::Text,
        mei: Some("dir"),
        musicxml: Some("words"),
        lilypond: Some("\\markup"),
    },
    MarkDef {
        name: "pedal",
        shorthand: None,
        anchor: Anchor::Span,
        takes: Argument::None,
        mei: Some("pedal"),
        musicxml: Some("pedal"),
        lilypond: Some("\\sustain"),
    },
    MarkDef {
        name: "ottava",
        shorthand: None,
        anchor: Anchor::Span,
        takes: Argument::Number,
        mei: Some("octave"),
        musicxml: Some("octave-shift"),
        lilypond: Some("\\ottava"),
    },
];

/// The value written after a mark's name.
///
/// Ordered and hashed, because it is part of a fact's canonical payload key
/// (`docs/rules/kernel/05` N3): two `mark text` occurrences over the same span are
/// the same fact only when they say the same thing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MarkArgument {
    /// A quoted string, as written.
    Text(String),
    /// A whole number, which may be negative.
    Number(i32),
}

impl MarkArgument {
    /// Which [`Argument`] this value satisfies.
    pub fn kind(&self) -> Argument {
        match self {
            Self::Text(_) => Argument::Text,
            Self::Number(_) => Argument::Number,
        }
    }
}

impl std::fmt::Display for MarkArgument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(text) => f.write_str(text),
            Self::Number(number) => write!(f, "{number}"),
        }
    }
}

/// Look a mark up by the name the composer wrote.
pub fn lookup_mark(name: &str) -> Option<&'static MarkDef> {
    VOCABULARY.iter().find(|def| def.name == name)
}

/// Every mark's name, for the diagnostic that lists them and the suggestion
/// that guesses among them.
pub fn names() -> Vec<&'static str> {
    VOCABULARY.iter().map(|def| def.name).collect()
}

/// The names a note may trail, for the suggestion that guesses among them.
///
/// Separate from [`names`] because the two sites accept different halves of
/// the table: offering `pedal` to someone who wrote `g4 1/4 pedale;` would
/// send them to a mark they cannot write there.
pub fn note_names() -> Vec<&'static str> {
    VOCABULARY
        .iter()
        .filter(|def| matches!(def.anchor, Anchor::Note(_)))
        .map(|def| def.name)
        .collect()
}

/// The names the `mark` statement accepts.
pub fn statement_names() -> Vec<&'static str> {
    VOCABULARY
        .iter()
        .filter(|def| matches!(def.anchor, Anchor::Point | Anchor::Span))
        .map(|def| def.name)
        .collect()
}

/// A mark as written (roadmap §2: not a gate multiplier).
///
/// Holds its vocabulary row rather than a string, so asking what MEI calls it
/// cannot fail. A name that is not in the vocabulary is rejected where it is
/// read — by [`lookup_mark`] on the way in, and by `Deserialize` on the way back —
/// which is what leaves every later question total.
#[derive(Clone, Copy)]
pub struct Mark(&'static MarkDef);

impl Mark {
    /// The mark of this name, or `None` when musa has no such mark.
    pub fn parse(name: &str) -> Option<Self> {
        lookup_mark(name).map(Self)
    }

    /// The name, spelled as it is written.
    pub fn name(self) -> &'static str {
        self.0.name
    }

    /// The vocabulary row: what each backend calls it.
    pub fn def(self) -> &'static MarkDef {
        self.0
    }

    /// Where this mark attaches.
    pub fn anchor(self) -> Anchor {
        self.0.anchor
    }

    /// The slot this mark prints in, when it is written on a note.
    pub fn slot(self) -> Option<Slot> {
        match self.0.anchor {
            Anchor::Note(slot) => Some(slot),
            Anchor::Point | Anchor::Span => None,
        }
    }

    /// What is written after the name.
    pub fn takes(self) -> Argument {
        self.0.takes
    }
}

// The name, not the row: a debug snapshot that printed every backend spelling
// would move whenever a backend's column changed, which is a fact about the
// exporter and not about the score being snapshotted.
impl std::fmt::Debug for Mark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mark({:?})", self.0.name)
    }
}

impl std::fmt::Display for Mark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.name)
    }
}

// Identity is the name. Two rows never share one, and the name is what the
// canonical payload key (docs/rules/kernel/05 N3) is built from — so ordering marks
// by anything else would make the semantic hash depend on table order.
impl PartialEq for Mark {
    fn eq(&self, other: &Self) -> bool {
        self.0.name == other.0.name
    }
}

impl Eq for Mark {}

impl PartialOrd for Mark {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Mark {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.name.cmp(other.0.name)
    }
}

impl Hash for Mark {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.name.hash(state);
    }
}

impl Serialize for Mark {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.name)
    }
}

impl<'de> Deserialize<'de> for Mark {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Self::parse(&name).ok_or_else(|| serde::de::Error::custom(format!("`{name}` is not a mark musa knows")))
    }
}
