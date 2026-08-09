//! Formatter laws and layout snapshots.
//!
//! Laws (roadmap §17.3): `format(format(x)) == format(x)` and
//! `parse(format(parse(x)))` equals `parse(x)` up to whitespace trivia —
//! tested as `proptest` properties over a grammar-directed generator plus
//! the example corpus.

use musa_language::{ParsedDocument, SyntaxElement, SyntaxKind, format, parse};
use proptest::prelude::*;

const GLASS_MOUNTAIN: &str = include_str!("../../../examples/glass-mountain.musa");
const INVENTION: &str = include_str!("../../../examples/invention.musa");
const TUPLET_FIXTURE: &str = include_str!("../../../examples/tuplet-fixture.musa");

fn fmt(source: &str) -> String {
    format(&parse(source)).text().to_string()
}

/// The significant (non-whitespace) token sequence of a document: the
/// "same program up to formatting" oracle.
fn significant_tokens(doc: &ParsedDocument) -> Vec<(SyntaxKind, String)> {
    doc.syntax()
        .descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| token.kind() != SyntaxKind::Whitespace)
        .map(|token| (token.kind(), token.text().to_string()))
        .collect()
}

fn assert_semantics_preserved(source: &str, formatted: &str) {
    assert_eq!(
        significant_tokens(&parse(source)),
        significant_tokens(&parse(formatted)),
        "formatting must not change the program"
    );
}

#[test]
fn examples_format_to_themselves() {
    assert_eq!(fmt(GLASS_MOUNTAIN), GLASS_MOUNTAIN);
    assert_eq!(fmt(INVENTION), INVENTION);
    assert_eq!(fmt(TUPLET_FIXTURE), TUPLET_FIXTURE);
}

#[test]
fn messy_source_is_canonicalized() {
    let source = "piece   \"Messy\"{\n\ttempo quarter=72;\n\n\nmeter 4/4;\nkey a minor;\nscore{\npart p{\nclef treble;\nvoice v{\nc5   1/4\n[a3   c4  e4]/2\nrest 1/4\n}\n}\n}\n}\n";
    let formatted = fmt(source);
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
    insta::assert_snapshot!(formatted);
}

/// A short-form duration is part of the note's word, and the formatter leaves
/// both spellings exactly as the composer wrote them: `format` rewrites
/// whitespace, so it cannot prefer one over the other.
#[test]
fn both_spellings_of_a_duration_survive_formatting() {
    let source = "piece \"D\" {\n    score {\n        part p {\n            voice v {\n                c5/4\n                d5/4.\n                e5/4..\n                f5 1/4\n                rest/8\n                g5/4 to 1\n            }\n        }\n    }\n}\n";
    let formatted = fmt(source);
    assert_eq!(formatted, source, "the shapes are already canonical");
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
}

/// Spacing is not lost around a short duration when the source has too much
/// of it, and the dot never swallows the note after it.
#[test]
fn a_short_duration_closes_up_to_its_note() {
    let source = "piece \"D\" { score { part p { voice v { c5 / 4 .   d5/4 } } } }";
    let formatted = fmt(source);
    assert!(formatted.contains("c5/4."), "got:\n{formatted}");
    assert!(formatted.contains("d5/4"), "got:\n{formatted}");
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
}

#[test]
fn comments_keep_their_attachment() {
    let source = "piece \"C\" {\n    // header comment\n    meter 4/4; // trailing\n    score {\n        part p {\n            voice v {\n                c5/4 /* inline block */\n                // detached comment\n                d5/4\n            }\n        }\n    }\n}\n";
    let formatted = fmt(source);
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
    insta::assert_snapshot!(formatted);
}

#[test]
fn blank_lines_are_capped_at_one() {
    let source = "piece \"B\" {\n    meter 4/4;\n\n\n\n    key c major;\n\n\n    score {\n        part p {\n            voice v {\n                c5/1\n            }\n        }\n    }\n}\n";
    let formatted = fmt(source);
    assert!(!formatted.contains("\n\n\n"), "no double blank lines");
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
    insta::assert_snapshot!(formatted);
}

/// The one formatting exception: a bar is a horizontal thing.
///
/// Four cases in one fixture, because what matters is that they sit next to
/// each other and still read as four bars: a bar written stacked comes back
/// on one line, a bar already on one line stays there, a bar too wide for the
/// line breaks like any other block, and a bar carrying a comment breaks
/// because a comment needs a line of its own.
#[test]
fn a_bar_that_fits_is_written_on_one_line() {
    let source = "piece \"Bars\" {\n    meter 4/4;\n    score {\n        part p {\n            voice v {\n                bar head {\n                    c4/4\n                    d4/4\n                    e4/4\n                    f4/4\n                }\n                bar { g4/2 a4/2 }\n                bar { c4/16 d4/16 e4/16 f4/16 g4/16 a4/16 b4/16 c5/16 d5/16 e5/16 f5/16 g5/16 a5/16 b5/16 c6/8 }\n                bar {\n                    // the last one\n                    c4/1\n                }\n            }\n        }\n    }\n}\n";
    let formatted = fmt(source);
    assert!(
        formatted.contains("                bar head { c4/4 d4/4 e4/4 f4/4 }\n"),
        "a stacked bar comes back onto its line:\n{formatted}"
    );
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
    insta::assert_snapshot!(formatted);
}

