//! What an editor needs from events text (docs/rules/events/01-grammar.md): how to
//! colour it, and what to put in an outline.
//!
//! The grammar lives in this crate, so its lexis does too. An editor that
//! coloured events files from a copy of the keyword list somewhere else would
//! be a second grammar with no test holding it to the first; here, adding a
//! keyword to [`crate::text`]'s cursor and forgetting it below is a difference
//! two lines of test can see.
//!
//! Everything here is *classification*, not parsing. It is total, it never
//! fails, and it makes no claim that the file is well formed — a half-typed
//! events file still colours and still outlines, which is the whole reason an
//! editor cannot use the parser for either.
//!
//! Byte-offset advances are total on the sizes an events file can have; the
//! workspace arithmetic lint is allowed module-wide (see musa-events/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use std::ops::Range;

/// The words the grammar reserves, in the order the reference gives them.
///
/// `EventTrack` is not here: it is a *type* and is classified as one, which is
/// the distinction a reader wants — `track` builds a value and `EventTrack`
/// names what the value is.
const KEYWORDS: [&str; 14] = [
    "events",
    "composition",
    "track",
    "occurrence",
    "let",
    "in",
    "follow",
    "together",
    "shift",
    "scale",
    "restrict",
    "from",
    "to",
    "by",
];

/// What one span of events text is.
///
/// Seven classes, and deliberately not more: the format has no operators to
/// distinguish, no lifetimes, no attributes, and one comment form. A class
/// per *visual* decision is what an editor can actually use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventsTokenClass {
    /// A `%` line — the header, and every note under it.
    Comment,
    /// A word the grammar reserves.
    Keyword,
    /// `EventTrack`, and the coordinate and payload type inside its brackets.
    Type,
    /// A bound name: what a `let` introduces and a variable spells.
    Name,
    /// A rational, or one side of one.
    Number,
    /// A quoted string: the composition's name, a payload, a mark.
    Text,
    /// Brackets, braces, separators, `=`, `@`.
    Punctuation,
}

/// Classify every non-whitespace span of `text`, in order.
///
/// Total: unrecognized bytes come back as [`EventsTokenClass::Punctuation`] one at a
/// time rather than being skipped, so the spans a caller receives tile the
/// document apart from its whitespace. Whitespace itself is not reported —
/// nobody colours it, and reporting it would double the traffic to say so.
pub fn events_classify(text: &str) -> Vec<(Range<usize>, EventsTokenClass)> {
    let mut out = Vec::new();
    let mut at = 0;
    // The coordinate and payload type follow `EventTrack` and `[`, and are the
    // one place a bare word means a type rather than a variable. Tracking that
    // costs one bool and saves an editor from colouring `WrittenTime` and
    // `ScoreFact` as `let`-bound names.
    let mut after_event_track = false;
    while at < text.len() {
        let rest = text.get(at..).unwrap_or_default();
        let Some(character) = rest.chars().next() else {
            break;
        };
        if character.is_whitespace() {
            at += character.len_utf8();
            continue;
        }
        let (length, class) = match character {
            '%' => (rest.find('\n').unwrap_or(rest.len()), EventsTokenClass::Comment),
            '"' => (string_length(rest), EventsTokenClass::Text),
            '0'..='9' => (
                span_of(rest, |c| c.is_ascii_digit() || c == '/'),
                EventsTokenClass::Number,
            ),
            '-' if rest
                .get(1..2)
                .is_some_and(|next| next.starts_with(|c: char| c.is_ascii_digit())) =>
            {
                (
                    span_of(rest, |c| c.is_ascii_digit() || c == '/' || c == '-'),
                    EventsTokenClass::Number,
                )
            }
            c if c.is_alphabetic() || c == '_' => {
                let length = span_of(rest, |c| c.is_alphanumeric() || c == '_' || c == '-');
                let word = rest.get(..length).unwrap_or_default();
                // The type constructor and its two arguments:
                // `EventTrack[WrittenTime, ScoreFact]` is one type written in
                // three words, and the last one is whatever the payload calls
                // itself.
                let class = if word == "EventTrack" || after_event_track {
                    EventsTokenClass::Type
                } else if KEYWORDS.contains(&word) {
                    EventsTokenClass::Keyword
                } else {
                    EventsTokenClass::Name
                };
                after_event_track = word == "EventTrack";
                (length, class)
            }
            _ => (character.len_utf8(), EventsTokenClass::Punctuation),
        };
        // `[` and the `,` between the two arguments keep the flag alive;
        // anything else ends it, so a stray `EventTrack` on its own line does
        // not retype the next word in the file.
        if !matches!(class, EventsTokenClass::Type) && character != '[' && character != ',' {
            after_event_track = false;
        }
        let length = length.max(1);
        out.push((at..(at + length).min(text.len()), class));
        at += length;
    }
    out
}

