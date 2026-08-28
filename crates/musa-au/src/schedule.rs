//! The C ABI a macOS `AUv3` MIDI Processor reads a checked Musa piece through.
//!
//! The other half of this crate hands a host an instrument to play. This half
//! hands it a *piece*: a finite, immutable schedule of MIDI messages the host
//! places on its own timeline, in Logic's MIDI FX slot, routed onward to
//! whatever instrument the musician chose.
//!
//! Everything here forwards [`musa_project::HostedSchedule`], which forwards
//! `musa-notation`'s one MIDI decision path. Nothing musical is decided at
//! this boundary, and nothing is computed here that was not computed before
//! the render thread saw the handle.
//!
//! # Which calls a callback may make
//!
//! [`musa_au_open_schedule`] reads files and allocates; a processor calls it
//! from a worker. Everything else is a *read* of an already-built array:
//! [`musa_au_schedule_lower_bound`] and [`musa_au_schedule_active`] are binary
//! searches into immutable slices, they allocate nothing and lock nothing, and
//! they are the two calls a render block makes. That is what makes a seek cost
//! a search rather than a replay from bar one.

use std::ffi::{CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use musa_project::{
    HostedActiveNote, HostedSchedule, HostedScheduleRequest, HostedTimeline, MidiMode, open_hosted_schedule,
};

use crate::abi::borrow;

/// Positions are seconds on the piece's own exact physical schedule.
pub const MUSA_AU_TIMELINE_PIECE: u32 = 0;
/// Positions are quarter notes on the host's musical timeline.
pub const MUSA_AU_TIMELINE_HOST: u32 = 1;

/// The piece as written: notated durations, no interpretation of dynamics.
pub const MUSA_AU_MIDI_SCORE: u32 = 0;
/// The piece as played: the performance a checked source describes.
pub const MUSA_AU_MIDI_PERFORMANCE: u32 = 1;

/// One scheduled MIDI message at a position on the schedule's timeline.
///
/// The position is a double because a host callback compares it against its
/// own sample time or beat position, and neither of those is rational. The
/// conversion happened once, on the control side; nothing here rounds.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct MusaAuScheduleEvent {
    /// Where the message sounds, in the timeline's own unit.
    pub position: f64,
    /// Which part it came from, as an index into the schedule's parts.
    pub part: u32,
    /// The MIDI status byte, channel included.
    pub status: u8,
    /// The first data byte.
    pub data1: u8,
    /// The second data byte, zero when the status takes only one.
    pub data2: u8,
    /// Must be zero.
    pub reserved: u8,
}

/// One note that is sounding across a position: an attack already given and a
/// release still ahead.
///
/// A host that seeks into the middle of a held note needs to re-attack it, and
/// needs to know when to stop it. Both ends are here so it never has to search
/// backwards for either.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct MusaAuSpan {
    /// Where the attack was, in the timeline's own unit.
    pub start: f64,
    /// Where the release is.
    pub end: f64,
    /// Which part it came from.
    pub part: u32,
    /// The note-on status byte, channel included.
    pub status: u8,
    /// The key.
    pub note: u8,
    /// The attack velocity.
    pub velocity: u8,
    /// Must be zero.
    pub reserved: u8,
}

impl From<HostedActiveNote> for MusaAuSpan {
    fn from(note: HostedActiveNote) -> Self {
        Self {
            start: note.start,
            end: note.end,
            part: note.part,
            status: note.status,
            note: note.note,
            velocity: note.velocity,
            reserved: 0,
        }
    }
}

/// A piece projected as MIDI, or why there is not one. Opaque.
///
/// As with a preparation, a refusal is a result rather than a null return: a
/// polytempo piece asked for on the host's timeline has a reason, and a
/// musician has somewhere to read it.
pub struct MusaAuSchedule {
    schedule: Option<Ready>,
    message: CString,
}

