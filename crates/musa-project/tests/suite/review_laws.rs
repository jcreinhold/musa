//! Laws for prompt 207's Review surface.
//!
//! Review is a temporary reading of one immutable take. Everything here runs
//! through `ProjectSession`'s review facade — never the private stages — and
//! the load-bearing claim, asserted after every gesture, is that the canonical
//! source never moves: acceptance settles what the phrase *is*, and placing it
//! into the score is a later transaction.
//!
//! The marks are checked against the corpus prompt 203 measured, one fixture
//! per observed ambiguity class. A clean fixture asking nothing is as much a
//! law as a swung one asking once.

#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::panic)]

use musa_project::{
    AmbiguityKind, GroupIntent, ProjectCommand, ProjectSession, ReviewAction, ReviewAudition, ReviewError, ReviewFacts,
};

use crate::transcription_corpus::{CORPUS, Corpus, clock_of, session, synthesize};

/// The source every review in this file runs against; nothing may change it.
const UNTOUCHED: &str = r#"piece "Proposal laws" {
    meter 4/4;
    key c major;
    score { part p { voice v { rest/1 } } }
}
"#;

fn open(id: &str) -> (ProjectSession, ReviewFacts) {
    let body: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
    let fixture = body
        .fixtures
        .iter()
        .find(|fixture| fixture.id == id)
        .unwrap_or_else(|| panic!("the corpus has a `{id}` fixture"));
    let mut session = session();
    let facts = session
        .begin_review(
            &fixture.id,
            &synthesize(fixture),
            clock_of(&fixture.clock),
            96,
            "4/4",
            None,
            "standard",
        )
        .expect("the fixture composes");
    (session, facts)
}

fn refuse(id: &str) -> musa_project::ProposalError {
    let body: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
    let fixture = body
        .fixtures
        .iter()
        .find(|fixture| fixture.id == id)
        .unwrap_or_else(|| panic!("the corpus has a `{id}` fixture"));
    let mut session = session();
    session
        .begin_review(
            &fixture.id,
            &synthesize(fixture),
            clock_of(&fixture.clock),
            96,
            "4/4",
            None,
            "standard",
        )
        .expect_err("this fixture is refused")
}

fn kinds(facts: &ReviewFacts) -> Vec<AmbiguityKind> {
    facts.ambiguities.iter().map(|mark| mark.kind).collect()
}

#[test]
fn a_reading_the_search_is_sure_of_asks_nothing_and_is_ready_to_keep() {
    for clean in [
        "straight-known",
        "syncopated-known",
        "triplet-known",
        "pickup-asymmetric",
    ] {
        let (mut session, facts) = open(clean);
        assert!(
            facts.ambiguities.is_empty(),
            "`{clean}` should ask nothing, asked {:?}",
            kinds(&facts)
        );
        assert!(facts.losses.is_empty(), "`{clean}` should lose nothing");
        assert!(facts.source.is_some(), "`{clean}` should be writable");
        let sealed = session.accept_review().expect("a clean reading can be kept");
        assert!(sealed.sealed);
        assert_eq!(
            session.snapshot().source(),
            UNTOUCHED,
            "accepting must not write source"
        );
    }
}

#[test]
fn each_measured_ambiguity_class_raises_its_own_mark_and_no_other() {
    let (_, swing) = open("swing-known");
    assert!(
        kinds(&swing).iter().all(|&kind| kind == AmbiguityKind::Placement),
        "a swung phrase asks where the offbeats fall: {:?}",
        kinds(&swing)
    );

    let (_, rolled) = open("rolled-and-block-chords");
    assert!(
        kinds(&rolled).iter().all(|&kind| kind == AmbiguityKind::OnsetGroup),
        "spread chords ask chord or rolled: {:?}",
        kinds(&rolled)
    );

    let (_, crossing) = open("crossing-voices");
    assert!(
        kinds(&crossing).contains(&AmbiguityKind::Voice),
        "crossing lines ask which note carries on which line"
    );

    let (_, free) = open("rubato-free");
    assert_eq!(
        kinds(&free).first(),
        Some(&AmbiguityKind::Pulse),
        "a take played without a click asks for a pulse first"
    );
}

