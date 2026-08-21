//! What each backend does with tempo marks and hairpins.
//!
//! Same contract as the annotation suite: the snapshots pin the exact bytes,
//! and this pins the facts a reader would notice if a backend dropped one.
//! A tempo the export forgets is a piece that plays at the wrong speed
//! everywhere else; a wedge with no mark at the end of it is a sign that
//! says "get louder" and never says how loud.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ImportSources, SourceDocument, compile};

use musa_notation::{NotationOptions, NotationTarget, render_notation};
use musa_score::ScoreSnapshot;

const MOTIFS: &str = include_str!("../../../../examples/album/library/motifs.musa");
const PATCHES: &str = include_str!("../../../../examples/album/library/patches.musa");
const OPENING: &str = include_str!("../../../../examples/album/pieces/01-opening.musa");

/// The album's opening piece, compiled with its libraries in hand.
fn opening() -> ScoreSnapshot {
    let mut imports = ImportSources::default();
    imports.insert("examples/album/library/motifs.musa", MOTIFS);
    imports.insert("examples/album/library/patches.musa", PATCHES);
    compile(
        &SourceDocument::new(OPENING, "examples/album/pieces/01-opening.musa"),
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    )
    .into_snapshot()
    .expect("the album fixture compiles")
}

fn render(target: NotationTarget) -> String {
    render_notation(&opening(), target, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string()
}

fn assert_contains(text: &str, needles: &[&str], what: &str) {
    for needle in needles {
        assert!(text.contains(needle), "{what} should contain `{needle}`");
    }
}

/// MEI states a metronome mark twice — as attributes for a consumer that
/// computes with it, as text for one that prints it — and spans a hairpin by
/// the notes at its ends.
#[test]
fn mei_carries_both_tempos_and_the_hairpin() {
    let mei = render(NotationTarget::Mei);
    assert_contains(
        &mei,
        &["mm=\"72\"", "mm=\"108\"", "mm.unit=\"4\"", "<hairpin", "form=\"cres\""],
        "the MEI export",
    );
    assert_eq!(mei.matches("<tempo").count(), 2, "a starting tempo and one change");
}

/// `MusicXML` writes the mark for a reader (`<metronome>`) and the same fact
/// for a player (`<sound tempo>`), and closes a wedge with the dynamic it
/// arrived at.
#[test]
fn musicxml_carries_both_tempos_and_the_hairpin() {
    let xml = render(NotationTarget::MusicXml);
    assert_contains(
        &xml,
        &[
            "<beat-unit>quarter</beat-unit>",
            "<per-minute>72</per-minute>",
            "<per-minute>108</per-minute>",
            "<sound tempo=\"72\"/>",
            "<wedge type=\"crescendo\" number=\"1\"/>",
            "<wedge type=\"stop\" number=\"1\"/>",
        ],
        "the MusicXML export",
    );
}

/// `LilyPond` writes both as a user would by hand — including the word the
/// second marking carries, which `\tempo` takes ahead of the number — and a
/// wedge that opens on a note and closes on a dynamic.
#[test]
fn lilypond_carries_both_tempos_and_the_hairpin() {
    let ly = render(NotationTarget::LilyPond);
    assert_contains(
        &ly,
        &["\\tempo 4 = 72", "\\tempo \"poco più mosso\" 4 = 108", "\\<", "\\f"],
        "the LilyPond export",
    );
}

/// A diminuendo is the other wedge, and each backend says so in its own way
/// rather than printing a crescendo backwards.
#[test]
fn a_diminuendo_is_the_other_wedge_everywhere() {
    let source = "piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major; score {
        part p { voice v { diminuendo to p { c5/4 d5/4 e5/4 f5/4 } } } } }";
    let score = compile(&SourceDocument::new(source, "d.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles");
    let of = |target| {
        render_notation(&score, target, &NotationOptions::default())
            .expect("renders")
            .text()
            .to_string()
    };
    assert!(of(NotationTarget::Mei).contains("form=\"dim\""));
    assert!(of(NotationTarget::MusicXml).contains("<wedge type=\"diminuendo\" number=\"1\"/>"));
    assert!(of(NotationTarget::LilyPond).contains("\\>"));
}