/// A schedule and the strings a C caller reads beside it, built once.
struct Ready {
    schedule: HostedSchedule,
    music: CString,
    assets: CString,
    piece: CString,
    parts: Vec<CString>,
    losses: Vec<CString>,
    /// Scratch the active-note query fills, so a caller with a small buffer
    /// still learns the whole count without this side allocating for it.
    scratch: Vec<HostedActiveNote>,
}

/// Open one piece as a schedule. Never null.
///
/// Control side only. `timeline` is `MUSA_AU_TIMELINE_*` and `mode` is
/// `MUSA_AU_MIDI_*`; a word this library does not know is refused rather than
/// defaulted, because a processor that silently chose the piece's timeline
/// when the document asked for the host's would be playing different music.
///
/// # Safety
/// `project` is NUL-terminated UTF-8. `piece` is the same or null; a null
/// `piece` means the project's first piece.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_open_schedule(
    project: *const c_char,
    piece: *const c_char,
    mode: u32,
    timeline: u32,
) -> *mut MusaAuSchedule {
    let opened = catch_unwind(AssertUnwindSafe(|| {
        let project = unsafe { borrow(project) }?;
        let piece = if piece.is_null() {
            None
        } else {
            Some(unsafe { borrow(piece) }?.to_owned())
        };
        let mode = match mode {
            MUSA_AU_MIDI_SCORE => MidiMode::Score,
            MUSA_AU_MIDI_PERFORMANCE => MidiMode::Performance,
            other => return Err(format!("{other} is not a reading this library knows")),
        };
        let timeline = match timeline {
            MUSA_AU_TIMELINE_PIECE => HostedTimeline::Piece,
            MUSA_AU_TIMELINE_HOST => HostedTimeline::Host,
            other => return Err(format!("{other} is not a timeline this library knows")),
        };
        let schedule = open_hosted_schedule(&HostedScheduleRequest {
            project: PathBuf::from(project),
            piece,
            mode,
            timeline,
        })
        .map_err(|error| error.to_string())?;
        Ready::new(schedule)
    }))
    .unwrap_or_else(|_| Err("opening the schedule panicked".to_owned()));
    let (schedule, message) = match opened {
        Ok(ready) => (Some(ready), String::new()),
        Err(message) => (None, message),
    };
    Box::into_raw(Box::new(MusaAuSchedule {
        schedule,
        message: CString::new(message).unwrap_or_default(),
    }))
}

impl Ready {
    fn new(schedule: HostedSchedule) -> Result<Self, String> {
        let identity = schedule.identity();
        let music = text(&identity.music)?;
        let assets = text(&identity.assets)?;
        let piece = text(&identity.piece)?;
        let parts = schedule
            .parts()
            .iter()
            .map(|part| text(&part.name))
            .collect::<Result<Vec<_>, _>>()?;
        let losses = schedule
            .losses()
            .iter()
            .map(|loss| text(loss))
            .collect::<Result<Vec<_>, _>>()?;
        // One slot per note in the piece is the most any position can hold,
        // so the query never grows this and never allocates.
        let scratch = vec![blank(); schedule.events().len()];
        Ok(Self {
            schedule,
            music,
            assets,
            piece,
            parts,
            losses,
            scratch,
        })
    }
}

/// Whether the piece became a schedule.
///
/// # Safety
/// `schedule` is null, or a live pointer from [`musa_au_open_schedule`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_ok(schedule: *const MusaAuSchedule) -> i32 {
    let Some(schedule) = (unsafe { schedule.as_ref() }) else {
        return 0;
    };
    i32::from(schedule.schedule.is_some())
}

/// Why it did not; empty when it did.
///
/// Borrowed from the schedule and valid until it is released.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_message(schedule: *const MusaAuSchedule) -> *const c_char {
    let Some(schedule) = (unsafe { schedule.as_ref() }) else {
        return c"the schedule is null".as_ptr();
    };
    schedule.message.as_ptr()
}

