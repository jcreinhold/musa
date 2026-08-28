//! One clock authority: the wire, the follower, and the leader's stream.
//!
//! Every law here runs against a virtual clock and a byte slice, because
//! every decision worth checking is made by a state machine and not by a
//! device. What a real cable adds is jitter, and jitter is measured by the
//! one ignored law at the bottom rather than asserted about here.

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::arithmetic_side_effects)]

use musa_playback::testing::SyncCallbackHarness;
use musa_playback::{
    ClockAuthority, LiveMidiPacket, PULSES_PER_QUARTER, SyncIntent, SyncLock, SyncMessage, SyncObservation,
    SyncOptions, SyncParser, SyncPosition, SyncProtocol, SyncRefusal, SyncSource, TransportFollower, transport_packets,
};
use num_rational::Ratio;

/// Every transport fact one byte stream carries, in order.
fn read(bytes: &[u8]) -> Vec<SyncMessage> {
    let mut parser = SyncParser::new();
    bytes.iter().filter_map(|&byte| parser.feed(byte)).collect()
}

/// The same stream, chopped into the callbacks a backend might deliver.
fn read_in_pieces(chunks: &[&[u8]]) -> Vec<SyncMessage> {
    let mut parser = SyncParser::new();
    let mut messages = Vec::new();
    for chunk in chunks {
        messages.extend(chunk.iter().filter_map(|&byte| parser.feed(byte)));
    }
    messages
}

fn at(micros: u64, message: SyncMessage) -> SyncObservation {
    SyncObservation {
        micros,
        device_micros: micros,
        message,
    }
}

/// One quarter note of clock, at one pulse every `interval` microseconds.
fn clock(follower: &mut TransportFollower, from: u64, interval: u64, pulses: u64) -> u64 {
    let mut now = from;
    for _ in 0..pulses {
        now += interval;
        follower.observe(at(now, SyncMessage::Clock));
    }
    now
}

/// A quarter of 120 is half a second, so twenty-four pulses of 20,833 µs.
const AT_120: u64 = 20_833;

#[test]
fn a_message_split_across_callbacks_reads_the_same_as_one_whole_one() {
    let whole = read(&[0xF2, 0x10, 0x00]);
    assert_eq!(whole, vec![SyncMessage::SongPosition(16)]);
    assert_eq!(read_in_pieces(&[&[0xF2], &[0x10], &[0x00]]), whole);
    assert_eq!(read_in_pieces(&[&[0xF2, 0x10], &[0x00]]), whole);
}

#[test]
fn a_clock_byte_inside_another_message_disturbs_nothing() {
    // 0xF8 lands between the status and the data of a note-on, and between
    // the two data bytes of a song position. Both messages still complete.
    assert_eq!(
        read(&[0x90, 0xF8, 0x3C, 0x40, 0xF2, 0x10, 0xF8, 0x00]),
        vec![SyncMessage::Clock, SyncMessage::Clock, SyncMessage::SongPosition(16)]
    );
}

#[test]
fn running_status_repeats_a_channel_message_and_never_a_transport_one() {
    // Three note-ons under one status byte, with clock interleaved, and then
    // a song position that must not be read as more of the run.
    let read = read(&[0x90, 0x3C, 0x40, 0xF8, 0x3E, 0x40, 0x40, 0x40, 0xF2, 0x08, 0x00]);
    assert_eq!(read, vec![SyncMessage::Clock, SyncMessage::SongPosition(8)]);
    // A system-common message clears the running status, so the bare data
    // bytes after it assemble nothing at all.
    assert_eq!(
        read_in_pieces(&[&[0xF2, 0x00, 0x00], &[0x3C, 0x40]]),
        vec![SyncMessage::SongPosition(0)]
    );
}

#[test]
fn a_stream_joined_mid_message_waits_rather_than_guessing() {
    // Two data bytes with no status before them are not a song position.
    assert_eq!(read(&[0x08, 0x00, 0xFA]), vec![SyncMessage::Start]);
}

#[test]
fn system_exclusive_is_skipped_and_the_clock_inside_it_still_arrives() {
    assert_eq!(
        read(&[0xF0, 0x7E, 0xF8, 0x01, 0x02, 0xF7, 0xFA]),
        vec![SyncMessage::Clock, SyncMessage::Start]
    );
}

