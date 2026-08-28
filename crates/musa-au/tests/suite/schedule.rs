//! The MIDI Processor half of the boundary, at the ABI.
//!
//! `hosted_schedule_laws.rs` in `musa-project` establishes what a schedule
//! *means*. What is left for this side is that the meaning survives the C
//! boundary: a position crosses as a double and comes back naming the same
//! message, a caller with too small a buffer is told so rather than handed a
//! truncated chord, and every entry point answers a null handle rather than
//! dereferencing it.

use std::ffi::{CStr, CString};

use musa_au::{
    MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_MIDI_SCORE, MUSA_AU_TIMELINE_HOST, MUSA_AU_TIMELINE_PIECE, MusaAuSchedule,
    MusaAuScheduleEvent, MusaAuSpan, musa_au_open_schedule, musa_au_schedule_active,
    musa_au_schedule_active_scan_start, musa_au_schedule_count, musa_au_schedule_event, musa_au_schedule_extent,
    musa_au_schedule_identity_music, musa_au_schedule_loss, musa_au_schedule_loss_count, musa_au_schedule_lower_bound,
    musa_au_schedule_message, musa_au_schedule_ok, musa_au_schedule_part_channel, musa_au_schedule_part_count,
    musa_au_schedule_part_name, musa_au_schedule_release, musa_au_schedule_timeline,
};

fn path(name: &str) -> CString {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    CString::new(path.to_string_lossy().into_owned()).expect("no NUL in a repository path")
}

/// A schedule that opened, or the reason it did not, as a panic naming it.
fn open(name: &str, mode: u32, timeline: u32) -> *mut MusaAuSchedule {
    let project = path(name);
    let schedule = unsafe { musa_au_open_schedule(project.as_ptr(), std::ptr::null(), mode, timeline) };
    assert_eq!(
        unsafe { musa_au_schedule_ok(schedule) },
        1,
        "{}",
        unsafe { CStr::from_ptr(musa_au_schedule_message(schedule)) }.to_string_lossy()
    );
    schedule
}

fn events(schedule: *const MusaAuSchedule) -> Vec<MusaAuScheduleEvent> {
    (0..unsafe { musa_au_schedule_count(schedule) })
        .map(|index| {
            let mut out = blank_event();
            assert_eq!(unsafe { musa_au_schedule_event(schedule, index, &raw mut out) }, 1);
            out
        })
        .collect()
}

#[test]
fn a_piece_crosses_as_a_finite_schedule_a_host_can_read_by_index() {
    let schedule = open("invention.musa", MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_TIMELINE_PIECE);
    let messages = events(schedule);
    assert!(!messages.is_empty());
    assert!(messages.windows(2).all(|pair| pair[0].position <= pair[1].position));
    assert!(messages.iter().all(|event| event.reserved == 0));
    assert_eq!(
        unsafe { musa_au_schedule_extent(schedule) },
        messages.last().expect("messages").position
    );
    assert_eq!(unsafe { musa_au_schedule_timeline(schedule) }, MUSA_AU_TIMELINE_PIECE);

    let mut past = blank_event();
    assert_eq!(
        unsafe {
            musa_au_schedule_event(
                schedule,
                u32::try_from(messages.len()).expect("a small piece"),
                &raw mut past,
            )
        },
        0,
        "past the end writes nothing"
    );
    unsafe { musa_au_schedule_release(schedule) };
}

#[test]
fn a_block_is_a_half_open_range_of_indices() {
    let schedule = open("invention.musa", MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_TIMELINE_PIECE);
    let count = unsafe { musa_au_schedule_count(schedule) };
    let extent = unsafe { musa_au_schedule_extent(schedule) };
    let width = 0.125_f64;
    let blocks = ((extent / width).ceil() as usize).saturating_add(2);
    let mut seen = Vec::new();
    for block in 0..blocks {
        let boundary = |at: usize| unsafe { musa_au_schedule_lower_bound(schedule, width * at as f64) };
        seen.extend(boundary(block)..boundary(block + 1));
    }
    assert_eq!(seen, (0..count).collect::<Vec<_>>(), "every message, exactly once");
    unsafe { musa_au_schedule_release(schedule) };
}

