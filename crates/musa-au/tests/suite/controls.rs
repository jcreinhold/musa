//! The parameter tree and the output buses, across the C boundary.
//!
//! `06-daw-boundary.md` Rule D2 decides what may be published, and the tests
//! that check *which* controls those are live beside the projection, in
//! `musa-project`. What is measured here is the crossing itself: that the
//! descriptors survive as C data, that a host parameter event reaches the
//! prepared instrument through the same declared mapping a MIDI event does,
//! that neither the host's block size nor the number of outputs it enabled
//! changes what bus zero says, and that the address table a document carries
//! comes back meaning the same thing.

use std::ffi::{CStr, CString};

use musa_au::{
    MUSA_AU_CONTROL_CONTINUOUS, MUSA_AU_EVENT_NOTE_OFF, MUSA_AU_EVENT_NOTE_ON, MusaAuControl, MusaAuEvent,
    MusaAuInstrument, MusaAuPreparation, musa_au_instrument_release, musa_au_preparation_control,
    musa_au_preparation_control_count, musa_au_preparation_control_display, musa_au_preparation_control_identity,
    musa_au_preparation_control_kind, musa_au_preparation_control_summary, musa_au_preparation_control_table,
    musa_au_preparation_control_update_rate, musa_au_preparation_loss, musa_au_preparation_loss_count,
    musa_au_preparation_ok, musa_au_preparation_output, musa_au_preparation_output_count,
    musa_au_preparation_output_role, musa_au_preparation_release, musa_au_preparation_take, musa_au_prepare,
    musa_au_render_outputs, musa_au_unbound_events,
};

use super::differential::{Fixture, RATE, message, prepare, render};

/// The multi-output example, by the path a C caller would pass.
fn glass_mountain() -> CString {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/glass-mountain.musa");
    CString::new(path.to_string_lossy().into_owned()).expect("no NUL in a repository path")
}

/// Prepare an arbitrary project and part, carrying an optional saved table.
fn open(project: &CStr, part: &CStr, table: Option<&CStr>) -> *mut MusaAuPreparation {
    let preparation = unsafe {
        musa_au_prepare(
            project.as_ptr(),
            std::ptr::null(),
            part.as_ptr(),
            RATE,
            table.map_or(std::ptr::null(), CStr::as_ptr),
        )
    };
    assert!(!preparation.is_null(), "preparation is never null");
    assert_eq!(unsafe { musa_au_preparation_ok(preparation) }, 1, "{}", unsafe {
        message(preparation)
    });
    preparation
}

/// A fresh prepared instrument, and nothing carried over from a previous one.
///
/// Deliberately not `reset` between renders: a reset silences voices, but it
/// leaves a parameter where the host last put it and lets the studio's tail
/// keep decaying. Comparing two renders means comparing two instruments that
/// started from the same place, and only preparation gives that.
fn take(project: &CStr, part: &CStr) -> *mut MusaAuInstrument {
    let preparation = open(project, part, None);
    let instrument = unsafe { musa_au_preparation_take(preparation) };
    assert!(!instrument.is_null());
    unsafe { musa_au_preparation_release(preparation) };
    instrument
}

unsafe fn borrowed(value: *const std::ffi::c_char) -> String {
    assert!(!value.is_null(), "a published string is never null");
    unsafe { CStr::from_ptr(value) }.to_string_lossy().into_owned()
}

fn control(preparation: *const MusaAuPreparation, index: u32) -> MusaAuControl {
    let mut out = MusaAuControl {
        address: 0,
        minimum: 0.0,
        maximum: 0.0,
        default_value: 0.0,
        flags: 0,
    };
    assert_eq!(
        unsafe { musa_au_preparation_control(preparation, index, &raw mut out) },
        1,
        "control {index} is published"
    );
    out
}

fn address_of(preparation: *const MusaAuPreparation, identity: &str) -> u64 {
    let count = unsafe { musa_au_preparation_control_count(preparation) };
    (0..count)
        .find(|&index| unsafe { borrowed(musa_au_preparation_control_identity(preparation, index)) } == identity)
        .map(|index| control(preparation, index).address)
        .expect("the identity this test names is published")
}

