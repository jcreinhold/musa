//! What a second `meter` does.
//!
//! Three things are worth a test here and nothing else is: that a change moves
//! the barlines and renumbers the measures after it, that the two refusals the
//! coordinate system rests on actually refuse, and that a bar is checked
//! against the meter in force *where it sits* rather than against the piece's
//! first one — which is the whole difference between this and a scalar.

#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{MusicalTime, Scope};
use num_rational::Ratio;

fn compiled(source: &str) -> musa_compiler::Compilation {
    compile(&SourceDocument::new(source, "meter.musa"), &CompileOptions::default())
}

fn errors(source: &str) -> Vec<String> {
    compiled(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

/// Two 4/4 bars, then 3/4, then two 3/4 bars.
const TUNE: &str = "piece \"p\" { meter 4/4; score { part a { voice b { \
                    bar { c4/1 } bar { d4/1 } meter 3/4; bar { e4/2. } bar { f4/2. } \
                    } } } }";

#[test]
fn a_second_meter_moves_the_barlines_after_it() {
    let compilation = compiled(TUNE);
    assert!(compilation.diagnostics().is_empty(), "{:?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("it compiles");
    let bars = score.bars(musa_score::Scope::Piece);
    let whole = |n: i64| MusicalTime::new(Ratio::from_integer(n));

    // Measures 1 and 2 are whole notes; 3 and 4 are three quarters.
    assert_eq!(bars.at(whole(0)).measure, 1);
    assert_eq!(bars.at(whole(2)).measure, 3);
    assert_eq!(bars.measure_at(whole(2)).end, MusicalTime::new(Ratio::new(11, 4)));
    assert_eq!(bars.at(MusicalTime::new(Ratio::new(11, 4))).measure, 4);

    // And `time_of` still inverts `at` across the seam.
    assert_eq!(bars.time_of(3, Ratio::ONE), Some(whole(2)));
    assert_eq!(bars.time_of(4, Ratio::ONE), Some(MusicalTime::new(Ratio::new(11, 4))));
}

#[test]
fn the_meter_track_answers_where_rather_than_what() {
    let compilation = compiled(TUNE);
    let score = compilation.snapshot().expect("it compiles");
    let meters = score.meters();
    assert!(!meters.is_constant(), "a piece that changes meter is not constant");
    assert_eq!(meters.changes(Scope::Piece).count(), 2);
    let opening = score.meter_at(Scope::Piece, MusicalTime::ZERO);
    assert_eq!((opening.numerator(), opening.denominator()), (4, 4));
    let later = score.meter_at(Scope::Piece, MusicalTime::new(Ratio::from_integer(2)));
    assert_eq!((later.numerator(), later.denominator()), (3, 4));
}

#[test]
fn a_bar_is_measured_against_the_meter_it_sits_in() {
    // The 3/4 bar would have been a whole note short under the piece's first
    // meter, and the 4/4 bar would have been a quarter too long under its
    // second. Both are right where they are, and swapping them is not.
    let wrong = "piece \"p\" { meter 4/4; score { part a { voice b { \
                 bar { c4/1 } meter 3/4; bar { d4/1 } } } } }";
    assert_eq!(
        errors(wrong),
        vec!["this bar is 1/4 too long".to_owned()],
        "the second bar is checked in 3/4"
    );
}

#[test]
fn a_change_that_misses_a_barline_is_refused() {
    let mid = "piece \"p\" { meter 4/4; score { part a { voice b { \
               c4/4 meter 3/4; d4/4 e4/4 f4/4 } } } }";
    assert_eq!(errors(mid), vec!["a meter change must land on a barline".to_owned()]);
}

#[test]
fn a_change_inside_material_is_refused() {
    // A motif is written once and can be played anywhere; a meter change is
    // nothing but a place. The refusal is what keeps `Share` sound.
    let inside = "piece \"p\" { meter 4/4; motif m() { meter 3/4; c4/1 } \
                  score { part a { voice b { use m; } } } }";
    assert_eq!(
        errors(inside),
        vec!["a meter change belongs to the piece, not to material".to_owned()]
    );
    let in_a_repeat = "piece \"p\" { meter 4/4; score { part a { voice b { \
                       repeat 2 { meter 3/4; c4/1 } } } } }";
    assert_eq!(
        errors(in_a_repeat),
        vec!["a meter change belongs to the piece, not to material".to_owned()]
    );
    // An *unnamed* bar is not material — it is played once, at one place —
    // so a meter inside one is refused by the barline rule instead, which is
    // the rule that actually applies to it.
    let mid_bar = "piece \"p\" { meter 4/4; score { part a { voice b { \
                   bar { c4/2 meter 3/4; d4/2 } } } } }";
    assert_eq!(
        errors(mid_bar),
        vec!["a meter change must land on a barline".to_owned()]
    );
}

#[test]
fn two_voices_may_both_name_the_same_change_and_may_not_disagree() {
    let agreed = "piece \"p\" { meter 4/4; score { part a { \
                  voice b { bar { c4/1 } meter 3/4; bar { d4/2. } } \
                  voice c { bar { e4/1 } meter 3/4; bar { f4/2. } } } } }";
    assert!(errors(agreed).is_empty(), "{:?}", errors(agreed));

    let disagreed = "piece \"p\" { meter 4/4; score { part a { \
                     voice b { bar { c4/1 } meter 3/4; bar { d4/2. } } \
                     voice c { bar { e4/1 } meter 5/8; bar { f4/2. } } } } }";
    assert_eq!(errors(disagreed), vec!["two meters at the same place".to_owned()]);
}
