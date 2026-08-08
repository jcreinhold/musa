//! The notation vocabulary, as a table.
//!
//! Musa has no fermata. The reason was never that a fermata is hard — the
//! parser has accepted `g4 1/4 fermata;` since prompt 27 and validates
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

/// One entry in the notation vocabulary: what a mark is called here, and what
/// each backend calls it.
///
/// A mark's spelling in three formats is one fact about the mark. Splitting it
/// three ways is what produced three `match`es that could disagree.
#[derive(Clone, Copy, Debug)]
pub struct MarkDef {
    /// The name the composer writes.
    pub name: &'static str,
    /// MEI's `@artic` value.
    pub mei: &'static str,
    /// The `<articulations>` child element `MusicXML` uses.
    pub musicxml: &'static str,
    /// `LilyPond`'s script suffix. The `^`/`_` side character is the engraver's
    /// decision and is not part of the mark.
    pub lilypond: &'static str,
}

/// Every mark musa reads.
///
/// Exactly the five that existed as enum variants. Rows are added by the
/// prompt that has a piece needing them, never in advance.
pub const VOCABULARY: &[MarkDef] = &[
    MarkDef {
        name: "staccato",
        mei: "stacc",
        musicxml: "staccato",
        lilypond: ".",
    },
    MarkDef {
        name: "staccatissimo",
        mei: "stacciss",
        musicxml: "staccatissimo",
        lilypond: "!",
    },
    MarkDef {
        name: "tenuto",
        mei: "ten",
        musicxml: "tenuto",
        lilypond: "-",
    },
    MarkDef {
        name: "accent",
        mei: "acc",
        musicxml: "accent",
        lilypond: ">",
    },
    MarkDef {
        name: "marcato",
        mei: "marc",
        musicxml: "strong-accent",
        lilypond: "^",
    },
];

/// Look a mark up by the name the composer wrote.
pub fn lookup_mark(name: &str) -> Option<&'static MarkDef> {
    VOCABULARY.iter().find(|def| def.name == name)
}

/// Every mark's name, for the diagnostic that lists them and the suggestion
/// that guesses among them.
pub(crate) fn names() -> Vec<&'static str> {
    VOCABULARY.iter().map(|def| def.name).collect()
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
// canonical payload key (docs/kernel/05 N3) is built from — so ordering marks
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