#[test]
fn the_four_transport_bytes_are_the_four_transport_facts() {
    assert_eq!(
        read(&[0xFA, 0xFB, 0xFC, 0xF8, 0xFE, 0xFF]),
        vec![
            SyncMessage::Start,
            SyncMessage::Continue,
            SyncMessage::Stop,
            SyncMessage::Clock,
        ]
    );
}

#[test]
fn a_quarter_frame_carries_its_piece_and_its_nibble() {
    assert_eq!(
        read(&[0xF1, 0x35]),
        vec![SyncMessage::QuarterFrame { piece: 3, value: 5 }]
    );
}

#[test]
fn the_follower_estimates_the_leaders_tempo_and_locks_on_it() {
    let mut follower = TransportFollower::new(SyncProtocol::MidiClock);
    assert_eq!(follower.status().lock, SyncLock::Idle);
    assert_eq!(
        follower.observe(at(0, SyncMessage::Start)),
        Some(SyncIntent::Start(SyncPosition::Quarters(Ratio::ZERO)))
    );
    assert_eq!(follower.status().lock, SyncLock::Acquiring);
    clock(&mut follower, 0, AT_120, 2 * PULSES_PER_QUARTER);
    let status = follower.status();
    assert_eq!(status.lock, SyncLock::Locked);
    assert!(
        status.tempo_bpm.is_some_and(|bpm| (bpm - 120.0).abs() < 0.1),
        "estimated {:?}",
        status.tempo_bpm
    );
    assert_eq!(status.position, Some(SyncPosition::Quarters(Ratio::from_integer(2))));
}

#[test]
fn a_steady_clock_is_a_measurement_and_not_an_instruction() {
    let mut follower = TransportFollower::new(SyncProtocol::MidiClock);
    follower.observe(at(0, SyncMessage::Start));
    let mut now = 0;
    for _ in 0..4 * PULSES_PER_QUARTER {
        now += AT_120;
        assert_eq!(follower.observe(at(now, SyncMessage::Clock)), None);
    }
    assert!(follower.status().drift_micros.abs() < 1_000);
}

#[test]
fn jitter_is_measured_and_never_moves_the_position() {
    let mut follower = TransportFollower::new(SyncProtocol::MidiClock);
    follower.observe(at(0, SyncMessage::Start));
    let mut now = 0;
    for pulse in 0..2 * PULSES_PER_QUARTER {
        // A pulse that arrives 3 ms early and one 3 ms late, alternating.
        now += if pulse % 2 == 0 { AT_120 + 3_000 } else { AT_120 - 3_000 };
        follower.observe(at(now, SyncMessage::Clock));
    }
    let status = follower.status();
    assert!(status.jitter_micros >= 2_500, "jitter {}", status.jitter_micros);
    // Forty-eight pulses is two quarters however unevenly they arrived.
    assert_eq!(status.position, Some(SyncPosition::Quarters(Ratio::from_integer(2))));
}

#[test]
fn a_leader_that_stops_sending_is_lost_rather_than_free_running() {
    let mut follower = TransportFollower::new(SyncProtocol::MidiClock);
    follower.observe(at(0, SyncMessage::Start));
    let last = clock(&mut follower, 0, AT_120, PULSES_PER_QUARTER);
    assert_eq!(follower.idle_at(last + 100_000), None);
    assert_eq!(follower.idle_at(last + 900_000), Some(SyncIntent::Stop));
    let status = follower.status();
    assert_eq!(
        (status.lock, status.running, status.dropouts),
        (SyncLock::Lost, false, 1)
    );
}

