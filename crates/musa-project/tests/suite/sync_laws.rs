//! One clock authority, from the session's side.
//!
//! Prompt 213 proved that what a workstation receives live is what it reads
//! from the file. This is the other half of the boundary: which side owns the
//! transport, what the clock Musa sends actually states, and what it refuses
//! to state rather than approximate (`06-daw-boundary.md` §4).

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::arithmetic_side_effects)]

use std::path::PathBuf;

use musa_project::{
    ClockAuthority, LiveMidiOptions, MidiMode, MidiOutputMode, MidiOutputTarget, ProjectSession, SyncOptions,
    SyncProtocol, SyncSource,
};

/// A quarter at 120, four of them, and one voice to play them.
const STEADY: &str = r#"piece "Clock" {
    tempo 1/4 = 120;
    meter 4/4;
    key c major;

    score {
        part one {
            clef treble;
            voice upper { | c4/4 d4/4 e4/4 f4/4 }
        }
    }
}
"#;

fn example(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn options(sync: SyncOptions) -> LiveMidiOptions {
    LiveMidiOptions {
        mode: MidiMode::Performance,
        sources: MidiOutputMode::SourcePerPart,
        target: MidiOutputTarget::VirtualSources,
        sync,
    }
}

/// Where each clock pulse falls in one planned run, in microseconds.
fn pulses(session: &ProjectSession, options: &LiveMidiOptions) -> Vec<u64> {
    let (projection, _) = session
        .plan_midi_output(options)
        .unwrap_or_else(|error| panic!("planning: {error}"));
    projection
        .packets()
        .iter()
        .filter(|packet| packet.bytes[0] == 0xF8)
        .map(|packet| packet.micros)
        .collect()
}

#[test]
fn a_session_that_does_not_synchronize_publishes_exactly_its_parts() {
    let session = ProjectSession::from_text(STEADY.to_owned(), "clock.musa");
    let (projection, report) = session
        .plan_midi_output(&options(SyncOptions::default()))
        .expect("planning");
    assert_eq!(report.authority(), None);
    assert_eq!(report.ports().len(), report.parts().len());
    assert!(
        projection.packets().iter().all(|packet| packet.len == 3),
        "a run with no clock carries only channel messages"
    );
}

#[test]
fn a_leading_session_publishes_one_further_port_for_the_clock() {
    let session = ProjectSession::from_text(STEADY.to_owned(), "clock.musa");
    let (_, report) = session
        .plan_midi_output(&options(SyncOptions::leading()))
        .expect("planning");
    assert_eq!(report.authority(), Some(ClockAuthority::Musa));
    assert_eq!(report.protocol(), Some(SyncProtocol::MidiClock));
    assert_eq!(report.ports().len(), report.parts().len() + 1);
    let clock = report.ports().last().expect("the clock port");
    assert!(clock.name.ends_with("· clock"), "named {}", clock.name);
    assert!(clock.parts.is_empty(), "the clock is not a part");
    // Every part still plays on the port prompt 213 gave it.
    assert!(report.parts().iter().all(|part| part.port < clock.port));
    assert!(report.limits().is_some_and(|limits| limits.contains("no meter")));
}

#[test]
fn the_clock_states_the_pieces_own_tempo_and_nothing_else() {
    let session = ProjectSession::from_text(STEADY.to_owned(), "clock.musa");
    let pulses = pulses(&session, &options(SyncOptions::leading()));
    // Four quarters at 120 is two seconds, and twenty-four pulses a quarter.
    assert_eq!(pulses.len(), 97);
    assert_eq!(pulses[0], 0);
    assert_eq!(pulses[24], 500_000);
    assert_eq!(pulses[48], 1_000_000);
    assert_eq!(pulses[96], 2_000_000);
    assert!(pulses.windows(2).all(|pair| pair[0] < pair[1]), "the clock rises");
}

#[test]
fn the_transport_stream_opens_with_a_position_and_closes_with_a_stop() {
    let session = ProjectSession::from_text(STEADY.to_owned(), "clock.musa");
    let (projection, _) = session
        .plan_midi_output(&options(SyncOptions::leading()))
        .expect("planning");
    let transport = projection
        .packets()
        .iter()
        .filter(|packet| packet.bytes[0] >= 0xF0)
        .map(|packet| (packet.micros, packet.bytes[0], packet.len))
        .collect::<Vec<_>>();
    assert_eq!(transport.first(), Some(&(0, 0xF2, 3)), "a song position first");
    assert_eq!(transport.get(1), Some(&(0, 0xFA, 1)), "then start");
    assert_eq!(transport.last(), Some(&(2_000_000, 0xFC, 1)), "and a stop at the end");
    // The whole schedule is one list in one order.
    assert!(
        projection
            .packets()
            .windows(2)
            .all(|pair| pair[0].micros <= pair[1].micros),
        "the run is sorted"
    );
}

#[test]
fn one_clock_lane_refuses_a_polytempo_piece_until_a_scope_is_named() {
    let session = ProjectSession::from_text(example("canon-x.musa"), "canon-x.musa");
    let refusal = session
        .plan_midi_output(&options(SyncOptions::leading()))
        .expect_err("a polytempo piece has no one tempo to state");
    assert!(refusal.to_string().contains("name the scope"), "{refusal}");
}

#[test]
fn a_named_reference_is_synchronized_and_the_others_are_reported() {
    let session = ProjectSession::from_text(example("canon-x.musa"), "canon-x.musa");
    let mut sync = SyncOptions::leading();
    sync.reference = Some("rising".to_owned());
    let (_, report) = session.plan_midi_output(&options(sync)).expect("planning");
    assert_eq!(report.reference(), Some("rising"));
    assert_eq!(report.unsynchronized(), ["falling".to_owned()]);
}

#[test]
fn a_reference_the_piece_does_not_have_is_named_rather_than_ignored() {
    let session = ProjectSession::from_text(example("canon-x.musa"), "canon-x.musa");
    let mut sync = SyncOptions::leading();
    sync.reference = Some("viola".to_owned());
    let refusal = session.plan_midi_output(&options(sync)).expect_err("no such part");
    assert!(refusal.to_string().contains("`viola`"), "{refusal}");
}

#[test]
fn a_session_leads_or_follows_and_never_both() {
    let mut session = ProjectSession::from_text(STEADY.to_owned(), "clock.musa");
    let both = SyncOptions {
        send: Some(SyncProtocol::MidiClock),
        follow: Some(SyncSource {
            id: "logic".to_owned(),
            protocol: SyncProtocol::MidiTimecode,
        }),
        reference: None,
    };
    let refusal = session
        .plan_midi_output(&options(both.clone()))
        .expect_err("two authorities");
    assert!(refusal.to_string().contains("one clock authority"), "{refusal}");
    let refusal = session.start_sync(&both).expect_err("two authorities");
    assert!(refusal.to_string().contains("one clock authority"), "{refusal}");
}

#[test]
fn following_needs_one_named_external_source() {
    let mut session = ProjectSession::from_text(STEADY.to_owned(), "clock.musa");
    let refusal = session
        .start_sync(&SyncOptions::default())
        .expect_err("nothing to follow");
    assert!(refusal.to_string().contains("external source"), "{refusal}");
    let refusal = session
        .start_sync(&SyncOptions::leading())
        .expect_err("leading is not following");
    assert!(refusal.to_string().contains("external source"), "{refusal}");
}

#[test]
fn a_source_that_is_not_there_is_a_follower_that_has_heard_nothing() {
    let mut session = ProjectSession::from_text(STEADY.to_owned(), "clock.musa");
    let status = session
        .start_sync(&SyncOptions::following(SyncSource {
            id: "musa-test-clock-that-does-not-exist".to_owned(),
            protocol: SyncProtocol::MidiClock,
        }))
        .expect("an absent source is not an error");
    assert_eq!(status.lock, musa_project::SyncLock::Idle);
    assert!(!status.running);
    assert_eq!(session.sync_tick(), 0);
    assert_eq!(session.sync_losses().map(|losses| losses.queue_overflow), Some(0));
    session.stop_sync();
    assert!(session.sync_status().is_none());
}

#[test]
fn the_authority_cannot_change_while_a_run_is_in_progress() {
    let mut session = ProjectSession::from_text(STEADY.to_owned(), "clock.musa");
    session
        .start_sync(&SyncOptions::following(SyncSource {
            id: "musa-test-clock-that-does-not-exist".to_owned(),
            protocol: SyncProtocol::MidiTimecode,
        }))
        .expect("following");
    let refusal = session
        .start_midi_output(&options(SyncOptions::leading()))
        .expect_err("the external source already leads");
    assert!(refusal.to_string().contains("cannot change while a run"), "{refusal}");
    // Asking again for the authority already in force is not a change.
    session
        .start_sync(&SyncOptions::following(SyncSource {
            id: "musa-test-clock-that-does-not-exist".to_owned(),
            protocol: SyncProtocol::MidiClock,
        }))
        .expect("the same authority");
    session.stop_sync();
    // And once nothing leads, planning a leading run is allowed again.
    session
        .plan_midi_output(&options(SyncOptions::leading()))
        .expect("planning after the follower stopped");
}
