//! Parser contract tests: losslessness, recovery, diagnostics, typed views.
//!
//! Snapshots (`insta`) are the review surface for tree shapes and diagnostic
//! text; regenerating them requires intent (`INSTA_UPDATE=always`), so a
//! parser change that alters trees or messages shows up for review.

// Test helpers use expect()/panic! on statically-valid inputs: a failure is a
// bug in the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
// These suites pick one variant out of `VoiceItem` and ignore the rest. The
// lint exists so that a new variant is considered everywhere it matters, and
// a test that asks "which of these are tempo markings" is not one of those
// places: listing twenty-odd variants would be noise that says nothing.
#![allow(clippy::wildcard_enum_match_arm)]

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

/// Every part of every error, so the snapshots are the diagnostics' goldens
/// and not just their first lines.
fn print_errors(doc: &musa_language::ParsedDocument) -> String {
    doc.errors()
        .iter()
        .map(|error| {
            let mut lines = vec![format!(
                "{}..{}: {error}",
                u32::from(error.range().start()),
                u32::from(error.range().end())
            )];
            lines.push(format!("  label: {}", error.label()));
            if let Some(help) = error.help() {
                lines.push(format!("  help: {help}"));
            }
            if let Some((title, replacement)) = error.fix() {
                lines.push(format!("  fix: {title} -> {replacement:?}"));
            }
            lines.join("\n")
        })
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
            .is_some_and(|error| error.message().contains("missing `;`")),
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
        doc.errors()
            .iter()
            .any(|error| error.message().contains("is never closed")),
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
            | VoiceItem::Repeat(_)
            | VoiceItem::Bar(_)
            | VoiceItem::Ending(_)
            | VoiceItem::Stretch(_)
            | VoiceItem::Retrograde(_)
            | VoiceItem::Invert(_)
            | VoiceItem::Phrase(_)
            | VoiceItem::Mark(_)
            | VoiceItem::Grace(_)
            | VoiceItem::Hairpin(_)
            | VoiceItem::Tempo(_)
            | VoiceItem::Meter(_)
            | VoiceItem::Key(_)
            | VoiceItem::Clef(_)
            | VoiceItem::Mobile(_)
            | VoiceItem::Improvise(_) => panic!("unexpected item"),
        }
    }
    assert_eq!(mark.as_deref(), Some("mf"));
    assert_eq!(ratio.as_deref(), Some("3/2"));
    assert_eq!(slurred, vec![true, false], "only the first note is tied");
}

#[test]
fn the_transformations_and_their_bodies_are_typed_views() {
    let source = "piece \"x\" { score { part p { voice v { \
                  stretch 3/2 { c4 1/4; } retrograde { d4 1/4; e4 1/4; } \
                  invert around c5 { f4 1/4; } } } } }";
    let doc = parse(source);
    assert_eq!(print_errors(&doc), "");
    let piece = PieceDecl::from_root(&doc.syntax()).expect("piece");
    let items = piece
        .score()
        .and_then(|score| score.parts().into_iter().next())
        .and_then(|part| part.voices().into_iter().next())
        .expect("voice")
        .items();

    assert_eq!(items.len(), 3, "stretch, retrograde, invert");
    let mut factor = None;
    let mut reversed = 0;
    let mut axis = None;
    for item in items {
        match item {
            VoiceItem::Stretch(stretch) => {
                factor = stretch.factor();
                assert_eq!(stretch.items().len(), 1);
            }
            // The block's items are in the order they are *written*. That
            // they sound backwards is elaboration's business, not the tree's.
            VoiceItem::Retrograde(retrograde) => reversed = retrograde.items().len(),
            VoiceItem::Invert(invert) => {
                axis = invert.axis();
                assert_eq!(invert.items().len(), 1);
            }
            VoiceItem::Note(_)
            | VoiceItem::Rest(_)
            | VoiceItem::Chord(_)
            | VoiceItem::Use(_)
            | VoiceItem::Transpose(_)
            | VoiceItem::Repeat(_)
            | VoiceItem::Bar(_)
            | VoiceItem::Ending(_)
            | VoiceItem::Slur(_)
            | VoiceItem::Dynamic(_)
            | VoiceItem::Tuplet(_)
            | VoiceItem::Phrase(_)
            | VoiceItem::Mark(_)
            | VoiceItem::Grace(_)
            | VoiceItem::Hairpin(_)
            | VoiceItem::Tempo(_)
            | VoiceItem::Meter(_)
            | VoiceItem::Key(_)
            | VoiceItem::Clef(_)
            | VoiceItem::Mobile(_)
            | VoiceItem::Improvise(_) => panic!("unexpected item"),
        }
    }
    assert_eq!(factor.as_deref(), Some("3/2"));
    assert_eq!(reversed, 2);
    assert_eq!(axis.as_deref(), Some("c5"));
}

