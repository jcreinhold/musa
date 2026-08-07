//! Parser contract tests: losslessness, recovery, diagnostics, typed views.
//!
//! Snapshots (`insta`) are the review surface for tree shapes and diagnostic
//! text; regenerating them requires intent (`INSTA_UPDATE=always`), so a
//! parser change that alters trees or messages shows up for review.

// Test helpers use expect()/panic! on statically-valid inputs: a failure is a
// bug in the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_language::ast::{PieceDecl, VoiceItem};
use musa_language::{SyntaxElement, SyntaxKind, SyntaxNode, parse};

const GLASS_MOUNTAIN: &str = include_str!("../../../examples/glass-mountain.musa");
const INVENTION: &str = include_str!("../../../examples/invention.musa");

fn print_tree(node: &SyntaxNode) -> String {
    let mut lines = Vec::new();
    write_node(node, 0, &mut lines);
    lines.join("")
}

fn write_node(node: &SyntaxNode, indent: usize, lines: &mut Vec<String>) {
    let range = node.text_range();
    lines.push(format!(
        "{:indent$}{:?}@{}..{}\n",
        "",
        node.kind(),
        u32::from(range.start()),
        u32::from(range.end()),
        indent = indent,
    ));
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) => write_node(&child, indent.saturating_add(2), lines),
            SyntaxElement::Token(token) => {
                let range = token.text_range();
                lines.push(format!(
                    "{:indent$}{:?}@{}..{} {:?}\n",
                    "",
                    token.kind(),
                    u32::from(range.start()),
                    u32::from(range.end()),
                    token.text(),
                    indent = indent.saturating_add(2),
                ));
            }
        }
    }
}

fn print_errors(doc: &musa_language::ParsedDocument) -> String {
    doc.errors()
        .iter()
        .map(|error| format!("{error}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_round_trip(source: &str) {
    let doc = parse(source);
    assert_eq!(
        doc.syntax().text().to_string(),
        source,
        "the tree must reproduce the source exactly"
    );
}

#[test]
fn glass_mountain_parses_cleanly() {
    let doc = parse(GLASS_MOUNTAIN);
    assert_eq!(doc.errors(), &[], "errors: {}", print_errors(&doc));
    assert_round_trip(GLASS_MOUNTAIN);
    insta::assert_snapshot!(print_tree(&doc.syntax()));
}

#[test]
fn invention_parses_cleanly() {
    let doc = parse(INVENTION);
    assert_eq!(doc.errors(), &[], "errors: {}", print_errors(&doc));
    assert_round_trip(INVENTION);
    insta::assert_snapshot!(print_tree(&doc.syntax()));
}

#[test]
fn missing_semicolon_recovers_and_continues() {
    let source = "piece \"x\" {\n    meter 4/4\n    meter 2/2;\n}\n";
    let doc = parse(source);
    assert_eq!(doc.errors().len(), 1, "errors: {}", print_errors(&doc));
    assert!(
        doc.errors()
            .first()
            .is_some_and(|error| error.message().contains("expected `;`")),
        "errors: {}",
        print_errors(&doc)
    );
    // Both meter statements still parse.
    let meters = doc
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::MeterStmt)
        .count();
    assert_eq!(meters, 2);
    assert_round_trip(source);
    insta::assert_snapshot!(print_errors(&doc));
    insta::assert_snapshot!(print_tree(&doc.syntax()));
}

#[test]
fn unclosed_brace_recovers_at_eof() {
    let source = "piece \"x\" {\n    score {\n        part p {\n";
    let doc = parse(source);
    assert!(!doc.errors().is_empty());
    assert!(
        doc.errors().iter().any(|error| error.message().contains("unclosed")),
        "errors: {}",
        print_errors(&doc)
    );
    assert_round_trip(source);
    insta::assert_snapshot!(print_errors(&doc));
}

#[test]
fn lex_error_token_recovers_and_continues() {
    let source = "piece \"x\" {\n    score {\n        part p {\n            voice v {\n                c5 *;\n                d5 1/4;\n            }\n        }\n    }\n}\n";
    let doc = parse(source);
    assert!(!doc.errors().is_empty());
    // The statement after the broken one still parses.
    let notes: Vec<String> = doc
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::NoteStmt)
        .filter_map(|node| {
            node.children_with_tokens()
                .filter_map(SyntaxElement::into_token)
                .find(|token| token.kind() == SyntaxKind::PitchLiteral)
                .map(|token| token.text().to_string())
        })
        .collect();
    assert_eq!(notes, ["c5", "d5"]);
    assert_round_trip(source);
    insta::assert_snapshot!(print_errors(&doc));
}

#[test]
fn stray_declaration_is_wrapped_in_error_node() {
    let source = "piece \"x\" {\n    frobnicate;\n    meter 4/4;\n}\n";
    let doc = parse(source);
    assert_eq!(doc.errors().len(), 1, "errors: {}", print_errors(&doc));
    assert!(
        doc.syntax().descendants().any(|node| node.kind() == SyntaxKind::Error),
        "an ERROR node must wrap the stray construct"
    );
    // Recovery consumed through the semicolon; the meter parses cleanly.
    assert!(
        doc.syntax()
            .descendants()
            .any(|node| node.kind() == SyntaxKind::MeterStmt)
    );
    assert_round_trip(source);
    insta::assert_snapshot!(print_tree(&doc.syntax()));
}

