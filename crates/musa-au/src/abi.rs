//! The boundary itself: `#[repr(C)]` data, opaque handles, and entry points.

use std::ffi::{CStr, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

use musa_project::HostedInput;

use crate::instrument::{Event, Hosted};

/// The version of this ABI.
///
/// A host bundle and this library are built together, so this is not a
/// negotiation — it is the assertion that they were. The extension reads it
/// once at load and refuses rather than rendering through a layout it was not
/// compiled against.
pub const MUSA_AU_ABI_VERSION: u32 = 2;

/// Note on: `data1` is the note number, `data2` the attack velocity.
pub const MUSA_AU_EVENT_NOTE_ON: u8 = 1;
/// Note off: `data1` is the note number, `data2` the release velocity.
pub const MUSA_AU_EVENT_NOTE_OFF: u8 = 2;
/// Control change: `data1` is the controller number, `data2` its value.
pub const MUSA_AU_EVENT_CONTROLLER: u8 = 3;
/// Pitch bend: `data1` is the low seven bits, `data2` the high seven.
pub const MUSA_AU_EVENT_PITCH_BEND: u8 = 4;
/// Channel pressure: `data1` is the pressure.
pub const MUSA_AU_EVENT_CHANNEL_PRESSURE: u8 = 5;
/// Polyphonic key pressure: `data1` is the note, `data2` the pressure.
pub const MUSA_AU_EVENT_KEY_PRESSURE: u8 = 6;
/// A host parameter change: `address` names the control, `value` is its new
/// value in the control's own domain, and `ramp` is how many frames the host
/// asked it to take getting there.
pub const MUSA_AU_EVENT_PARAMETER: u8 = 7;

/// Bit zero of a control's flags: the source declares it continuous, so a
/// ramp towards a new value is something the declaration supports.
pub const MUSA_AU_CONTROL_CONTINUOUS: u32 = 1;

/// The greatest number of outputs one render call may fill.
///
/// A bound rather than a limit on what a source may declare: the render side
/// copies channel pointers onto its stack, and a stack array has to have a
/// size. A source that declares more outputs than this still prepares; the
/// component publishes the first eight.
pub const MUSA_AU_MAX_OUTPUTS: u32 = 8;

/// One host event, at a sample offset inside the block being rendered.
///
/// Fixed layout, no pointers, and no channel: a Music Device instance is one
/// part of one piece, so which MIDI channel the host used is the host's
/// routing question and not a musical one.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct MusaAuEvent {
    /// The parameter address, for `MUSA_AU_EVENT_PARAMETER`. Zero otherwise,
    /// and zero is never a published address.
    pub address: u64,
    /// Offset from the first frame of this block.
    pub frame: u32,
    /// The host's own identity for the voice this begins or ends. Musa pairs
    /// an attack with its release by identity rather than by note number, so
    /// a host that overlaps the same pitch stays unambiguous.
    pub voice: u32,
    /// Frames the host asked a parameter change to take. Zero is a point
    /// change, and a control the source does not declare continuous takes
    /// every change as one.
    pub ramp: u32,
    /// The new value of a parameter, in the control's own declared domain.
    pub value: f32,
    /// One of the `MUSA_AU_EVENT_*` constants.
    pub kind: u8,
    /// First data byte, meaning fixed by `kind`.
    pub data1: u8,
    /// Second data byte, meaning fixed by `kind`.
    pub data2: u8,
    /// Must be zero.
    pub reserved: u8,
}

impl MusaAuEvent {
    /// One MIDI event, with every parameter field left at zero.
    ///
    /// A Rust caller writes this rather than a struct literal so that the
    /// fields a MIDI event does not use are zeroed in one place instead of at
    /// every construction site.
    #[must_use]
    pub const fn midi(frame: u32, voice: u32, kind: u8, data1: u8, data2: u8) -> Self {
        Self {
            address: 0,
            frame,
            voice,
            ramp: 0,
            value: 0.0,
            kind,
            data1,
            data2,
            reserved: 0,
        }
    }

