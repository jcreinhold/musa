//! Groove: the beat moves, the page does not.
//!
//! Roadmap §2 says notated duration ≠ performed duration. Until a groove
//! existed nothing in musa used that row — the notated duration *was* the
//! performed duration, so the distinction was a promise rather than a fact.
//! These tests are the fact.
//!
//! Two of them matter more than the rest. `a_groove_never_reaches_the_page`
//! is the layer separation itself: swing the whole band and the engraver's
//! answer must not move by one byte. `a_groove_follows_a_meter_change` shows
//! a swing asks what a pair is per instant, so it follows a meter change
//! without knowing there was one.

// A failure of these is a bug in the fixture, not in a caller's input.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
// Fixtures index their own fixed-length results; a bad index is a broken
// fixture, and the panic names it immediately.
#![allow(clippy::indexing_slicing)]
// Exact rational comparisons throughout; see `time.rs`.
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{
    CompileOptions, PerformanceEvent, PerformanceOptions, ScoreSnapshot, SourceDocument, compile, lower_performance,
};

/// A piece whose only variable is what the profile says about the beat.
fn piece(profile: &str) -> String {
    format!(
        "piece \"Feel\" {{ tempo 1/4 = 60; meter 4/4; key c major;
            performance {{ profile band {{ {profile} }} }}
            score {{ part p {{ profile band; voice v {{
                c5/8 d5/8 e5/8 f5/8
                g5/8 a5/8 b5/8 c6/8
            }} }} }} }}"
    )
}

fn score_of(text: &str) -> ScoreSnapshot {
    compile(&SourceDocument::new(text, "feel.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles")
}

/// The frame each note starts at, in written order.
fn onsets(score: &ScoreSnapshot) -> Vec<u64> {
    let plan = lower_performance(score, &PerformanceOptions::default()).expect("lowers");
    let mut frames: Vec<u64> = plan
        .lanes()
        .iter()
        .flat_map(|lane| lane.events().iter())
        .filter_map(|event| match event {
            PerformanceEvent::NoteOn { frame, .. } => Some(*frame),
            PerformanceEvent::NoteOff { .. } | PerformanceEvent::Parameter { .. } => None,
        })
        .collect();
    frames.sort_unstable();
    frames
}

/// Straight eighths at 60 bpm are one second apart in pairs; swung, the first
/// of each pair takes two thirds of the pair and the pair boundary holds.
#[test]
fn a_swing_moves_the_offbeat_and_not_the_beat() {
    let straight = onsets(&score_of(&piece("")));
    let swung = onsets(&score_of(&piece("groove swing { ratio = 2/3; }")));
    assert_eq!(straight.len(), swung.len(), "the same notes are played either way");
    // A quarter is one second at 1/4 = 60, so a pair of eighths spans one
    // second and its midpoint is at half of it, or two thirds of it swung.
    let rate = PerformanceOptions::default().sample_rate;
    let second = u64::from(rate);
    for (index, (plain, swing)) in straight.iter().zip(&swung).enumerate() {
        if index % 2 == 0 {
            assert_eq!(plain, swing, "note {index} is on the beat and must not move");
        } else {
            assert!(swing > plain, "note {index} is the offbeat and must be late");
        }
    }
    // At 1/4 = 60 a whole note is four seconds. The second eighth is written
    // at 1/8 — half a second — and sounds at 1/6, which is two thirds of one.
    assert_eq!(straight[1], second / 2);
    assert_eq!(swung[1], second * 2 / 3);
}

/// The check that matters most: an engraver must not learn that the band
/// swings, because there is no swung notation to draw.
#[test]
fn a_groove_never_reaches_the_page() {
    let straight = score_of(&piece(""));
    let swung = score_of(&piece("groove swing { ratio = 2/3; }"));
    let written = |score: &ScoreSnapshot| {
        score
            .parts()
            .iter()
            .flat_map(|(_, part)| part.voices().map(|(_, voice)| voice.events().to_vec()))
            .flatten()
            .map(|event| (event.onset, event.notated_duration.value))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        written(&straight),
        written(&swung),
        "the groove moved the written score, which is exactly what it must never do"
    );
}

/// A swing asks what a "pair" is at each instant, so a meter change carries
/// it without either side knowing about the other.
#[test]
fn a_groove_follows_a_meter_change() {
    let source = "piece \"Turn\" { tempo 1/4 = 60; meter 4/4; key c major;
        performance { profile band { groove swing { ratio = 2/3; } } }
        score { part p { profile band; voice v {
            c5/8 d5/8 c5/8 d5/8 c5/8 d5/8 c5/8 d5/8
            meter 6/8;
            e5/16 f5/16 e5/16 f5/16 e5/16 f5/16
            e5/16 f5/16 e5/16 f5/16 e5/16 f5/16
        } } } }";
    let frames = onsets(&score_of(source));
    // In 4/4 the pair is two eighths, so the eighth-note offbeats moved. In
    // 6/8 the pair is two sixteenths, so the *sixteenth* offbeats moved and
    // the eighths did not — a different set of notes, decided by the meter.
    let straight = "piece \"Turn\" { tempo 1/4 = 60; meter 4/4; key c major;
        score { part p { voice v {
            c5/8 d5/8 c5/8 d5/8 c5/8 d5/8 c5/8 d5/8
            meter 6/8;
            e5/16 f5/16 e5/16 f5/16 e5/16 f5/16
            e5/16 f5/16 e5/16 f5/16 e5/16 f5/16
        } } } }";
    let plain = onsets(&score_of(straight));
    assert_eq!(frames.len(), plain.len());
    let moved: Vec<bool> = frames.iter().zip(&plain).map(|(a, b)| a != b).collect();
    // Eight eighths: every other one is an offbeat and moves.
    assert_eq!(&moved[..8], &[false, true, false, true, false, true, false, true]);
    // Twelve sixteenths in 6/8: same alternation, at the smaller unit — which
    // only happens because the pair length came from the meter in force.
    assert_eq!(
        &moved[8..],
        &[
            false, true, false, true, false, true, false, true, false, true, false, true
        ]
    );
}