#[test]
fn every_descriptor_crosses_the_boundary_whole() {
    let fixture = Fixture::new();
    let preparation = open(&fixture.project, &fixture.part, None);
    let count = unsafe { musa_au_preparation_control_count(preparation) };
    assert_eq!(count, 5, "the edition instrument declares five normalized controls");
    for index in 0..count {
        let numbers = control(preparation, index);
        assert_ne!(numbers.address, 0);
        assert_eq!((numbers.minimum, numbers.maximum), (0.0, 1.0));
        assert!(numbers.default_value >= 0.0 && numbers.default_value <= 1.0);
        for text in [
            unsafe { borrowed(musa_au_preparation_control_identity(preparation, index)) },
            unsafe { borrowed(musa_au_preparation_control_display(preparation, index)) },
            unsafe { borrowed(musa_au_preparation_control_summary(preparation, index)) },
            unsafe { borrowed(musa_au_preparation_control_update_rate(preparation, index)) },
        ] {
            assert!(!text.is_empty(), "control {index} crossed with an empty field");
        }
        assert_eq!(
            unsafe { borrowed(musa_au_preparation_control_kind(preparation, index)) },
            "Normalized"
        );
    }
    // `expression` is continuous and `emphasis` is not, and the flag says so
    // rather than a host guessing from the name.
    let expression = (0..count)
        .find(|&index| unsafe { borrowed(musa_au_preparation_control_display(preparation, index)) } == "expression")
        .expect("expression is published");
    assert_eq!(
        control(preparation, expression).flags & MUSA_AU_CONTROL_CONTINUOUS,
        MUSA_AU_CONTROL_CONTINUOUS
    );
    let emphasis = (0..count)
        .find(|&index| unsafe { borrowed(musa_au_preparation_control_display(preparation, index)) } == "emphasis")
        .expect("emphasis is published");
    assert_eq!(control(preparation, emphasis).flags & MUSA_AU_CONTROL_CONTINUOUS, 0);
    unsafe { musa_au_preparation_release(preparation) };
}

#[test]
fn a_control_that_could_not_be_carried_crosses_as_a_sentence() {
    let fixture = Fixture::new();
    let preparation = open(&fixture.project, &fixture.part, None);
    let count = unsafe { musa_au_preparation_loss_count(preparation) };
    assert_eq!(count, 2, "a typed relation and a domainless ratio stayed behind");
    for index in 0..count {
        let text = unsafe { borrowed(musa_au_preparation_loss(preparation, index)) };
        assert!(text.contains("is not a host parameter"), "{text}");
    }
    assert!(unsafe { musa_au_preparation_loss(preparation, count) }.is_null());
    unsafe { musa_au_preparation_release(preparation) };
}

#[test]
fn the_address_table_a_document_carries_comes_back_meaning_the_same_thing() {
    let fixture = Fixture::new();
    let first = open(&fixture.project, &fixture.part, None);
    let table = unsafe { borrowed(musa_au_preparation_control_table(first)) };
    let before: Vec<u64> = (0..unsafe { musa_au_preparation_control_count(first) })
        .map(|index| control(first, index).address)
        .collect();
    unsafe { musa_au_preparation_release(first) };

    let carried = CString::new(table).expect("a table has no NUL");
    let again = open(&fixture.project, &fixture.part, Some(&carried));
    let after: Vec<u64> = (0..unsafe { musa_au_preparation_control_count(again) })
        .map(|index| control(again, index).address)
        .collect();
    assert_eq!(before, after);
    unsafe { musa_au_preparation_release(again) };
}

#[test]
fn a_table_the_boundary_cannot_read_refuses_rather_than_starting_over() {
    let fixture = Fixture::new();
    let preparation = unsafe {
        musa_au_prepare(
            fixture.project.as_ptr(),
            std::ptr::null(),
            fixture.part.as_ptr(),
            RATE,
            c"nonsense".as_ptr(),
        )
    };
    assert!(!preparation.is_null());
    assert_eq!(unsafe { musa_au_preparation_ok(preparation) }, 0);
    let refusal = unsafe { message(preparation) };
    assert!(refusal.contains("control table"), "{refusal}");
    unsafe { musa_au_preparation_release(preparation) };
}

