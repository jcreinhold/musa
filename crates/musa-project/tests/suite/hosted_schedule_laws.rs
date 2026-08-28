//! One piece, as a host reads it by position.
//!
//! `06-daw-boundary.md` §3 gives the host the transport, so what is tested
//! here is not that a sequencer advances but that a *query* answers: the same
//! messages come out however the host cuts the timeline into blocks, a seek
//! costs a search rather than a replay, and the notes already sounding at the
//! place a host jumped to are the ones whose release is still ahead of it.
//!
//! Nothing musical is decided by the code under test. `musa-notation` decides
//! every MIDI fact, and the differential against its own schedule is what
//! keeps that true.

// A fixture that does not contain what a test looks for is the test failing.
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::float_cmp)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use std::path::PathBuf;

use musa_project::{
    HostedActiveNote, HostedSchedule, HostedScheduleRequest, HostedTimeline, MidiMode, open_hosted_schedule,
};

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

fn open(name: &str, timeline: HostedTimeline, mode: MidiMode) -> HostedSchedule {
    open_hosted_schedule(&HostedScheduleRequest {
        project: example(name),
        piece: None,
        mode,
        timeline,
    })
    .expect("the example schedules")
}

fn invention(timeline: HostedTimeline) -> HostedSchedule {
    open("invention.musa", timeline, MidiMode::Performance)
}

#[test]
fn a_piece_becomes_a_finite_schedule_of_its_own_messages() {
    let schedule = invention(HostedTimeline::Piece);
    assert!(!schedule.events().is_empty(), "the fixture sounds");
    assert!(!schedule.parts().is_empty(), "the fixture has a part");
    assert!(
        schedule
            .events()
            .windows(2)
            .all(|pair| pair[0].position <= pair[1].position),
        "the schedule is in the order it sounds"
    );
    assert_eq!(schedule.extent(), schedule.events().last().expect("messages").position);
    assert_eq!(schedule.timeline(), HostedTimeline::Piece);
}

#[test]
fn every_message_belongs_to_exactly_one_half_open_block() {
    // §4: the host chooses the block boundaries and they are not meaning. A
    // message exactly on a seam belongs to the later block, once.
    let schedule = invention(HostedTimeline::Piece);
    for width in [0.001_f64, 0.05, 0.37, 2.5] {
        let blocks = ((schedule.extent() / width).ceil() as usize).saturating_add(2);
        let mut seen = Vec::new();
        // One boundary series, read twice: block `n` ends exactly where block
        // `n + 1` begins. That contiguity is what makes a partition a
        // partition, and it is the host's to keep — a schedule queried at two
        // boundaries that do not meet has been asked about two different
        // timelines.
        let boundary = |block: usize| width * block as f64;
        for block in 0..blocks {
            seen.extend(schedule.lower_bound(boundary(block))..schedule.lower_bound(boundary(block.saturating_add(1))));
        }
        let all: Vec<usize> = (0..schedule.events().len()).collect();
        assert_eq!(seen, all, "a block width of {width} lost or repeated a message");
    }
}

#[test]
fn a_seek_starts_where_the_host_is_and_not_at_the_beginning() {
    // `93-the-audio-unit-shape.md` §6 measured exactly this on the trial: the
    // search after a seek starts at the index the position names.
    let schedule = invention(HostedTimeline::Piece);
    let middle = schedule.extent() / 2.0;
    let index = schedule.lower_bound(middle);
    assert!(index > 0, "the fixture has messages before its midpoint");
    assert!(index < schedule.events().len(), "and messages after it");
    assert!(schedule.events()[index.saturating_sub(1)].position < middle);
    assert!(schedule.events()[index].position >= middle);
    assert_eq!(schedule.lower_bound(0.0), 0);
    assert_eq!(schedule.lower_bound(schedule.extent() + 1.0), schedule.events().len());
}

