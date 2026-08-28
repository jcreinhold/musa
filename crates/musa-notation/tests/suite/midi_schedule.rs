//! Laws for the shared MIDI schedule.
//!
//! [`musa_notation::midi_schedule`] is the one place a MIDI decision is made:
//! which channel a part sounds on, what velocity a dynamic becomes, where a
//! note starts in exact seconds and in ticks, and what could not be carried.
//! A Standard MIDI File is one reading of that schedule and a live `CoreMIDI`
//! projection is another, so what these laws hold is the schedule itself —
//! everything downstream inherits whatever they prove
//! (`docs/rules/across-stages/06-daw-boundary.md` §2).

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// Fixtures index their own fixed-length results; a bad index is a broken
// fixture, and the panic names it immediately.
#![allow(clippy::indexing_slicing)]
// Tick and channel arithmetic over small integers.
#![allow(clippy::arithmetic_side_effects)]
// Building a fixture's source text one part at a time.
#![allow(clippy::format_collect)]
#![allow(clippy::format_push_string)]

use midly::{MidiMessage, Smf, TrackEventKind};
use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_notation::{MidiMode, MidiOptions, MidiSchedule, midi_schedule, write_schedule};

const PROFILE_FIXTURE: &str = include_str!("../../../../examples/profile-fixture.musa");

fn schedule_of(source: &str, mode: MidiMode) -> MidiSchedule {
    let score = compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default())
        .into_snapshot()
        .expect("compiles");
    let performance = musa_compiler::lower_gestures(&score).expect("checked source performance lowers");
    midi_schedule(
        &performance,
        &MidiOptions {
            mode,
            ..MidiOptions::default()
        },
    )
    .expect("schedules")
}

/// A piece with more sounding parts than there are melodic channels. The
/// channel rule has to say something about the sixteenth part, and what it
/// says has to be recorded rather than silently arranged.
fn crowded_piece(parts: usize) -> String {
    let mut source =
        String::from("piece \"Crowded\" {\n    tempo 1/4 = 60;\n    meter 4/4;\n    key c major;\n\n    score {\n");
    for index in 0..parts {
        source.push_str(&format!(
            "        part line{index} {{\n            clef treble;\n            voice one {{ | c4/1 }}\n        }}\n"
        ));
    }
    source.push_str("    }\n}\n");
    source
}

#[test]
fn the_schedule_places_every_note_the_written_file_places() {
    for mode in [MidiMode::Score, MidiMode::Performance] {
        let schedule = schedule_of(PROFILE_FIXTURE, mode);
        let written = write_schedule(&schedule).expect("writes");
        let smf = Smf::parse(written.bytes()).expect("parses");

        // What the file says, as (track, tick, key, velocity) note-ons.
        let mut from_file = Vec::new();
        for (index, track) in smf.tracks.iter().enumerate() {
            let mut tick = 0u64;
            for event in track {
                tick += u64::from(event.delta.as_int());
                let TrackEventKind::Midi { channel, message } = event.kind else {
                    continue;
                };
                if let MidiMessage::NoteOn { key, vel } = message {
                    from_file.push((index, tick, channel.as_int(), key.as_int(), vel.as_int()));
                }
            }
        }

        // What the schedule says, read the same way. Track zero is the
        // conductor, so a part's track is its index plus one.
        let mut from_schedule = Vec::new();
        for message in schedule.messages() {
            if message.is_note_on() {
                from_schedule.push((
                    message.part + 1,
                    message.tick,
                    message.channel(),
                    message.data[0],
                    message.data[1],
                ));
            }
        }

        from_file.sort_unstable();
        from_schedule.sort_unstable();
        assert_eq!(
            from_file, from_schedule,
            "the file and the schedule disagree in {mode:?}"
        );
        assert!(!from_file.is_empty(), "the fixture sounds nothing in {mode:?}");
    }
}

#[test]
fn every_part_holds_its_own_channel_and_percussion_is_left_alone() {
    let schedule = schedule_of(PROFILE_FIXTURE, MidiMode::Score);
    let channels = schedule.parts().iter().map(|part| part.channel).collect::<Vec<_>>();
    assert_eq!(channels, vec![0, 1]);
    for part in schedule.parts() {
        assert_ne!(part.channel, 9, "channel ten belongs to percussion");
    }
    for message in schedule.messages() {
        assert_eq!(message.channel(), schedule.parts()[message.part].channel);
    }
}

#[test]
fn the_schedule_rises_in_seconds_and_in_ticks_together() {
    for mode in [MidiMode::Score, MidiMode::Performance] {
        let schedule = schedule_of(PROFILE_FIXTURE, mode);
        for pair in schedule.messages().windows(2) {
            assert!(pair[0].seconds <= pair[1].seconds, "seconds fall back in {mode:?}");
            assert!(pair[0].tick <= pair[1].tick, "ticks fall back in {mode:?}");
        }
        for pair in schedule.meta().windows(2) {
            assert!(pair[0].seconds <= pair[1].seconds);
        }
    }
}

