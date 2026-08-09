//! Tempo and expression curves (docs/prompts/36; roadmap §6.3, §6.4).
//!
//! Two things change over a piece's length and neither of them changes a
//! note: how fast it goes, and how loud. This suite pins what that means.
//!
//! For tempo, the claim is arithmetic and checkable: a bar at 60 lasts a
//! second per quarter, a bar at 120 lasts half of one, and a piece with a
//! change in it lasts exactly the sum — no drift accumulated at the seam.
//! For hairpins, the claim is about endpoints: a wedge starts at whatever
//! was in force and arrives exactly at the mark it names, and everything
//! between is a straight line across the notes it covers.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    CompileOptions, MusicalTime, PerformanceEvent, PerformanceOptions, ScoreSnapshot, SourceDocument, compile,
    lower_performance,
};

const RATE: u32 = 48_000;

fn score_of(source: &str) -> ScoreSnapshot {
    let compilation = compile(&SourceDocument::new(source, "curve.musa"), &CompileOptions::default());
    let messages: Vec<String> = compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    compilation
        .into_snapshot()
        .unwrap_or_else(|| panic!("expected a snapshot; diagnostics: {messages:?}"))
}

/// Four bars of quarter notes in 4/4, with `header` written before the score.
fn piece(header: &str, voice: &str) -> String {
    format!(
        "piece \"P\" {{ tempo 1/4 = 60; {header} meter 4/4; key c major;
         score {{ part p {{ voice v {{ {voice} }} }} }} }}"
    )
}

const SIXTEEN_QUARTERS: &str = "c5/4 c5/4 c5/4 c5/4 c5/4 c5/4 c5/4 c5/4
     c5/4 c5/4 c5/4 c5/4 c5/4 c5/4 c5/4 c5/4";

/// Sixteen quarters with `statement` written between the `before`th and the
/// one after it. A tempo change stands where it happens — the marking has no
/// coordinate of its own, it has a place in the music.
fn quarters_with_tempo(before: usize, statement: &str) -> String {
    let mut notes: Vec<String> = std::iter::repeat_n("c5/4".to_owned(), 16).collect();
    notes.insert(before, statement.to_owned());
    notes.join(" ")
}

/// The onset frames of every note-on, in order.
fn onsets(score: &ScoreSnapshot) -> Vec<u64> {
    let plan = lower_performance(
        score,
        &PerformanceOptions {
            sample_rate: RATE,
            ..PerformanceOptions::default()
        },
    )
    .expect("schedules");
    let mut frames: Vec<u64> = plan
        .lanes()
        .iter()
        .flat_map(musa_compiler::PerformanceLane::events)
        .filter_map(|event| match event {
            PerformanceEvent::NoteOn { frame, .. } => Some(*frame),
            PerformanceEvent::NoteOff { .. } | PerformanceEvent::Parameter { .. } => None,
        })
        .collect();
    frames.sort_unstable();
    frames
}

/// The amplitude of every note-on, in order.
fn amplitudes(score: &ScoreSnapshot) -> Vec<f32> {
    let plan = lower_performance(score, &PerformanceOptions::default()).expect("schedules");
    let mut notes: Vec<(u64, f32)> = plan
        .lanes()
        .iter()
        .flat_map(musa_compiler::PerformanceLane::events)
        .filter_map(|event| match event {
            PerformanceEvent::NoteOn { frame, note, .. } => Some((*frame, note.amplitude)),
            PerformanceEvent::NoteOff { .. } | PerformanceEvent::Parameter { .. } => None,
        })
        .collect();
    notes.sort_by_key(|(frame, _)| *frame);
    notes.into_iter().map(|(_, amplitude)| amplitude).collect()
}

/// Before the change, a quarter at 60 is a second; after it, a quarter at 120
/// is half of one. Every onset is exactly where those two rates put it —
/// which is the whole claim of an *integrated* tempo map.
#[test]
fn a_tempo_change_moves_every_note_after_it_and_none_before() {
    let score = score_of(&piece("", &quarters_with_tempo(8, "tempo 1/4 = 120;")));
    // Bars 1–2 at 60 bpm: one second each quarter. Bars 3–4 at 120: half.
    let expected: Vec<u64> = (0..8)
        .map(|index| u64::from(RATE) * index)
        .chain((0..8).map(|index| u64::from(RATE) * 8 + u64::from(RATE) * index / 2))
        .collect();
    assert_eq!(onsets(&score), expected);
}

/// The same piece without the change: the second half is where 60 bpm puts
/// it, so the test above is measuring the tempo and not the notes.
#[test]
fn a_piece_without_a_change_is_the_tempo_it_declares() {
    let score = score_of(&piece("", SIXTEEN_QUARTERS));
    let expected: Vec<u64> = (0..16).map(|index| u64::from(RATE) * index).collect();
    assert_eq!(onsets(&score), expected);
}