#[test]
fn the_notes_already_sounding_are_the_ones_whose_release_is_still_ahead() {
    let schedule = invention(HostedTimeline::Piece);
    let mut into = [blank(); 64];
    // The brute force this index replaces: every attack before here whose
    // release is after here. If the two ever disagree, the index is wrong and
    // not merely slow.
    let sweep = (1..40)
        .map(|step| schedule.extent() * f64::from(step) / 40.0)
        .chain(schedule.events().iter().map(|event| event.position))
        .chain(midpoints(&schedule));
    for position in sweep {
        let found = schedule.active(position, &mut into);
        let expected = brute_force(&schedule, position);
        assert_eq!(found, expected.len(), "at {position}");
        let mut reported: Vec<(f64, u8, u8)> = into[..found]
            .iter()
            .map(|note| (note.start, note.status, note.note))
            .collect();
        let mut wanted: Vec<(f64, u8, u8)> = expected
            .iter()
            .map(|note| (note.start, note.status, note.note))
            .collect();
        reported.sort_by(|left, right| left.partial_cmp(right).expect("finite positions"));
        wanted.sort_by(|left, right| left.partial_cmp(right).expect("finite positions"));
        assert_eq!(reported, wanted, "at {position}");
    }
}

#[test]
fn a_note_that_starts_exactly_here_is_not_also_already_sounding() {
    // Otherwise a seek to an attack emits it twice: once as re-entry and once
    // as the block's own message.
    let schedule = invention(HostedTimeline::Piece);
    let mut into = [blank(); 64];
    for event in schedule.events().iter().filter(|event| event.status & 0xF0 == 0x90) {
        let found = schedule.active(event.position, &mut into);
        assert!(
            !into[..found].iter().any(|note| note.start == event.position),
            "an attack at {} was reported as already sounding",
            event.position
        );
    }
}

#[test]
fn seeking_into_the_middle_does_not_start_reading_at_the_beginning() {
    // The Stop list forbids a whole-piece scan on seek. What makes that true
    // is that the notes finished before here are skipped by a search rather
    // than passed over one at a time.
    let schedule = invention(HostedTimeline::Piece);
    let late = schedule.extent() * 0.9;
    assert!(
        schedule.active_scan_start(late) > 0,
        "a seek near the end still began at the first note of the piece"
    );
    assert_eq!(
        schedule.active_scan_start(0.0),
        0,
        "and from the start it begins at the start"
    );
}

#[test]
fn a_buffer_too_small_is_told_how_many_it_could_not_hold() {
    // A host that offered one slot still learns there were three, which is
    // what lets it ask again rather than re-enter a chord as one note.
    let schedule = open("counterpoint.musa", HostedTimeline::Piece, MidiMode::Performance);
    let mut into = [blank(); 64];
    let position = midpoints(&schedule)
        .find(|position| schedule.active(*position, &mut into) > 1)
        .expect("the fixture holds two notes at once somewhere");
    let full = schedule.active(position, &mut into);
    let mut cramped = [blank(); 1];
    assert_eq!(
        schedule.active(position, &mut cramped),
        full,
        "a caller with one slot is still told the whole count"
    );
    assert_eq!(cramped[0].start, into[0].start, "and the slot it did have was filled");
}

#[test]
fn the_two_timelines_carry_the_same_messages_in_the_same_order() {
    // The timeline decides *where* a message goes, never what it is. A host
    // that follows its own tempo is playing the same piece.
    let piece = invention(HostedTimeline::Piece);
    let host = invention(HostedTimeline::Host);
    let spelling = |schedule: &HostedSchedule| -> Vec<(u32, u8, [u8; 2])> {
        schedule
            .events()
            .iter()
            .map(|event| (event.part, event.status, event.data))
            .collect()
    };
    assert_eq!(spelling(&piece), spelling(&host));
    assert_eq!(host.timeline().unit(), "quarter notes");
    assert_eq!(piece.timeline().unit(), "seconds");
    assert_ne!(
        piece.extent(),
        host.extent(),
        "seconds and quarters are different numbers"
    );
}

#[test]
fn the_hosts_timeline_refuses_a_piece_that_has_no_single_grid() {
    // `canon-x.musa` is polytempo. There is no one quarter-note grid to lay
    // on the host's, so §6 says refuse rather than pick a tempo.
    let refusal = open_hosted_schedule(&HostedScheduleRequest {
        project: example("canon-x.musa"),
        piece: None,
        mode: MidiMode::Performance,
        timeline: HostedTimeline::Host,
    })
    .expect_err("a polytempo piece cannot be placed on one grid");
    assert!(refusal.to_string().contains("polytempo"), "{refusal}");

    // And on its own timeline it plays, because every message keeps the
    // second it sounds at.
    let schedule = open("canon-x.musa", HostedTimeline::Piece, MidiMode::Performance);
    assert!(!schedule.events().is_empty());
    assert!(
        schedule.losses().iter().any(|loss| loss.starts_with("polytempo:")),
        "{:?}",
        schedule.losses()
    );
}

