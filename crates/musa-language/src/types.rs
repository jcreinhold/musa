//! The type vocabulary, and the spellings it replaced.
//!
//! A type is spelled with a capital. That is what lets `key` be a statement
//! and `Key` be a type without the parser holding a whitelist of the keywords
//! a type is allowed to be: the lexer settles it, because the two words are
//! not the same word.
//!
//! Both lists are data rather than a match, because three programs need them
//! — the parser, to say what a removed spelling became; the compiler, to read
//! a type; and the language server, to offer one — and a vocabulary each of
//! them spelled separately would be three vocabularies.

/// Every value type the compiler owns and does not parameterize, with the one
/// line an editor shows beside it.
///
/// `Option` and `List` are absent because they are keywords and carry their
/// own documentation (`keywords.rs`); everything here is an ordinary
/// identifier the lexer cannot tell from any other.
pub const PRIMITIVE_TYPES: &[(&str, &str)] = &[
    ("Unit", "the type with exactly one value"),
    ("Bool", "`true` or `false`"),
    ("Nat", "a whole number, zero or more"),
    ("Ratio", "an exact rational number"),
    ("Duration", "a notated duration, as an exact rational"),
    ("Pitch", "a written pitch: letter, accidental, and octave"),
    (
        "NoteName",
        "a letter and an accidental, with no octave: C♯ and D♭ are two",
    ),
    ("Interval", "a written interval: quality and number"),
    ("Scale", "an ordered collection rooted on a tonic"),
    ("Key", "a tonic and a mode, as a key signature means them"),
    ("Degree", "a position in a scale, with any alteration"),
    ("Frame", "a scale placed in a register"),
    ("ChordClass", "a root and a quality, before any voicing"),
    ("Triad", "a `ChordClass` with exactly three members"),
    ("Roman", "a roman numeral, read against a key"),
    ("Voicing", "a chord class laid out in actual pitches"),
    ("Pc12", "a pitch class modulo twelve, where C♯ and D♭ are one"),
    ("PcSet12", "a set of `Pc12`s"),
    ("Row12", "an ordering of all twelve `Pc12`s"),
    ("Music", "notated material"),
];

/// Every type spelling this language removed, with the one that replaced it.
///
/// The table is finite and closed: it exists so a file written against the
/// old vocabulary gets one complaint carrying the rewrite, and nothing is
/// ever added to it except by another deliberate respelling.
pub const RESPELLED_TYPES: &[(&str, &str)] = &[
    ("unit", "Unit"),
    ("bool", "Bool"),
    ("nat", "Nat"),
    ("ratio", "Ratio"),
    ("duration", "Duration"),
    ("pitch", "Pitch"),
    ("pitchclass", "NoteName"),
    ("interval", "Interval"),
    ("scale", "Scale"),
    ("key", "Key"),
    ("degree", "Degree"),
    ("frame", "Frame"),
    ("chord_class", "ChordClass"),
    ("triad", "Triad"),
    ("roman", "Roman"),
    ("voicing", "Voicing"),
    ("pc12", "Pc12"),
    ("pcset12", "PcSet12"),
    ("row12", "Row12"),
    ("music", "Music"),
    ("option", "Option"),
    ("list", "List"),
];

/// The spelling that replaced `name`, when `name` is one this language
/// removed.
#[must_use]
pub fn respelled_type(name: &str) -> Option<&'static str> {
    RESPELLED_TYPES
        .iter()
        .find(|(was, _)| *was == name)
        .map(|(_, now)| *now)
}

#[cfg(test)]
mod tests {
    use super::{PRIMITIVE_TYPES, RESPELLED_TYPES, respelled_type};

    #[test]
    fn every_primitive_is_reachable_from_the_spelling_it_replaced() {
        for (name, _) in PRIMITIVE_TYPES {
            assert!(
                RESPELLED_TYPES.iter().any(|(_, now)| now == name),
                "{name} has no old spelling, so a file written before this change cannot be told what to write"
            );
        }
    }

    #[test]
    fn a_replacement_is_never_itself_replaced() {
        for (_, now) in RESPELLED_TYPES {
            assert_eq!(
                respelled_type(now),
                None,
                "`{now}` is both a spelling and a removed one"
            );
        }
    }
}