#[test]
fn a_parameter_event_reaches_the_instrument() {
    let fixture = Fixture::new();
    let preparation = open(&fixture.project, &fixture.part, None);
    let address = address_of(preparation, "std.performance::expression");
    unsafe { musa_au_preparation_release(preparation) };
    let energy = |value: f32| {
        let instrument = take(&fixture.project, &fixture.part);
        let frames = render(
            instrument,
            &[
                MusaAuEvent::parameter(0, address, value, 0),
                MusaAuEvent::midi(1, 1, MUSA_AU_EVENT_NOTE_ON, 60, 100),
            ],
            4_096,
            512,
        );
        unsafe { musa_au_instrument_release(instrument) };
        frames.iter().map(|sample| sample.abs()).sum::<f32>()
    };
    assert!(
        energy(1.0) > energy(0.05) * 2.0,
        "a published parameter changed nothing that can be heard"
    );
}

#[test]
fn an_address_the_source_does_not_publish_is_counted_rather_than_guessed_at() {
    let fixture = Fixture::new();
    let instrument = prepare(&fixture);
    render(instrument, &[MusaAuEvent::parameter(0, 0xdead_beef, 0.5, 0)], 128, 128);
    assert_eq!(
        unsafe { musa_au_unbound_events(instrument) },
        1,
        "an address nothing answers to is a loss the host can read"
    );
    unsafe { musa_au_instrument_release(instrument) };
}

#[test]
fn a_parameter_history_is_the_same_music_under_every_block_partition() {
    // `06-daw-boundary.md` §4: a block boundary is not meaning, and Theorem
    // R1-batch says a different partition of the same frames is
    // unobservable. Parameter events are scheduled by the host at sample
    // offsets, so they are exactly where that could go wrong.
    let fixture = Fixture::new();
    let preparation = open(&fixture.project, &fixture.part, None);
    let expression = address_of(preparation, "std.performance::expression");
    let brightness = address_of(preparation, "std.performance::brightness");
    unsafe { musa_au_preparation_release(preparation) };

    let history = [
        MusaAuEvent::parameter(0, expression, 1.0, 0),
        MusaAuEvent::midi(11, 1, MUSA_AU_EVENT_NOTE_ON, 60, 100),
        MusaAuEvent::parameter(129, brightness, 0.75, 0),
        MusaAuEvent::parameter(577, expression, 0.2, 640),
        MusaAuEvent::midi(1_000, 2, MUSA_AU_EVENT_NOTE_ON, 67, 90),
        MusaAuEvent::parameter(1_301, brightness, 0.1, 128),
        MusaAuEvent::midi(2_048, 1, MUSA_AU_EVENT_NOTE_OFF, 60, 0),
    ];
    let frames = 6_000;
    let once = |block: usize| {
        let instrument = take(&fixture.project, &fixture.part);
        let rendered = render(instrument, &history, frames, block);
        unsafe { musa_au_instrument_release(instrument) };
        rendered
    };
    let reference = once(frames);
    for block in [1, 17, 64, 512, 4_096] {
        let partitioned = once(block);
        assert_eq!(
            reference.len(),
            partitioned.len(),
            "a partition of {block} produced a different number of frames"
        );
        assert!(
            reference == partitioned,
            "the host's block size of {block} changed the music"
        );
    }
}

// === Outputs ===============================================================

#[test]
fn the_declared_outputs_cross_with_their_names_and_roles() {
    let project = glass_mountain();
    let preparation = open(&project, c"violin", None);
    let count = unsafe { musa_au_preparation_output_count(preparation) };
    let published: Vec<(String, u32)> = (0..count)
        .map(|index| {
            (
                unsafe { borrowed(musa_au_preparation_output(preparation, index)) },
                unsafe { musa_au_preparation_output_role(preparation, index) },
            )
        })
        .collect();
    assert_eq!(
        published,
        [("main".to_owned(), 0), ("violin".to_owned(), 1), ("hall".to_owned(), 2),]
    );
    assert!(unsafe { musa_au_preparation_output(preparation, count) }.is_null());
    unsafe { musa_au_preparation_release(preparation) };
}

