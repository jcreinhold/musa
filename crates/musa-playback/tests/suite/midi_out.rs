//! Laws for live MIDI output, with no device and no thread in them.
//!
//! Every one drives the same sender the macOS backend drives, against a fake
//! clock the test advances by hand and a cable that keeps what it was given.
//! What they hold is the send loop's whole contract: order, timing, the one
//! late policy, the bound on a window, and the panic sequence.

#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::arithmetic_side_effects)]

use musa_playback::testing::MidiOutputHarness;
use musa_playback::{LiveMidiPacket, LiveMidiPart, MidiOutputConfig, MidiOutputMode, MidiOutputTarget};

const WINDOW: u64 = 4_000;
const TOLERANCE: u64 = 20_000;

fn parts() -> Vec<LiveMidiPart> {
    vec![
        LiveMidiPart {
            name: "flute".to_owned(),
            channel: 0,
        },
        LiveMidiPart {
            name: "cello".to_owned(),
            channel: 1,
        },
    ]
}

/// Two parts, one note each, a quarter of a second apart.
fn phrase() -> Vec<LiveMidiPacket> {
    vec![
        LiveMidiPacket {
            micros: 0,
            part: 0,
            bytes: [0x90, 72, 80],
        },
        LiveMidiPacket {
            micros: 0,
            part: 1,
            bytes: [0x91, 48, 80],
        },
        LiveMidiPacket {
            micros: 250_000,
            part: 0,
            bytes: [0x80, 72, 0],
        },
        LiveMidiPacket {
            micros: 250_000,
            part: 1,
            bytes: [0x81, 48, 0],
        },
    ]
}

fn config(mode: MidiOutputMode) -> MidiOutputConfig {
    MidiOutputConfig {
        mode,
        target: MidiOutputTarget::VirtualSources,
        client: "Musa".to_owned(),
    }
}

fn harness(mode: MidiOutputMode, packets: Vec<LiveMidiPacket>) -> MidiOutputHarness {
    MidiOutputHarness::new(&config(mode), &parts(), packets, WINDOW, TOLERANCE)
}

#[test]
fn a_source_per_part_publishes_one_named_port_for_each() {
    let harness = harness(MidiOutputMode::SourcePerPart, phrase());
    let ports = harness.ports();
    assert_eq!(ports.len(), 2);
    assert_eq!(ports[0].name, "Musa · flute");
    assert_eq!(ports[0].parts, vec!["flute".to_owned()]);
    assert_eq!(ports[1].name, "Musa · cello");
}

#[test]
fn one_channelized_source_carries_every_part() {
    let harness = harness(MidiOutputMode::SingleChannelized, phrase());
    let ports = harness.ports();
    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0].name, "Musa");
    assert_eq!(ports[0].parts, vec!["flute".to_owned(), "cello".to_owned()]);
}

#[test]
fn every_message_reaches_a_port_once_in_the_order_it_was_scheduled() {
    let mut harness = harness(MidiOutputMode::SourcePerPart, phrase());
    harness.run(WINDOW);
    let sent = harness
        .received()
        .iter()
        .map(|&(_, _, bytes)| bytes)
        .collect::<Vec<_>>();
    assert_eq!(sent, vec![[0x90, 72, 80], [0x91, 48, 80], [0x80, 72, 0], [0x81, 48, 0]]);
    assert_eq!(harness.counters().sent, 4);
    assert_eq!(harness.counters().dropped, 0);
    assert_eq!(harness.counters().late, 0);
}

#[test]
fn a_source_per_part_run_puts_each_part_on_its_own_port() {
    let mut harness = harness(MidiOutputMode::SourcePerPart, phrase());
    harness.run(WINDOW);
    for &(port, _, bytes) in harness.received() {
        // The flute is port zero on channel zero; the cello is port one on
        // channel one. Nothing crosses.
        assert_eq!(usize::from(bytes[0] & 0x0F), port);
    }
}

#[test]
fn a_channelized_run_puts_every_part_on_the_one_port() {
    let mut harness = harness(MidiOutputMode::SingleChannelized, phrase());
    harness.run(WINDOW);
    assert_eq!(harness.received().len(), 4);
    assert!(harness.received().iter().all(|&(port, _, _)| port == 0));
    let channels = harness
        .received()
        .iter()
        .map(|&(_, _, bytes)| bytes[0] & 0x0F)
        .collect::<Vec<_>>();
    assert_eq!(channels, vec![0, 1, 0, 1]);
}

#[test]
fn nothing_is_handed_over_before_its_window() {
    let mut harness = harness(MidiOutputMode::SourcePerPart, phrase());
    harness.pump(0);
    // Only the two attacks are inside the first window.
    assert_eq!(harness.received().len(), 2);
    harness.pump(WINDOW);
    assert_eq!(
        harness.received().len(),
        2,
        "a release a quarter second away went early"
    );
    harness.pump(250_000);
    assert_eq!(harness.received().len(), 4);
}

#[test]
fn a_message_is_stamped_at_its_moment_and_never_before_now() {
    let mut harness = harness(MidiOutputMode::SourcePerPart, phrase());
    harness.pump(0);
    assert!(harness.received().iter().all(|&(_, at, _)| at == 0));
    harness.pump(248_000);
    // Due at 250_000 and inside the 4 ms window, so it keeps its own moment.
    assert!(harness.received()[2..].iter().all(|&(_, at, _)| at == 250_000));
}

