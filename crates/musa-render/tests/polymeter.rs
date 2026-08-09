//! What each backend does with staves counted and paced differently
//! (docs/prompts/75).
//!
//! The four formats do not agree, and the differences are the point.
//! `MusicXML` has a measure list per part, so it says polymeter exactly.
//! `LilyPond` says it once Timing is moved from Score to Staff, which is what
//! the notation manual's polymetric section does and what the export writes.
//! MEI states each staff's meter but numbers measures for the *score*, so a
//! piece whose barlines diverge is exported with a warning rather than
//! quietly mis-barred. SMF has one tempo track and one meter track and says
//! neither.
//!
//! The one thing all four agree on is that none of it moves a note.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ScoreSnapshot, SourceDocument, compile};
use musa_render::{NotationOptions, NotationTarget, RenderedNotation, render_notation};

const BULGARIAN: &str = include_str!("../../../examples/bulgarian.musa");
const HEMIOLA: &str = include_str!("../../../examples/hemiola.musa");
const CANON_X: &str = include_str!("../../../examples/canon-x.musa");

fn score_of(source: &str, name: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(source, name), &CompileOptions::default())
        .into_snapshot()
        .expect("the fixture compiles")
}

fn rendered(source: &str, name: &str, target: NotationTarget) -> RenderedNotation {
    render_notation(&score_of(source, name), target, &NotationOptions::default()).expect("renders")
}

fn render(source: &str, name: &str, target: NotationTarget) -> String {
    rendered(source, name, target).text().to_string()
}

/// `MusicXML` gives each part its own measures, so it carries divergent
/// barlines with nothing lost: two time signatures, and eight bars of 7/8
/// beside seven of 4/4.
#[test]
fn musicxml_gives_each_part_its_own_bars() {
    let xml = rendered(BULGARIAN, "bulgarian.musa", NotationTarget::MusicXml);
    assert_eq!(xml.text().matches("<time>").count(), 2);
    assert_eq!(xml.text().matches("<beats>7</beats>").count(), 1);
    assert_eq!(xml.text().matches("<beats>4</beats>").count(), 1);
    assert_eq!(xml.text().matches("<measure ").count(), 15, "eight and seven");
    assert!(xml.warnings().is_empty(), "nothing is lost: {:?}", xml.warnings());
}

/// `LilyPond` keeps Timing at Score by default, so a `\time` written in one
/// staff is written for all of them. Polymeter therefore needs a context
/// move, and the export writes it — without it the file would engrave the
/// wrong piece rather than fail.
#[test]
fn lilypond_moves_timing_to_the_staff() {
    let ly = render(BULGARIAN, "bulgarian.musa", NotationTarget::LilyPond);
    assert!(ly.contains("\\remove \"Timing_translator\""));
    assert!(ly.contains("\\consists \"Timing_translator\""));
    assert!(ly.contains("\\time 7/8") && ly.contains("\\time 4/4"));
    // A piece in one meter needs no such block, and gaining one would be a
    // context change every reader of the file has to account for.
    let plain = render(HEMIOLA, "hemiola.musa", NotationTarget::LilyPond);
    assert!(plain.contains("\\remove \"Timing_translator\""), "6/8 against 3/4");
    let single = "piece \"P\" { meter 4/4; score { part p { voice v { c5 1/1; } } } }";
    assert!(!render(single, "single.musa", NotationTarget::LilyPond).contains("\\layout"));
}

/// MEI states a staff's own meter on its `<staffDef>`, which is MEI's way of
/// saying a staff is counted differently from the score around it — and then
/// says what it cannot do, because `<measure>` is the score's and the barlines
/// here diverge.
#[test]
fn mei_states_each_staffs_meter_and_says_what_it_cannot() {
    let mei = rendered(BULGARIAN, "bulgarian.musa", NotationTarget::Mei);
    assert!(mei.text().contains("<staffDef n=\"2\" lines=\"5\" meter.count=\"4\""));
    assert!(
        mei.warnings().iter().any(|line| line.contains("numbers measures")),
        "{:?}",
        mei.warnings()
    );
    // Same bar length, so nothing diverges and there is nothing to warn
    // about: the two shapes of polymeter cost different things and the
    // warning has to tell them apart.
    let same = rendered(HEMIOLA, "hemiola.musa", NotationTarget::Mei);
    assert!(
        !same.warnings().iter().any(|line| line.contains("numbers measures")),
        "{:?}",
        same.warnings()
    );
}

/// Under polytempo the piece's marking is a reading no staff plays, so it is
/// not printed: each staff prints its own, and every backend that can attach
/// a tempo to a staff does.
#[test]
fn each_staff_prints_its_own_tempo() {
    let ly = render(CANON_X, "canon-x.musa", NotationTarget::LilyPond);
    assert!(ly.contains("\\tempo \"accel.\" 4 = 60"));
    assert!(ly.contains("\\tempo \"rit.\" 4 = 180"));
    assert!(!ly.contains("4 = 120"), "the piece's tempo is nobody's");
    assert!(ly.contains("\\consists \"Metronome_mark_engraver\""));

    let mei = rendered(CANON_X, "canon-x.musa", NotationTarget::Mei);
    assert!(
        mei.text()
            .contains("<tempo staff=\"1\" tstamp=\"1\" place=\"above\" mm=\"60\"")
    );
    assert!(
        mei.text()
            .contains("<tempo staff=\"2\" tstamp=\"1\" place=\"above\" mm=\"180\"")
    );
    assert!(
        mei.warnings().iter().any(|line| line.contains("second conductor")),
        "{:?}",
        mei.warnings()
    );

    let xml = render(CANON_X, "canon-x.musa", NotationTarget::MusicXml);
    assert_eq!(xml.matches("<sound tempo=").count(), 2, "one per part, and no third");
}

/// The barlines move; the noteheads do not. The same music barred two ways
/// engraves the same notes, which is the layer table stated as a test for the
/// third time (roadmap §2).
#[test]
fn no_barline_moves_a_notehead() {
    let notes = "c5 1/8; d5 1/8; e5 1/8; f5 1/8; g5 1/8; a5 1/8; b5 1/8; c6 1/8;";
    let piece =
        |part: &str| format!("piece \"P\" {{ meter 4/4; score {{ part p {{ {part} voice v {{ {notes} }} }} }} }}");
    let pitches = |xml: &str| {
        xml.match_indices("<step>")
            .filter_map(|(index, _)| xml.get(index..index.saturating_add(8)).map(str::to_owned))
            .collect::<Vec<_>>()
    };
    let barred = render(&piece("meter 7/8;"), "barred.musa", NotationTarget::MusicXml);
    let plain = render(&piece(""), "plain.musa", NotationTarget::MusicXml);
    assert_eq!(pitches(&barred), pitches(&plain));
}