#[test]
fn an_ametric_take_is_refused_rather_than_written_onto_a_grid() {
    assert!(matches!(
        refuse("unmeasured"),
        musa_project::ProposalError::SearchRefused(musa_project::Refusal::WriteSource)
    ));
}

#[test]
fn every_mark_offers_two_or_three_readings_with_exactly_one_drawn() {
    for fixture in ["swing-known", "rolled-and-block-chords", "crossing-voices"] {
        let (_, facts) = open(fixture);
        for mark in &facts.ambiguities {
            assert!(
                (2..=3).contains(&mark.choices.len()),
                "`{}` in `{fixture}` offers {} readings",
                mark.id,
                mark.choices.len()
            );
            assert_eq!(
                mark.choices.iter().filter(|choice| choice.current).count(),
                1,
                "`{}` in `{fixture}` must draw exactly one of its readings",
                mark.id
            );
            let mut labels = mark
                .choices
                .iter()
                .map(|choice| choice.label.clone())
                .collect::<Vec<_>>();
            labels.sort();
            let before = labels.len();
            labels.dedup();
            assert_eq!(before, labels.len(), "`{}` offers the same words twice", mark.id);
            assert!(!mark.explanation.is_empty());
            assert!(!mark.notes.is_empty());
        }
    }
}

#[test]
fn choosing_a_reading_settles_that_mark_and_leaves_the_others_standing() {
    let (mut session, before) = open("rolled-and-block-chords");
    let mark = before.ambiguities[0].clone();
    let other = mark
        .choices
        .iter()
        .find(|choice| !choice.current)
        .expect("a mark offers a reading other than the one drawn");
    let after = session
        .review_act(&ReviewAction::Choose {
            ambiguity: mark.id.clone(),
            choice: other.id.clone(),
        })
        .expect("choosing an offered reading");
    assert!(
        !after.ambiguities.iter().any(|standing| standing.id == mark.id),
        "the chosen mark is settled"
    );
    assert_eq!(
        after.ambiguities.len(),
        before.ambiguities.len() - 1,
        "settling one mark settles exactly one"
    );
    assert_eq!(after.history, vec![other.label.clone()]);
    assert_eq!(session.snapshot().source(), UNTOUCHED);
}

#[test]
fn a_decision_never_changes_where_a_note_came_from() {
    let (mut session, before) = open("crossing-voices");
    let mark = before
        .ambiguities
        .iter()
        .find(|mark| mark.kind == AmbiguityKind::Voice)
        .expect("the crossing fixture asks about voices")
        .clone();
    let after = session
        .review_act(&ReviewAction::Choose {
            ambiguity: mark.id,
            choice: "separate".to_owned(),
        })
        .expect("uncrossing the lines");
    let was = before.notes.iter().map(|note| note.derivation).collect::<Vec<_>>();
    let is = after.notes.iter().map(|note| note.derivation).collect::<Vec<_>>();
    assert_eq!(
        was, is,
        "a decision changes how a note is written, never which key it was"
    );
    assert_ne!(
        before.notes.iter().map(|note| note.voice).collect::<Vec<_>>(),
        after.notes.iter().map(|note| note.voice).collect::<Vec<_>>(),
        "and this one really did change the lines"
    );
}

#[test]
fn taking_back_a_decision_restores_the_exact_reading_before_it() {
    let (mut session, before) = open("crossing-voices");
    let mark = before
        .ambiguities
        .iter()
        .find(|mark| mark.kind == AmbiguityKind::Voice)
        .expect("the crossing fixture asks about voices")
        .clone();
    session
        .review_act(&ReviewAction::Choose {
            ambiguity: mark.id,
            choice: "separate".to_owned(),
        })
        .expect("uncrossing the lines");
    let back = session.review_undo().expect("taking it back");
    assert_eq!(back.notes, before.notes);
    assert_eq!(back.ambiguities, before.ambiguities, "and the mark stands again");
    assert!(back.history.is_empty());
    assert_eq!(
        session.review_undo().expect_err("this gesture is refused"),
        ReviewError::NothingToUndo
    );
}

