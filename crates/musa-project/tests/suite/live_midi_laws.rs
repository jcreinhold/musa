//! Live MIDI output and the Standard MIDI file, held to each other.
//!
//! The claim prompt 213 makes is that a workstation receiving Musa live and a
//! workstation reading `performance.mid` receive the same performance. That
//! is only a claim if nothing checks it, so what these laws do is read both
//! and compare them: the same parts, the same bytes, in the same order, at
//! the same moments — differing only in the container each is carried in and
//! in how each spells a moment (`06-daw-boundary.md` §2).

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::arithmetic_side_effects)]

use std::path::PathBuf;

use midly::{Smf, Timing, TrackEventKind};
use musa_project::{
    ExportRequest, LiveMidiOptions, MidiMode, MidiOutputMode, MidiOutputTarget, ProjectSession, SyncOptions,
};

const PROFILED: &str = "profile-fixture.musa";

fn example(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn session(name: &str) -> ProjectSession {
    ProjectSession::from_text(example(name), name)
}

fn options(mode: MidiMode) -> LiveMidiOptions {
    LiveMidiOptions {
        mode,
        sources: MidiOutputMode::SourcePerPart,
        target: MidiOutputTarget::VirtualSources,
        sync: SyncOptions::default(),
    }
}

/// One channel message as the file states it: absolute tick and three bytes,
/// grouped by the track it was written on.
fn from_file(session: &ProjectSession, mode: MidiMode) -> (u32, Vec<Vec<(u64, [u8; 3])>>) {
    let artifact = session
        .export(ExportRequest::Midi(mode))
        .expect("the fixture exports MIDI");
    let smf = Smf::parse(artifact.as_bytes()).expect("the export parses");
    let Timing::Metrical(ticks_per_quarter) = smf.header.timing else {
        panic!("the file is not metrical");
    };
    // The one tempo the conductor track states, in microseconds a quarter.
    let mut micros_per_quarter = 500_000u32;
    for event in &smf.tracks[0] {
        if let TrackEventKind::Meta(midly::MetaMessage::Tempo(tempo)) = event.kind {
            micros_per_quarter = tempo.as_int();
            break;
        }
    }
    let mut parts = Vec::new();
    for track in smf.tracks.iter().skip(1) {
        let mut tick = 0u64;
        let mut messages = Vec::new();
        for event in track {
            tick += u64::from(event.delta.as_int());
            let TrackEventKind::Midi { channel, message } = event.kind else {
                continue;
            };
            let bytes = match message {
                midly::MidiMessage::NoteOn { key, vel } => [0x90 | channel.as_int(), key.as_int(), vel.as_int()],
                midly::MidiMessage::NoteOff { key, vel } => [0x80 | channel.as_int(), key.as_int(), vel.as_int()],
                midly::MidiMessage::Aftertouch { .. }
                | midly::MidiMessage::Controller { .. }
                | midly::MidiMessage::ProgramChange { .. }
                | midly::MidiMessage::ChannelAftertouch { .. }
                | midly::MidiMessage::PitchBend { .. } => continue,
            };
            messages.push((tick, bytes));
        }
        parts.push(messages);
    }
    let micros_per_tick_numerator = u64::from(micros_per_quarter);
    for messages in &mut parts {
        for (tick, _) in messages.iter_mut() {
            let scaled = *tick * micros_per_tick_numerator;
            let denominator = u64::from(ticks_per_quarter.as_int());
            *tick = (scaled + denominator / 2) / denominator;
        }
    }
    (u32::from(ticks_per_quarter.as_int()), parts)
}

/// The same, as the live run states it.
fn from_live(session: &ProjectSession, mode: MidiMode) -> Vec<Vec<(u64, [u8; 3])>> {
    let (projection, _) = session.plan_midi_output(&options(mode)).expect("the fixture projects");
    let mut parts = vec![Vec::new(); projection.parts().len()];
    for packet in projection.packets() {
        parts[packet.part].push((packet.micros, packet.bytes));
    }
    parts
}

#[test]
fn the_live_run_and_the_written_file_carry_the_same_performance() {
    for mode in [MidiMode::Score, MidiMode::Performance] {
        let session = session(PROFILED);
        let (_, written) = from_file(&session, mode);
        let live = from_live(&session, mode);
        assert_eq!(written.len(), live.len(), "a different number of parts in {mode:?}");
        for (index, (written, live)) in written.iter().zip(&live).enumerate() {
            assert_eq!(
                written, live,
                "part {index} differs between the file and the run in {mode:?}"
            );
        }
        assert!(live.iter().any(|part| !part.is_empty()), "nothing sounds in {mode:?}");
    }
}

#[test]
fn the_two_readings_agree_on_every_channel() {
    let session = session(PROFILED);
    let (projection, report) = session
        .plan_midi_output(&options(MidiMode::Performance))
        .expect("projects");
    for (index, part) in projection.parts().iter().enumerate() {
        assert_eq!(report.parts()[index].name, part.name);
        assert_eq!(report.parts()[index].channel, part.channel);
    }
    for packet in projection.packets() {
        assert_eq!(packet.bytes[0] & 0x0F, projection.parts()[packet.part].channel);
    }
}

#[test]
fn a_run_publishes_one_named_port_for_each_part_and_says_where_each_plays() {
    let session = session(PROFILED);
    let (_, report) = session
        .plan_midi_output(&options(MidiMode::Performance))
        .expect("projects");
    assert_eq!(report.ports().len(), report.parts().len());
    for (index, part) in report.parts().iter().enumerate() {
        assert_eq!(part.port, index);
        assert!(report.ports()[index].name.contains(&part.name));
        assert!(report.ports()[index].name.contains(PROFILED.trim_end_matches(".musa")));
    }
    assert_eq!(report.target(), "virtual sources");
    assert_eq!(report.mode(), MidiMode::Performance);
}

#[test]
fn one_channelized_source_carries_every_part_and_the_report_says_so() {
    let session = session(PROFILED);
    let (_, report) = session
        .plan_midi_output(&LiveMidiOptions {
            sources: MidiOutputMode::SingleChannelized,
            ..options(MidiMode::Performance)
        })
        .expect("projects");
    assert_eq!(report.ports().len(), 1);
    assert!(report.parts().iter().all(|part| part.port == 0));
    let channels = report.parts().iter().map(|part| part.channel).collect::<Vec<_>>();
    assert_eq!(channels, vec![0, 1], "the parts were merged onto one channel");
}

#[test]
fn the_live_run_loses_exactly_what_the_projection_loses_and_says_which() {
    let session = session(PROFILED);
    let (projection, report) = session
        .plan_midi_output(&options(MidiMode::Performance))
        .expect("projects");
    let kinds = projection.losses().iter().map(|loss| loss.kind()).collect::<Vec<_>>();
    assert!(kinds.contains(&"notation"), "the run claims MIDI carries spelling");
    assert!(kinds.contains(&"tuning"), "the run claims MIDI states a tuning");
    assert_eq!(
        report.losses().iter().map(|loss| loss.kind()).collect::<Vec<_>>(),
        kinds,
        "the report and the projection disagree about what was lost"
    );
    assert!(
        projection.losses().iter().all(|loss| !loss.message().is_empty()),
        "a loss was recorded without saying what it was"
    );
}

#[test]
fn the_report_states_how_exactly_this_platform_places_a_message() {
    let session = session(PROFILED);
    let (_, report) = session
        .plan_midi_output(&options(MidiMode::Performance))
        .expect("projects");
    let timing = report.timing();
    assert!(
        timing.contains("moment"),
        "the report says nothing about when a message lands: {timing}"
    );
}

#[test]
fn score_and_performance_are_two_different_runs() {
    let session = session(PROFILED);
    let (score, _) = session.plan_midi_output(&options(MidiMode::Score)).expect("projects");
    let (performance, _) = session
        .plan_midi_output(&options(MidiMode::Performance))
        .expect("projects");
    assert_ne!(
        score.packets(),
        performance.packets(),
        "the score and the performance are the same document"
    );
    let keys = |packets: &[musa_project::LiveMidiPacket]| {
        let mut keys = packets
            .iter()
            .filter(|packet| packet.bytes[0] & 0xF0 == 0x90)
            .map(|packet| (packet.part, packet.bytes[1]))
            .collect::<Vec<_>>();
        keys.sort_unstable();
        keys
    };
    assert_eq!(
        keys(score.packets()),
        keys(performance.packets()),
        "the two readings are not of the same notes"
    );
}

#[test]
fn a_piece_that_never_compiled_refuses_rather_than_sending_something_else() {
    let session = ProjectSession::from_text("piece \"broken\" { score {", "broken.musa");
    let refusal = session.plan_midi_output(&options(MidiMode::Performance));
    assert!(matches!(refusal, Err(musa_project::ProjectError::NoValidScore)));
}

#[test]
fn nothing_is_running_until_something_starts() {
    let mut session = session(PROFILED);
    assert!(!session.is_midi_output_running());
    assert_eq!(session.midi_output_counters(), None);
    // Stopping a run that never started is a no-op, not a panic.
    session.stop_midi_output();
    assert!(!session.is_midi_output_running());
}