#[test]
fn a_specialized_occurrence_carries_its_overrides_and_takes_no_semicolon() {
    let source = "piece \"x\" { motif m() { c4 1/4; } score { part p { voice v { \
                  use m(); use m() with { note 2 = d5; note 3 = ef5; } } } } }";
    let doc = parse(source);
    assert_eq!(print_errors(&doc), "");
    let piece = PieceDecl::from_root(&doc.syntax()).expect("piece");
    let items = piece
        .score()
        .and_then(|score| score.parts().into_iter().next())
        .and_then(|part| part.voices().into_iter().next())
        .expect("voice")
        .items();

    let calls: Vec<_> = items
        .into_iter()
        .filter_map(|item| match item {
            VoiceItem::Use(call) => Some(call),
            VoiceItem::Note(_)
            | VoiceItem::Rest(_)
            | VoiceItem::Chord(_)
            | VoiceItem::Transpose(_)
            | VoiceItem::Repeat(_)
            | VoiceItem::Bar(_)
            | VoiceItem::Ending(_)
            | VoiceItem::Slur(_)
            | VoiceItem::Dynamic(_)
            | VoiceItem::Tuplet(_)
            | VoiceItem::Stretch(_)
            | VoiceItem::Retrograde(_)
            | VoiceItem::Invert(_)
            | VoiceItem::Phrase(_)
            | VoiceItem::Mark(_)
            | VoiceItem::Grace(_)
            | VoiceItem::Hairpin(_)
            | VoiceItem::Tempo(_)
            | VoiceItem::Meter(_)
            | VoiceItem::Key(_)
            | VoiceItem::Clef(_)
            | VoiceItem::Mobile(_)
            | VoiceItem::Improvise(_) => None,
        })
        .collect();
    assert_eq!(calls.len(), 2);
    let plain = calls.first().expect("the plain call");
    assert!(plain.overrides().is_empty(), "an ordinary call specializes nothing");
    // The clause's tokens are not the call's arguments: a specialized
    // occurrence still calls the motif with what it was given.
    assert_eq!(plain.args(), Vec::<String>::new());

    let special = calls.get(1).expect("the specialized call");
    assert_eq!(special.args(), Vec::<String>::new());
    let spelled: Vec<(Option<String>, Option<String>)> = special
        .overrides()
        .iter()
        .map(|each| (each.position(), each.pitch()))
        .collect();
    assert_eq!(
        spelled,
        vec![
            (Some("2".to_owned()), Some("d5".to_owned())),
            (Some("3".to_owned()), Some("ef5".to_owned())),
        ]
    );
}

// --- Phase 3: imports and curves (docs/prompts/36) ------------------------

const OPENING: &str = include_str!("../../../examples/album/pieces/01-opening.musa");
const LIBRARY: &str = include_str!("../../../examples/album/library/motifs.musa");

/// The album fixture parses cleanly and round-trips, both halves of it: a
/// piece that imports, and the library it imports.
#[test]
fn the_album_fixture_parses_cleanly() {
    for source in [OPENING, LIBRARY] {
        let doc = parse(source);
        assert_eq!(doc.errors(), &[], "errors: {}", print_errors(&doc));
        assert_round_trip(source);
    }
}

