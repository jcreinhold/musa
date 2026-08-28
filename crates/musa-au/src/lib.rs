//! The C ABI a macOS `AUv3` Music Device renders a checked Musa instrument
//! through.
//!
//! The fifth shell, and like the other four it adds no semantics: every
//! musical decision was already made by `musa-project`, and this crate's whole
//! job is to make one prepared instrument reachable from Objective-C without
//! a Rust type, an allocator, a `String`, or a panic crossing the boundary.
//!
//! # The two sides
//!
//! **Control side.** [`musa_au_prepare`] compiles a project, verifies its
//! assets, prepares audio for the host's exact rate, and returns an opaque
//! preparation carrying either an instrument or a diagnostic. It reads files,
//! allocates freely, and takes as long as it takes; an extension calls it from
//! a worker and never from a callback.
//!
//! **Render side.** [`musa_au_render`] consumes events already delivered by
//! the host and fills preallocated buffers. It allocates nothing, locks
//! nothing, performs no I/O, logs nothing, and destroys nothing.
//! [`musa_au_instrument_release`] is what destroys, and it belongs on the
//! control side for that reason.
//!
//! # The rules this boundary keeps
//!
//! - **Nothing unwinds into C.** Every entry point catches, because a panic
//!   crossing a `extern "C"` frame is undefined behavior and a render thread
//!   is the worst place to discover that.
//! - **Nothing borrowed escapes.** Strings handed out are NUL-terminated and
//!   owned by the object that produced them; they live exactly as long as it
//!   does, which the header states beside each one.
//! - **Every pointer is checked for null once, at the boundary.** Past that
//!   the Rust side reasons about references.
//! - **Layout is fixed and asserted on both sides.** [`MusaAuEvent`] is
//!   `#[repr(C)]` here and carries `_Static_assert`s in the generated header,
//!   so a disagreement is a compile error rather than a rendered artifact.
//!
//! # Safety
//!
//! Every `unsafe extern "C"` function here shares one contract, stated once:
//! pointers are either null or were produced by this library and have not been
//! released; string pointers are NUL-terminated UTF-8; `events` points to
//! `count` initialized [`MusaAuEvent`] values; `left` and `right` each point
//! to `frames` writable floats and do not alias; and no handle is used from
//! two threads at once. Each function's own `# Safety` section says only what
//! it adds to that.

#![allow(
    unsafe_code,
    reason = "this crate is a C ABI and nothing else; the workspace ban keeps `unsafe` out of crates that do not need \
              it, and `TRUST.md` names every place this one does"
)]

mod abi;
mod header;
mod instrument;

pub use crate::abi::{
    MUSA_AU_ABI_VERSION, MUSA_AU_MAX_OUTPUTS, MusaAuControl, MusaAuEvent, MusaAuInstrument, MusaAuPreparation,
    musa_au_abi_version, musa_au_instrument_release, musa_au_preparation_control, musa_au_preparation_control_count,
    musa_au_preparation_control_display, musa_au_preparation_control_identity, musa_au_preparation_control_kind,
    musa_au_preparation_control_summary, musa_au_preparation_control_table, musa_au_preparation_control_update_rate,
    musa_au_preparation_identity_assets, musa_au_preparation_identity_music, musa_au_preparation_identity_part,
    musa_au_preparation_identity_piece, musa_au_preparation_input, musa_au_preparation_input_count,
    musa_au_preparation_loss, musa_au_preparation_loss_count, musa_au_preparation_message, musa_au_preparation_ok,
    musa_au_preparation_output, musa_au_preparation_output_count, musa_au_preparation_output_role,
    musa_au_preparation_release, musa_au_preparation_take, musa_au_prepare, musa_au_render, musa_au_render_outputs,
    musa_au_rendered_frames, musa_au_reset, musa_au_unbound_events,
};
pub use crate::abi::{
    MUSA_AU_CONTROL_CONTINUOUS, MUSA_AU_EVENT_CHANNEL_PRESSURE, MUSA_AU_EVENT_CONTROLLER, MUSA_AU_EVENT_KEY_PRESSURE,
    MUSA_AU_EVENT_NOTE_OFF, MUSA_AU_EVENT_NOTE_ON, MUSA_AU_EVENT_PARAMETER, MUSA_AU_EVENT_PITCH_BEND,
};
pub use crate::header::header;

#[cfg(test)]
extern crate self as musa_au;

#[cfg(test)]
#[path = "../tests/suite/main.rs"]
mod suite;