#[test]
fn a_lost_leader_relocks_when_it_starts_again() {
    let mut follower = TransportFollower::new(SyncProtocol::MidiClock);
    follower.observe(at(0, SyncMessage::Start));
    let last = clock(&mut follower, 0, AT_120, PULSES_PER_QUARTER);
    follower.idle_at(last + 900_000);
    assert_eq!(follower.status().lock, SyncLock::Lost);
    let restart = last + 1_000_000;
    assert_eq!(
        follower.observe(at(restart, SyncMessage::Start)),
        Some(SyncIntent::Start(SyncPosition::Quarters(Ratio::ZERO)))
    );
    clock(&mut follower, restart, AT_120, 2 * PULSES_PER_QUARTER);
    let status = follower.status();
    assert_eq!(
        (status.lock, status.running, status.dropouts),
        (SyncLock::Locked, true, 1)
    );
    assert_eq!(status.position, Some(SyncPosition::Quarters(Ratio::from_integer(2))));
}

#[test]
fn a_song_position_seeks_to_the_sixteenth_it_names() {
    let mut follower = TransportFollower::new(SyncProtocol::MidiClock);
    // Sixteen MIDI beats is sixteen sixteenths, which is four quarters.
    assert_eq!(
        follower.observe(at(0, SyncMessage::SongPosition(16))),
        Some(SyncIntent::Seek(SyncPosition::Quarters(Ratio::from_integer(4))))
    );
    assert_eq!(
        follower.observe(at(1, SyncMessage::Continue)),
        Some(SyncIntent::Continue)
    );
    clock(&mut follower, 1, AT_120, PULSES_PER_QUARTER);
    // The pulses count on from where the pointer put us, not from zero.
    assert_eq!(
        follower.status().position,
        Some(SyncPosition::Quarters(Ratio::from_integer(5)))
    );
}

#[test]
fn stopping_leaves_the_position_and_takes_the_lock() {
    let mut follower = TransportFollower::new(SyncProtocol::MidiClock);
    follower.observe(at(0, SyncMessage::Start));
    let last = clock(&mut follower, 0, AT_120, PULSES_PER_QUARTER);
    assert_eq!(follower.observe(at(last, SyncMessage::Stop)), Some(SyncIntent::Stop));
    let status = follower.status();
    assert!(!status.running);
    assert_eq!(status.lock, SyncLock::Idle);
    assert_eq!(status.position, Some(SyncPosition::Quarters(Ratio::ONE)));
}

/// One MTC reading of `hh:mm:ss:ff` at the rate `rate` names, as eight
/// quarter-frame messages.
fn quarter_frames(hour: u8, minute: u8, second: u8, frame: u8, rate: u8) -> [SyncMessage; 8] {
    let nibbles = [
        frame & 0x0F,
        (frame >> 4) & 0x01,
        second & 0x0F,
        (second >> 4) & 0x03,
        minute & 0x0F,
        (minute >> 4) & 0x03,
        hour & 0x0F,
        ((rate & 0x03) << 1) | ((hour >> 4) & 0x01),
    ];
    std::array::from_fn(|piece| SyncMessage::QuarterFrame {
        piece: piece as u8,
        value: nibbles[piece],
    })
}

#[test]
fn timecode_is_read_whole_or_not_at_all() {
    let mut follower = TransportFollower::new(SyncProtocol::MidiTimecode);
    let reading = quarter_frames(0, 0, 1, 0, 1);
    for (index, message) in reading.iter().take(7).enumerate() {
        assert_eq!(follower.observe(at(index as u64, *message)), None);
    }
    // 00:00:01:00 at 25 frames, plus the two frames the wire is behind.
    assert_eq!(
        follower.observe(at(7, reading[7])),
        Some(SyncIntent::Start(SyncPosition::Seconds(Ratio::new(27, 25))))
    );
    assert_eq!(follower.status().tempo_bpm, None, "MTC carries no tempo");
}

#[test]
fn a_timecode_stream_joined_halfway_waits_for_the_next_whole_reading() {
    let mut follower = TransportFollower::new(SyncProtocol::MidiTimecode);
    let reading = quarter_frames(0, 0, 1, 0, 1);
    // Joined at piece 4: nothing is assembled from a half reading.
    for message in reading.iter().skip(4) {
        assert_eq!(follower.observe(at(0, *message)), None);
    }
    for message in reading.iter().take(7) {
        assert_eq!(follower.observe(at(1, *message)), None);
    }
    assert!(follower.observe(at(1, reading[7])).is_some());
}