    /// One host parameter change, with every MIDI field left at zero.
    #[must_use]
    pub const fn parameter(frame: u32, address: u64, value: f32, ramp: u32) -> Self {
        Self {
            address,
            frame,
            voice: 0,
            ramp,
            value,
            kind: MUSA_AU_EVENT_PARAMETER,
            data1: 0,
            data2: 0,
            reserved: 0,
        }
    }

    /// The decoded event, or `None` for a kind this version does not define.
    pub(crate) fn decode(self) -> Option<Event> {
        match self.kind {
            MUSA_AU_EVENT_NOTE_ON => Some(Event::NoteOn {
                voice: self.voice,
                note: self.data1,
                velocity: self.data2,
            }),
            MUSA_AU_EVENT_NOTE_OFF => Some(Event::NoteOff {
                voice: self.voice,
                velocity: self.data2,
            }),
            MUSA_AU_EVENT_CONTROLLER => Some(Event::Input {
                input: HostedInput::Controller(self.data1),
                value: i16::from(self.data2),
                key: None,
            }),
            // Two seven-bit halves become the signed fourteen-bit value the
            // prepared instrument expects; the centre is silence, not 8192.
            // Both halves are at most 127, so the wrapping here is exact —
            // it is spelled that way because nothing at this boundary may
            // panic, not because the arithmetic is in doubt.
            MUSA_AU_EVENT_PITCH_BEND => Some(Event::Input {
                input: HostedInput::PitchBend,
                value: i16::from(self.data2)
                    .wrapping_mul(128)
                    .wrapping_add(i16::from(self.data1))
                    .wrapping_sub(8192),
                key: None,
            }),
            MUSA_AU_EVENT_CHANNEL_PRESSURE => Some(Event::Input {
                input: HostedInput::ChannelPressure,
                value: i16::from(self.data1),
                key: None,
            }),
            MUSA_AU_EVENT_KEY_PRESSURE => Some(Event::Input {
                input: HostedInput::KeyPressure,
                value: i16::from(self.data2),
                key: Some(self.data1),
            }),
            MUSA_AU_EVENT_PARAMETER => Some(Event::Parameter {
                address: self.address,
                value: self.value,
                ramp: u64::from(self.ramp),
            }),
            _ => None,
        }
    }
}

/// The result of one preparation: an instrument, or why there is not one.
///
/// Opaque. A failed preparation is an ordinary result rather than a null
/// return, because the reason is the useful part and a host has somewhere to
/// show it.
pub struct MusaAuPreparation {
    hosted: Option<Hosted>,
    message: std::ffi::CString,
}

/// A prepared instrument a host may render. Opaque.
pub struct MusaAuInstrument {
    hosted: Hosted,
}

/// The ABI version this library was built as.
#[unsafe(no_mangle)]
pub extern "C" fn musa_au_abi_version() -> u32 {
    MUSA_AU_ABI_VERSION
}

/// Compile, verify, and prepare one part's instrument.
///
/// Control side only: this reads files and allocates. Never null; release the
/// result with [`musa_au_preparation_release`] whether it succeeded or not.
///
/// `table` is the parameter-address table a restored document carried, or
/// null for a component that has never been saved. A table this library
/// cannot read is a refused preparation rather than a fresh start: assigning
/// new addresses under a host's existing automation would move the controls
/// out from under it.
///
/// # Safety
/// `project` and `part` are NUL-terminated UTF-8. `piece` and `table` are the
/// same or null; a null `piece` means the project's first piece.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_prepare(
    project: *const c_char,
    piece: *const c_char,
    part: *const c_char,
    sample_rate: u32,
    table: *const c_char,
) -> *mut MusaAuPreparation {
    let prepared = catch_unwind(AssertUnwindSafe(|| {
        let project = unsafe { borrow(project) }?;
        let part = unsafe { borrow(part) }?;
        let piece = if piece.is_null() {
            None
        } else {
            Some(unsafe { borrow(piece) }?)
        };
        let table = if table.is_null() {
            None
        } else {
            Some(unsafe { borrow(table) }?)
        };
        Hosted::open(project, piece, part, sample_rate, table)
    }))
    .unwrap_or_else(|_| Err("preparing the instrument panicked".to_owned()));
    let (hosted, message) = match prepared {
        Ok(hosted) => (Some(hosted), String::new()),
        Err(message) => (None, message),
    };
    Box::into_raw(Box::new(MusaAuPreparation {
        hosted,
        message: std::ffi::CString::new(message).unwrap_or_default(),
    }))
}

