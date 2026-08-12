//! Chord symbols and sounded voicings, across every backend.
//!
//! A chord symbol above the staff and the notes below it are two different
//! facts. The suite proves that by writing a piece where they disagree: the
//! harmony lane says `fmaj7` and the voice sounds a C-sharp minor seventh.
//! Every export must carry both, unchanged, and neither must be derived from
//! the other.
//!
//! The second thing proved here is spelling. `c#4 e4 g#4 b4` is four sharps
//! and no flats in the notation backends, and four key numbers in MIDI —
//! written pitch and sounded pitch survive on their own terms.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, PerformanceOptions, ScoreSnapshot, SourceDocument, compile, lower_performance};
use musa_render::{MidiMode, MidiOptions, NotationOptions, NotationTarget, render_midi, render_notation};

/// A lead sheet whose symbol and whose notes deliberately disagree.
///
/// `fmaj7` is what a reader sees above the staff; `stack c#4 minor7` is what
/// sounds. Nothing in musa reconciles them, and this fixture is what says so.
const DISAGREEING: &str = "\
piece \"Chords\" {
    tempo 1/4 = 60;
    meter 4/4;

    score {
        harmony { at 1:1 fmaj7; }
        part p { voice v { stack c#4 minor7/1 } }
    }
}
";

fn score() -> ScoreSnapshot {
    compile(
        &SourceDocument::new(DISAGREEING, "chords.musa"),
        &CompileOptions::default(),
    )
    .into_snapshot()
    .expect("compiles")
}

fn render(target: NotationTarget) -> String {
    render_notation(&score(), target, &NotationOptions::default())
        .expect("renders")
        .text()
        .to_string()
}

fn assert_contains(text: &str, needles: &[&str], what: &str) {
    for needle in needles {
        assert!(text.contains(needle), "{what} should contain `{needle}`:\n{text}");
    }
}

#[test]
fn mei_keeps_the_symbol_and_the_spelling_apart() {
    let mei = render(NotationTarget::Mei);
    assert_contains(
        &mei,
        &["<harm staff=\"1\" tstamp=\"1\" place=\"above\">fmaj7</harm>"],
        "MEI",
    );
    // The sounded chord is four notes, spelled: two sharps written out as
    // accidentals, and no F anywhere — the symbol did not become notes.
    for note in [
        "oct=\"4\" pname=\"c\" accid=\"s\"",
        "oct=\"4\" pname=\"e\"",
        "oct=\"4\" pname=\"g\" accid=\"s\"",
        "oct=\"4\" pname=\"b\"",
    ] {
        assert!(mei.contains(note), "MEI should sound `{note}`:\n{mei}");
    }
    assert!(!mei.contains("pname=\"f\""), "the symbol's root must not be sounded");
}

#[test]
fn lilypond_keeps_the_symbol_and_the_spelling_apart() {
    let ly = render(NotationTarget::LilyPond);
    // `LilyPond` writes the symbol in a ChordNames context, in its own
    // spelling, and the notes in the staff, in theirs.
    assert_contains(&ly, &["\\new ChordNames", "f1:maj7", "<cs' e' gs' b'>1"], "LilyPond");
}

#[test]
fn musicxml_keeps_the_symbol_and_the_spelling_apart() {
    let xml = render(NotationTarget::MusicXml);
    // The symbol is a `<harmony>` rooted on F; the notes are `<pitch>`
    // elements rooted on C-sharp. MusicXML has both, and they say what they
    // were written as.
    assert_contains(
        &xml,
        &[
            "<root-step>F</root-step>",
            "<kind text=\"fmaj7\">major-seventh</kind>",
            "<step>C</step>",
            "<alter>1</alter>",
            "<step>G</step>",
        ],
        "MusicXML",
    );
    assert!(
        !xml.contains("<step>F</step>"),
        "the symbol's root must not appear as a sounded pitch:\n{xml}"
    );
}

#[test]
fn midi_sounds_the_notes_and_not_the_symbol() {
    let performance = lower_performance(&score(), &PerformanceOptions::default()).expect("lowers");
    let bytes = render_midi(
        &performance,
        &MidiOptions {
            mode: MidiMode::Score,
            ..MidiOptions::default()
        },
    )
    .expect("renders");
    let smf = midly::Smf::parse(&bytes).expect("parses");
    let mut keys: Vec<u8> = Vec::new();
    for track in &smf.tracks {
        for event in track {
            if let midly::TrackEventKind::Midi {
                message: midly::MidiMessage::NoteOn { key, vel },
                ..
            } = event.kind
                && vel.as_int() > 0
            {
                keys.push(key.as_int());
            }
        }
    }
    keys.sort_unstable();
    // C-sharp 4, E4, G-sharp 4, B4 — the voicing, and only the voicing. A
    // chord symbol is not a sound, so nothing here comes from `fmaj7`.
    assert_eq!(keys, vec![61, 64, 68, 71]);
}
