//! What each backend does with music that has no barlines.
//!
//! Three of the four formats have a way to say it and say it differently:
//! MEI marks the measure as not controlled by the meter, `MusicXML` gives the
//! barline a style of `none`, and `LilyPond` has `\cadenzaOn`. SMF has no way
//! to say it at all, so its time signatures simply stop — the honest silence
//! rather than a signature of no beats.
//!
//! What none of them does is move a note. Where the barlines stop, the clock
//! does not (roadmap §2).

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, PerformanceOptions, ScoreSnapshot, SourceDocument, compile, lower_performance};
use musa_render::{MidiOptions, NotationOptions, NotationTarget, render_midi, render_notation};

const CADENZA: &str = include_str!("../../../../examples/cadenza.musa");
const CHANT: &str = include_str!("../../../../examples/chant.musa");

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

/// `LilyPond` opens and closes a cadenza exactly once around the unmeasured
/// stretch, and writes no bar check inside it — a bar check asserts a barline
/// falls here, and inside a cadenza none does.
#[test]
fn lilypond_brackets_a_cadenza() {
    let ly = render(CADENZA, "cadenza.musa", NotationTarget::LilyPond);
    assert_eq!(ly.matches("\\cadenzaOn").count(), 1);
    assert_eq!(ly.matches("\\cadenzaOff").count(), 1);
    let open = ly.find("\\cadenzaOn").expect("opens");
    let close = ly.find("\\cadenzaOff").expect("closes");
    let inside = ly.get(open..close).expect("a stretch between them");
    assert!(!inside.contains('|'), "no bar check inside the cadenza");
}

/// A piece that is unmeasured from the first note opens in `\cadenzaOn` and
/// never leaves it, and prints no time signature: there is none.
#[test]
fn lilypond_opens_a_chant_unmeasured() {
    let ly = render(CHANT, "chant.musa", NotationTarget::LilyPond);
    assert_eq!(ly.matches("\\cadenzaOn").count(), 1);
    assert_eq!(ly.matches("\\cadenzaOff").count(), 0);
    assert!(!ly.contains("\\time"), "no time signature to print");
}

/// MEI says it with the vocabulary MEI has: the measure is not controlled by
/// the meter, and the line that would close it is not drawn.
#[test]
fn mei_marks_the_measure_as_uncontrolled() {
    let mei = render(CADENZA, "cadenza.musa", NotationTarget::Mei);
    assert!(mei.contains("metcon=\"false\""), "the cadenza measure");
    assert!(mei.contains("right=\"invis\""), "and its barline");
    assert_eq!(
        mei.matches("metcon=\"false\"").count(),
        1,
        "one measure, not the movement"
    );
    // An unmeasured piece states no meter at all, rather than a meter of no
    // beats — `meter.count="0"` would be a document that means something
    // else.
    let chant = render(CHANT, "chant.musa", NotationTarget::Mei);
    assert!(!chant.contains("meter.count"), "nothing to state");
}

/// `MusicXML` closes an unmeasured measure with a barline it does not draw,
/// and the movement's numbering carries straight on: measure 5 is the
/// cadenza, measure 6 is the music after it.
#[test]
fn musicxml_writes_a_barline_it_does_not_draw() {
    let xml = render(CADENZA, "cadenza.musa", NotationTarget::MusicXml);
    assert_eq!(xml.matches("<bar-style>none</bar-style>").count(), 1);
    assert_eq!(xml.matches("number=\"6\"").count(), 1, "the movement resumes at 6");
    assert_eq!(xml.matches("number=\"5\"").count(), 1, "and the cadenza was 5");
    let chant = render(CHANT, "chant.musa", NotationTarget::MusicXml);
    assert!(
        !chant.contains("<beats>"),
        "an unmeasured piece prints no time signature"
    );
}

/// The barlines stop; the notes do not move. The same music with and without
/// the `senza` engraves the same noteheads, which is the layer table stated
/// as a test.
#[test]
fn no_barline_moves_a_notehead() {
    let unwrapped = CADENZA.replace("senza {", "// senza {").replace(
        "                }\n\n                // Measure 6",
        "                // Measure 6",
    );
    let with = render(CADENZA, "cadenza.musa", NotationTarget::Mei);
    let without = render(&unwrapped, "cadenza.musa", NotationTarget::Mei);
    let notes = |mei: &str| {
        mei.match_indices("<note ")
            .filter_map(|(index, _)| {
                let rest = mei.get(index..)?;
                let end = rest.find('>')?.saturating_add(1);
                rest.get(..end).map(str::to_owned)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(notes(&with).len(), notes(&without).len(), "the same notes");
}

/// SMF has no way to say "the barlines stop", so musa says nothing: the
/// cadenza contributes no time signature, and the movement's 4/4 stands on
/// either side of it. A signature of no beats would be a file that means
/// something else.
#[test]
fn midi_says_nothing_where_there_is_no_meter() {
    let count = |source: &str| {
        let score = score_of(source, "unmeasured.musa");
        let performance = lower_performance(&score, &PerformanceOptions::default()).expect("lowers");
        let bytes = render_midi(&performance, &MidiOptions::default()).expect("renders");
        let smf = midly::Smf::parse(&bytes).expect("parses");
        smf.tracks
            .iter()
            .flatten()
            .filter(|event| {
                matches!(
                    event.kind,
                    midly::TrackEventKind::Meta(midly::MetaMessage::TimeSignature(..))
                )
            })
            .count()
    };
    // The opening 4/4 and the 4/4 that resumes after the cadenza. The
    // cadenza itself contributes none.
    assert_eq!(count(CADENZA), 2);
    assert_eq!(count(CHANT), 0, "a piece with no meter states no meter");
}
