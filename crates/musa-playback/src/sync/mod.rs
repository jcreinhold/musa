//! One clock authority, and the machinery for being on either side of it.
//!
//! A session that both sends a clock and follows one has two transports and
//! no answer to which is right, so `06-daw-boundary.md` §4 allows exactly one
//! authority: Musa leads, or one named external source does. [`SyncOptions`]
//! is the place that is decided, [`SyncOptions::authority`] is the place it is
//! *checked*, and a configuration that names two is refused before anything
//! opens rather than discovered by drift.
//!
//! Leading and following are deliberately asymmetric, because the protocols
//! are:
//!
//! - Leading sends MIDI clock with a song-position pointer and start/stop, on
//!   a port of its own, merged into prompt 213's packet list so the transport
//!   stream and the notes are one schedule with one clock.
//! - Following reads MIDI clock *or* MTC. [`parser::SyncParser`] runs in the
//!   callback and does nothing but read bytes;
//!   [`follower::TransportFollower`] runs on the control side and does every
//!   piece of arithmetic.
//!
//! What neither protocol carries is stated rather than approximated: MIDI
//! clock has no meter, no key, and no exact rational position, and MTC has no
//! tempo at all. See [`SyncProtocol::limits`].

mod follower;
mod input;
mod leader;
mod parser;

pub use follower::{
    PULSES_PER_QUARTER, RESYNC_MICROS, SYNC_STATUS_VERSION, SyncIntent, SyncLock, SyncObservation, SyncPosition,
    SyncStatus, TransportFollower,
};
#[doc(hidden)]
pub use input::SyncCallbackHarness;
pub use input::{SyncInput, SyncInputLosses};
pub use leader::transport_packets;
pub use parser::{SyncMessage, SyncParser};

/// The synchronization protocols Musa speaks.
///
/// Both are MIDI 1.0 and both are what Apple's current guide documents Logic
/// Pro sending. Logic documents *slaving* to MTC only, which is why the two
/// directions are not symmetric and why the how-to says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyncProtocol {
    /// Twenty-four pulses a quarter note, with start, continue, stop, and a
    /// song-position pointer.
    MidiClock,
    /// MIDI Timecode: `hh:mm:ss:ff` of the leader's own wall clock.
    MidiTimecode,
}

impl SyncProtocol {
    /// The stable word for this protocol, as a report and a command line
    /// both use it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MidiClock => "midi-clock",
            Self::MidiTimecode => "mtc",
        }
    }

    /// What this protocol cannot say, stated before the session starts.
    ///
    /// `06-daw-boundary.md` §4 asks for exactly this: the limits are reported
    /// up front, not discovered by drift.
    #[must_use]
    pub const fn limits(self) -> &'static str {
        match self {
            Self::MidiClock => {
                "MIDI clock carries no meter, no key, and no exact rational position: it counts \
                 twenty-four pulses to a quarter and a song position to the sixteenth"
            }
            Self::MidiTimecode => {
                "MTC carries physical frames of the leader's wall clock and no tempo, no meter, \
                 no key, and no musical position at all"
            }
        }
    }

    /// Whether Musa can send this protocol.
    ///
    /// Musa leads with MIDI clock. Generating MTC would mean publishing a
    /// wall clock of Musa's own for another program to slave to, which is a
    /// different feature from following one and is not this prompt's.
    #[must_use]
    pub const fn can_send(self) -> bool {
        matches!(self, Self::MidiClock)
    }
}

/// One external source to follow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncSource {
    /// The stable input identifier, as [`crate::MidiInput::devices`] reports it.
    pub id: String,
    /// What that source is expected to send.
    pub protocol: SyncProtocol,
}

/// Which side owns the transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClockAuthority {
    /// Musa owns play, stop, continue, and seek, and sends the clock.
    Musa,
    /// One named external source owns them, and Musa follows.
    External,
}

impl ClockAuthority {
    /// The stable word for this authority.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Musa => "musa-leads",
            Self::External => "external-leads",
        }
    }
}

/// Why a synchronization configuration was refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SyncRefusal {
    /// Both sides were asked to lead.
    #[error("a session has one clock authority: `send` and `follow` cannot both be set")]
    TwoAuthorities,
    /// One protocol was asked to go both ways at once.
    #[error("Musa cannot send and follow {0} at the same time")]
    SameProtocol(&'static str),
    /// A protocol Musa does not generate was named as one to send.
    #[error("Musa does not send {0}")]
    CannotSend(&'static str),
    /// The authority was changed while a run was in progress.
    #[error("the clock authority cannot change while a run is in progress")]
    WhileRunning,
    /// The piece has several simultaneous tempos and no reference was named.
    #[error(
        "this piece is polytempo and one MIDI clock states one tempo: name the scope to \
         synchronize, and the others are reported unsynchronized"
    )]
    UnnamedReference,
}

/// The version of [`SyncOptions`] this build reads.
pub const SYNC_OPTIONS_VERSION: u32 = 1;

/// What a session decides about the clock.
///
/// The two fields can both be set, and that is the point: the refusal is
/// something a caller can *make*, so the check has something to check.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncOptions {
    /// The protocol Musa sends, if it leads.
    pub send: Option<SyncProtocol>,
    /// The source Musa follows, if it does not.
    pub follow: Option<SyncSource>,
    /// The named scope whose tempo one clock states, for a polytempo piece.
    pub reference: Option<String>,
}

impl SyncOptions {
    /// Musa sends MIDI clock on a port of its own.
    #[must_use]
    pub const fn leading() -> Self {
        Self {
            send: Some(SyncProtocol::MidiClock),
            follow: None,
            reference: None,
        }
    }

    /// Musa follows one named source.
    #[must_use]
    pub const fn following(source: SyncSource) -> Self {
        Self {
            send: None,
            follow: Some(source),
            reference: None,
        }
    }

    /// Which side owns the transport, or `None` when neither does and Musa
    /// simply plays — which is prompt 213's behaviour and stays the default.
    ///
    /// # Errors
    ///
    /// [`SyncRefusal::TwoAuthorities`] when both sides are named,
    /// [`SyncRefusal::SameProtocol`] when one protocol is asked to go both
    /// ways, and [`SyncRefusal::CannotSend`] for a protocol Musa does not
    /// generate.
    pub fn authority(&self) -> Result<Option<ClockAuthority>, SyncRefusal> {
        match (self.send, self.follow.as_ref()) {
            (Some(send), Some(follow)) if send == follow.protocol => Err(SyncRefusal::SameProtocol(send.name())),
            (Some(_), Some(_)) => Err(SyncRefusal::TwoAuthorities),
            (Some(send), None) if !send.can_send() => Err(SyncRefusal::CannotSend(send.name())),
            (Some(_), None) => Ok(Some(ClockAuthority::Musa)),
            (None, Some(_)) => Ok(Some(ClockAuthority::External)),
            (None, None) => Ok(None),
        }
    }

    /// The protocol in force, whichever side it is on.
    #[must_use]
    pub fn protocol(&self) -> Option<SyncProtocol> {
        self.send.or_else(|| self.follow.as_ref().map(|source| source.protocol))
    }
}