#[test]
fn a_seek_costs_a_search_and_not_a_replay() {
    // §6 of `93-the-audio-unit-shape.md` measured this on the trial: after a
    // seek the reading starts at the index the position names.
    let schedule = open("invention.musa", MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_TIMELINE_PIECE);
    let extent = unsafe { musa_au_schedule_extent(schedule) };
    let messages = events(schedule);
    let index = unsafe { musa_au_schedule_lower_bound(schedule, extent / 2.0) } as usize;
    assert!(index > 0 && index < messages.len());
    assert!(messages[index - 1].position < extent / 2.0);
    assert!(messages[index].position >= extent / 2.0);
    assert!(
        unsafe { musa_au_schedule_active_scan_start(schedule, extent * 0.9) } > 0,
        "a seek near the end still began at the first note"
    );
    unsafe { musa_au_schedule_release(schedule) };
}

#[test]
fn the_notes_a_seek_lands_inside_cross_with_both_their_ends() {
    let schedule = open("counterpoint.musa", MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_TIMELINE_PIECE);
    let extent = unsafe { musa_au_schedule_extent(schedule) };
    let mut into = [blank_span(); 64];
    let mut ever = 0usize;
    for step in 1..40 {
        let position = extent * f64::from(step) / 40.0;
        let found = unsafe { musa_au_schedule_active(schedule, position, into.as_mut_ptr(), 64) } as usize;
        ever = ever.max(found);
        for span in into.iter().take(found) {
            assert!(span.start < position && span.end > position, "at {position}");
            assert_eq!(span.status & 0xF0, 0x90, "a sounding note is an attack");
            assert!(span.velocity > 0);
            assert_eq!(span.reserved, 0);
        }
    }
    assert!(ever > 0, "the fixture holds notes across a position");
    unsafe { musa_au_schedule_release(schedule) };
}

#[test]
fn a_host_with_too_few_slots_is_told_how_many_there_were() {
    let schedule = open("counterpoint.musa", MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_TIMELINE_PIECE);
    let extent = unsafe { musa_au_schedule_extent(schedule) };
    let mut into = [blank_span(); 64];
    let (position, full) = (1..400)
        .map(|step| extent * f64::from(step) / 400.0)
        .map(|position| {
            let found = unsafe { musa_au_schedule_active(schedule, position, into.as_mut_ptr(), 64) };
            (position, found)
        })
        .find(|(_, found)| *found > 1)
        .expect("the fixture holds two notes at once somewhere");

    let mut cramped = [blank_span(); 1];
    assert_eq!(
        unsafe { musa_au_schedule_active(schedule, position, cramped.as_mut_ptr(), 1) },
        full,
        "the count is the truth even when the buffer is not"
    );
    assert_eq!(cramped[0], into[0], "and the slot it did have was filled");

    // A capacity of zero is a question, not a write.
    assert_eq!(
        unsafe { musa_au_schedule_active(schedule, position, std::ptr::null_mut(), 0) },
        full
    );
    unsafe { musa_au_schedule_release(schedule) };
}

#[test]
fn the_parts_a_schedule_names_are_the_channels_its_messages_carry() {
    let schedule = open("counterpoint.musa", MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_TIMELINE_PIECE);
    let count = unsafe { musa_au_schedule_part_count(schedule) };
    assert!(count > 0);
    let channels: Vec<u32> = (0..count)
        .map(|index| {
            let name = unsafe { CStr::from_ptr(musa_au_schedule_part_name(schedule, index)) };
            assert!(!name.to_bytes().is_empty(), "a part the score named");
            unsafe { musa_au_schedule_part_channel(schedule, index) }
        })
        .collect();
    for event in events(schedule) {
        let channel = u32::from(event.status & 0x0F);
        assert_eq!(
            channels.get(event.part as usize),
            Some(&channel),
            "a message's part and its status byte are two spellings of one fact"
        );
    }
    assert!(unsafe { musa_au_schedule_part_name(schedule, count) }.is_null());
    assert_eq!(unsafe { musa_au_schedule_part_channel(schedule, count) }, 16);
    unsafe { musa_au_schedule_release(schedule) };
}