#[test]
fn a_timecode_reading_where_it_was_expected_is_a_confirmation_not_a_seek() {
    let mut follower = TransportFollower::new(SyncProtocol::MidiTimecode);
    for message in quarter_frames(0, 0, 1, 0, 1) {
        follower.observe(at(0, message));
    }
    // Two frames later at 25 frames a second is where the next whole reading
    // is expected: it confirms, and asks for nothing.
    for message in quarter_frames(0, 0, 1, 2, 1) {
        assert_eq!(follower.observe(at(80_000, message)), None);
    }
    assert_eq!(follower.status().lock, SyncLock::Locked);
    // A reading a whole second away is a seek.
    let jumped = quarter_frames(0, 0, 2, 2, 1)
        .into_iter()
        .filter_map(|message| follower.observe(at(160_000, message)))
        .collect::<Vec<_>>();
    assert_eq!(
        jumped,
        vec![SyncIntent::Seek(SyncPosition::Seconds(Ratio::new(2 * 25 + 4, 25)))]
    );
}

#[test]
fn the_callback_reads_bytes_and_the_queue_counts_what_it_could_not_hold() {
    let mut harness = SyncCallbackHarness::new();
    // Far more clock than the ring holds, and none of it drained.
    for _ in 0..8_192 {
        harness.receive(0, 0, &[0xF8]);
    }
    assert!(harness.losses().queue_overflow > 0);
    let mut read = 0;
    while harness.poll().is_some() {
        read += 1;
    }
    assert_eq!(read + harness.losses().queue_overflow, 8_192);
}

#[test]
fn the_leaders_stream_states_a_position_starts_pulses_and_stops() {
    let pulses = [0, 20_833, 41_666];
    let packets = transport_packets(3, &pulses, 500_000);
    let wire = packets
        .iter()
        .map(|packet| (packet.micros, packet.bytes[0], packet.len))
        .collect::<Vec<_>>();
    assert_eq!(
        wire,
        vec![
            (0, 0xF2, 3),
            (0, 0xFA, 1),
            (0, 0xF8, 1),
            (20_833, 0xF8, 1),
            (41_666, 0xF8, 1),
            (500_000, 0xFC, 1),
        ]
    );
    assert!(
        packets.iter().all(|packet: &LiveMidiPacket| packet.part == 3),
        "the whole stream rides the port it was routed to"
    );
}

#[test]
fn one_authority_or_none_and_never_two() {
    assert_eq!(SyncOptions::default().authority(), Ok(None));
    assert_eq!(SyncOptions::leading().authority(), Ok(Some(ClockAuthority::Musa)));
    let source = SyncSource {
        id: "logic".to_owned(),
        protocol: SyncProtocol::MidiTimecode,
    };
    assert_eq!(
        SyncOptions::following(source.clone()).authority(),
        Ok(Some(ClockAuthority::External))
    );
    // Sending and following the same protocol is the ambiguity by name.
    let same = SyncOptions {
        send: Some(SyncProtocol::MidiClock),
        follow: Some(SyncSource {
            id: "logic".to_owned(),
            protocol: SyncProtocol::MidiClock,
        }),
        reference: None,
    };
    assert_eq!(
        same.authority(),
        Err(SyncRefusal::SameProtocol(SyncProtocol::MidiClock.name()))
    );
    // Two different protocols is still two authorities.
    let two = SyncOptions {
        send: Some(SyncProtocol::MidiClock),
        follow: Some(source),
        reference: None,
    };
    assert_eq!(two.authority(), Err(SyncRefusal::TwoAuthorities));
    // Musa does not generate timecode, and says so rather than sending none.
    let generated = SyncOptions {
        send: Some(SyncProtocol::MidiTimecode),
        follow: None,
        reference: None,
    };
    assert_eq!(
        generated.authority(),
        Err(SyncRefusal::CannotSend(SyncProtocol::MidiTimecode.name()))
    );
}

#[test]
fn every_protocol_states_what_it_cannot_carry() {
    for protocol in [SyncProtocol::MidiClock, SyncProtocol::MidiTimecode] {
        assert!(!protocol.limits().is_empty());
        assert!(!protocol.name().is_empty());
    }
    assert!(SyncProtocol::MidiClock.can_send());
    assert!(!SyncProtocol::MidiTimecode.can_send());
}