/// The seam is exact. A tempo change on a beat that is not a whole number of
/// frames from the start would drift if the map rounded each segment; it
/// accumulates in rationals and rounds once, so the note at the change lands
/// on the frame the arithmetic says.
#[test]
fn the_frame_at_a_tempo_change_is_the_sum_of_what_came_before() {
    // 7 bpm makes a quarter 60/7 seconds — never a whole number of frames.
    let source = piece("", &quarters_with_tempo(4, "tempo 1/4 = 7;"));
    let score = score_of(&source);
    let frames = onsets(&score);
    let at_change = frames.get(4).copied().expect("a note at the change");
    assert_eq!(at_change, u64::from(RATE) * 4, "bar 1 at 60 bpm is four seconds");
    let next = frames.get(5).copied().expect("a note after the change");
    // 60/7 of a second, rounded once.
    let expected = at_change + (f64::from(RATE) * 60.0 / 7.0).round() as u64;
    assert_eq!(next, expected);
}

/// A header says how fast the piece starts exactly once. Every later marking
/// is a place in the music, written where it happens like a meter or a key
/// change — so there is no second way to place one, and no coordinate that
/// could be out of range.
#[test]
fn the_header_says_how_fast_the_piece_starts_exactly_once() {
    let compilation = compile(
        &SourceDocument::new(piece("tempo 1/4 = 90;", SIXTEEN_QUARTERS), "curve.musa"),
        &CompileOptions::default(),
    );
    let messages: Vec<String> = compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("this piece already says how fast it starts")),
        "a second starting tempo should be refused, got {messages:?}"
    );
}

/// The marking and the map, kept apart and shown apart: *Meno mosso* with no
/// number is printed notation and moves no clock, so every note lands exactly
/// where it landed without it.
#[test]
fn a_tempo_word_with_no_number_moves_nothing() {
    let with_word = score_of(&piece("", &quarters_with_tempo(8, "tempo \"Meno mosso\";")));
    assert_eq!(onsets(&with_word), onsets(&score_of(&piece("", SIXTEEN_QUARTERS))));
}

/// A hairpin is a line between two loudnesses. It leaves whatever was in
/// force and arrives exactly at the mark it names — the endpoints are the
/// part a reader can check, and the notes between are evenly spaced.
#[test]
fn a_hairpin_leaves_the_prevailing_mark_and_arrives_at_its_own() {
    let source = "piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major;
        performance { profile album {
            dynamic p { amplitude = 0.2; }
            dynamic f { amplitude = 1; }
        } }
        score { part p { profile album; voice v {
            dynamic p;
            crescendo to f { c5/4 c5/4 c5/4 c5/4 c5/4 }
        } } } }";
    let amplitudes = amplitudes(&score_of(source));
    // Five notes, four steps of 0.2: p, and then evenly up to f.
    assert_eq!(amplitudes, [0.2, 0.4, 0.6, 0.8, 1.0]);
}

/// A diminuendo is the same line in the other direction, and the mark it
/// arrives at stays in force for the notes after it: a hairpin is a way of
/// getting somewhere, not a detour.
#[test]
fn a_hairpin_leaves_its_mark_in_force_after_it() {
    let source = "piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major;
        performance { profile album {
            dynamic p { amplitude = 0.2; }
            dynamic f { amplitude = 1; }
        } }
        score { part p { profile album; voice v {
            dynamic f;
            diminuendo to p { c5/4 c5/4 c5/4 }
            c5/4
        } } } }";
    assert_eq!(amplitudes(&score_of(source)), [1.0, 0.6, 0.2, 0.2]);
}

/// A hairpin over notes a profile says nothing about changes nothing: with
/// no profile, everything is neutral, and prompt 28's guarantee — a piece
/// that declares none sounds exactly as it did before profiles existed —
/// survives the arrival of hairpins.
#[test]
fn a_hairpin_without_a_profile_is_neutral() {
    let source = "piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major;
        score { part p { voice v {
            crescendo to f { c5/4 c5/4 c5/4 c5/4 }
        } } } }";
    assert_eq!(amplitudes(&score_of(source)), [1.0, 1.0, 1.0, 1.0]);
}

/// A hairpin changes no note's place or length: it is interpretation, and
/// interpretation does not move the score (roadmap §2).
#[test]
fn a_hairpin_moves_no_note() {
    let plain = "piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major;
        score { part p { voice v { c5/4 c5/4 c5/4 c5/4 } } } }";
    let with_hairpin = "piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major;
        score { part p { voice v { crescendo to f { c5/4 c5/4 c5/4 c5/4 } } } } }";
    let times = |score: &ScoreSnapshot| {
        score
            .parts()
            .iter()
            .flat_map(|(_, part)| part.voices().map(|(_, voice)| voice))
            .flat_map(|voice| voice.events().iter())
            .map(|event| (event.onset, event.notated_duration.value))
            .collect::<Vec<(MusicalTime, _)>>()
    };
    assert_eq!(times(&score_of(plain)), times(&score_of(with_hairpin)));
}