#[test]
fn a_note_long_past_is_not_attacked_and_its_release_still_goes() {
    let mut harness = harness(MidiOutputMode::SourcePerPart, phrase());
    // The first pump is the run's origin, so nothing is late yet; the second
    // arrives a whole second after the phrase should have ended.
    harness.pump(0);
    harness.pump(1_000_000);
    let sent = harness
        .received()
        .iter()
        .map(|&(_, _, bytes)| bytes)
        .collect::<Vec<_>>();
    assert_eq!(sent, vec![[0x90, 72, 80], [0x91, 48, 80], [0x80, 72, 0], [0x81, 48, 0]]);
    let counters = harness.counters();
    assert_eq!(counters.dropped, 0, "no attack was late in this run");
    assert_eq!(counters.late, 2, "both releases went out after their moment");
}

#[test]
fn an_attack_whose_moment_is_long_past_is_dropped_rather_than_played_wrong() {
    let late = vec![
        LiveMidiPacket {
            micros: 0,
            part: 0,
            bytes: [0x90, 60, 80],
        },
        LiveMidiPacket {
            micros: 10_000,
            part: 0,
            bytes: [0x90, 62, 80],
        },
        LiveMidiPacket {
            micros: 500_000,
            part: 0,
            bytes: [0x80, 60, 0],
        },
    ];
    let mut harness = harness(MidiOutputMode::SourcePerPart, late);
    harness.pump(0);
    // 600 ms later: the second attack is far outside tolerance, the release
    // is not.
    harness.pump(600_000);
    let sent = harness
        .received()
        .iter()
        .map(|&(_, _, bytes)| bytes)
        .collect::<Vec<_>>();
    assert_eq!(sent, vec![[0x90, 60, 80], [0x80, 60, 0]]);
    let counters = harness.counters();
    assert_eq!(counters.dropped, 1);
    assert_eq!(counters.late, 1);
    assert_eq!(counters.sent, 2);
}

#[test]
fn a_window_fuller_than_the_batch_leaves_the_rest_for_the_next_one() {
    // Five hundred simultaneous attacks: more than one batch, and none of
    // them may be invented, reordered, or lost.
    let crowd = (0..500)
        .map(|index| LiveMidiPacket {
            micros: 0,
            part: 0,
            bytes: [0x90, (index % 128) as u8, 80],
        })
        .collect::<Vec<_>>();
    let mut harness = harness(MidiOutputMode::SourcePerPart, crowd);
    harness.pump(0);
    let after_one = harness.received().len();
    assert!(after_one < 500, "the whole crowd went out in one window");
    assert_eq!(after_one, 256, "the window is bounded at one batch");
    harness.pump(1_000);
    harness.pump(2_000);
    assert_eq!(harness.received().len(), 500);
    assert_eq!(harness.counters().dropped, 0);
    assert_eq!(harness.counters().sent, 500);
}

#[test]
fn stopping_releases_every_note_the_run_left_sounding() {
    let mut harness = harness(MidiOutputMode::SourcePerPart, phrase());
    harness.pump(0);
    let attacks = harness.received().len();
    harness.panic_at(100_000);
    let after = &harness.received()[attacks..];
    let flute = after
        .iter()
        .filter(|&&(port, _, _)| port == 0)
        .map(|&(_, _, bytes)| bytes)
        .collect::<Vec<_>>();
    assert_eq!(
        flute,
        vec![[0x80, 72, 0], [0xB0, 120, 0], [0xB0, 64, 0], [0xB0, 123, 0]],
        "the note is released first, then the channel is quieted"
    );
    let cello = after
        .iter()
        .filter(|&&(port, _, _)| port == 1)
        .map(|&(_, _, bytes)| bytes)
        .collect::<Vec<_>>();
    assert_eq!(
        cello,
        vec![[0x81, 48, 0], [0xB1, 120, 0], [0xB1, 64, 0], [0xB1, 123, 0]]
    );
}

#[test]
fn a_panic_after_a_finished_run_releases_nothing_and_still_quiets_the_channels() {
    let mut harness = harness(MidiOutputMode::SourcePerPart, phrase());
    harness.run(WINDOW);
    let played = harness.received().len();
    harness.panic_at(1_000_000);
    let after = &harness.received()[played..];
    assert!(
        after.iter().all(|&(_, _, bytes)| bytes[0] & 0xF0 == 0xB0),
        "a note that already ended was released twice"
    );
    assert_eq!(after.len(), 6, "three controllers on each of two channels");
}

#[test]
fn a_port_that_refuses_is_counted_and_the_other_keeps_playing() {
    let mut harness = harness(MidiOutputMode::SourcePerPart, phrase());
    harness.break_port(0);
    harness.run(WINDOW);
    assert!(
        harness.received().iter().all(|&(port, _, _)| port == 1),
        "the broken port delivered something"
    );
    let counters = harness.counters();
    assert_eq!(counters.refused, 2, "both flute messages were refused");
    assert_eq!(counters.sent, 2, "both cello messages went out");
}

#[test]
fn a_packet_naming_a_part_the_run_never_routed_is_counted_not_guessed_at() {
    let stray = vec![LiveMidiPacket {
        micros: 0,
        part: 7,
        bytes: [0x90, 60, 80],
    }];
    let mut harness = harness(MidiOutputMode::SourcePerPart, stray);
    harness.run(WINDOW);
    assert!(harness.received().is_empty());
    assert_eq!(harness.counters().dropped, 1);
}