/// Render `frames` into `outputs` stereo buffers, one block at a time.
fn render_outputs(
    instrument: *mut MusaAuInstrument,
    events: &[MusaAuEvent],
    frames: usize,
    block: usize,
    outputs: usize,
) -> Vec<Vec<f32>> {
    let mut collected = vec![Vec::with_capacity(frames * 2); outputs];
    let mut buffers = vec![vec![0.0f32; block]; outputs * 2];
    let mut start = 0usize;
    while start < frames {
        let count = block.min(frames - start);
        let end = start + count;
        let inside: Vec<MusaAuEvent> = events
            .iter()
            .filter(|event| (event.frame as usize) >= start && (event.frame as usize) < end)
            .map(|event| MusaAuEvent {
                frame: event.frame - u32::try_from(start).expect("a block start fits"),
                ..*event
            })
            .collect();
        for buffer in &mut buffers {
            buffer[..count].fill(0.0);
        }
        let mut channels: Vec<*mut f32> = buffers.iter_mut().map(|buffer| buffer.as_mut_ptr()).collect();
        unsafe {
            musa_au_render_outputs(
                instrument,
                if inside.is_empty() {
                    std::ptr::null()
                } else {
                    inside.as_ptr()
                },
                u32::try_from(inside.len()).expect("a block holds few events"),
                channels.as_mut_ptr(),
                u32::try_from(channels.len()).expect("a handful of channels"),
                u32::try_from(count).expect("a block fits"),
            );
        }
        for (output, into) in collected.iter_mut().enumerate() {
            let left = &buffers[output * 2];
            let right = &buffers[output * 2 + 1];
            for (l, r) in left.iter().zip(right).take(count) {
                into.push(*l);
                into.push(*r);
            }
        }
        start = end;
    }
    collected
}

#[test]
fn bus_zero_does_not_depend_on_how_many_outputs_a_host_enabled() {
    // The documented GarageBand fallback: a host that exposes one stereo
    // output gets the same main output as one that takes all three. Nothing
    // about the mix changes to make the fallback work.
    let project = glass_mountain();
    let events = [
        MusaAuEvent::midi(0, 1, MUSA_AU_EVENT_NOTE_ON, 64, 100),
        MusaAuEvent::midi(3_000, 1, MUSA_AU_EVENT_NOTE_OFF, 64, 0),
    ];
    let once = |outputs: usize| {
        let instrument = take(&project, c"violin");
        let rendered = render_outputs(instrument, &events, 8_000, 256, outputs);
        unsafe { musa_au_instrument_release(instrument) };
        rendered
    };
    let all = once(3);
    let alone = once(1);
    assert!(all[0] == alone[0], "the main output changed with the bus count");
    assert!(
        all[1].iter().any(|sample| *sample != 0.0),
        "the part output delivered nothing"
    );
    assert!(
        all[2].iter().any(|sample| *sample != 0.0),
        "the room the part sends to delivered nothing"
    );
    assert!(all[1] != all[0], "the part output is not the main output");
}

#[test]
fn an_output_render_is_the_same_music_under_every_block_partition() {
    let project = glass_mountain();
    let preparation = open(&project, c"violin", None);
    let expression = address_of(preparation, "std.performance::expression");
    unsafe { musa_au_preparation_release(preparation) };
    let events = [
        MusaAuEvent::midi(0, 1, MUSA_AU_EVENT_NOTE_ON, 64, 100),
        MusaAuEvent::parameter(301, expression, 0.3, 512),
        MusaAuEvent::midi(2_000, 1, MUSA_AU_EVENT_NOTE_OFF, 64, 0),
    ];
    let frames = 4_000;
    let once = |block: usize| {
        let instrument = take(&project, c"violin");
        let rendered = render_outputs(instrument, &events, frames, block, 3);
        unsafe { musa_au_instrument_release(instrument) };
        rendered
    };
    let reference = once(frames);
    for block in [1, 33, 512] {
        assert!(
            reference == once(block),
            "a block size of {block} changed what an output said"
        );
    }
}

#[test]
fn a_channel_count_that_is_not_a_stereo_pair_renders_nothing() {
    let fixture = Fixture::new();
    let instrument = prepare(&fixture);
    let mut left = vec![7.0f32; 64];
    let mut channels = [left.as_mut_ptr()];
    unsafe {
        musa_au_render_outputs(instrument, std::ptr::null(), 0, channels.as_mut_ptr(), 1, 64);
    }
    // Untouched: an odd channel count is a caller that has not said which
    // channel it means, and guessing would write into memory on that guess.
    assert!(left.iter().all(|sample| *sample == 7.0));
    unsafe {
        musa_au_render_outputs(instrument, std::ptr::null(), 0, std::ptr::null(), 0, 64);
    }
    unsafe { musa_au_instrument_release(instrument) };
}
