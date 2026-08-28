//! The Rust side of the boundary: what a handle actually holds.
//!
//! Kept apart from `abi` so that everything about *rendering* is testable
//! without a raw pointer in sight, and so the C layer is only ever
//! null-checking, catching, and forwarding.

use std::ffi::CString;

use musa_project::{HostedInput, HostedInstrument, HostedOutcome, HostedRequest, open_hosted_instrument};

/// One host event, already decoded from the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    NoteOn {
        voice: u32,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        voice: u32,
        velocity: u8,
    },
    Input {
        input: HostedInput,
        value: i16,
        key: Option<u8>,
    },
}

/// A prepared instrument and the bounded counters a host may read.
///
/// The counters are the only thing the render side writes that anybody else
/// reads, and they are saturating on purpose: a counter that wrapped would
/// turn "the host sent us 5 billion things we cannot play" into "all is well".
pub(crate) struct Hosted {
    instrument: HostedInstrument,
    /// Frames produced since preparation or the last reset.
    rendered: u64,
    /// Events the instrument's source binds nothing for.
    unbound: u32,
    /// Owned, NUL-terminated copies of everything the header hands out as a
    /// `const char *`. Built once here so that no accessor allocates.
    music: CString,
    assets: CString,
    piece: CString,
    part: CString,
    inputs: Vec<CString>,
}

impl Hosted {
    pub(crate) fn open(project: &str, piece: Option<&str>, part: &str, sample_rate: u32) -> Result<Self, String> {
        let instrument = open_hosted_instrument(&HostedRequest {
            project: std::path::PathBuf::from(project),
            piece: piece.map(str::to_owned),
            part: part.to_owned(),
            sample_rate,
        })
        .map_err(|error| error.to_string())?;
        let identity = instrument.identity();
        let inputs = instrument
            .inputs()
            .iter()
            .map(|input| text(&input.name()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            music: text(&identity.music)?,
            assets: text(&identity.assets)?,
            piece: text(&identity.piece)?,
            part: text(&identity.part)?,
            inputs,
            instrument,
            rendered: 0,
            unbound: 0,
        })
    }

    pub(crate) const fn music(&self) -> &CString {
        &self.music
    }

    pub(crate) const fn assets(&self) -> &CString {
        &self.assets
    }

    pub(crate) const fn piece(&self) -> &CString {
        &self.piece
    }

    pub(crate) const fn part(&self) -> &CString {
        &self.part
    }

    pub(crate) fn inputs(&self) -> &[CString] {
        &self.inputs
    }

    pub(crate) const fn unbound(&self) -> u32 {
        self.unbound
    }

    pub(crate) const fn rendered(&self) -> u64 {
        self.rendered
    }

    /// Render one block: apply each event at its own frame, then step.
    ///
    /// Allocation-free — it decodes each wire event in place rather than
    /// collecting them, which is the whole reason this takes the ABI type
    /// instead of a tidy slice of decoded ones. `events` must be sorted by
    /// frame; a host delivers them that way, because Apple's render-event
    /// list is a linked list in time order. An event past the block's last
    /// frame is applied at the end rather than dropped: dropping it would
    /// lose a note-off and hang a voice.
    pub(crate) fn render(&mut self, events: &[crate::abi::MusaAuEvent], left: &mut [f32], right: &mut [f32]) {
        let mut pending = events.iter().peekable();
        let mut frames = 0u64;
        for (frame, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            while let Some(event) = pending.next_if(|event| event.frame as usize <= frame) {
                self.apply(event.decode());
            }
            let [a, b] = self.instrument.step();
            *l = a;
            *r = b;
            frames = frames.saturating_add(1);
        }
        for event in pending {
            self.apply(event.decode());
        }
        self.rendered = self.rendered.saturating_add(frames);
    }

    fn apply(&mut self, event: Option<Event>) {
        let Some(event) = event else {
            // A kind this ABI version does not define. Counted, never
            // guessed at: a future host sending a future event must not be
            // answered with an approximation of it.
            self.unbound = self.unbound.saturating_add(1);
            return;
        };
        let outcome = match event {
            Event::NoteOn { voice, note, velocity } => self.instrument.note_on(voice, note, velocity),
            Event::NoteOff { voice, velocity } => self.instrument.note_off(voice, velocity),
            Event::Input { input, value, key } => self.instrument.input(input, value, key),
        };
        if outcome == HostedOutcome::Unbound {
            self.unbound = self.unbound.saturating_add(1);
        }
    }

    pub(crate) fn reset(&mut self) {
        self.instrument.reset();
        self.rendered = 0;
    }
}

/// A C string, refusing an interior NUL rather than truncating at it.
fn text(value: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| format!("`{value}` cannot cross a C boundary: it contains a NUL byte"))
}