/// How many messages the whole piece is.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_count(schedule: *const MusaAuSchedule) -> u32 {
    match unsafe { ready(schedule) } {
        Some(ready) => u32::try_from(ready.schedule.events().len()).unwrap_or(u32::MAX),
        None => 0,
    }
}

/// Where the piece ends, in the timeline's own unit.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_extent(schedule: *const MusaAuSchedule) -> f64 {
    unsafe { ready(schedule) }.map_or(0.0, |ready| ready.schedule.extent())
}

/// The index of the first message at or after `position`.
///
/// The whole random-access contract in one call: a block is the half-open
/// range from this at its start to this at its end, so a message on a block
/// boundary is emitted by the later block, exactly once, whatever boundaries
/// the host chose. Callable from a render block.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_lower_bound(schedule: *const MusaAuSchedule, position: f64) -> u32 {
    match unsafe { ready(schedule) } {
        Some(ready) => u32::try_from(ready.schedule.lower_bound(position)).unwrap_or(u32::MAX),
        None => 0,
    }
}

/// One message. Returns zero and writes nothing past the end.
///
/// Callable from a render block.
///
/// # Safety
/// As [`musa_au_schedule_ok`], and `out` addresses one writable
/// [`MusaAuScheduleEvent`] or is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_event(
    schedule: *const MusaAuSchedule,
    index: u32,
    out: *mut MusaAuScheduleEvent,
) -> i32 {
    let Some(out) = (unsafe { out.as_mut() }) else {
        return 0;
    };
    let Some(event) = unsafe { ready(schedule) }.and_then(|ready| ready.schedule.events().get(index as usize)) else {
        return 0;
    };
    *out = MusaAuScheduleEvent {
        position: event.position,
        part: event.part,
        status: event.status,
        data1: event.data[0],
        data2: event.data[1],
        reserved: 0,
    };
    1
}

/// The notes sounding across `position`, for a host that has just seeked.
///
/// Writes up to `capacity` of them into `out` and returns how many there
/// were — which may be more than it wrote, so a caller that offered too few
/// slots learns that rather than silently re-entering part of a chord.
/// Callable from a render block: the search is bounded by a precomputed index
/// and nothing is allocated.
///
/// A note attacked exactly at `position` is not one of these. The block
/// starting there emits its note-on itself, and re-entering it here would
/// sound it twice.
///
/// # Safety
/// As [`musa_au_schedule_ok`], and `out` addresses `capacity` writable
/// [`MusaAuSpan`] values, or is null when `capacity` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_active(
    schedule: *mut MusaAuSchedule,
    position: f64,
    out: *mut MusaAuSpan,
    capacity: u32,
) -> u32 {
    let Some(ready) = (unsafe { schedule.as_mut() }).and_then(|schedule| schedule.schedule.as_mut()) else {
        return 0;
    };
    let found = ready.schedule.active(position, &mut ready.scratch);
    let wrote = found.min(capacity as usize).min(ready.scratch.len());
    if wrote > 0 && !out.is_null() {
        for (index, note) in ready.scratch.iter().take(wrote).enumerate() {
            // In bounds by `wrote`, which is `capacity` at most.
            unsafe { out.add(index).write(MusaAuSpan::from(*note)) };
        }
    }
    u32::try_from(found).unwrap_or(u32::MAX)
}

/// The index the active-note search starts at for `position`.
///
/// Instrumentation, not music: a harness reads it to prove that seeking into
/// the middle of a piece does not begin at its first note. `06-daw-boundary.md`
/// forbids the whole-piece scan; this is how a test sees that it is absent.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_active_scan_start(schedule: *const MusaAuSchedule, position: f64) -> u32 {
    match unsafe { ready(schedule) } {
        Some(ready) => u32::try_from(ready.schedule.active_scan_start(position)).unwrap_or(u32::MAX),
        None => 0,
    }
}

/// How many parts the piece projects, and what each one is.
///
/// The name is borrowed from the schedule; the channel is the one every
/// message of that part carries, which is what makes an event's `part` and its
/// status byte two spellings of the same fact.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_part_count(schedule: *const MusaAuSchedule) -> u32 {
    match unsafe { ready(schedule) } {
        Some(ready) => u32::try_from(ready.parts.len()).unwrap_or(u32::MAX),
        None => 0,
    }
}