#[test]
fn typed_wrappers_expose_the_document_structure() {
    let doc = parse(GLASS_MOUNTAIN);
    let piece_opt = PieceDecl::from_root(&doc.syntax());
    assert!(piece_opt.is_some(), "expected a piece declaration");
    let Some(piece) = piece_opt else { return };
    assert_eq!(piece.name().as_deref(), Some("Glass Mountain"));
    assert!(piece.tempo().is_some());
    assert_eq!(piece.meter().and_then(|meter| meter.value()).as_deref(), Some("4/4"));
    assert_eq!(piece.motifs().len(), 1);
    assert_eq!(
        piece.motifs().first().and_then(|motif| motif.name()),
        Some("sigh".to_string())
    );
    let score_opt = piece.score();
    assert!(score_opt.is_some(), "expected a score");
    let Some(score) = score_opt else { return };
    let part_names: Vec<String> = score.parts().iter().filter_map(|part| part.name()).collect();
    assert_eq!(part_names, ["violin", "strings"]);
    let violin_opt = score.parts().into_iter().next();
    assert!(violin_opt.is_some(), "expected a violin part");
    let Some(violin) = violin_opt else { return };
    let voices = violin.voices();
    let lead_opt = voices.first();
    assert!(lead_opt.is_some(), "expected a lead voice");
    let Some(lead) = lead_opt else { return };
    assert_eq!(lead.name().as_deref(), Some("lead"));
    let items = lead.items();
    assert_eq!(items.len(), 2);
    assert!(matches!(items.first(), Some(VoiceItem::Use(_))));
    assert!(matches!(items.get(1), Some(VoiceItem::Transpose(_))));
}

// --- Phase 2: ties, slurs, dynamics, articulations, tuplets ---------------

const TUPLET_FIXTURE: &str = include_str!("../../../examples/tuplet-fixture.musa");

#[test]
fn tuplet_fixture_parses_cleanly() {
    let doc = parse(TUPLET_FIXTURE);
    assert_eq!(print_errors(&doc), "", "fixture must parse without diagnostics");
    assert_eq!(doc.syntax().text().to_string(), TUPLET_FIXTURE, "losslessness");
}

/// An articulation and a pitch reference are both bare identifiers, and the
/// tree has to keep them apart without counting tokens.
#[test]
fn articulations_do_not_shadow_the_pitch_or_the_duration() {
    let source = "piece \"x\" { motif m(root: pitch = c4, len: duration = 1/4) { root len accent staccato; } \
                  score { part p { voice v { use m(); } } } }";
    let doc = parse(source);
    assert_eq!(print_errors(&doc), "");
    let piece = PieceDecl::from_root(&doc.syntax()).expect("piece");
    let motif = piece.motifs().into_iter().next().expect("motif");
    let items = motif.items();
    let Some(VoiceItem::Note(note)) = items.into_iter().next() else {
        panic!("expected a note statement");
    };
    assert_eq!(note.pitch().as_deref(), Some("root"));
    assert_eq!(note.duration(), None, "a duration parameter is not a literal");
    assert_eq!(note.articulations(), vec!["accent".to_string(), "staccato".to_string()]);
    assert!(!note.tied());
}

#[test]
fn typed_views_read_the_new_statements() {
    let source = "piece \"x\" { score { part p { voice v { \
                  dynamic mf; tuplet 3/2 { c4 1/8; } slur { d4 1/4 ~; d4 1/4; } } } } }";
    let doc = parse(source);
    assert_eq!(print_errors(&doc), "");
    let piece = PieceDecl::from_root(&doc.syntax()).expect("piece");
    let voice = piece
        .score()
        .and_then(|score| score.parts().into_iter().next())
        .and_then(|part| part.voices().into_iter().next())
        .expect("voice");
    let items = voice.items();
    assert_eq!(items.len(), 3, "dynamic, tuplet, slur");
    let mut ratio = None;
    let mut slurred = Vec::new();
    let mut mark = None;
    for item in items {
        match item {
            VoiceItem::Dynamic(dynamic) => mark = dynamic.mark(),
            VoiceItem::Tuplet(tuplet) => ratio = tuplet.ratio(),
            VoiceItem::Slur(slur) => {
                for inner in slur.items() {
                    if let VoiceItem::Note(note) = inner {
                        slurred.push(note.tied());
                    }
                }
            }
            VoiceItem::Note(_)
            | VoiceItem::Rest(_)
            | VoiceItem::Chord(_)
            | VoiceItem::Use(_)
            | VoiceItem::Transpose(_)
            | VoiceItem::Repeat(_) => panic!("unexpected item"),
        }
    }
    assert_eq!(mark.as_deref(), Some("mf"));
    assert_eq!(ratio.as_deref(), Some("3/2"));
    assert_eq!(slurred, vec![true, false], "only the first note is tied");
}