/// Whether the preparation produced an instrument.
///
/// # Safety
/// `preparation` is null, or a live pointer from [`musa_au_prepare`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_ok(preparation: *const MusaAuPreparation) -> i32 {
    let Some(preparation) = (unsafe { preparation.as_ref() }) else {
        return 0;
    };
    i32::from(preparation.hosted.is_some())
}

/// Why the preparation produced nothing; empty when it succeeded.
///
/// Borrowed from the preparation and valid until it is released.
///
/// # Safety
/// As [`musa_au_preparation_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_message(preparation: *const MusaAuPreparation) -> *const c_char {
    let Some(preparation) = (unsafe { preparation.as_ref() }) else {
        return c"the preparation is null".as_ptr();
    };
    preparation.message.as_ptr()
}

macro_rules! identity_accessor {
    ($name:ident, $field:ident, $what:literal) => {
        #[doc = concat!("The prepared instrument's ", $what, ", or an empty string when there is none.")]
        ///
        /// Borrowed from the preparation and valid until it is released.
        ///
        /// # Safety
        /// As [`musa_au_preparation_ok`].
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(preparation: *const MusaAuPreparation) -> *const c_char {
            match unsafe { preparation.as_ref() }.and_then(|preparation| preparation.hosted.as_ref()) {
                Some(hosted) => hosted.$field().as_ptr(),
                None => c"".as_ptr(),
            }
        }
    };
}

identity_accessor!(musa_au_preparation_identity_music, music, "semantic music identity");
identity_accessor!(
    musa_au_preparation_identity_assets,
    assets,
    "verified asset closure identity"
);
identity_accessor!(musa_au_preparation_identity_piece, piece, "piece name");
identity_accessor!(musa_au_preparation_identity_part, part, "part name");

/// How many MIDI dimensions the instrument's source binds.
///
/// # Safety
/// As [`musa_au_preparation_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_input_count(preparation: *const MusaAuPreparation) -> u32 {
    match unsafe { preparation.as_ref() }.and_then(|preparation| preparation.hosted.as_ref()) {
        Some(hosted) => u32::try_from(hosted.inputs().len()).unwrap_or(u32::MAX),
        None => 0,
    }
}

/// The name of one bound MIDI dimension, or null past the end.
///
/// Borrowed from the preparation and valid until it is released.
///
/// # Safety
/// As [`musa_au_preparation_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_input(preparation: *const MusaAuPreparation, index: u32) -> *const c_char {
    match unsafe { preparation.as_ref() }.and_then(|preparation| preparation.hosted.as_ref()) {
        Some(hosted) => hosted
            .inputs()
            .get(index as usize)
            .map_or(std::ptr::null(), |name| name.as_ptr()),
        None => std::ptr::null(),
    }
}

/// One exposed control's numbers, as a host builds a parameter from them.
///
/// Fixed layout and no pointers, so a caller reads it by value; the strings
/// beside it are separate accessors because a borrowed `const char *` and a
/// returned struct have different lifetimes and mixing them hides that.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct MusaAuControl {
    /// The stable host address. Never zero.
    pub address: u64,
    /// The inclusive ends of the control's declared domain.
    pub minimum: f32,
    /// The upper end of that domain.
    pub maximum: f32,
    /// The declared default, inside that domain.
    pub default_value: f32,
    /// `MUSA_AU_CONTROL_*` bits.
    pub flags: u32,
}

