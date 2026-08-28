//! The Rust side of the boundary: what a handle actually holds.
//!
//! Kept apart from `abi` so that everything about *rendering* is testable
//! without a raw pointer in sight, and so the C layer is only ever
//! null-checking, catching, and forwarding.

use std::ffi::CString;

use musa_project::{
    HostedInput, HostedInstrument, HostedOutcome, HostedOutputRole, HostedRequest, open_hosted_instrument,
};

/// One host event, already decoded from the wire.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    Parameter {
        address: u64,
        value: f32,
        ramp: u64,
    },
}

/// One exposed control, as the C side reads it.
///
/// Every string is owned and NUL-terminated here so that no accessor
/// allocates, and the numbers are the descriptor a host builds its parameter
/// from. There is no catalogue behind this: each field came from the source
/// declaration `musa-project` projected.
pub(crate) struct Control {
    pub(crate) identity: CString,
    pub(crate) display: CString,
    pub(crate) summary: CString,
    pub(crate) kind: CString,
    pub(crate) update_rate: CString,
    pub(crate) address: u64,
    pub(crate) minimum: f32,
    pub(crate) maximum: f32,
    pub(crate) default: f32,
    pub(crate) continuous: bool,
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
    controls: Vec<Control>,
    losses: Vec<CString>,
    outputs: Vec<CString>,
    output_roles: Vec<u32>,
    table: CString,
    /// One frame of every projected output past bus zero. Preallocated,
    /// because reporting an extra bus may not allocate on the render thread.
    outputs_frame: Vec<[f32; 2]>,
}

impl Hosted {
    pub(crate) fn open(
        project: &str,
        piece: Option<&str>,
        part: &str,
        sample_rate: u32,
        table: Option<&str>,
    ) -> Result<Self, String> {
        let instrument = open_hosted_instrument(&HostedRequest {
            project: std::path::PathBuf::from(project),
            piece: piece.map(str::to_owned),
            part: part.to_owned(),
            sample_rate,
            table: table.map(str::to_owned),
        })
        .map_err(|error| error.to_string())?;
        let identity = instrument.identity();
        let inputs = instrument
            .inputs()
            .iter()
            .map(|input| text(&input.name()))
            .collect::<Result<Vec<_>, _>>()?;
        let controls = instrument
            .controls()
            .iter()
            .map(|control| {
                Ok(Control {
                    identity: text(&control.identity)?,
                    display: text(&control.display)?,
                    summary: text(&control.summary)?,
                    kind: text(&control.kind)?,
                    update_rate: text(&control.update_rate)?,
                    address: control.address,
                    minimum: control.minimum,
                    maximum: control.maximum,
                    default: control.default,
                    continuous: control.continuous,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let losses = instrument
            .control_losses()
            .iter()
            .map(|loss| text(&loss.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let outputs = instrument
            .outputs()
            .iter()
            .map(|output| text(&output.name))
            .collect::<Result<Vec<_>, _>>()?;
        let output_roles = instrument
            .outputs()
            .iter()
            .map(|output| match output.role {
                HostedOutputRole::Main => 0,
                HostedOutputRole::Part => 1,
                HostedOutputRole::Bus => 2,
            })
            .collect();
        let table = text(&instrument.control_table().encode())?;
        let outputs_frame = vec![[0.0; 2]; outputs.len().saturating_sub(1)];
        Ok(Self {
            controls,
            losses,
            outputs,
            output_roles,
            table,
            outputs_frame,
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

    pub(crate) fn controls(&self) -> &[Control] {
        &self.controls
    }

    pub(crate) fn losses(&self) -> &[CString] {
        &self.losses
    }

    pub(crate) fn outputs(&self) -> &[CString] {
        &self.outputs
    }

    pub(crate) fn output_role(&self, index: usize) -> u32 {
        self.output_roles.get(index).copied().unwrap_or(0)
    }

    pub(crate) const fn table(&self) -> &CString {
        &self.table
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

    /// Render one block into every output the host asked for.
    ///
    /// `channels` is two writable pointers per output — left then right, bus
    /// zero first — each addressing `frames` floats and none aliasing another.
    /// Bus zero is the same frame [`Self::render`] produces, so a host that
    /// takes one output and a host that takes all of them hear the same main
    /// output; the rest are reads of buffers the frame already wrote.
    ///
    /// Real-time safe. The scratch it fills was allocated at preparation.
    pub(crate) fn render_outputs(&mut self, events: &[crate::abi::MusaAuEvent], channels: &[*mut f32], frames: usize) {
        let outputs = channels.len() / 2;
        if outputs == 0 {
            return;
        }
        // Every channel the caller offered is written, including one this
        // instrument does not project: an output nothing reaches is silence,
        // and leaving the buffer alone would hand a host whatever was in it.
        let extra = outputs.saturating_sub(1);
        let mut pending = events.iter().peekable();
        let mut produced = 0u64;
        for frame in 0..frames {
            while let Some(event) = pending.next_if(|event| event.frame as usize <= frame) {
                self.apply(event.decode());
            }
            let [left, right] = self.instrument.step_outputs(&mut self.outputs_frame);
            // SAFETY: the caller promises every pointer addresses `frames`
            // writable floats and that none of them alias, and `frame` is
            // below `frames`. `channels` is indexed only below its own
            // length, which is what bounds `outputs` and `extra`.
            unsafe {
                write_frame(channels, 0, frame, [left, right]);
                for index in 0..extra {
                    let value = self.outputs_frame.get(index).copied().unwrap_or([0.0; 2]);
                    write_frame(channels, index.saturating_add(1), frame, value);
                }
            }
            produced = produced.saturating_add(1);
        }
        for event in pending {
            self.apply(event.decode());
        }
        self.rendered = self.rendered.saturating_add(produced);
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
            Event::Parameter { address, value, ramp } => self.instrument.set_control(address, value, ramp),
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

/// Write one stereo frame of one output.
///
/// # Safety
/// `channels` holds at least `2 * (output + 1)` entries, each addressing more
/// than `frame` writable floats, and no two of them alias.
unsafe fn write_frame(channels: &[*mut f32], output: usize, frame: usize, value: [f32; 2]) {
    let base = output.saturating_mul(2);
    let (Some(&left), Some(&right)) = (channels.get(base), channels.get(base.saturating_add(1))) else {
        return;
    };
    unsafe {
        left.add(frame).write(value[0]);
        right.add(frame).write(value[1]);
    }
}

/// A C string, refusing an interior NUL rather than truncating at it.
fn text(value: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| format!("`{value}` cannot cross a C boundary: it contains a NUL byte"))
}
