//! The negative half of the contract in `../../TRUST.md`.
//!
//! Every entry point takes pointers a caller could get wrong, and the crate
//! promises to answer rather than crash. These are the cases that can be
//! exercised from Rust: a violated aliasing or threading obligation is
//! undefined behavior, so testing it would be testing nothing.

use std::ffi::CString;

use musa_au::{
    MUSA_AU_ABI_VERSION, MUSA_AU_EVENT_NOTE_ON, MusaAuEvent, musa_au_abi_version, musa_au_instrument_release,
    musa_au_preparation_input, musa_au_preparation_input_count, musa_au_preparation_message, musa_au_preparation_ok,
    musa_au_preparation_release, musa_au_preparation_take, musa_au_prepare, musa_au_render, musa_au_rendered_frames,
    musa_au_reset, musa_au_unbound_events,
};

use super::differential::{Fixture, RATE, prepare};

/// The version a caller reads is the version this was built as.
#[test]
fn the_library_states_its_abi_version() {
    assert_eq!(musa_au_abi_version(), MUSA_AU_ABI_VERSION);
}

/// A null handle is answered, not dereferenced.
#[test]
fn every_accessor_tolerates_a_null_handle() {
    assert_eq!(unsafe { musa_au_preparation_ok(std::ptr::null()) }, 0);
    assert_eq!(unsafe { musa_au_preparation_input_count(std::ptr::null()) }, 0);
    assert!(unsafe { musa_au_preparation_input(std::ptr::null(), 0) }.is_null());
    assert!(unsafe { musa_au_preparation_take(std::ptr::null_mut()) }.is_null());
    assert_eq!(unsafe { musa_au_unbound_events(std::ptr::null()) }, 0);
    assert_eq!(unsafe { musa_au_rendered_frames(std::ptr::null()) }, 0);
    let message = unsafe { std::ffi::CStr::from_ptr(musa_au_preparation_message(std::ptr::null())) };
    assert!(!message.to_bytes().is_empty(), "a null preparation says so");
    // Releasing and rendering nothing are no-ops rather than crashes.
    unsafe { musa_au_preparation_release(std::ptr::null_mut()) };
    unsafe { musa_au_instrument_release(std::ptr::null_mut()) };
    unsafe { musa_au_reset(std::ptr::null_mut()) };
    unsafe {
        musa_au_render(
            std::ptr::null_mut(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        );
    }
}

/// A null string argument is a diagnosed preparation, not a crash.
#[test]
fn a_null_path_is_diagnosed() {
    let part = c"piano";
    let preparation = unsafe { musa_au_prepare(std::ptr::null(), std::ptr::null(), part.as_ptr(), RATE) };
    assert!(!preparation.is_null());
    assert_eq!(unsafe { musa_au_preparation_ok(preparation) }, 0);
    let message = unsafe { std::ffi::CStr::from_ptr(musa_au_preparation_message(preparation)) };
    assert!(message.to_string_lossy().contains("null"), "{message:?}");
    // A failed preparation still answers every accessor, with nothing.
    assert!(unsafe { musa_au_preparation_take(preparation) }.is_null());
    assert_eq!(unsafe { musa_au_preparation_input_count(preparation) }, 0);
    unsafe { musa_au_preparation_release(preparation) };
}

/// A path that is not valid UTF-8 is refused by name.
#[test]
fn a_path_that_is_not_utf8_is_refused() {
    // 0xFF is not a valid UTF-8 byte in any position.
    let path = CString::new(vec![b'/', 0xFF, b'x']).expect("no interior NUL");
    let part = c"piano";
    let preparation = unsafe { musa_au_prepare(path.as_ptr(), std::ptr::null(), part.as_ptr(), RATE) };
    assert_eq!(unsafe { musa_au_preparation_ok(preparation) }, 0);
    let message = unsafe { std::ffi::CStr::from_ptr(musa_au_preparation_message(preparation)) };
    assert!(message.to_string_lossy().contains("UTF-8"), "{message:?}");
    unsafe { musa_au_preparation_release(preparation) };
}

/// A rate of zero is refused rather than prepared for.
#[test]
fn a_zero_sample_rate_is_refused() {
    let fixture = Fixture::new();
    let preparation = unsafe { musa_au_prepare(fixture.project.as_ptr(), std::ptr::null(), fixture.part.as_ptr(), 0) };
    assert_eq!(unsafe { musa_au_preparation_ok(preparation) }, 0);
    unsafe { musa_au_preparation_release(preparation) };
}

/// A part the piece does not have is refused by name.
#[test]
fn an_absent_part_is_refused_by_name() {
    let fixture = Fixture::new();
    let part = c"contrabassoon";
    let preparation = unsafe { musa_au_prepare(fixture.project.as_ptr(), std::ptr::null(), part.as_ptr(), RATE) };
    assert_eq!(unsafe { musa_au_preparation_ok(preparation) }, 0);
    let message = unsafe { std::ffi::CStr::from_ptr(musa_au_preparation_message(preparation)) };
    assert!(message.to_string_lossy().contains("contrabassoon"), "{message:?}");
    unsafe { musa_au_preparation_release(preparation) };
}

/// An instrument is taken once; a second take is null, not a second handle.
#[test]
fn a_preparation_yields_one_instrument() {
    let fixture = Fixture::new();
    let preparation =
        unsafe { musa_au_prepare(fixture.project.as_ptr(), std::ptr::null(), fixture.part.as_ptr(), RATE) };
    let first = unsafe { musa_au_preparation_take(preparation) };
    assert!(!first.is_null());
    let second = unsafe { musa_au_preparation_take(preparation) };
    assert!(second.is_null(), "a preparation must not hand out two instruments");
    unsafe { musa_au_preparation_release(preparation) };
    unsafe { musa_au_instrument_release(first) };
}

/// A zero-frame block is a legal no-op, and a null event pointer with a
/// nonzero count is treated as no events rather than dereferenced.
#[test]
fn a_degenerate_block_renders_nothing() {
    let fixture = Fixture::new();
    let instrument = prepare(&fixture);
    let mut left = [0.0f32; 4];
    let mut right = [0.0f32; 4];
    unsafe {
        musa_au_render(
            instrument,
            std::ptr::null(),
            9,
            left.as_mut_ptr(),
            right.as_mut_ptr(),
            0,
        );
    }
    assert_eq!(unsafe { musa_au_rendered_frames(instrument) }, 0);
    assert_eq!(left, [0.0; 4]);
    unsafe { musa_au_instrument_release(instrument) };
}

/// An event kind this version does not define is counted, never guessed at.
#[test]
fn an_unknown_event_kind_is_counted() {
    let fixture = Fixture::new();
    let instrument = prepare(&fixture);
    let mut left = [0.0f32; 8];
    let mut right = [0.0f32; 8];
    let events = [
        MusaAuEvent {
            frame: 0,
            voice: 1,
            kind: MUSA_AU_EVENT_NOTE_ON,
            data1: 60,
            data2: 100,
            reserved: 0,
        },
        MusaAuEvent {
            frame: 1,
            voice: 0,
            kind: 200,
            data1: 0,
            data2: 0,
            reserved: 0,
        },
    ];
    unsafe {
        musa_au_render(instrument, events.as_ptr(), 2, left.as_mut_ptr(), right.as_mut_ptr(), 8);
    }
    assert_eq!(unsafe { musa_au_unbound_events(instrument) }, 1);
    unsafe { musa_au_instrument_release(instrument) };
}

/// An event past the end of the block still happens.
///
/// Dropping it would lose a note-off and hang a voice, which is the failure a
/// host would report as "Musa keeps sounding after I stop".
#[test]
fn an_event_past_the_block_is_still_applied() {
    let fixture = Fixture::new();
    let instrument = prepare(&fixture);
    let mut left = [0.0f32; 4];
    let mut right = [0.0f32; 4];
    let events = [MusaAuEvent {
        frame: 4_000,
        voice: 1,
        kind: 200,
        data1: 0,
        data2: 0,
        reserved: 0,
    }];
    unsafe {
        musa_au_render(instrument, events.as_ptr(), 1, left.as_mut_ptr(), right.as_mut_ptr(), 4);
    }
    assert_eq!(
        unsafe { musa_au_unbound_events(instrument) },
        1,
        "an event past the last frame was dropped"
    );
    unsafe { musa_au_instrument_release(instrument) };
}