#[test]
fn every_note_carries_a_musical_name_a_reader_could_speak() {
    let (_, facts) = open("crossing-voices");
    for note in &facts.notes {
        assert!(note.name.starts_with(&note.pitch), "a name starts with the note");
        assert!(note.name.contains("line "), "and says which line: {}", note.name);
        assert!(note.name.contains("bar "), "and where it falls: {}", note.name);
    }
    let first = &facts.notes[0];
    assert_eq!(first.name, format!("{}, 1/4, line 1, bar 1, beat 1", first.pitch));
}

#[test]
fn setting_every_value_writes_every_value_and_still_names_what_it_cannot_spell() {
    let (mut session, before) = open("swing-known");
    assert!(before.source.is_none(), "a swung phrase has no exact written form yet");
    assert!(!before.losses.is_empty(), "and says which note it is");
    let after = session
        .review_act(&ReviewAction::Transform {
            notes: (0..before.notes.len()).collect(),
            intent: GroupIntent::SetEachDuration {
                duration: "1/8".to_owned(),
            },
        })
        .expect("writing every value as an eighth");
    assert!(after.notes.iter().all(|note| note.end_ticks == 12));
    assert_eq!(
        after.history,
        vec!["write every value as 1/8 across 8 notes".to_owned()]
    );
    assert_eq!(
        after.changed.len(),
        before.notes.iter().filter(|note| note.end_ticks != 12).count(),
        "one spoken result covers exactly the notes that read differently"
    );
    // The values are settled and the swung *onsets* are not, so the phrase
    // still has no exact written form — and still says so by name rather than
    // rounding itself onto the grid.
    assert!(!after.losses.is_empty());
    assert_eq!(session.snapshot().source(), UNTOUCHED);
}

#[test]
fn scaling_a_passage_keeps_its_rhythm_and_changes_its_rate() {
    let (mut session, before) = open("straight-known");
    let after = session
        .review_act(&ReviewAction::Transform {
            notes: (0..before.notes.len()).collect(),
            intent: GroupIntent::ScaleDurations {
                ratio: "1/2".to_owned(),
            },
        })
        .expect("halving the values");
    for (was, is) in before.notes.iter().zip(&after.notes) {
        assert_eq!(is.end_ticks, was.end_ticks / 2);
    }
    assert!(after.source.is_some(), "and it is still a phrase that can be written");
    assert_eq!(session.snapshot().source(), UNTOUCHED);
}

#[test]
fn a_pitch_command_moves_the_page_the_way_the_source_path_moves_it() {
    let (mut session, before) = open("straight-known");
    let after = session
        .review_act(&ReviewAction::Transform {
            notes: (0..before.notes.len()).collect(),
            intent: GroupIntent::TransposeBy {
                interval: "up P5".to_owned(),
            },
        })
        .expect("transposing the phrase");
    for (was, is) in before.notes.iter().zip(&after.notes) {
        let moved = musa_score::WrittenPitch::parse(&was.pitch)
            .and_then(|pitch| pitch.transpose(musa_score::Interval::parse("P5", false).expect("a fifth")))
            .expect("a fifth up is a note");
        assert_eq!(is.pitch, moved.to_string());
    }
    assert_eq!(session.snapshot().source(), UNTOUCHED);
}