/// A library is its own root. `PieceDecl` finds nothing in one, which is what
/// makes "an imported file has no score" a fact about the grammar rather than
/// a rule the compiler has to enforce.
#[test]
fn a_library_is_a_different_root_than_a_piece() {
    let doc = parse(LIBRARY);
    assert!(
        PieceDecl::from_root(&doc.syntax()).is_none(),
        "a library is not a piece"
    );
    let library = musa_language::ast::LibraryDecl::from_root(&doc.syntax()).expect("a library");
    let names: Vec<Option<String>> = library
        .motifs()
        .iter()
        .map(musa_language::ast::MotifDecl::name)
        .collect();
    assert_eq!(names, [Some("rise".to_owned()), Some("fall".to_owned())]);
}

/// The header holds the tempo a piece starts in and nothing else; a tempo it
/// changes to is written in the voice that reaches it, like the meter and the
/// key. One statement, two places (prompt 72).
#[test]
fn the_header_holds_one_tempo_and_a_change_is_written_in_the_voice() {
    let doc = parse(OPENING);
    let piece = PieceDecl::from_root(&doc.syntax()).expect("a piece");
    assert_eq!(piece.tempos().len(), 1);
    let start = piece.tempo().expect("a starting tempo");
    assert!(start.has_metronome());
    assert_eq!(start.text(), None);
    let changes: Vec<Option<String>> = piece
        .score()
        .expect("a score")
        .parts()
        .into_iter()
        .flat_map(|part| part.voices())
        .flat_map(|voice| voice.items())
        .filter_map(|item| match item {
            VoiceItem::Tempo(tempo) => Some(tempo.text()),
            _ => None,
        })
        .collect();
    assert_eq!(changes, [Some("poco più mosso".to_owned())]);
}

/// The three forms of a tempo marking, and the one that shapes the grammar:
/// a word with no number prints and changes no clock (prompt 72).
#[test]
fn a_tempo_marking_may_be_a_number_a_word_or_both() {
    let doc = parse(
        "piece \"T\" { tempo 1/4 = 92; meter 4/4; key c major;
            score { part p { voice v {
                c5 1/4;
                tempo 1/4 = 132 \"Allegro vivace\";
                d5 1/4;
                tempo \"Andante\";
                e5 1/2;
            } } } }",
    );
    assert_eq!(doc.errors(), &[], "errors: {}", print_errors(&doc));
    let piece = PieceDecl::from_root(&doc.syntax()).expect("a piece");
    let markings: Vec<(bool, Option<String>)> = piece
        .score()
        .expect("a score")
        .parts()
        .into_iter()
        .flat_map(|part| part.voices())
        .flat_map(|voice| voice.items())
        .filter_map(|item| match item {
            VoiceItem::Tempo(tempo) => Some((tempo.has_metronome(), tempo.text())),
            _ => None,
        })
        .collect();
    assert_eq!(
        markings,
        [
            (true, Some("Allegro vivace".to_owned())),
            (false, Some("Andante".to_owned()))
        ]
    );
}

/// A `use` at the top of a piece is a path, not a motif call: the same word
/// means two things and the parser tells them apart by what follows it.
#[test]
fn an_import_is_a_path_and_a_motif_use_is_a_call() {
    let doc = parse(OPENING);
    let piece = PieceDecl::from_root(&doc.syntax()).expect("a piece");
    let paths: Vec<Option<String>> = piece
        .imports()
        .iter()
        .map(musa_language::ast::ImportStmt::path)
        .collect();
    assert_eq!(
        paths,
        [
            Some("../library/motifs.musa".to_owned()),
            Some("../library/patches.musa".to_owned())
        ]
    );
    // And inside the voice, `use rise();` is still a call.
    let items = piece
        .score()
        .and_then(|score| score.parts().into_iter().next())
        .and_then(|part| part.voices().into_iter().next())
        .map(|voice| voice.items())
        .unwrap_or_default();
    assert!(items.iter().any(|item| matches!(item, VoiceItem::Use(_))));
}