/// One part's name, or null past the end.
///
/// Borrowed from the schedule and valid until it is released.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_part_name(schedule: *const MusaAuSchedule, index: u32) -> *const c_char {
    unsafe { ready(schedule) }
        .and_then(|ready| ready.parts.get(index as usize))
        .map_or(std::ptr::null(), |name| name.as_ptr())
}

/// One part's MIDI channel, or 16 past the end.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_part_channel(schedule: *const MusaAuSchedule, index: u32) -> u32 {
    unsafe { ready(schedule) }
        .and_then(|ready| ready.schedule.parts().get(index as usize))
        .map_or(16, |part| u32::from(part.channel))
}

/// How many things this projection could not carry.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_loss_count(schedule: *const MusaAuSchedule) -> u32 {
    match unsafe { ready(schedule) } {
        Some(ready) => u32::try_from(ready.losses.len()).unwrap_or(u32::MAX),
        None => 0,
    }
}

/// One loss as a whole sentence, or null past the end.
///
/// Borrowed from the schedule and valid until it is released.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_loss(schedule: *const MusaAuSchedule, index: u32) -> *const c_char {
    unsafe { ready(schedule) }
        .and_then(|ready| ready.losses.get(index as usize))
        .map_or(std::ptr::null(), |loss| loss.as_ptr())
}

/// Which timeline the positions are counted on, as `MUSA_AU_TIMELINE_*`.
///
/// # Safety
/// As [`musa_au_schedule_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_timeline(schedule: *const MusaAuSchedule) -> u32 {
    match unsafe { ready(schedule) }.map(|ready| ready.schedule.timeline()) {
        Some(HostedTimeline::Piece) | None => MUSA_AU_TIMELINE_PIECE,
        Some(HostedTimeline::Host) => MUSA_AU_TIMELINE_HOST,
    }
}

macro_rules! schedule_identity {
    ($name:ident, $field:ident, $what:literal) => {
        #[doc = concat!("The schedule's ", $what, ", or an empty string when there is none.")]
        ///
        /// Borrowed from the schedule and valid until it is released.
        ///
        /// # Safety
        /// As [`musa_au_schedule_ok`].
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(schedule: *const MusaAuSchedule) -> *const c_char {
            match unsafe { ready(schedule) } {
                Some(ready) => ready.$field.as_ptr(),
                None => c"".as_ptr(),
            }
        }
    };
}

schedule_identity!(musa_au_schedule_identity_music, music, "source identity");
schedule_identity!(musa_au_schedule_identity_assets, assets, "asset-closure identity");
schedule_identity!(musa_au_schedule_identity_piece, piece, "piece name");

/// Release a schedule. Control side only: this frees the whole piece.
///
/// # Safety
/// `schedule` is null, or a live pointer from [`musa_au_open_schedule`] that
/// has not already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn musa_au_schedule_release(schedule: *mut MusaAuSchedule) {
    if schedule.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(schedule) });
}

/// The built half of a live schedule handle.
///
/// # Safety
/// `schedule` is null, or a live pointer from [`musa_au_open_schedule`]. The
/// borrow lives no longer than that handle does, which is the same promise
/// every accessor in this crate rests on.
unsafe fn ready<'a>(schedule: *const MusaAuSchedule) -> Option<&'a Ready> {
    unsafe { schedule.as_ref() }.and_then(|schedule| schedule.schedule.as_ref())
}

fn blank() -> HostedActiveNote {
    HostedActiveNote {
        start: 0.0,
        end: 0.0,
        part: 0,
        status: 0,
        note: 0,
        velocity: 0,
    }
}

/// A C string, refusing an interior NUL rather than truncating at it.
fn text(value: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| format!("`{value}` cannot cross a C boundary: it contains a NUL byte"))
}
