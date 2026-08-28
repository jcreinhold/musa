//! The leader's stream: the bytes Musa sends when it owns the clock.
//!
//! One pure function, because that is all leading is. The caller — which is
//! the only place that holds a tempo map — says at which microsecond each of
//! the twenty-four-a-quarter pulses falls; this turns that into the same kind
//! of packet prompt 213's notes already are, so the transport stream and the
//! music are *one* schedule walked by one send loop. Two schedules would be
//! two clocks, which is the thing §4 forbids.

use crate::midi_out::LiveMidiPacket;

/// Song position pointer: the only two-data-byte system-common message here.
const SONG_POSITION: u8 = 0xF2;
const START: u8 = 0xFA;
const STOP: u8 = 0xFC;
const CLOCK: u8 = 0xF8;

/// The transport stream for one run, in schedule order.
///
/// `part` is the index the caller routed the clock port to; `pulse_micros`
/// are the moments of the pulses, which the caller took from the piece's own
/// exact tempo map; `stop_micros` is where the run ends.
///
/// The stream opens by stating position zero and then starting, which is what
/// a receiver needs to place the first pulse, and closes with a stop so a
/// follower is not left rolling past the end of the music.
#[must_use]
pub fn transport_packets(part: usize, pulse_micros: &[u64], stop_micros: u64) -> Vec<LiveMidiPacket> {
    let mut packets = Vec::with_capacity(pulse_micros.len().saturating_add(3));
    packets.push(LiveMidiPacket {
        micros: 0,
        part,
        bytes: [SONG_POSITION, 0, 0],
        len: 3,
    });
    packets.push(LiveMidiPacket {
        micros: 0,
        part,
        bytes: [START, 0, 0],
        len: 1,
    });
    packets.extend(pulse_micros.iter().map(|&micros| LiveMidiPacket {
        micros,
        part,
        bytes: [CLOCK, 0, 0],
        len: 1,
    }));
    packets.push(LiveMidiPacket {
        micros: stop_micros,
        part,
        bytes: [STOP, 0, 0],
        len: 1,
    });
    packets
}