#[test]
fn the_two_readings_and_the_two_timelines_are_four_different_documents() {
    let played = open("invention.musa", MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_TIMELINE_PIECE);
    let written = open("invention.musa", MUSA_AU_MIDI_SCORE, MUSA_AU_TIMELINE_PIECE);
    let on_host = open("invention.musa", MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_TIMELINE_HOST);

    let spelling = |schedule: *const MusaAuSchedule| -> Vec<(u32, u8, u8, u8)> {
        events(schedule)
            .into_iter()
            .map(|event| (event.part, event.status, event.data1, event.data2))
            .collect()
    };
    assert_eq!(
        spelling(played),
        spelling(on_host),
        "the timeline moves the messages and does not change them"
    );
    assert_ne!(
        spelling(played),
        spelling(written),
        "the two readings are different music"
    );
    assert_ne!(
        unsafe { musa_au_schedule_extent(played) },
        unsafe { musa_au_schedule_extent(on_host) },
        "seconds and quarter notes are different numbers"
    );
    assert_eq!(unsafe { musa_au_schedule_timeline(on_host) }, MUSA_AU_TIMELINE_HOST);

    for schedule in [played, written, on_host] {
        unsafe { musa_au_schedule_release(schedule) };
    }
}

#[test]
fn a_schedule_states_what_it_could_not_carry_and_what_it_came_from() {
    let schedule = open("invention.musa", MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_TIMELINE_PIECE);
    let count = unsafe { musa_au_schedule_loss_count(schedule) };
    assert!(count > 0, "a MIDI effect has nowhere to put the conductor");
    let losses: Vec<String> = (0..count)
        .map(|index| {
            unsafe { CStr::from_ptr(musa_au_schedule_loss(schedule, index)) }
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert!(losses.iter().all(|loss| loss.contains(' ')), "{losses:?} are sentences");
    assert!(unsafe { musa_au_schedule_loss(schedule, count) }.is_null());
    let music = unsafe { CStr::from_ptr(musa_au_schedule_identity_music(schedule)) };
    assert!(!music.to_bytes().is_empty());
    unsafe { musa_au_schedule_release(schedule) };
}

#[test]
fn a_piece_with_no_single_grid_is_refused_rather_than_flattened() {
    let project = path("canon-x.musa");
    let refused = unsafe {
        musa_au_open_schedule(
            project.as_ptr(),
            std::ptr::null(),
            MUSA_AU_MIDI_PERFORMANCE,
            MUSA_AU_TIMELINE_HOST,
        )
    };
    assert_eq!(unsafe { musa_au_schedule_ok(refused) }, 0);
    let message = unsafe { CStr::from_ptr(musa_au_schedule_message(refused)) }.to_string_lossy();
    assert!(message.contains("polytempo"), "{message}");
    assert_eq!(
        unsafe { musa_au_schedule_count(refused) },
        0,
        "a refusal has no messages"
    );
    unsafe { musa_au_schedule_release(refused) };
}

#[test]
fn a_word_this_library_does_not_know_is_refused_rather_than_defaulted() {
    let project = path("invention.musa");
    for (mode, timeline) in [(9, MUSA_AU_TIMELINE_PIECE), (MUSA_AU_MIDI_SCORE, 9)] {
        let refused = unsafe { musa_au_open_schedule(project.as_ptr(), std::ptr::null(), mode, timeline) };
        assert_eq!(unsafe { musa_au_schedule_ok(refused) }, 0);
        let message = unsafe { CStr::from_ptr(musa_au_schedule_message(refused)) }.to_string_lossy();
        assert!(message.contains("this library knows"), "{message}");
        unsafe { musa_au_schedule_release(refused) };
    }
}

#[test]
fn every_schedule_accessor_tolerates_a_null_handle() {
    let null: *mut MusaAuSchedule = std::ptr::null_mut();
    assert_eq!(unsafe { musa_au_schedule_ok(null) }, 0);
    assert_eq!(unsafe { musa_au_schedule_count(null) }, 0);
    assert_eq!(unsafe { musa_au_schedule_extent(null) }, 0.0);
    assert_eq!(unsafe { musa_au_schedule_lower_bound(null, 1.0) }, 0);
    assert_eq!(unsafe { musa_au_schedule_active_scan_start(null, 1.0) }, 0);
    assert_eq!(unsafe { musa_au_schedule_part_count(null) }, 0);
    assert_eq!(unsafe { musa_au_schedule_part_channel(null, 0) }, 16);
    assert_eq!(unsafe { musa_au_schedule_loss_count(null) }, 0);
    assert_eq!(unsafe { musa_au_schedule_timeline(null) }, MUSA_AU_TIMELINE_PIECE);
    assert!(unsafe { musa_au_schedule_part_name(null, 0) }.is_null());
    assert!(unsafe { musa_au_schedule_loss(null, 0) }.is_null());
    assert!(
        !unsafe { CStr::from_ptr(musa_au_schedule_message(null)) }
            .to_bytes()
            .is_empty()
    );
    // An identity accessor answers an empty string rather than null, so a
    // caller can read it without a branch it would have got wrong.
    assert!(
        unsafe { CStr::from_ptr(musa_au_schedule_identity_music(null)) }
            .to_bytes()
            .is_empty()
    );

    let mut event = blank_event();
    assert_eq!(unsafe { musa_au_schedule_event(null, 0, &raw mut event) }, 0);
    let mut span = [blank_span(); 1];
    assert_eq!(unsafe { musa_au_schedule_active(null, 0.0, span.as_mut_ptr(), 1) }, 0);

    let schedule = open("invention.musa", MUSA_AU_MIDI_PERFORMANCE, MUSA_AU_TIMELINE_PIECE);
    assert_eq!(
        unsafe { musa_au_schedule_event(schedule, 0, std::ptr::null_mut()) },
        0,
        "nowhere to write is not a write"
    );
    unsafe { musa_au_schedule_release(schedule) };
    unsafe { musa_au_schedule_release(std::ptr::null_mut()) };
}

#[test]
fn a_project_that_is_not_there_is_refused_by_name() {
    let project = path("does-not-exist.musa");
    let refused = unsafe {
        musa_au_open_schedule(
            project.as_ptr(),
            std::ptr::null(),
            MUSA_AU_MIDI_PERFORMANCE,
            MUSA_AU_TIMELINE_PIECE,
        )
    };
    assert_eq!(unsafe { musa_au_schedule_ok(refused) }, 0);
    assert!(
        !unsafe { CStr::from_ptr(musa_au_schedule_message(refused)) }
            .to_bytes()
            .is_empty()
    );
    unsafe { musa_au_schedule_release(refused) };

    let refused = unsafe {
        musa_au_open_schedule(
            std::ptr::null(),
            std::ptr::null(),
            MUSA_AU_MIDI_PERFORMANCE,
            MUSA_AU_TIMELINE_PIECE,
        )
    };
    assert_eq!(unsafe { musa_au_schedule_ok(refused) }, 0);
    unsafe { musa_au_schedule_release(refused) };
}

fn blank_event() -> MusaAuScheduleEvent {
    MusaAuScheduleEvent {
        position: 0.0,
        part: 0,
        status: 0,
        data1: 0,
        data2: 0,
        reserved: 0,
    }
}

fn blank_span() -> MusaAuSpan {
    MusaAuSpan {
        start: 0.0,
        end: 0.0,
        part: 0,
        status: 0,
        note: 0,
        velocity: 0,
        reserved: 0,
    }
}
