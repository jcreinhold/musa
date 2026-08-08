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
    let source = "piece   \"Messy\"{\n\ttempo quarter=72;\n\n\nmeter 4/4;\nkey a minor;\nscore{\npart p{\nclef treble;\nvoice v{\nc5   1/4;\nchord [a3,c4,e4] 1/2;\nrest 1/4;\n}\n}\n}\n}\n";
    let formatted = fmt(source);
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
    insta::assert_snapshot!(formatted);
}

#[test]
fn comments_keep_their_attachment() {
    let source = "piece \"C\" {\n    // header comment\n    meter 4/4; // trailing\n    score {\n        part p {\n            voice v {\n                c5 1/4; /* inline block */\n                // detached comment\n                d5 1/4;\n            }\n        }\n    }\n}\n";
    let formatted = fmt(source);
    assert_eq!(fmt(&formatted), formatted, "idempotence");
    assert_semantics_preserved(source, &formatted);
    insta::assert_snapshot!(formatted);
}

#[test]
fn blank_lines_are_capped_at_one() {
    let source = "piece \"B\" {\n    meter 4/4;\n\n\n\n    key c major;\n\n\n    score {\n        part p {\n            voice v {\n                c5 1;\n            }\n        }\n    }\n}\n";
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
    let source = "piece \"Bars\" {\n    meter 4/4;\n    score {\n        part p {\n            voice v {\n                bar head {\n                    c4 1/4;\n                    d4 1/4;\n                    e4 1/4;\n                    f4 1/4;\n                }\n                bar { g4 1/2; a4 1/2; }\n                bar { c4 1/16; d4 1/16; e4 1/16; f4 1/16; g4 1/16; a4 1/16; b4 1/16; c5 1/16; d5 1/16; e5 1/16; f5 1/16; g5 1/16; a5 1/16; b5 1/16; c6 1/8; }\n                bar {\n                    // the last one\n                    c4 1;\n                }\n            }\n        }\n    }\n}\n";
    let formatted = fmt(source);
    assert!(
        formatted.contains("                bar head { c4 1/4; d4 1/4; e4 1/4; f4 1/4; }\n"),
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
        prop::sample::select(vec!["", "s", "ss", "f", "ff", "n"]),
        -1i8..=8,
    )
        .prop_map(|(letter, accidental, octave)| format!("{letter}{accidental}{octave}"))
}

fn duration() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["1", "1/2", "1/4", "3/8", "1/8", "1/12", "1/16"]).prop_map(str::to_string)
}

fn gap() -> impl Strategy<Value = String> {
    prop::sample::select(vec![" ", "  ", "\t", "\n", "\n   "]).prop_map(str::to_string)
}

fn separator() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["\n", "\n\n", "\n\n\n", "\n    "]).prop_map(str::to_string)
}

/// A statement with nothing after it on its line, so a run of them can be
/// joined with spaces without a comment eating what follows.
fn statement() -> impl Strategy<Value = String> {
    prop_oneof![
        (pitch(), duration(), gap()).prop_map(|(p, d, g)| format!("{p}{g}{d};")),
        duration().prop_map(|d| format!("rest {d};")),
        (prop::collection::vec(pitch(), 2..=4), duration())
            .prop_map(|(pitches, d)| format!("chord [{}] {d};", pitches.join(", "))),
    ]
}

/// The same, plus a trailing comment and plus bars — which have their own
/// line-length rule, and are therefore the one construct whose formatting
/// depends on how wide it is. Bars do not nest, so what goes inside one is a
/// plain statement.
fn item() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => statement(),
        2 => (pitch(), duration()).prop_map(|(p, d)| format!("{p} {d}; // note")),
        3 => (prop::collection::vec(statement(), 1..=6), gap())
            .prop_map(|(inside, g)| format!("bar {{{g}{}{g}}}", inside.join(" "))),
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