/// How many source-declared controls the instrument exposes.
///
/// # Safety
/// As [`musa_au_preparation_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_control_count(preparation: *const MusaAuPreparation) -> u32 {
    match unsafe { preparation.as_ref() }.and_then(|preparation| preparation.hosted.as_ref()) {
        Some(hosted) => u32::try_from(hosted.controls().len()).unwrap_or(u32::MAX),
        None => 0,
    }
}

/// One control's numbers. Returns zero and writes nothing past the end.
///
/// # Safety
/// As [`musa_au_preparation_ok`], and `out` addresses one writable
/// [`MusaAuControl`] or is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_control(
    preparation: *const MusaAuPreparation,
    index: u32,
    out: *mut MusaAuControl,
) -> i32 {
    let Some(out) = (unsafe { out.as_mut() }) else {
        return 0;
    };
    let Some(control) = unsafe { preparation.as_ref() }
        .and_then(|preparation| preparation.hosted.as_ref())
        .and_then(|hosted| hosted.controls().get(index as usize))
    else {
        return 0;
    };
    *out = MusaAuControl {
        address: control.address,
        minimum: control.minimum,
        maximum: control.maximum,
        default_value: control.default,
        flags: if control.continuous {
            MUSA_AU_CONTROL_CONTINUOUS
        } else {
            0
        },
    };
    1
}

macro_rules! control_text {
    ($name:ident, $field:ident, $what:literal) => {
        #[doc = concat!("One control's ", $what, ", or null past the end.")]
        ///
        /// Borrowed from the preparation and valid until it is released.
        ///
        /// # Safety
        /// As [`musa_au_preparation_ok`].
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(preparation: *const MusaAuPreparation, index: u32) -> *const c_char {
            unsafe { preparation.as_ref() }
                .and_then(|preparation| preparation.hosted.as_ref())
                .and_then(|hosted| hosted.controls().get(index as usize))
                .map_or(std::ptr::null(), |control| control.$field.as_ptr())
        }
    };
}

control_text!(
    musa_au_preparation_control_identity,
    identity,
    "canonical source identity"
);
control_text!(musa_au_preparation_control_display, display, "declared name");
control_text!(
    musa_au_preparation_control_summary,
    summary,
    "declared one-sentence description"
);
control_text!(musa_au_preparation_control_kind, kind, "source value kind");
control_text!(
    musa_au_preparation_control_update_rate,
    update_rate,
    "source update rate"
);

/// How many declared controls this boundary could not carry.
///
/// # Safety
/// As [`musa_au_preparation_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_loss_count(preparation: *const MusaAuPreparation) -> u32 {
    match unsafe { preparation.as_ref() }.and_then(|preparation| preparation.hosted.as_ref()) {
        Some(hosted) => u32::try_from(hosted.losses().len()).unwrap_or(u32::MAX),
        None => 0,
    }
}

/// One projection loss as a whole sentence, or null past the end.
///
/// Borrowed from the preparation and valid until it is released.
///
/// # Safety
/// As [`musa_au_preparation_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_loss(preparation: *const MusaAuPreparation, index: u32) -> *const c_char {
    unsafe { preparation.as_ref() }
        .and_then(|preparation| preparation.hosted.as_ref())
        .and_then(|hosted| hosted.losses().get(index as usize))
        .map_or(std::ptr::null(), |loss| loss.as_ptr())
}

/// The complete parameter-address table, for the host document to carry.
///
/// Borrowed from the preparation and valid until it is released.
///
/// # Safety
/// As [`musa_au_preparation_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_control_table(preparation: *const MusaAuPreparation) -> *const c_char {
    match unsafe { preparation.as_ref() }.and_then(|preparation| preparation.hosted.as_ref()) {
        Some(hosted) => hosted.table().as_ptr(),
        None => c"".as_ptr(),
    }
}