#[test]
fn a_note_on_names_the_source_event_and_a_release_does_not() {
    let schedule = schedule_of(PROFILE_FIXTURE, MidiMode::Performance);
    let mut named = 0usize;
    for message in schedule.messages() {
        if message.is_note_on() {
            assert!(message.origin.is_some(), "a sounded note came from nowhere");
            named += 1;
        } else {
            assert!(message.origin.is_none(), "a release claims an origin of its own");
        }
    }
    assert_eq!(named, 11, "the fixture sounds eleven notes, the tied pair as one");
}

#[test]
fn the_extent_covers_every_message_and_the_last_one_ends_it() {
    let schedule = schedule_of(PROFILE_FIXTURE, MidiMode::Score);
    let last = schedule.messages().last().expect("the fixture sounds");
    assert_eq!(schedule.extent(), last.seconds);
    for message in schedule.messages() {
        assert!(message.seconds <= schedule.extent());
    }
}

#[test]
fn a_sixteenth_melodic_part_shares_a_channel_and_the_sharing_is_recorded() {
    let schedule = schedule_of(&crowded_piece(17), MidiMode::Score);
    assert_eq!(schedule.parts().len(), 17);
    let channels = schedule.parts().iter().map(|part| part.channel).collect::<Vec<_>>();
    // Fifteen melodic channels, with channel ten skipped, then sharing.
    assert_eq!(channels[0..9], [0, 1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(channels[9], 10);
    assert!(!channels.contains(&9), "percussion was stolen");
    assert_eq!(channels[15], channels[0], "the sixteenth part invented a channel");

    let losses = schedule
        .losses()
        .iter()
        .filter(|loss| loss.kind() == "channel")
        .collect::<Vec<_>>();
    assert_eq!(losses.len(), 1, "the sharing is stated once, for the whole piece");
    assert!(
        losses[0].message().contains("line15"),
        "the loss names the first part to share"
    );
}

#[test]
fn a_piece_with_no_channel_pressure_records_no_channel_loss() {
    let schedule = schedule_of(PROFILE_FIXTURE, MidiMode::Score);
    assert!(schedule.losses().iter().all(|loss| loss.kind() != "channel"));
}

#[test]
fn the_two_modes_read_the_same_notes_and_time_them_differently() {
    let score = schedule_of(PROFILE_FIXTURE, MidiMode::Score);
    let performance = schedule_of(PROFILE_FIXTURE, MidiMode::Performance);

    let keys = |schedule: &MidiSchedule| {
        let mut keys = schedule
            .messages()
            .iter()
            .filter(|message| message.is_note_on())
            .map(|message| (message.part, message.data[0]))
            .collect::<Vec<_>>();
        keys.sort_unstable();
        keys
    };
    assert_eq!(keys(&score), keys(&performance), "the notes are not the same notes");

    let velocities = |schedule: &MidiSchedule| {
        schedule
            .messages()
            .iter()
            .filter(|message| message.is_note_on())
            .map(|message| message.data[1])
            .collect::<Vec<_>>()
    };
    assert_ne!(
        velocities(&score),
        velocities(&performance),
        "the profiles changed nothing"
    );
    let releases = |schedule: &MidiSchedule| {
        schedule
            .messages()
            .iter()
            .filter(|message| !message.is_note_on())
            .map(|message| message.seconds)
            .collect::<Vec<_>>()
    };
    assert_ne!(
        releases(&score),
        releases(&performance),
        "the staccato gates shortened nothing"
    );
}

#[test]
fn the_schedule_is_the_same_schedule_twice() {
    let first = schedule_of(PROFILE_FIXTURE, MidiMode::Performance);
    let second = schedule_of(PROFILE_FIXTURE, MidiMode::Performance);
    assert_eq!(first.messages(), second.messages());
    assert_eq!(first.parts(), second.parts());
    assert_eq!(first.meta(), second.meta());
}

#[test]
fn the_conductor_states_the_tempo_before_anything_sounds() {
    let schedule = schedule_of(PROFILE_FIXTURE, MidiMode::Score);
    let opening = schedule
        .meta()
        .iter()
        .take_while(|meta| meta.tick == 0)
        .map(|meta| &meta.what)
        .collect::<Vec<_>>();
    assert!(matches!(opening[0], musa_notation::MidiMeta::Meter(_)));
    assert!(matches!(opening[1], musa_notation::MidiMeta::Key(_)));
    assert!(matches!(opening[2], musa_notation::MidiMeta::Tempo(_)));
    // Nothing sounds before the conductor has said all three.
    let first_sound = schedule.messages()[0].tick;
    assert!(schedule.meta().iter().all(|meta| meta.tick <= first_sound));
}