#[test]
fn a_refused_gesture_leaves_the_reading_exactly_as_it_was() {
    let (mut session, before) = open("straight-known");
    let error = session
        .review_act(&ReviewAction::Respell {
            note: 0,
            pitch: "gb9".to_owned(),
        })
        .expect_err("that is a different note, not a different spelling");
    assert!(matches!(error, ReviewError::Refused(_)));
    assert_eq!(session.review().expect("still reviewing").notes, before.notes);

    assert_eq!(
        session
            .review_act(&ReviewAction::Transform {
                notes: vec![99],
                intent: GroupIntent::ScaleDurations { ratio: "2".to_owned() },
            })
            .expect_err("this gesture is refused"),
        ReviewError::NoSuchNote(99)
    );
    assert!(matches!(
        session
            .review_act(&ReviewAction::Choose {
                ambiguity: "placement-9".to_owned(),
                choice: "t0".to_owned(),
            })
            .expect_err("this gesture is refused"),
        ReviewError::NoSuchAmbiguity(_)
    ));
    assert_eq!(session.review().expect("still reviewing").notes, before.notes);
    assert_eq!(session.snapshot().source(), UNTOUCHED);
}

#[test]
fn one_note_is_already_one_note_and_offers_no_chord_reading() {
    let (mut session, _) = open("straight-known");
    assert!(matches!(
        session
            .review_act(&ReviewAction::MakeChord { group: 0 })
            .expect_err("this gesture is refused"),
        ReviewError::Refused(_)
    ));
    assert!(matches!(
        session
            .review_act(&ReviewAction::Split { group: 0 })
            .expect_err("this gesture is refused"),
        ReviewError::Refused(_)
    ));
}

#[test]
fn a_cluster_reads_as_one_chord_or_as_the_notes_that_were_played() {
    let (mut session, before) = open("rolled-and-block-chords");
    let group = before.ambiguities[0].notes.clone();
    let chord = session
        .review_act(&ReviewAction::MakeChord { group: 0 })
        .expect("reading it as a chord");
    let ticks = chord.notes[group[0]].onset_ticks;
    assert!(group.iter().all(|&note| chord.notes[note].onset_ticks == ticks));
    session.review_undo().expect("taking it back");
    let rolled = session
        .review_act(&ReviewAction::Split { group: 0 })
        .expect("rolling it");
    assert!(
        rolled.notes[group[0]].onset_ticks < rolled.notes[group[1]].onset_ticks,
        "a rolled chord is written as separate onsets"
    );
}

#[test]
fn tying_a_note_writes_it_through_and_untying_stops_it_at_the_key() {
    let (mut session, before) = open("straight-known");
    let tied = session
        .review_act(&ReviewAction::Tie { note: 0, tied: true })
        .expect("writing it through");
    assert_eq!(
        tied.notes[0].end_ticks,
        before.notes[1].onset_ticks - before.notes[0].onset_ticks
    );
    session.review_undo().expect("taking it back");
    let last = before.notes.len() - 1;
    assert!(matches!(
        session
            .review_act(&ReviewAction::Tie { note: last, tied: true })
            .expect_err("this gesture is refused"),
        ReviewError::Refused(_)
    ));
}

#[test]
fn a_pulse_needs_more_than_one_tap_and_settles_the_mark_when_it_has_them() {
    let (mut session, before) = open("rubato-free");
    assert!(matches!(
        session
            .review_act(&ReviewAction::Tap {
                beats_micros: vec![0],
                downbeat: Some(0),
            })
            .expect_err("this gesture is refused"),
        ReviewError::Refused(_)
    ));
    let onsets = before
        .notes
        .iter()
        .map(|note| u64::from(note.onset_ticks) * 20_000)
        .collect::<Vec<_>>();
    let after = session
        .review_act(&ReviewAction::Tap {
            beats_micros: vec![onsets[0], onsets[1], onsets[2]],
            downbeat: Some(0),
        })
        .expect("three taps are a pulse");
    assert!(
        !after.ambiguities.iter().any(|mark| mark.kind == AmbiguityKind::Pulse),
        "tapped beats settle the pulse"
    );
    assert_eq!(session.snapshot().source(), UNTOUCHED);
}

#[test]
fn a_reading_with_a_length_it_cannot_write_cannot_be_kept_yet() {
    let (mut session, facts) = open("swing-known");
    assert!(facts.source.is_none());
    assert!(matches!(
        session.accept_review().expect_err("this gesture is refused"),
        ReviewError::Refused(_)
    ));
}

