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
pub const MUSA_AU_ABI_VERSION: u32 = 1;

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

/// One host event, at a sample offset inside the block being rendered.
///
/// Fixed layout, no pointers, and no channel: a Music Device instance is one
/// part of one piece, so which MIDI channel the host used is the host's
/// routing question and not a musical one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct MusaAuEvent {
    /// Offset from the first frame of this block.
    pub frame: u32,
    /// The host's own identity for the voice this begins or ends. Musa pairs
    /// an attack with its release by identity rather than by note number, so
    /// a host that overlaps the same pitch stays unambiguous.
    pub voice: u32,
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
/// # Safety
/// `project` and `part` are NUL-terminated UTF-8. `piece` is the same or null,
/// and null means the project's first piece.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_prepare(
    project: *const c_char,
    piece: *const c_char,
    part: *const c_char,
    sample_rate: u32,
) -> *mut MusaAuPreparation {
    let prepared = catch_unwind(AssertUnwindSafe(|| {
        let project = unsafe { borrow(project) }?;
        let part = unsafe { borrow(part) }?;
        let piece = if piece.is_null() {
            None
        } else {
            Some(unsafe { borrow(piece) }?)
        };
        Hosted::open(project, piece, part, sample_rate)
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
unsafe fn borrow<'a>(value: *const c_char) -> Result<&'a str, String> {
    if value.is_null() {
        return Err("a required string argument was null".to_owned());
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|_| "a string argument was not valid UTF-8".to_owned())
}
