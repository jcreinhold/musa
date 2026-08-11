//! MIDI backend tests.
//!
//! The suite reads its own output back with `midly` rather than snapshotting
//! bytes: bytes prove the encoder is stable, but only a parse proves the file
//! says what we meant (roadmap §17.4). What is asserted is the event list —
//! keys, velocities, and tick spans — which is the contract a sampler reads.
//!
//! The two modes are the interesting axis. Score MIDI is the neutral reading
//! and must be identical whether or not the piece declares profiles;
//! performance MIDI is the interpreted one and must differ exactly where the
//! profile says it should.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// Fixtures index their own fixed-length results; a bad index is a broken
// fixture, and the panic names it immediately.
#![allow(clippy::indexing_slicing)]
// Tick arithmetic over small integers.
#![allow(clippy::arithmetic_side_effects)]

use midly::{MidiMessage, Smf, Timing, TrackEventKind};
use musa_compiler::{CompileOptions, PerformanceOptions, SourceDocument, compile, lower_performance};
use musa_render::{MidiMode, MidiOptions, render_midi};

const PROFILE_FIXTURE: &str = include_str!("../../../examples/profile-fixture.musa");
const TUPLET_FIXTURE: &str = include_str!("../../../examples/tuplet-fixture.musa");

fn midi_of(source: &str, mode: MidiMode) -> Vec<u8> {
    let score = compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles");
    let performance = lower_performance(&score, &PerformanceOptions::default()).expect("lowers");
    render_midi(
        &performance,
        &MidiOptions {
            mode,
            ..MidiOptions::default()
        },
    )
    .expect("renders")
}

/// One sounded note as the file states it: `(track, key, velocity, ticks)`.
#[derive(Debug, PartialEq, Eq)]
struct Sounded {
    track: usize,
    key: u8,
    velocity: u8,
    start: u32,
    length: u32,
}

/// Parse a rendered file back into its sounded notes.
fn read_back(bytes: &[u8]) -> Vec<Sounded> {
    let smf = Smf::parse(bytes).expect("parses");
    let mut sounded = Vec::new();
    for (index, track) in smf.tracks.iter().enumerate() {
        let mut tick = 0u32;
        let mut open: Vec<(u8, u8, u32)> = Vec::new();
        for event in track {
            tick += event.delta.as_int();
            let TrackEventKind::Midi { message, .. } = event.kind else {
                continue;
            };
            if let MidiMessage::NoteOn { key, vel } = message {
                open.push((key.as_int(), vel.as_int(), tick));
            } else if let MidiMessage::NoteOff { key, .. } = message {
                let key = key.as_int();
                if let Some(position) = open.iter().position(|(open_key, _, _)| *open_key == key) {
                    let (key, velocity, start) = open.remove(position);
                    sounded.push(Sounded {
                        track: index,
                        key,
                        velocity,
                        start,
                        length: tick - start,
                    });
                }
            }
        }
        assert!(open.is_empty(), "track {index} left a note hanging");
    }
    sounded.sort_by_key(|note| (note.track, note.start, note.key));
    sounded
}

#[test]
fn the_file_is_metrical_with_one_track_per_part_plus_tempo() {
    let bytes = midi_of(PROFILE_FIXTURE, MidiMode::Score);
    let smf = Smf::parse(&bytes).expect("parses");
    assert!(matches!(smf.header.timing, Timing::Metrical(_)));
    assert_eq!(smf.tracks.len(), 3, "a tempo track plus the flute and the cello");
}

#[test]
fn written_pitch_reaches_midi_through_twelve_tet() {
    let source = "piece \"x\" { tempo 1/4 = 60; meter 4/4; score { part p { voice v { c4/4 a4/4 \
                  c#4/4 db4/4 } } } }";
    let keys: Vec<u8> = read_back(&midi_of(source, MidiMode::Score))
        .iter()
        .map(|note| note.key)
        .collect();
    // Middle C is 60 and A4 is 69; C♯ and D♭ are one number and two spellings
    // — which is exactly why the score keeps the spelling and MIDI does not.
    assert_eq!(keys, vec![60, 69, 61, 61]);
}

#[test]
fn score_mode_states_written_values_and_one_velocity() {
    let notes = read_back(&midi_of(PROFILE_FIXTURE, MidiMode::Score));
    let velocities: Vec<u8> = notes.iter().map(|note| note.velocity).collect();
    let first = velocities.first().copied().expect("the fixture sounds");
    assert!(
        velocities.iter().all(|velocity| *velocity == first),
        "score MIDI has no opinion about loudness: {velocities:?}"
    );
    // Every note lasts its written value: the fixture's opening quarters are
    // a full quarter, gates notwithstanding.
    let flute = notes.iter().filter(|note| note.track == 1).collect::<Vec<_>>();
    assert_eq!(flute.first().map(|note| note.length), Some(480));
}