/// How many outputs the source declares for this part, bus zero included.
///
/// # Safety
/// As [`musa_au_preparation_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_output_count(preparation: *const MusaAuPreparation) -> u32 {
    match unsafe { preparation.as_ref() }.and_then(|preparation| preparation.hosted.as_ref()) {
        Some(hosted) => u32::try_from(hosted.outputs().len()).unwrap_or(u32::MAX),
        None => 0,
    }
}

/// One output's source name, or null past the end.
///
/// Borrowed from the preparation and valid until it is released.
///
/// # Safety
/// As [`musa_au_preparation_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_output(
    preparation: *const MusaAuPreparation,
    index: u32,
) -> *const c_char {
    unsafe { preparation.as_ref() }
        .and_then(|preparation| preparation.hosted.as_ref())
        .and_then(|hosted| hosted.outputs().get(index as usize))
        .map_or(std::ptr::null(), |output| output.as_ptr())
}

/// What one output is: `0` main, `1` a part output, `2` a declared bus.
///
/// # Safety
/// As [`musa_au_preparation_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_output_role(preparation: *const MusaAuPreparation, index: u32) -> u32 {
    match unsafe { preparation.as_ref() }.and_then(|preparation| preparation.hosted.as_ref()) {
        Some(hosted) => hosted.output_role(index as usize),
        None => 0,
    }
}

/// Take the instrument out of the preparation, leaving it empty.
///
/// Returns null if the preparation failed or has already been taken. The
/// instrument outlives the preparation; release it with
/// [`musa_au_instrument_release`].
///
/// # Safety
/// `preparation` is null, or a live pointer from [`musa_au_prepare`] that no
/// other thread is using.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_take(preparation: *mut MusaAuPreparation) -> *mut MusaAuInstrument {
    let Some(preparation) = (unsafe { preparation.as_mut() }) else {
        return std::ptr::null_mut();
    };
    preparation.hosted.take().map_or(std::ptr::null_mut(), |hosted| {
        Box::into_raw(Box::new(MusaAuInstrument { hosted }))
    })
}

/// Release a preparation. Control side: this frees.
///
/// # Safety
/// `preparation` is null, or a live pointer from [`musa_au_prepare`] that is
/// not released twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_preparation_release(preparation: *mut MusaAuPreparation) {
    if preparation.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(preparation) });
}

/// Render one block into two preallocated channel buffers.
///
/// Real-time safe: no allocation, no lock, no I/O, no logging, no
/// destruction. Events must be sorted by `frame`.
///
/// # Safety
/// `left` and `right` each address `frames` writable floats and do not alias
/// each other. `events` addresses `count` initialized [`MusaAuEvent`] values,
/// or is null when `count` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_render(
    instrument: *mut MusaAuInstrument,
    events: *const MusaAuEvent,
    count: u32,
    left: *mut f32,
    right: *mut f32,
    frames: u32,
) {
    let Some(instrument) = (unsafe { instrument.as_mut() }) else {
        return;
    };
    if left.is_null() || right.is_null() {
        return;
    }
    let frames = frames as usize;
    let events = if events.is_null() || count == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(events, count as usize) }
    };
    let left = unsafe { std::slice::from_raw_parts_mut(left, frames) };
    let right = unsafe { std::slice::from_raw_parts_mut(right, frames) };
    // `catch_unwind` here is not defensive style: a panic unwinding through
    // this frame into Objective-C is undefined behavior, and the render
    // thread is where that would first be observed.
    if catch_unwind(AssertUnwindSafe(|| instrument.hosted.render(events, left, right))).is_err() {
        // A block that panicked has written some frames and not others.
        // Silence is the only honest thing left to hand the host: leaving
        // the rest is stale memory, and there is nothing to log from here.
        left.fill(0.0);
        right.fill(0.0);
    }
}

