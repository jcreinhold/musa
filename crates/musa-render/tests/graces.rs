//! Grace notes on the page (prompt 71).
//!
//! The law this file exists for is one line long: **the engraving does not
//! depend on the profile.** A grace note is where that is hardest to keep,
//! because every interchange format offers to help — `MusicXML` with
//! `steal-time-previous`, MEI with `@grace="acc"`, `LilyPond` with
//! `\acciaccatura` — and each of those is a *performance* answer offered in
//! the middle of a notation file. Taking any of them would settle in the
//! engraving a question musa settles in the profile, per part, from the same
//! page.
//!
//! So the tests here are of two kinds: the pitches and order are printed, and
//! the reading is not.

// A failure of these is a bug in the fixture, not in a caller's input.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, ScoreSnapshot, SourceDocument, compile};
use musa_render::{NotationOptions, NotationTarget, render_notation};

/// A piece whose only variable is what the profile says about grace notes.
fn piece(grace: &str) -> String {
    format!(
        "piece \"Leaning\" {{ tempo 1/4 = 60; meter 4/4; key c major;
            performance {{ profile band {{ {grace} }} }}
            score {{ part p {{ profile band; voice v {{
                c5 1/4; d5 1/4;
                grace {{ b4; a4; }}
                c5 1/2;
            }} }} }} }}"
    )
}

fn score_of(text: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(text, "graces.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles")
}

fn render(text: &str, target: NotationTarget) -> String {
    render_notation(&score_of(text), target, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string()
}

const ON_THE_BEAT: &str = "grace { steal = 1/8; from = principal; }";
const AHEAD_OF_IT: &str = "grace { steal = 1/32; from = previous; }";

#[test]
fn the_page_does_not_say_how_a_grace_is_played() {
    for target in [NotationTarget::Mei, NotationTarget::MusicXml, NotationTarget::LilyPond] {
        // Not "similar": identical. A profile is a reading of the page, and a
        // reading that changed the page would be an edit.
        assert_eq!(
            render(&piece(ON_THE_BEAT), target),
            render(&piece(AHEAD_OF_IT), target),
            "{target:?} leaked the profile onto the page"
        );
    }
}

#[test]
fn mei_writes_a_grace_group_before_its_note() {
    let mei = render(&piece(ON_THE_BEAT), NotationTarget::Mei);
    assert!(mei.contains("<graceGrp attach=\"pre\">"), "{mei}");
    // `unknown`, not `acc` or `unacc`: those two are the reading.
    assert_eq!(mei.matches("grace=\"unknown\"").count(), 2, "{mei}");
    let group = mei.find("<graceGrp").expect("a group");
    let note = mei[group..].find("pname=\"c\"").expect("the principal follows");
    assert!(mei[group..group + note].contains("pname=\"b\""), "{mei}");
}

#[test]
fn musicxml_writes_grace_notes_without_settling_them() {
    let xml = render(&piece(ON_THE_BEAT), NotationTarget::MusicXml);
    assert_eq!(xml.matches("<grace slash=\"yes\"/>").count(), 2, "{xml}");
    // The two attributes that would settle it, and the element that would
    // make a grace occupy time.
    assert!(!xml.contains("steal-time"), "{xml}");
    let group = xml.find("<grace").expect("a grace");
    let end = xml[group..].find("</note>").expect("the grace note ends");
    assert!(!xml[group..group + end].contains("<duration>"), "{xml}");
}

#[test]
fn lilypond_writes_the_neutral_grace_command() {
    let ly = render(&piece(ON_THE_BEAT), NotationTarget::LilyPond);
    assert!(ly.contains("\\grace { b'8 a'8 }"), "{ly}");
    // The two commands that carry a reading with them.
    assert!(!ly.contains("acciaccatura") && !ly.contains("appoggiatura"), "{ly}");
}

#[test]
fn the_written_order_is_the_printed_order() {
    let printed = |graces: &str| {
        let text = format!(
            "piece \"Order\" {{ tempo 1/4 = 60; meter 4/4; key c major;
                score {{ part p {{ voice v {{ grace {{ {graces} }} c5 1/1; }} }} }} }}"
        );
        render(&text, NotationTarget::LilyPond)
    };
    assert!(printed("d5; e5;").contains("\\grace { d''8 e''8 }"));
    assert!(printed("e5; d5;").contains("\\grace { e''8 d''8 }"));
}
