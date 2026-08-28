//! The byte-level sync parser: one state machine, no heap, no branches on
//! how the platform happened to chop the stream.
//!
//! A MIDI cable is a stream of bytes, and every convenience a backend offers
//! on top of that — one callback a message, timing bytes filtered out — is a
//! convenience this parser deliberately does not need. It is fed one byte at
//! a time, so a message split across two callbacks and a message arriving
//! whole are the same case. Three facts about the wire drive its shape:
//!
//! - **Real-time bytes may appear anywhere.** `0xF8` clock can land between
//!   the status and the data of a note-on. Handling one must therefore leave
//!   the pending message exactly as it was.
//! - **Running status is real.** A channel message may omit its status byte
//!   and repeat the previous one. The parser has to know each status's arity
//!   to tell a repeated message from a data byte of the current one.
//! - **System-common messages clear running status.** `0xF2` is not a
//!   channel message and does not become the running one.
//!
//! Nothing here allocates, and every field is fixed-size, so the whole thing
//! is legal inside the MIDI callback (`docs/rules/across-stages/06-daw-boundary.md` §4).

/// One transport or position fact read off the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncMessage {
    /// One of the twenty-four pulses that make a quarter note.
    Clock,
    /// Play from the beginning.
    Start,
    /// Play from where the last stop left off.
    Continue,
    /// Stop, keeping the position.
    Stop,
    /// An absolute position in MIDI beats, which are sixteenth notes.
    SongPosition(u16),
    /// One eighth of an MTC frame pair: `piece` says which nibble of
    /// `hh:mm:ss:ff` and the rate, and `value` is that nibble.
    QuarterFrame {
        /// Which of the eight nibbles, `0..=7`.
        piece: u8,
        /// The nibble, `0..=15`.
        value: u8,
    },
}

/// A bounded, allocation-free MIDI stream reader.
///
/// Feed it every byte the port delivers, in order. It answers with the
/// transport facts and ignores everything else, including the channel
/// messages a keyboard sends — those are [`crate::MidiInput`]'s subject, and
/// a sync source is chosen separately from a keyboard.
#[derive(Clone, Copy, Debug, Default)]
pub struct SyncParser {
    /// The status byte of the message being assembled, or zero for none.
    status: u8,
    /// Data bytes that status wants.
    wanted: u8,
    /// Data bytes collected so far.
    have: u8,
    /// The two data bytes, held as fields rather than an array so the whole
    /// reader stays a `const fn` with no indexing in it.
    first: u8,
    second: u8,
    /// The last channel status, which a data byte may resume.
    running: u8,
    /// Inside a system-exclusive payload, where no byte means anything to us.
    in_sysex: bool,
}

impl SyncParser {
    /// A reader at the start of a stream.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            status: 0,
            wanted: 0,
            have: 0,
            first: 0,
            second: 0,
            running: 0,
            in_sysex: false,
        }
    }

    /// Read one byte, and answer with the message it completed.
    ///
    /// At most one message completes per byte, which is what makes the
    /// caller's loop bounded: `n` bytes yield at most `n` observations.
    pub const fn feed(&mut self, byte: u8) -> Option<SyncMessage> {
        if byte >= 0xF8 {
            // A real-time byte is delivered inside anything and disturbs
            // nothing: the pending message keeps its status and its data.
            return match byte {
                0xF8 => Some(SyncMessage::Clock),
                0xFA => Some(SyncMessage::Start),
                0xFB => Some(SyncMessage::Continue),
                0xFC => Some(SyncMessage::Stop),
                // Active sensing, reset, and the two undefined bytes.
                _ => None,
            };
        }
        if byte >= 0x80 {
            return self.status(byte);
        }
        self.datum(byte)
    }

    const fn status(&mut self, byte: u8) -> Option<SyncMessage> {
        match byte {
            0xF0 => {
                self.in_sysex = true;
                self.running = 0;
                self.status = 0;
                self.have = 0;
                return None;
            }
            0xF7 => {
                self.in_sysex = false;
                self.status = 0;
                self.have = 0;
                return None;
            }
            _ => {}
        }
        self.in_sysex = false;
        self.have = 0;
        self.status = byte;
        self.wanted = arity(byte);
        // Only channel messages become the running status; a system-common
        // message clears it, which is why an `0xF2` in the middle of a run of
        // note-ons ends that run.
        self.running = if byte < 0xF0 { byte } else { 0 };
        if self.wanted == 0 {
            self.status = 0;
        }
        None
    }

    const fn datum(&mut self, byte: u8) -> Option<SyncMessage> {
        if self.in_sysex {
            return None;
        }
        if self.status == 0 {
            if self.running == 0 {
                // A data byte with no status before it: the stream was
                // joined mid-message, and there is nothing to assemble.
                return None;
            }
            self.status = self.running;
            self.wanted = arity(self.running);
            self.have = 0;
        }
        match self.have {
            0 => self.first = byte,
            1 => self.second = byte,
            _ => {}
        }
        self.have = self.have.saturating_add(1);
        if self.have < self.wanted {
            return None;
        }
        let status = self.status;
        self.status = 0;
        self.have = 0;
        message(status, self.first, self.second)
    }
}

/// How many data bytes a status byte wants.
const fn arity(status: u8) -> u8 {
    if status < 0xF0 {
        return match status & 0xF0 {
            // Program change and channel pressure carry one; everything else
            // on a channel carries two.
            0xC0 | 0xD0 => 1,
            _ => 2,
        };
    }
    match status {
        // Quarter frame and song select.
        0xF1 | 0xF3 => 1,
        // Song position pointer.
        0xF2 => 2,
        // Tune request and everything undefined.
        _ => 0,
    }
}

/// The message a completed status and its data denote, where it is one we
/// follow.
const fn message(status: u8, first: u8, second: u8) -> Option<SyncMessage> {
    match status {
        0xF1 => Some(SyncMessage::QuarterFrame {
            piece: (first >> 4) & 0x07,
            value: first & 0x0F,
        }),
        0xF2 => Some(SyncMessage::SongPosition((second as u16) << 7 | (first as u16))),
        // Channel messages and song select complete here and are not ours.
        _ => None,
    }
}