/// How far a run of characters satisfying `keep` extends.
fn span_of(rest: &str, keep: impl Fn(char) -> bool) -> usize {
    rest.find(|c: char| !keep(c)).unwrap_or(rest.len())
}

/// How long the string literal starting at `rest` is, escapes included.
///
/// An unterminated string runs to the end of the file rather than to the end
/// of the line: that is what the reader does with it, and a highlighter that
/// disagreed would show a file the parser does not see.
fn string_length(rest: &str) -> usize {
    let mut length = 1;
    let mut escaped = false;
    for character in rest.chars().skip(1) {
        length += character.len_utf8();
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return length;
        }
    }
    length
}

/// Every name a `let` binds, with the span of the name itself.
///
/// The outline of an event-track term: a file's structure is the shared material it
/// names, because that is the only thing in the grammar a reader navigates
/// *to*. `follow` and `together` blocks are structure too, but anonymous
/// structure, and an outline entry with no name is a row that says nothing.
///
/// Built on [`events_classify`] rather than on the parser, for the reason the module
/// gives: an outline that vanished while a file was mid-edit would be an
/// outline nobody could use to navigate the edit.
pub fn events_bindings(text: &str) -> Vec<(Range<usize>, String)> {
    let tokens = events_classify(text);
    let mut out = Vec::new();
    for pair in tokens.windows(2) {
        let [(keyword, EventsTokenClass::Keyword), (name, EventsTokenClass::Name)] = pair else {
            continue;
        };
        if text.get(keyword.clone()) == Some("let")
            && let Some(spelling) = text.get(name.clone())
        {
            out.push((name.clone(), spelling.to_owned()));
        }
    }
    out
}

/// What an event track word means, for an editor to show under a caret.
///
/// One sentence each, and no examples: the reference is
/// `docs/rules/events/01-grammar.md` and `10-term-calculus.md`, and a hover that
/// tried to be the reference would be a hover nobody finishes reading. The
/// sentences say what the construct *denotes*, because that is the question a
/// reader of interchange text actually has.
pub fn events_keyword_doc(word: &str) -> Option<&'static str> {
    Some(match word {
        "events" => "Opens the file and names the work it projects.",
        "composition" => "The file's one composition, and the type of event track it denotes.",
        "EventTrack" => "The type of a finite event track, in one coordinate of time, carrying one payload type.",
        "WrittenTime" => "The coordinate a score is notated in.",
        "PerformedTime" => "The coordinate a performance interpretation produces.",
        "PhysicalTime" => "The coordinate of physical seconds, reached through a tempo map.",
        "track" => "A literal event track: a duration, and the occurrences inside it.",
        "occurrence" => "One occurrence: a payload, and the half-open span it fills.",
        "let" => "Names a track so the term can say \"this is that material again\".",
        "in" => "The body a `let`'s name is visible in. Shadowing is rejected.",
        "follow" => "Lays its parts end to end, each starting where the last one ended.",
        "together" => "Sounds its parts from a common zero; the duration is the longest.",
        "shift" => "Moves a track later in ambient time, leaving the material unchanged.",
        "scale" => "Multiplies every span by a positive rational — augmentation, exactly.",
        "restrict" => "Observes a track through a window, keeping what is visible through it.",
        "from" | "to" => "The two ends of a span or a window, as exact rationals.",
        "by" => "The amount a `shift` or a `scale` acts by.",
        _ => return None,
    })
}