#[test]
fn score_mode_ignores_the_profiles_entirely() {
    // The same score with its `performance` block and part bindings removed
    // must produce the same score MIDI, byte for byte.
    let stripped = PROFILE_FIXTURE
        .lines()
        .filter(|line| !line.trim_start().starts_with("profile winds;"))
        .filter(|line| !line.trim_start().starts_with("profile strings;"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        midi_of(&stripped, MidiMode::Score),
        midi_of(PROFILE_FIXTURE, MidiMode::Score),
        "score MIDI is the neutral reading"
    );
}

#[test]
fn performance_mode_shortens_by_gate_and_scales_velocity_by_dynamic() {
    let notes = read_back(&midi_of(PROFILE_FIXTURE, MidiMode::Performance));
    let flute = notes.iter().find(|note| note.track == 1).expect("flute sounds");
    let cello = notes.iter().find(|note| note.track == 2).expect("cello sounds");
    // Both wrote a staccato quarter (480 ticks). `winds` gates it to 0.7,
    // `strings` to 0.5.
    assert_eq!((flute.length, cello.length), (336, 240));
    // `p` is 0.45 for the winds, `mf` is 0.6 for the strings.
    assert_eq!((flute.velocity, cello.velocity), (57, 76));
}

#[test]
fn an_unprofiled_piece_is_timed_the_same_in_both_modes() {
    let score = read_back(&midi_of(TUPLET_FIXTURE, MidiMode::Score));
    let performance = read_back(&midi_of(TUPLET_FIXTURE, MidiMode::Performance));
    let spans = |notes: &[Sounded]| {
        notes
            .iter()
            .map(|note| (note.track, note.key, note.start, note.length))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        spans(&score),
        spans(&performance),
        "with no profile there is no gate, so nothing is shortened"
    );
    // The velocities are the one place the two documents still differ, and
    // they differ by design rather than by interpretation: score MIDI states
    // its neutral velocity, performance MIDI states the amplitude it was
    // given, which with no dynamic realized is full. Neither is a reading of
    // a mark — the assertion is that *nothing in the source* moved them.
    assert!(score.iter().all(|note| note.velocity == 80));
    assert!(performance.iter().all(|note| note.velocity == 127));
}

#[test]
fn ties_sound_as_one_note() {
    // The tuplet fixture ties a quarter to a quarter across the barline; MIDI
    // has no tie, so it must arrive as one note of the summed length.
    let notes = read_back(&midi_of(TUPLET_FIXTURE, MidiMode::Score));
    assert!(
        notes.iter().any(|note| note.length == 960),
        "expected a half-note-long sounded event from the tie: {notes:?}"
    );
}

#[test]
fn output_is_deterministic() {
    for mode in [MidiMode::Score, MidiMode::Performance] {
        assert_eq!(midi_of(PROFILE_FIXTURE, mode), midi_of(PROFILE_FIXTURE, mode));
    }
}

/// A MIDI file is a performance, so it swings; a score-mode file is a
/// notation program's input, so it does not.
///
/// Both facts come out of the same plan, which is why `PerformedNote` carries
/// the written on-frame beside the scheduled one — the same shape
/// `notated_off` already had, in the other direction.
#[test]
fn a_performance_swings_and_a_score_does_not() {
    let swung = "piece \"Feel\" { tempo 1/4 = 60; meter 4/4; key c major;
        performance { profile band { groove swing { ratio = 2/3; } } }
        score { part p { profile band; voice v {
            c5/8 d5/8 c5/8 d5/8
        } } } }";
    let starts = |mode| {
        let mut starts: Vec<u32> = read_back(&midi_of(swung, mode))
            .into_iter()
            .map(|note| note.start)
            .collect();
        starts.sort_unstable();
        starts
    };
    let written = starts(MidiMode::Score);
    let played = starts(MidiMode::Performance);
    assert_eq!(written.len(), 4);
    // On the page the eighths are even, whatever the band does with them.
    let step = written[1];
    assert_eq!(written, vec![0, step, step * 2, step * 3]);
    // Played, the offbeats are late and the beats are exactly where they were.
    assert_eq!(played[0], written[0]);
    assert_eq!(played[2], written[2]);
    assert!(played[1] > written[1], "the offbeat did not swing: {played:?}");
    assert!(played[3] > written[3], "the offbeat did not swing: {played:?}");
}
