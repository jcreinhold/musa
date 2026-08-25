//! Tempo and expression curves (roadmap §6.3, §6.4).
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

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{MusicalTime, ScoreSnapshot};
use num_rational::Ratio;

use super::performance_support::notes_of;

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

/// Exact physical onset seconds of every gesture, in order.
fn onsets(score: &ScoreSnapshot) -> Vec<Ratio<i64>> {
    notes_of(score)
        .into_iter()
        .flatten()
        .map(|note| note.on_seconds)
        .collect()
}

/// The amplitude of every note-on, in order.
fn amplitudes(score: &ScoreSnapshot) -> Vec<Ratio<i64>> {
    notes_of(score)
        .into_iter()
        .flatten()
        .map(|note| note.amplitude)
        .collect()
}

/// Before the change, a quarter at 60 is a second; after it, a quarter at 120
/// is half of one. Every onset is exactly where those two rates put it —
/// which is the whole claim of an *integrated* tempo map.
#[test]
fn a_tempo_change_moves_every_note_after_it_and_none_before() {
    let score = score_of(&piece("", &quarters_with_tempo(8, "tempo 1/4 = 120;")));
    // Bars 1–2 at 60 bpm: one second each quarter. Bars 3–4 at 120: half.
    let expected: Vec<Ratio<i64>> = (0..8)
        .map(Ratio::from_integer)
        .chain((0..8).map(|index| Ratio::from_integer(8) + Ratio::new(index, 2)))
        .collect();
    assert_eq!(onsets(&score), expected);
}

/// The same piece without the change: the second half is where 60 bpm puts
/// it, so the test above is measuring the tempo and not the notes.
#[test]
fn a_piece_without_a_change_is_the_tempo_it_declares() {
    let score = score_of(&piece("", SIXTEEN_QUARTERS));
    let expected: Vec<Ratio<i64>> = (0..16).map(Ratio::from_integer).collect();
    assert_eq!(onsets(&score), expected);
}

/// The seam is exact. A tempo change on a beat that is not a whole number of
/// a sampled clock would drift if the map rounded each segment; the exact
/// gesture boundary accumulates in rationals instead.
#[test]
fn the_exact_time_at_a_tempo_change_is_the_sum_of_what_came_before() {
    // 7 bpm makes a quarter 60/7 seconds.
    let source = piece("", &quarters_with_tempo(4, "tempo 1/4 = 7;"));
    let score = score_of(&source);
    let times = onsets(&score);
    let at_change = times.get(4).copied().expect("a note at the change");
    assert_eq!(at_change, Ratio::from_integer(4), "bar 1 at 60 bpm is four seconds");
    let next = times.get(5).copied().expect("a note after the change");
    let expected = at_change + Ratio::new(60, 7);
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
    assert_eq!(
        amplitudes,
        [
            Ratio::new(1, 5),
            Ratio::new(2, 5),
            Ratio::new(3, 5),
            Ratio::new(4, 5),
            Ratio::ONE,
        ]
    );
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
    assert_eq!(
        amplitudes(&score_of(source)),
        [Ratio::ONE, Ratio::new(3, 5), Ratio::new(1, 5), Ratio::new(1, 5)]
    );
}

/// A hairpin over notes a profile says nothing about changes nothing: with
/// no profile, everything is neutral, and a piece that declares none sounds
/// exactly as it did before profiles existed — even with hairpins.
#[test]
fn a_hairpin_without_a_profile_is_neutral() {
    let source = "piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major;
        score { part p { voice v {
            crescendo to f { c5/4 c5/4 c5/4 c5/4 }
        } } } }";
    assert_eq!(amplitudes(&score_of(source)), [Ratio::ONE; 4]);
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