#[test]
fn an_accepted_reading_takes_no_further_decisions() {
    let (mut session, _) = open("straight-known");
    session.accept_review().expect("keeping a clean reading");
    assert_eq!(
        session
            .review_act(&ReviewAction::Tie { note: 0, tied: true })
            .expect_err("this gesture is refused"),
        ReviewError::Sealed
    );
    assert_eq!(
        session.review_undo().expect_err("this gesture is refused"),
        ReviewError::Sealed
    );
    assert_eq!(
        session.accept_review().expect_err("this gesture is refused"),
        ReviewError::Sealed
    );
}

#[test]
fn auditioning_the_other_performance_is_not_a_decision() {
    let (mut session, before) = open("straight-known");
    assert_eq!(before.audition, ReviewAudition::Played);
    let written = session
        .review_audition(ReviewAudition::Written)
        .expect("hearing the notation");
    assert_eq!(written.audition, ReviewAudition::Written);
    assert!(written.history.is_empty(), "listening decides nothing");
    assert_eq!(written.notes, before.notes);
}

#[test]
fn an_edit_to_the_piece_makes_the_review_stale_rather_than_gone() {
    let (mut session, before) = open("straight-known");
    assert!(before.current);
    session
        .apply(ProjectCommand::SetSource(format!("{UNTOUCHED}\n// a later thought\n")))
        .expect("an unrelated edit");
    let stale = session.review().expect("the phrase they played is still theirs");
    assert!(!stale.current, "and it says it was read against an earlier score");
    assert_eq!(stale.notes, before.notes);
}

#[test]
fn discarding_a_review_leaves_nothing_behind() {
    let (mut session, _) = open("straight-known");
    session.discard_review();
    assert!(session.review().is_none());
    assert_eq!(
        session
            .review_act(&ReviewAction::Tie { note: 0, tied: true })
            .expect_err("this gesture is refused"),
        ReviewError::NotReviewing
    );
    assert_eq!(
        session.review_undo().expect_err("this gesture is refused"),
        ReviewError::NotReviewing
    );
    assert_eq!(
        session.accept_review().expect_err("this gesture is refused"),
        ReviewError::NotReviewing
    );
    assert_eq!(session.snapshot().source(), UNTOUCHED);
}

#[test]
fn a_mark_keeps_its_name_across_a_decision_somewhere_else() {
    let (mut session, before) = open("crossing-voices");
    let names = before
        .ambiguities
        .iter()
        .map(|mark| mark.id.clone())
        .collect::<Vec<_>>();
    let settled = names
        .iter()
        .find(|id| id.starts_with("group-"))
        .expect("the crossing fixture has chord clusters")
        .clone();
    let after = session
        .review_act(&ReviewAction::Choose {
            ambiguity: settled.clone(),
            choice: "arpeggio".to_owned(),
        })
        .expect("reading one cluster as an arpeggio");
    for id in names.iter().filter(|id| **id != settled) {
        assert!(
            after.ambiguities.iter().any(|mark| mark.id == *id),
            "`{id}` should still be there under the same name, so focus survives"
        );
    }
}

#[test]
fn assigning_a_line_is_refused_past_the_four_a_keyboard_proposal_writes() {
    let (mut session, _) = open("crossing-voices");
    assert!(matches!(
        session
            .review_act(&ReviewAction::AssignVoice {
                notes: vec![0],
                voice: 4,
            })
            .expect_err("this gesture is refused"),
        ReviewError::Refused(_)
    ));
    let after = session
        .review_act(&ReviewAction::AssignVoice {
            notes: vec![0, 1],
            voice: 1,
        })
        .expect("putting two notes in the second line");
    assert_eq!(after.notes[0].voice, 1);
    assert_eq!(after.notes[1].voice, 1);
    assert_eq!(after.history, vec!["put 2 notes in line 2".to_owned()]);
}