/// Feel is per part, because an arrangement is sections disagreeing on
/// purpose: a swung horn over a straight bass is a thing people write.
#[test]
fn two_parts_may_disagree_about_the_beat() {
    let source = "piece \"Two\" { tempo 1/4 = 60; meter 4/4; key c major;
        performance {
            profile swung { groove swing { ratio = 2/3; } }
            profile flat { groove straight {} }
        }
        score {
            part horn { profile swung; voice v { c5/8 d5/8 c5/8 d5/8 } }
            part bass { profile flat; voice v { c3/8 d3/8 c3/8 d3/8 } }
        } }";
    let score = score_of(source);
    let plan = lower_performance(&score, &PerformanceOptions::default()).expect("lowers");
    let lane = |name: &str| {
        let lane = plan.lanes().iter().find(|lane| lane.name() == name).expect("a lane");
        let mut frames: Vec<u64> = lane
            .events()
            .iter()
            .filter_map(|event| match event {
                PerformanceEvent::NoteOn { frame, .. } => Some(*frame),
                PerformanceEvent::NoteOff { .. } | PerformanceEvent::Parameter { .. } => None,
            })
            .collect();
        frames.sort_unstable();
        frames
    };
    let horn = lane("horn");
    let bass = lane("bass");
    assert_eq!(horn[0], bass[0], "the downbeat is shared");
    assert!(horn[1] > bass[1], "the horn's offbeat is late and the bass's is not");
    assert_eq!(horn[2], bass[2], "and they meet again on the next beat");
}

/// A groove that could reorder notes — or whose pattern would not tile the
/// whole note — is refused where it is written, so the warp never has to
/// defend itself.
#[test]
fn a_groove_that_would_fold_time_is_refused() {
    for bad in [
        "groove swing { ratio = 0; }",
        "groove swing { ratio = 1; }",
        "groove push { grid = 1/8; by = -1/8; }",
        "groove push { grid = 1/8; by = 1/4; }",
        // A third of a whole note does not tile it, so the pattern would land
        // differently in every bar — found by `a_groove_keeps_the_whole_note`.
        "groove push { grid = 1/3; by = 1/64; }",
    ] {
        let compilation = compile(&SourceDocument::new(piece(bad), "bad.musa"), &CompileOptions::default());
        assert!(
            compilation
                .diagnostics()
                .iter()
                .any(|d| d.message.contains("cannot be laid over the beat")),
            "`{bad}` was accepted: {:?}",
            compilation.diagnostics()
        );
    }
}

/// A name that is not a groove is a diagnostic with the list, not a silent
/// straight performance.
#[test]
fn an_unknown_groove_is_named() {
    let compilation = compile(
        &SourceDocument::new(piece("groove swong { ratio = 2/3; }"), "bad.musa"),
        &CompileOptions::default(),
    );
    let said: Vec<&str> = compilation.diagnostics().iter().map(|d| d.message.as_str()).collect();
    assert!(
        said.iter().any(|message| message.contains("`swong` is not a groove")),
        "{said:?}"
    );
}

/// A part has one beat, so two grooves is a mistake rather than a composition.
#[test]
fn a_profile_has_at_most_one_groove() {
    let compilation = compile(
        &SourceDocument::new(piece("groove swing { ratio = 2/3; } groove straight {}"), "two.musa"),
        &CompileOptions::default(),
    );
    let said: Vec<&str> = compilation.diagnostics().iter().map(|d| d.message.as_str()).collect();
    assert!(
        said.iter().any(|message| message.contains("more than one groove")),
        "{said:?}"
    );
}

/// A piece that declares no groove performs exactly as it did before grooves
/// existed — the guarantee that lets this land without touching the corpus.
#[test]
fn no_groove_is_the_identity() {
    let plain = onsets(&score_of(&piece("")));
    let named = onsets(&score_of(&piece("groove straight {}")));
    assert_eq!(plain, named);
}