#[test]
fn apply_edits_replaces_ranges_in_order() {
    use musa_language::{TextEdit, apply_edits};
    use text_size::{TextRange, TextSize};
    let range = |start: u32, end: u32| TextRange::new(TextSize::from(start), TextSize::from(end));
    let edits = [TextEdit::new(range(4, 7), "three "), TextEdit::new(range(0, 3), "one")];
    assert_eq!(apply_edits("one two three", &edits), "one three  three");
    // Overlapping edits are skipped, not applied destructively.
    let overlapping = [TextEdit::new(range(0, 5), "x"), TextEdit::new(range(2, 6), "y")];
    assert_eq!(apply_edits("one two three", &overlapping), "xwo three");
}

// --- Generator for the formatting laws ------------------------------------

fn pitch() -> impl Strategy<Value = String> {
    (
        prop::sample::select(vec!['a', 'b', 'c', 'd', 'e', 'f', 'g']),
        prop::sample::select(vec!["", "#", "##", "b", "bb", "n"]),
        -1i8..=8,
    )
        .prop_map(|(letter, accidental, octave)| format!("{letter}{accidental}{octave}"))
}

fn duration() -> impl Strategy<Value = String> {
    // Both spellings, because both are legal and the formatter must leave
    // either as it found it: `/12` cannot be written short, and `/4.` cannot
    // be written any other way without saying 3/8.
    prop::sample::select(vec!["/1", "/2", "/4", "/4.", "/8", "/16", "1/12", "3/8", "1/4"]).prop_map(str::to_string)
}

fn gap() -> impl Strategy<Value = String> {
    prop::sample::select(vec![" ", "  ", "\t", "\n", "\n   "]).prop_map(str::to_string)
}

fn separator() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["\n", "\n\n", "\n\n\n", "\n    "]).prop_map(str::to_string)
}

/// A statement with nothing after it on its line, so a run of them can be
/// joined with spaces without a comment eating what follows.
///
/// An event carries no terminator: `gap()` is what stands between the pitch
/// and the duration, and a short duration closes up to the pitch, so the
/// generator writes the gap only where the long form allows one.
fn statement() -> impl Strategy<Value = String> {
    prop_oneof![
        (pitch(), duration(), gap()).prop_map(|(p, d, g)| format!("{p}{}{d}", spacing(&d, &g))),
        duration().prop_map(|d| format!("rest{}{d}", spacing(&d, " "))),
        (prop::collection::vec(pitch(), 2..=4), duration()).prop_map(|(pitches, d)| format!(
            "[{}]{}{d}",
            pitches.join(" "),
            spacing(&d, " ")
        )),
    ]
}

/// What separates a pitch from its duration: nothing at all when the duration
/// is written short, and the generated whitespace when it is written long.
fn spacing<'a>(duration: &str, gap: &'a str) -> &'a str {
    if duration.starts_with('/') { "" } else { gap }
}

/// The same, plus a trailing comment and plus bars — which have their own
/// line-length rule, and are therefore the one construct whose formatting
/// depends on how wide it is. Bars do not nest, so what goes inside one is a
/// plain statement.
fn item() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => statement(),
        2 => (pitch(), duration()).prop_map(|(p, d)| format!("{p}{}{d} // note", spacing(&d, " "))),
        3 => (prop::collection::vec(statement(), 1..=6), gap())
            .prop_map(|(inside, g)| format!("bar {{{g}{}{g}}}", inside.join(" "))),
        3 => (prop::collection::vec(statement(), 1..=6), gap())
            .prop_map(|(inside, g)| format!("|{g}{}", inside.join(" "))),
    ]
}

fn piece_source() -> impl Strategy<Value = String> {
    (prop::collection::vec(item(), 1..=12), prop::collection::vec(separator(), 12))
        .prop_map(|(items, separators)| {
            let mut body = String::new();
            for (index, item) in items.iter().enumerate() {
                body.push_str(item);
                if let Some(sep) = separators.get(index) {
                    body.push_str(sep);
                }
            }
            format!(
                "piece \"Gen\" {{\n    tempo quarter = 72;\n    meter 4/4;\n    key c major;\n    score {{\n        part p {{\n            voice v {{\n{body}\n            }}\n        }}\n    }}\n}}\n"
            )
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn format_is_idempotent(source in piece_source()) {
        let once = fmt(&source);
        let twice = fmt(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn format_preserves_semantics(source in piece_source()) {
        let formatted = fmt(&source);
        prop_assert_eq!(
            significant_tokens(&parse(&source)),
            significant_tokens(&parse(&formatted))
        );
    }

    #[test]
    fn generated_sources_parse_without_errors(source in piece_source()) {
        prop_assert_eq!(parse(&source).errors().len(), 0);
    }
}

#[test]
fn examples_satisfy_the_laws() {
    for source in [GLASS_MOUNTAIN, INVENTION] {
        let formatted = fmt(source);
        assert_eq!(fmt(&formatted), formatted, "idempotence");
        assert_semantics_preserved(source, &formatted);
    }
}
