//! What each backend does with a gradual tempo change.
//!
//! None of the four can draw a *rit.* as a function — MEI, `MusicXML`,
//! `LilyPond` and SMF all state tempo at instants — so the loss is real and the
//! question is what shape it takes. The answer is the same one an engraver
//! gives: print the word where it starts and the speed where it arrives, and
//! leave the middle to the reader. MIDI is the exception, because a player is
//! not a reader: it gets the ramp sampled finely enough to sound continuous.
//!
//! The shape itself stays in the score. That is the point of holding it as a
//! `Progress` rather than as whatever each format could carry.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_render::{NotationOptions, NotationTarget, render_notation};
use musa_score::ScoreSnapshot;

const RUBATO: &str = include_str!("../../../../examples/rubato.musa");
const RISER: &str = include_str!("../../../../examples/riser.musa");

fn score_of(source: &str, name: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(source, name), &CompileOptions::default())
        .into_snapshot()
        .expect("the fixture compiles")
}

fn render(source: &str, name: &str, target: NotationTarget) -> String {
    render_notation(&score_of(source, name), target, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string()
}

/// The page prints a ramp at both ends: the word where it begins, the speed
/// where it arrives. `rubato.musa` slows from 72 to 48 and then says *a
/// tempo*, so `LilyPond` writes 72, the word, 48, and 72 again.
#[test]
fn lilypond_prints_a_ramp_at_both_ends() {
    let ly = render(RUBATO, "rubato.musa", NotationTarget::LilyPond);
    for needle in [
        "\\tempo 4 = 72",
        "\\tempo \"rit.\" 4 = 72",
        "\\tempo 4 = 48",
        "\\tempo \"a tempo\" 4 = 72",
        "\\tempo \"poco rit.\"",
    ] {
        assert!(ly.contains(needle), "the LilyPond export should contain `{needle}`");
    }
}

/// A worded ramp with no destination prints its word and nothing else — no
/// second marking, because there is no second speed to state. The same law
/// the compiler suite states in frames, seen on the page.
#[test]
fn a_worded_ramp_prints_once() {
    let xml = render(RUBATO, "rubato.musa", NotationTarget::MusicXml);
    assert_eq!(xml.matches("poco rit.").count(), 1, "the word, once");
    // Four `<sound tempo>`: the opening 72, the 48 the rit. arrives at, the
    // a tempo 72 — and the 72 the rit. leaves from. The worded ramp
    // contributes none, which is what makes it notation only.
    assert_eq!(xml.matches("<sound tempo=").count(), 4);
}

/// MEI states the arrival as a real metronome mark rather than as words: a
/// consumer that computes with tempo gets a number at each end.
#[test]
fn mei_states_the_speed_a_ramp_arrives_at() {
    let mei = render(RUBATO, "rubato.musa", NotationTarget::Mei);
    assert!(mei.contains("mm=\"48\""), "the speed the rit. reaches");
    assert!(mei.contains(">rit.<"), "and the word it is printed under");
}

/// A ramp changes no note's written value or place. Slowing down is
/// interpretation of time, not a rewriting of it (roadmap §2), and the
/// engraved page is the evidence: the same music with and without.
#[test]
fn a_ramp_moves_no_notehead() {
    let with_ramp = render(RISER, "riser.musa", NotationTarget::Mei);
    let without = render(
        &RISER.replace("tempo 1/4 = 128 to 160 over 8/1 \"accel.\";", "tempo 1/4 = 128;"),
        "riser.musa",
        NotationTarget::Mei,
    );
    // The note elements themselves, and nothing around them: a tempo mark
    // lands between measures, and comparing whole neighbourhoods would be
    // comparing the very thing that is supposed to differ.
    let notes = |mei: &str| {
        mei.match_indices("<note ")
            .filter_map(|(index, _)| {
                let rest = mei.get(index..)?;
                let end = rest.find('>')?.saturating_add(1);
                rest.get(..end).map(str::to_owned)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(notes(&with_ramp), notes(&without));
}
