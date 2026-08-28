//! Laws for prompt 208's acceptance transaction.
//!
//! Keeping a phrase is the one step where derived evidence becomes canonical
//! source, so the laws here are about the boundary rather than about the
//! notation: nothing moves until it is kept, keeping moves everything at once
//! or not at all, one undo puts the piece back, and what is written is
//! ordinary source with no trace of the performance behind it.
//!
//! The takes are the corpus prompt 203 measured, read through the same
//! `ProjectSession` facade the desktop uses.

#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::panic)]

use musa_project::{PlacementError, ProjectCommand, ProjectSession, ReviewDestination, ReviewFacts, ReviewRequest};

use crate::transcription_corpus::{CORPUS, Corpus, clock_of, session, synthesize};

/// The piece every placement here writes into: one part, one voice, one bar
/// of silence, so what the phrase adds is exactly what changed.
const PIECE: &str = r#"piece "Proposal laws" {
    meter 4/4;
    key c major;
    score { part p { voice v { rest/1 } } }
}
"#;

/// One review, accepted and ready to keep.
fn accepted(id: &str) -> (ProjectSession, ReviewFacts) {
    let (mut session, _) = opened(id);
    let facts = session.accept_review().expect("the reading is exactly writable");
    (session, facts)
}

/// One review, opened and not yet accepted.
fn opened(id: &str) -> (ProjectSession, ReviewFacts) {
    let body: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
    let fixture = body
        .fixtures
        .iter()
        .find(|fixture| fixture.id == id)
        .unwrap_or_else(|| panic!("the corpus has a `{id}` fixture"));
    let events = synthesize(fixture);
    let mut session = session();
    let facts = session
        .begin_review(&ReviewRequest {
            take_name: &fixture.id,
            destination: ReviewDestination {
                part: "p".to_owned(),
                voice: Some("v".to_owned()),
            },
            events: &events,
            clock: clock_of(&fixture.clock),
            bar_ticks: 96,
            meter: "4/4",
            key: None,
            policy_name: "standard",
        })
        .expect("the fixture composes");
    (session, facts)
}