/// A hairpin is a block with a direction and a mark it arrives at, and the
/// notes it covers are its items.
#[test]
fn a_hairpin_names_its_direction_and_its_mark() {
    let doc = parse(
        "piece \"P\" { score { part p { voice v {\n    crescendo to f { c5 1/4; d5 1/4; }\n    diminuendo to pp { e5 1/4; }\n} } } }",
    );
    assert_eq!(doc.errors(), &[], "errors: {}", print_errors(&doc));
    let piece = PieceDecl::from_root(&doc.syntax()).expect("a piece");
    let items = piece
        .score()
        .and_then(|score| score.parts().into_iter().next())
        .and_then(|part| part.voices().into_iter().next())
        .map(|voice| voice.items())
        .unwrap_or_default();
    let hairpins: Vec<(bool, Option<String>, usize)> = items
        .iter()
        .filter_map(|item| match item {
            VoiceItem::Hairpin(hairpin) => Some((hairpin.grows(), hairpin.target(), hairpin.items().len())),
            VoiceItem::Note(_)
            | VoiceItem::Rest(_)
            | VoiceItem::Chord(_)
            | VoiceItem::Use(_)
            | VoiceItem::Transpose(_)
            | VoiceItem::Repeat(_)
            | VoiceItem::Bar(_)
            | VoiceItem::Ending(_)
            | VoiceItem::Slur(_)
            | VoiceItem::Dynamic(_)
            | VoiceItem::Tuplet(_)
            | VoiceItem::Stretch(_)
            | VoiceItem::Retrograde(_)
            | VoiceItem::Invert(_)
            | VoiceItem::Phrase(_)
            | VoiceItem::Mark(_)
            | VoiceItem::Grace(_)
            | VoiceItem::Tempo(_)
            | VoiceItem::Meter(_)
            | VoiceItem::Key(_)
            | VoiceItem::Clef(_)
            | VoiceItem::Mobile(_)
            | VoiceItem::Improvise(_) => None,
        })
        .collect();
    assert_eq!(
        hairpins,
        [(true, Some("f".to_owned()), 2), (false, Some("pp".to_owned()), 1)]
    );
}

/// The one-word constructs stay one word through the formatter: a coordinate
/// is `3:1` and a chord symbol is `fmaj7`, not `3: 1` and `fmaj 7`.
#[test]
fn a_coordinate_and_a_chord_symbol_format_as_one_word() {
    let source = "piece \"P\" {\n    score {\n        section \"A\" at 3:1;\n        harmony {\n            at 1:1 fmaj7;\n        }\n    }\n}\n";
    let formatted = musa_language::format(&parse(source));
    assert_eq!(formatted.text(), source);
}

/// `bar`, named and not, and `use` without parentheses.
///
/// The parentheses *are* the argument list, so material that takes no
/// arguments is played by naming it. That is what makes a bar reusable
/// without a second call syntax for it.
#[test]
fn bars_are_voice_items_and_a_name_is_played_without_parentheses() {
    let source = "piece \"B\" { score { part p { voice v { \
                  bar head { c4 1/2; d4 1/2; } bar { e4 1; } use head; } } } }";
    let doc = parse(source);
    assert_eq!(print_errors(&doc), "");
    let piece = PieceDecl::from_root(&doc.syntax()).expect("piece");
    let items = piece
        .score()
        .and_then(|score| score.parts().into_iter().next())
        .and_then(|part| part.voices().into_iter().next())
        .expect("voice")
        .items();
    let described: Vec<String> = items
        .iter()
        .map(|item| {
            if let VoiceItem::Bar(bar) = item {
                format!(
                    "bar {} of {}",
                    bar.name().unwrap_or_else(|| "-".to_owned()),
                    bar.items().len()
                )
            } else if let VoiceItem::Use(call) = item {
                format!(
                    "use {} with {} args",
                    call.motif().unwrap_or_default(),
                    call.args().len()
                )
            } else {
                "other".to_owned()
            }
        })
        .collect();
    assert_eq!(described, ["bar head of 2", "bar - of 1", "use head with 0 args"]);
}

/// A bar claims to be one measure, and a measure inside a measure is not a
/// thing the notation has a mark for.
#[test]
fn a_bar_inside_a_bar_is_refused() {
    let source = "piece \"B\" { score { part p { voice v { bar { bar { c4 1; } } } } } }";
    let doc = parse(source);
    assert!(
        print_errors(&doc).contains("bars do not nest"),
        "{}",
        print_errors(&doc)
    );
}
