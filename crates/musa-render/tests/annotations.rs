//! What each backend does with the annotation layer (docs/prompts/35).
//!
//! The snapshots in `mei.rs`, `musicxml.rs`, and `lilypond.rs` pin the exact
//! bytes; this suite pins the facts a reader would notice if a backend
//! silently dropped an annotation — which a snapshot update would happily
//! accept. Each format spells them differently, so the assertions are per
//! format, but the contract is one: every phrase, form marker, and chord
//! symbol in the source reaches every export, and none of them changes a
//! note.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ScoreSnapshot, SourceDocument, compile};
use musa_render::{NotationOptions, NotationTarget, render_notation};

const ANNOTATED: &str = include_str!("../../../examples/annotated.musa");

fn score_of(text: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles")
}

fn render(target: NotationTarget) -> String {
    render_notation(&score_of(ANNOTATED), target, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string()
}

fn assert_contains(text: &str, needles: &[&str], what: &str) {
    for needle in needles {
        assert!(text.contains(needle), "{what} should contain `{needle}`");
    }
}

/// MEI anchors a symbol either to a note (`startid`) or to a place in the
/// measure (`tstamp`). Phrases take the first, form markers and chord symbols
/// the second, because a bar line is a place even when no note starts there.
#[test]
fn mei_carries_every_annotation() {
    let mei = render(NotationTarget::Mei);
    assert_contains(
        &mei,
        &[
            "<harm staff=\"1\" tstamp=\"1\" place=\"above\">am</harm>",
            "<harm staff=\"1\" tstamp=\"1\" place=\"above\">fmaj7</harm>",
            "<harm staff=\"1\" tstamp=\"1\" place=\"above\">e7</harm>",
            "type=\"section\">Exposition</dir>",
            "type=\"section\">Development</dir>",
            "label=\"antecedent\"",
            "label=\"consequent\"",
        ],
        "the MEI export",
    );
    // The phrase bracket points at notes, and both ends must be real ones.
    for label in ["antecedent", "consequent"] {
        let element = mei
            .split('<')
            .find(|element| element.starts_with("phrase ") && element.contains(label))
            .unwrap_or_else(|| panic!("no <phrase> for `{label}`"));
        assert!(element.contains("startid=\"#event-"), "{label} has a start note");
        assert!(element.contains("endid=\"#event-"), "{label} has an end note");
    }
}

/// `MusicXML`'s `<kind>` says what musa read; its `text` attribute says what
/// the composer wrote. A consumer that understands neither still prints the
/// second.
#[test]
fn musicxml_carries_every_annotation() {
    let xml = render(NotationTarget::MusicXml);
    assert_contains(
        &xml,
        &[
            "<rehearsal>Exposition</rehearsal>",
            "<rehearsal>Development</rehearsal>",
            "<kind text=\"am\">minor</kind>",
            "<kind text=\"fmaj7\">major-seventh</kind>",
            "<kind text=\"e7\">dominant</kind>",
            "<root-step>A</root-step>",
            "<words>antecedent</words>",
            "<words>consequent</words>",
        ],
        "the MusicXML export",
    );
    assert_eq!(
        xml.matches("<bracket").count(),
        4,
        "two phrases open and close one bracket each"
    );
}

/// `LilyPond` gets the harmony as a `ChordNames` context, which is what it
/// engraves best and what a `LilyPond` user would write by hand.
#[test]
fn lilypond_carries_every_annotation() {
    let ly = render(NotationTarget::LilyPond);
    assert_contains(
        &ly,
        &[
            "\\new ChordNames \\chords",
            "\\chordmode",
            "a1:m",
            "f1:maj7",
            "e1:7",
            "\\mark \\markup { \\bold \"Exposition\" }",
            "\\mark \\markup { \\bold \"Development\" }",
            "^\\markup { \\italic \"antecedent\" }",
            "^\\markup { \\italic \"consequent\" }",
        ],
        "the LilyPond export",
    );
}

/// The chord symbols each format spells for one lane, side by side: this is
/// the table that catches a backend translating a symbol into a different
/// chord rather than a coarser one.
#[test]
fn a_chord_symbol_survives_every_backend_as_itself() {
    let source = "piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major; score {
        harmony { at 1:1 csus4; at 2:1 cmmaj7; at 3:1 bbdim7; at 4:1 c6; }
        part piano { voice one { c4 1; d4 1; e4 1; f4 1; } } } }";
    let score = score_of(source);
    let xml = render_notation(&score, NotationTarget::MusicXml, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string();
    assert_contains(
        &xml,
        &[
            "<kind text=\"csus4\">suspended-fourth</kind>",
            "<kind text=\"cmmaj7\">major-minor</kind>",
            "<kind text=\"bbdim7\">diminished-seventh</kind>",
            "<kind text=\"c6\">major-sixth</kind>",
        ],
        "the MusicXML kinds",
    );

    let ly = render_notation(&score, NotationTarget::LilyPond, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string();
    // LilyPond spells a raised seventh `7+`, which is how a minor-major
    // seventh is written.
    // The root is LilyPond's own spelling, not musa's: a flat is `f` there.
    assert_contains(&ly, &["c1:sus4", "c1:m7+", "bf1:dim7", "c1:6"], "the LilyPond chords");

    let mei = render_notation(&score, NotationTarget::Mei, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string();
    // MEI prints the symbol verbatim: it has no chord vocabulary to lose.
    assert_contains(&mei, &["csus4", "cmmaj7", "bbdim7", "c6"], "the MEI harmony");
}

/// A symbol written between beats keeps its place: the formats that count
/// beats say so, rather than rounding it to the bar line.
#[test]
fn a_symbol_off_the_downbeat_keeps_its_place() {
    let source = "piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major; score {
        section \"Turn\" at 1:3;
        harmony { at 1:1 c; at 1:3 g7; }
        part piano { voice one { c4 1/2; d4 1/2; } } } }";
    let score = score_of(source);
    let mei = render_notation(&score, NotationTarget::Mei, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string();
    assert_contains(
        &mei,
        &["tstamp=\"3\" place=\"above\">g7</harm>", "tstamp=\"3\""],
        "the MEI timestamps",
    );

    let xml = render_notation(&score, NotationTarget::MusicXml, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string();
    // Divisions are per quarter note, so beat 3 of a 4/4 bar is two of them.
    let divisions: i64 = xml
        .split("<divisions>")
        .nth(1)
        .and_then(|rest| rest.split('<').next())
        .and_then(|text| text.parse().ok())
        .expect("a divisions value");
    let offset = format!("<offset>{}</offset>", divisions * 2);
    assert!(xml.contains(&offset), "the MusicXML offsets should contain `{offset}`");
}