#[test]
fn a_schedule_says_what_a_midi_effect_cannot_carry() {
    let schedule = invention(HostedTimeline::Piece);
    assert!(
        schedule.losses().iter().any(|loss| loss.starts_with("conductor:")),
        "{:?}",
        schedule.losses()
    );
    assert!(
        !schedule.losses().iter().any(|loss| loss.starts_with("tempo:")),
        "the piece's own timeline is its own tempo, so nothing about tempo is lost"
    );
    assert!(
        invention(HostedTimeline::Host)
            .losses()
            .iter()
            .any(|loss| loss.starts_with("tempo:")),
        "on the host's timeline the piece's tempo map became a proportion, and that is a loss"
    );
}

#[test]
fn the_written_reading_and_the_played_one_are_different_documents() {
    let written = open("invention.musa", HostedTimeline::Piece, MidiMode::Score);
    let played = open("invention.musa", HostedTimeline::Piece, MidiMode::Performance);
    assert_eq!(written.mode(), MidiMode::Score);
    assert_eq!(played.mode(), MidiMode::Performance);
    let velocities = |schedule: &HostedSchedule| -> Vec<u8> {
        schedule
            .events()
            .iter()
            .filter(|event| event.status & 0xF0 == 0x90)
            .map(|event| event.data[1])
            .collect()
    };
    assert_ne!(
        velocities(&written),
        velocities(&played),
        "the played reading interprets dynamics and the written one does not"
    );
}

#[test]
fn a_schedule_names_what_it_was_made_from() {
    let schedule = invention(HostedTimeline::Piece);
    let identity = schedule.identity();
    assert!(!identity.music.is_empty());
    assert!(!identity.assets.is_empty());
    assert!(!identity.piece.is_empty());
}

#[test]
fn a_word_this_version_does_not_know_is_not_a_default() {
    assert_eq!(HostedTimeline::parse("piece"), Some(HostedTimeline::Piece));
    assert_eq!(HostedTimeline::parse("host"), Some(HostedTimeline::Host));
    assert_eq!(HostedTimeline::parse(""), None);
    assert_eq!(HostedTimeline::parse("Host"), None);
    assert_eq!(HostedTimeline::Piece.as_str(), "piece");
    assert_eq!(HostedTimeline::Host.as_str(), "host");
}

#[test]
fn a_source_that_does_not_compile_is_refused_by_name() {
    let refusal = open_hosted_schedule(&HostedScheduleRequest {
        project: example("does-not-exist.musa"),
        piece: None,
        mode: MidiMode::Performance,
        timeline: HostedTimeline::Piece,
    })
    .expect_err("there is no such piece");
    assert!(!refusal.to_string().is_empty());
}

/// Positions strictly between consecutive messages: the places a host's block
/// boundary can fall that no message sits exactly on.
fn midpoints(schedule: &HostedSchedule) -> impl Iterator<Item = f64> + '_ {
    schedule
        .events()
        .windows(2)
        .map(|pair| f64::midpoint(pair[0].position, pair[1].position))
        .filter(|position| position.is_finite())
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

/// The notes whose `[start, end)` contains `position`, found the slow way by
/// pairing every attack with its release. The index exists to make this fast,
/// not to make it different.
fn brute_force(schedule: &HostedSchedule, position: f64) -> Vec<HostedActiveNote> {
    let mut pending: Vec<HostedActiveNote> = Vec::new();
    let mut spans: Vec<HostedActiveNote> = Vec::new();
    for event in schedule.events() {
        let kind = event.status & 0xF0;
        if kind == 0x90 && event.data[1] > 0 {
            pending.push(HostedActiveNote {
                start: event.position,
                end: schedule.extent(),
                part: event.part,
                status: event.status,
                note: event.data[0],
                velocity: event.data[1],
            });
        } else if (kind == 0x80 || kind == 0x90)
            && let Some(index) = pending
                .iter()
                .rposition(|note| note.status & 0x0F == event.status & 0x0F && note.note == event.data[0])
        {
            let mut span = pending.remove(index);
            span.end = event.position;
            spans.push(span);
        }
    }
    spans.extend(pending);
    spans.retain(|span| span.start < position && span.end > position);
    spans
}