/// The one law that needs a real cable, and the only place a jitter figure
/// in the book comes from.
///
/// **What it protects.** That the leader's stream and the follower's parser
/// meet correctly through `CoreMIDI` and not merely through a `Vec<u8>`:
/// packets are delivered whole, the parser reads the bytes a real driver
/// sends, and the follower locks on a clock Musa itself generated.
///
/// **What still covers it fast.** Every decision in that path is checked
/// above without a device — the parser laws, the follower laws, and
/// `the_leaders_stream_states_a_position_starts_pulses_and_stops` for the
/// leader's bytes. What only this can add is the measurement.
///
/// **What is deferred.** It is ignored because it needs macOS with
/// `CoreMIDI`, publishes a real virtual source, and takes about two seconds
/// of wall clock. Run it with `cargo nextest run -p musa-playback
/// --run-ignored all` and record what it prints, with the host and the OS
/// version, in `docs/book/src/how-to/sync-transport.md`.
#[test]
#[cfg(target_os = "macos")]
#[ignore = "slow and needs a real CoreMIDI loopback: measures jitter rather than deciding anything"]
fn the_loopback_jitter_is_measured_ignored_needs_a_cable() {
    use musa_playback::{
        LiveMidiPart, MidiInput, MidiOutput, MidiOutputConfig, MidiOutputMode, MidiOutputTarget, SyncInput,
    };

    const CLIENT: &str = "musa loopback";
    let parts = vec![LiveMidiPart {
        name: "one".to_owned(),
        channel: 0,
    }];
    let config = MidiOutputConfig {
        mode: MidiOutputMode::SingleChannelized,
        target: MidiOutputTarget::VirtualSources,
        client: CLIENT.to_owned(),
        clock: true,
    };
    let mut output = MidiOutput::open(&config, &parts).expect("publishing a virtual source");

    // A virtual source Musa publishes is an input every other client sees.
    let clock_port = format!("{CLIENT} · clock");
    let source = MidiInput::devices()
        .into_iter()
        .find(|device| device.name == clock_port)
        .expect("the clock port Musa just published");
    let mut input = SyncInput::open(&source.id);
    assert!(input.device().is_some(), "the loopback opened");

    // Two seconds of clock at a quarter of 120.
    let pulses = (0..=96).map(|pulse| pulse * AT_120).collect::<Vec<_>>();
    let end = *pulses.last().expect("pulses");
    output
        .start(transport_packets(MidiOutput::clock_part(&parts), &pulses, end))
        .expect("starting the run");

    let mut follower = TransportFollower::new(SyncProtocol::MidiClock);
    let mut received = 0u64;
    // The run ends with a stop, which is the leader taking its own lock
    // back. What is measured is the run: the last reading while it was
    // rolling, not the state it left behind.
    let mut rolling = follower.status();
    let drain = |follower: &mut TransportFollower, input: &mut SyncInput, received: &mut u64, rolling: &mut _| {
        while let Some(observation) = input.poll() {
            *received += 1;
            follower.observe(observation);
            let status = follower.status();
            if status.running {
                *rolling = status;
            }
        }
    };
    while output.is_running() || received == 0 {
        drain(&mut follower, &mut input, &mut received, &mut rolling);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    drain(&mut follower, &mut input, &mut received, &mut rolling);
    output.stop();

    println!(
        "loopback: {received} messages, lock {:?}, tempo {:?}, jitter {} µs, drift {} µs, overflow {}",
        rolling.lock,
        rolling.tempo_bpm,
        rolling.jitter_micros,
        rolling.drift_micros,
        input.losses().queue_overflow
    );
    // A hundred messages: a song position, a start, ninety-seven pulses, and
    // a stop.
    assert_eq!(received, 100);
    assert_eq!(
        rolling.lock,
        SyncLock::Locked,
        "the follower locked on Musa's own clock"
    );
    assert!(!follower.status().running, "and the stop was heard");
    assert_eq!(input.losses().queue_overflow, 0);
    assert!(
        rolling.tempo_bpm.is_some_and(|bpm| (bpm - 120.0).abs() < 5.0),
        "estimated {:?}",
        rolling.tempo_bpm
    );
}