fn named(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

// --- nothing moves until it is kept ---------------------------------------

#[test]
fn accepting_a_reading_still_does_not_touch_the_source() {
    let (session, facts) = accepted("straight-known");
    assert!(facts.sealed, "accepting settles what the phrase is");
    assert_eq!(
        session.snapshot().source(),
        PIECE,
        "acceptance settles the reading; keeping it is the transaction that writes"
    );
}

#[test]
fn planning_writes_nothing_and_asking_twice_is_free() {
    let (session, _) = accepted("straight-known");
    let first = session.plan_review_placement(&[]).expect("the phrase fits");
    let again = session.plan_review_placement(&[]).expect("asking twice is free");
    assert_eq!(first, again);
    assert_eq!(session.snapshot().source(), PIECE);
    // The plan is against the current revision, and says so.
    assert_eq!(first.revision, session.snapshot().revision());
}

#[test]
fn a_reading_that_cannot_be_written_exactly_has_no_phrase_to_keep() {
    let (mut session, _) = opened("rubato-free");
    // It cannot even be accepted, so it certainly cannot be placed.
    assert!(session.accept_review().is_err());
    assert_eq!(
        session.plan_review_placement(&[]).expect_err("there is no phrase"),
        PlacementError::NotAccepted
    );
    assert_eq!(session.snapshot().source(), PIECE);
}

#[test]
fn a_reading_still_taking_decisions_cannot_be_kept() {
    let (mut session, _) = opened("straight-known");
    assert_eq!(
        session
            .place_review(&[])
            .expect_err("keeping is the second half of accepting"),
        PlacementError::NotAccepted
    );
    assert_eq!(session.snapshot().source(), PIECE);
}

// --- the transaction -------------------------------------------------------

#[test]
fn keeping_a_phrase_writes_it_once_and_says_what_it_did() {
    let (mut session, _) = accepted("straight-known");
    let before = session.snapshot().revision();
    let report = session.place_review(&[]).expect("the phrase fits");

    assert!(report.revision > before, "keeping is one revision");
    assert_eq!(report.part, "p");
    assert_eq!(report.voices, vec!["v".to_owned()]);
    assert_eq!(report.summary, "4 notes in 1 bar into p’s v");
    assert!(!report.events.is_empty(), "the notes it wrote are the notes to look at");

    let source = session.snapshot().source().to_owned();
    assert!(source.contains("| c4/4 d4/4 e4/4 f4/4"), "{source}");
    // It joined the music that was there; it did not replace it.
    assert!(source.contains("rest/1"), "{source}");
    assert!(session.snapshot().compiles());
}

#[test]
fn one_undo_puts_the_piece_back_exactly() {
    let (mut session, _) = accepted("straight-known");
    let before = session.snapshot().source().to_owned();
    let revision = session.snapshot().revision();

    session.place_review(&[]).expect("the phrase fits");
    session.undo().expect("keeping is an ordinary revision");

    assert_eq!(session.snapshot().source(), before);
    assert_eq!(session.snapshot().revision(), revision);
    assert!(session.snapshot().compiles());
}

#[test]
fn keeping_closes_the_review_and_lets_the_take_go() {
    let (mut session, _) = accepted("straight-known");
    session.place_review(&[]).expect("the phrase fits");

    assert!(session.review().is_none(), "the reading is source now");
    assert_eq!(
        session
            .review_latest_take("standard")
            .expect_err("the take is released"),
        musa_project::ProposalError::Policy("nothing has been played to review".to_owned())
    );
}

// --- the anchor is a pair of names ----------------------------------------

#[test]
fn edits_that_leave_the_anchor_alone_leave_the_phrase_placeable() {
    let (mut session, _) = accepted("straight-known");
    // An unrelated edit, at the capture revision's byte offsets and then some.
    session
        .apply(ProjectCommand::SetSource(
            PIECE.replace("Proposal laws", "Renamed piece"),
        ))
        .expect("an ordinary edit");
    assert!(
        !session.review().expect("still under review").current,
        "the take is older than the piece"
    );

    let report = session.place_review(&[]).expect("a name outlives a byte offset");
    assert_eq!(report.voices, vec!["v".to_owned()]);
    assert!(session.snapshot().source().contains("| c4/4 d4/4 e4/4 f4/4"));
    assert!(session.snapshot().source().contains("Renamed piece"));
}

#[test]
fn a_part_the_piece_no_longer_has_is_a_refusal_that_keeps_the_take() {
    let (mut session, _) = accepted("straight-known");
    session
        .apply(ProjectCommand::SetSource(
            "piece \"Proposal laws\" {\n    meter 4/4;\n    score { part other { voice v { rest/1 } } }\n}\n"
                .to_owned(),
        ))
        .expect("an ordinary edit");
    let before = session.snapshot().source().to_owned();

    assert_eq!(
        session.place_review(&[]).expect_err("there is nowhere to put it"),
        PlacementError::PieceChanged("p".to_owned())
    );
    assert_eq!(session.snapshot().source(), before, "a refusal writes nothing");
    assert!(session.review().is_some(), "and leaves the reading to go back to");
}

// --- lines --------------------------------------------------------------

#[test]
fn a_phrase_in_two_lines_will_not_be_kept_until_they_are_named() {
    let (mut session, _) = accepted("crossing-voices");
    assert_eq!(
        session
            .place_review(&[])
            .expect_err("adding a line is the composer's decision"),
        PlacementError::NeedsVoiceNames { needed: 2, given: 0 }
    );
    assert_eq!(session.snapshot().source(), PIECE);

    // The plan still offers names, because offering is not deciding.
    let plan = session.plan_review_placement(&[]).expect("the phrase fits");
    assert_eq!(plan.voices.len(), 2);
    assert_eq!(plan.voices[0].name, "v");
    assert!(!plan.voices[0].added, "the first line goes where the caret was");
    assert!(plan.voices[1].added, "the second is a line the score does not have");
    assert_eq!(plan.added_voices(), vec![plan.voices[1].name.as_str()]);
}

#[test]
fn naming_the_lines_adds_them_and_replaces_nothing() {
    let (mut session, _) = accepted("crossing-voices");
    let report = session
        .place_review(&named(&["v", "lower"]))
        .expect("named lines are a decision");

    assert_eq!(report.voices, named(&["v", "lower"]));
    assert!(report.summary.ends_with("adding one line"), "{}", report.summary);
    let source = session.snapshot().source().to_owned();
    assert!(source.contains("voice lower {"), "{source}");
    assert!(
        source.contains("rest/1"),
        "the music that was there is still there:\n{source}"
    );
    assert!(session.snapshot().compiles());
}

#[test]
fn two_lines_pointed_at_one_voice_are_refused() {
    let (mut session, _) = accepted("crossing-voices");
    assert_eq!(
        session
            .place_review(&named(&["v", "v"]))
            .expect_err("one line, one voice"),
        PlacementError::SameVoiceTwice("v".to_owned())
    );
    assert_eq!(session.snapshot().source(), PIECE);
}

#[test]
fn a_name_the_language_cannot_write_is_refused_before_the_parser_sees_it() {
    let (mut session, _) = accepted("crossing-voices");
    assert_eq!(
        session
            .place_review(&named(&["v", "2 hands"]))
            .expect_err("a voice name is an identifier"),
        PlacementError::NotAName("2 hands".to_owned())
    );
    assert_eq!(session.snapshot().source(), PIECE);
}

// --- the differential law -------------------------------------------------

/// Where a phrase came from is not one of the things that decides what it
/// writes.
///
/// Capture and Keep that differ in how the events were gathered and in what
/// the take ends up called; they converge on one `ReviewRequest`, and from
/// there on one proposal. So the law that makes "either origin, same source"
/// true is this one: equal readings at equal anchors write the same bytes,
/// whatever the take was named.
#[test]
fn equal_readings_at_equal_anchors_write_the_same_bytes() {
    let body: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
    let fixture = body
        .fixtures
        .iter()
        .find(|fixture| fixture.id == "straight-known")
        .expect("the corpus has it");
    let events = synthesize(fixture);

    let written = |take_name: &str| -> String {
        let mut session = session();
        session
            .begin_review(&ReviewRequest {
                take_name,
                destination: ReviewDestination {
                    part: "p".to_owned(),
                    voice: Some("v".to_owned()),
                },
                events: &events,
                clock: clock_of(&fixture.clock),
                bar_ticks: 96,
                meter: "4/4",
                key: None,
                policy_name: "standard",
            })
            .expect("the fixture composes");
        session.accept_review().expect("it is exactly writable");
        session.place_review(&[]).expect("the phrase fits");
        session.snapshot().source().to_owned()
    };

    // The names Capture and Keep that give their takes.
    assert_eq!(written("p@1"), written("recent@1"));
}