/// Render one block into every output the host negotiated.
///
/// Real-time safe, on the same terms as [`musa_au_render`]. `channels` holds
/// two pointers per output — left then right, bus zero first — and
/// `channel_count` is how many of them are supplied; an odd count renders
/// nothing rather than guessing which channel was meant. Outputs past
/// [`MUSA_AU_MAX_OUTPUTS`] are not filled.
///
/// Bus zero is exactly what [`musa_au_render`] would have produced, so a host
/// that enables one output and a host that enables all of them hear the same
/// main output.
///
/// # Safety
/// `channels` addresses `channel_count` pointers, each addressing `frames`
/// writable floats, and no two of them alias. `events` is as
/// [`musa_au_render`] requires.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_render_outputs(
    instrument: *mut MusaAuInstrument,
    events: *const MusaAuEvent,
    count: u32,
    channels: *const *mut f32,
    channel_count: u32,
    frames: u32,
) {
    let Some(instrument) = (unsafe { instrument.as_mut() }) else {
        return;
    };
    if channels.is_null() || channel_count == 0 || channel_count % 2 == 1 {
        return;
    }
    // Copied onto the stack rather than borrowed as a slice of the host's
    // memory for the duration of the render: this is the render thread, and
    // the array is what bounds the loop.
    let supplied = (channel_count as usize).min(MUSA_AU_MAX_OUTPUTS as usize * 2);
    let mut buffers = [std::ptr::null_mut::<f32>(); MUSA_AU_MAX_OUTPUTS as usize * 2];
    for (index, slot) in buffers.iter_mut().enumerate().take(supplied) {
        // SAFETY: the caller promises `channel_count` readable pointers, and
        // `index` is below `supplied`, which is at most `channel_count`.
        *slot = unsafe { channels.add(index).read() };
    }
    let Some(buffers) = buffers.get(..supplied) else {
        return;
    };
    if buffers.iter().any(|channel| channel.is_null()) {
        return;
    }
    let events = if events.is_null() || count == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(events, count as usize) }
    };
    let _outcome = catch_unwind(AssertUnwindSafe(|| {
        instrument.hosted.render_outputs(events, buffers, frames as usize);
    }));
}

/// Return the instrument to its prepared silence. Real-time safe.
///
/// # Safety
/// `instrument` is null, or a live pointer from [`musa_au_preparation_take`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_reset(instrument: *mut MusaAuInstrument) {
    let Some(instrument) = (unsafe { instrument.as_mut() }) else {
        return;
    };
    let _outcome = catch_unwind(AssertUnwindSafe(|| instrument.hosted.reset()));
}

/// How many events the instrument's source bound nothing for, saturating.
///
/// # Safety
/// As [`musa_au_reset`], and readable while rendering.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_unbound_events(instrument: *const MusaAuInstrument) -> u32 {
    unsafe { instrument.as_ref() }.map_or(0, |instrument| instrument.hosted.unbound())
}

/// How many frames the instrument has produced since preparation or reset.
///
/// # Safety
/// As [`musa_au_unbound_events`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_rendered_frames(instrument: *const MusaAuInstrument) -> u64 {
    unsafe { instrument.as_ref() }.map_or(0, |instrument| instrument.hosted.rendered())
}

/// Release an instrument.
///
/// **Control side.** This destroys a prepared graph, which is exactly what a
/// render callback may not do; an extension retires a replaced plan on its
/// worker.
///
/// # Safety
/// `instrument` is null, or a live pointer from [`musa_au_preparation_take`]
/// that is not released twice and is not being rendered.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_instrument_release(instrument: *mut MusaAuInstrument) {
    if instrument.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(instrument) });
}

/// One NUL-terminated argument as UTF-8, refusing null and invalid encoding.
pub(crate) unsafe fn borrow<'a>(value: *const c_char) -> Result<&'a str, String> {
    if value.is_null() {
        return Err("a required string argument was null".to_owned());
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|_| "a string argument was not valid UTF-8".to_owned())
}
