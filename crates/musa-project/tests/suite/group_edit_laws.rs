//! Group transformations: one selection, one preview, one source transaction.
//!
//! The laws here are the ones prompt 206 states. Three of them carry the
//! musical argument rather than the mechanical one. Setting each duration,
//! scaling durations, and moving on the staff are *different* commands, so a
//! test that only checked "the note changed" would let them collapse into one
//! (OMT ch. 009–012, 016). A transformation reaches the source through the
//! statement that spells it, so a generated note's transformation is a
//! transformation of the motif and says so before it happens. And a preview
//! is the transaction: what the composer accepts is the edit they read, at
//! the revision they read it at, once.

// A fixture that does not contain what a test looks for is the test failing.
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::panic)]

use std::path::PathBuf;

use musa_project::{
    GeneratedEditMode, GroupEdit, GroupIntent, ProjectCommand, ProjectError, ProjectSession, ScoreFacts,
};

fn example(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn session(name: &str) -> ProjectSession {
    ProjectSession::from_text(example(name), name)
}

fn score_facts(session: &ProjectSession) -> ScoreFacts {
    session.snapshot().score().cloned().expect("the example compiles")
}

/// The ids of a voice's events, in score order.
fn voice_events(facts: &ScoreFacts, voice: &str) -> Vec<String> {
    facts
        .events
        .iter()
        .filter(|event| event.voice == voice)
        .map(|event| event.id.clone())
        .collect()
}

fn nth_event(facts: &ScoreFacts, voice: &str, index: usize) -> String {
    voice_events(facts, voice)
        .get(index)
        .unwrap_or_else(|| panic!("{voice} has no event {index}"))
        .clone()
}

fn edit(events: &[String], intent: GroupIntent) -> GroupEdit {
    GroupEdit {
        events: events.to_vec(),
        intent,
        mode: GeneratedEditMode::EditDefinition,
    }
}

fn source(session: &ProjectSession) -> String {
    session.snapshot().source().to_owned()
}

/// Preview and commit in one step, the way the interface does.
fn transform(session: &mut ProjectSession, edit: &GroupEdit) -> Result<String, ProjectError> {
    let plan = session.plan_group_edit(edit)?;
    let (id, revision) = (plan.id(), plan.revision());
    session.apply(ProjectCommand::ApplyGroupEdit { plan: id, revision })?;
    Ok(source(session))
}

// ---------------------------------------------------------------- selection

#[test]
fn the_order_and_multiplicity_of_a_selection_do_not_matter() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let upper = voice_events(&facts, "upper");

    let forward = session
        .plan_group_edit(&edit(&upper, GroupIntent::ShiftAccidentals { steps: 1 }))
        .expect("a whole voice is transposable")
        .clone();

    let mut jumbled: Vec<String> = upper.iter().rev().cloned().collect();
    jumbled.push(upper[2].clone());
    jumbled.insert(0, upper[2].clone());
    let backward = session
        .plan_group_edit(&edit(&jumbled, GroupIntent::ShiftAccidentals { steps: 1 }))
        .expect("a rubber band collects ids in whatever order it met them");

    assert_eq!(forward.edits(), backward.edits());
    assert_eq!(forward.changed(), backward.changed());
    assert_eq!(forward.source(), backward.source());
}

#[test]
fn an_unknown_event_is_refused_by_name() {
    let mut session = session("glass-mountain.musa");
    let error = session
        .plan_group_edit(&edit(
            &["not-an-event".to_owned()],
            GroupIntent::MoveDiatonically { steps: 1 },
        ))
        .expect_err("a stale id is not silently dropped");
    assert!(matches!(error, ProjectError::NoSuchEvent(_)), "{error:?}");
}

// ------------------------------------------------------------- the commands

#[test]
fn setting_each_duration_writes_that_duration_on_every_note() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let upper = voice_events(&facts, "upper");

    let after = transform(
        &mut session,
        &edit(
            &upper,
            GroupIntent::SetEachDuration {
                duration: "1/8".to_owned(),
            },
        ),
    )
    .expect("four whole notes can be written as eighths");

    assert!(after.contains("c5/8"), "{after}");
    assert!(after.contains("g#4/8"), "{after}");
}

#[test]
fn scaling_durations_multiplies_what_is_written() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let upper = voice_events(&facts, "upper");

    let after = transform(
        &mut session,
        &edit(
            &upper,
            GroupIntent::ScaleDurations {
                ratio: "1/2".to_owned(),
            },
        ),
    )
    .expect("halving a whole note is a half note");

    assert!(after.contains("c5/2"), "{after}");
    assert!(after.contains("a4/2"), "{after}");
}

#[test]
fn scaling_and_setting_are_different_commands() {
    // The motif's five events are written /2, /4, /2, /4, /2. Setting makes
    // them equal; scaling keeps the rhythm and changes its size. A single
    // "change the duration" command could not say both (OMT ch. 009).
    let mut set = session("glass-mountain.musa");
    let facts = score_facts(&set);
    let lead: Vec<String> = voice_events(&facts, "lead").into_iter().take(5).collect();

    let scaled = transform(
        &mut session("glass-mountain.musa"),
        &edit(
            &lead,
            GroupIntent::ScaleDurations {
                ratio: "1/2".to_owned(),
            },
        ),
    )
    .expect("halving the motif keeps its shape");
    let flattened = transform(
        &mut set,
        &edit(
            &lead,
            GroupIntent::SetEachDuration {
                duration: "1/4".to_owned(),
            },
        ),
    )
    .expect("the motif can be written in even quarters");

    assert!(scaled.contains("rest/8") && scaled.contains("b4/8"), "{scaled}");
    assert!(
        flattened.contains("rest/4") && flattened.contains("b4/4"),
        "{flattened}"
    );
    assert_ne!(scaled, flattened);
}

#[test]
fn transposing_moves_by_the_written_interval() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let upper = voice_events(&facts, "upper");

    let after = transform(
        &mut session,
        &edit(
            &upper,
            GroupIntent::TransposeBy {
                interval: "up P5".to_owned(),
            },
        ),
    )
    .expect("a fifth up is spellable from every note here");

    assert!(after.contains("g5/1"), "{after}");
    assert!(after.contains("e5/1"), "{after}");
    assert!(after.contains("d#5/1"), "{after}");
}

#[test]
fn moving_on_the_staff_and_altering_a_note_are_different_commands() {
    // A staff step moves the notehead and leaves the sign alone; an
    // accidental shift leaves the notehead alone (OMT ch. 004, 016).
    let facts = score_facts(&session("glass-mountain.musa"));
    let sharp = vec![nth_event(&facts, "upper", 3)]; // `g#4/1`

    let stepped = transform(
        &mut session("glass-mountain.musa"),
        &edit(&sharp, GroupIntent::MoveDiatonically { steps: 1 }),
    )
    .expect("a staff step is always available");
    let altered = transform(
        &mut session("glass-mountain.musa"),
        &edit(&sharp, GroupIntent::ShiftAccidentals { steps: -1 }),
    )
    .expect("a sign can always come off");

    assert!(stepped.contains("a#4/1"), "{stepped}");
    assert!(altered.contains("g4/1"), "{altered}");
}

// ------------------------------------------------------- mixed and grouped

#[test]
fn a_rest_takes_a_rhythm_and_refuses_a_pitch() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let lead: Vec<String> = voice_events(&facts, "lead").into_iter().take(5).collect();
    let rest = lead[1].clone();

    let rhythm = session
        .plan_group_edit(&edit(
            &lead,
            GroupIntent::ScaleDurations {
                ratio: "1/2".to_owned(),
            },
        ))
        .expect("a rest has a duration like anything else");
    assert!(rhythm.unchanged().is_empty());
    assert!(rhythm.changed().contains(&rest));

    let pitched = session
        .plan_group_edit(&edit(&lead[1..3], GroupIntent::MoveDiatonically { steps: 1 }))
        .expect("the selection still has a note in it");
    assert_eq!(pitched.unchanged(), std::slice::from_ref(&rest));
    assert!(!pitched.changed().contains(&rest));
}

#[test]
fn a_selection_with_nothing_applicable_is_refused_rather_than_applied_emptily() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let rest = nth_event(&facts, "lead", 1);
    let error = session
        .plan_group_edit(&edit(
            std::slice::from_ref(&rest),
            GroupIntent::MoveDiatonically { steps: 1 },
        ))
        .expect_err("rests have no pitch to move");
    assert!(matches!(error, ProjectError::Uneditable(_)), "{error:?}");
}

#[test]
fn a_chord_moves_as_one_statement() {
    let mut session = session("tuplet-fixture.musa");
    let facts = score_facts(&session);
    let chord = nth_event(&facts, "bass", 0); // `[c3 g3]/2`

    let plan = session
        .plan_group_edit(&edit(
            std::slice::from_ref(&chord),
            GroupIntent::TransposeBy {
                interval: "up P5".to_owned(),
            },
        ))
        .expect("a chord transposes");
    assert_eq!(plan.definitions().len(), 1, "a chord is one statement, not two notes");
    assert_eq!(plan.definitions()[0].before, "c3 g3");
    assert!(plan.source().contains("[g3 d4]/2"), "{}", plan.source());
}

// ----------------------------------------------------- structures refusing

#[test]
fn a_tied_note_is_refused_with_its_reason() {
    let mut session = session("tuplet-fixture.musa");
    let facts = score_facts(&session);
    let tied = facts
        .events
        .iter()
        .find(|event| event.voice == "bass" && event.duration_spelling.contains('~'))
        .map(|event| event.id.clone())
        .expect("the cello ties a quarter across the barline");

    let error = session
        .plan_group_edit(&edit(
            std::slice::from_ref(&tied),
            GroupIntent::ScaleDurations {
                ratio: "1/2".to_owned(),
            },
        ))
        .expect_err("rewriting one link of a tie would change what the tie sums to");
    assert!(matches!(error, ProjectError::Uneditable(_)), "{error:?}");
    assert_eq!(source(&session), example("tuplet-fixture.musa"));
}

#[test]
fn a_tuplet_member_takes_a_scale_and_refuses_a_written_duration() {
    // Inside a tuplet the written value is not the counted one, so setting a
    // duration would say something the group already decides (OMT ch. 011).
    // A uniform factor is a different question: it composes with the group's
    // ratio and leaves the grouping intact.
    let mut session = ProjectSession::from_text(
        "piece \"t\" {\n    meter 4/4;\n    score { part p { voice v { tuplet 3/2 { c5/8 d5/8 e5/8 } } } }\n}\n"
            .to_owned(),
        "tuplet",
    );
    let facts = score_facts(&session);
    let inside = voice_events(&facts, "v");
    assert_eq!(inside.len(), 3, "the triplet has three members");

    let error = session
        .plan_group_edit(&edit(
            &inside,
            GroupIntent::SetEachDuration {
                duration: "1/8".to_owned(),
            },
        ))
        .expect_err("inside a tuplet, what is written is not what is counted");
    assert!(matches!(error, ProjectError::Uneditable(_)), "{error:?}");

    let scaled = session
        .plan_group_edit(&edit(&inside, GroupIntent::ScaleDurations { ratio: "2".to_owned() }))
        .expect("a uniform factor composes with the tuplet's own ratio");
    assert!(
        scaled.source().contains("tuplet 3/2 { c5/4 d5/4 e5/4 }"),
        "{}",
        scaled.source()
    );
}

#[test]
fn a_transformation_that_would_not_compile_is_refused_before_it_is_offered() {
    let mut session = session("tuplet-fixture.musa");
    let facts = score_facts(&session);
    let note = facts
        .events
        .iter()
        .find(|event| event.voice == "lead" && event.bar == 1 && event.duration_spelling == "1/4")
        .map(|event| event.id.clone())
        .expect("the flute writes a quarter in the first bar");

    let error = session
        .plan_group_edit(&edit(
            std::slice::from_ref(&note),
            GroupIntent::ScaleDurations { ratio: "2".to_owned() },
        ))
        .expect_err("a written barline says what the bar holds");
    assert!(matches!(error, ProjectError::RejectedEdit { .. }), "{error:?}");
}

// -------------------------------------------------------- generated music

#[test]
fn transforming_generated_music_transforms_the_motif_and_says_so() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let lead = voice_events(&facts, "lead");
    let second = lead[2].clone(); // `c5/2` of the first `use sigh(e5)`

    let plan = session
        .plan_group_edit(&edit(
            std::slice::from_ref(&second),
            GroupIntent::MoveDiatonically { steps: 1 },
        ))
        .expect("the motif spells that note");

    assert_eq!(plan.edits().len(), 1, "one statement is rewritten");
    assert_eq!(plan.definitions().len(), 1);
    let definition = &plan.definitions()[0];
    assert_eq!(definition.motif.as_deref(), Some("sigh"));
    assert_eq!(definition.occurrences.len(), 2, "the motif is used twice");
    assert_eq!(definition.selected, *std::slice::from_ref(&second));
    assert_eq!(definition.events.len(), 2, "both occurrences change");
    assert_eq!(definition.before, "c5");
    assert_eq!(definition.after, "d5");
    assert_eq!(plan.changed().len(), 2);
    assert!(plan.changed().contains(&second));
}

#[test]
fn both_occurrences_of_one_statement_ask_for_one_edit() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let lead = voice_events(&facts, "lead");
    // The same motif note, in the plain call and inside `transpose down P5`.
    let both = vec![lead[2].clone(), lead[7].clone()];

    let plan = session
        .plan_group_edit(&edit(&both, GroupIntent::MoveDiatonically { steps: 1 }))
        .expect("one statement spells both");
    assert_eq!(plan.edits().len(), 1);
    assert_eq!(plan.changed().len(), 2);
}

#[test]
fn a_pitch_the_source_names_with_a_parameter_is_refused() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let first = nth_event(&facts, "lead", 0); // `root/2`

    let error = session
        .plan_group_edit(&edit(
            std::slice::from_ref(&first),
            GroupIntent::TransposeBy {
                interval: "up P5".to_owned(),
            },
        ))
        .expect_err("the motif does not write that pitch, the call does");
    assert!(matches!(error, ProjectError::Uneditable(_)), "{error:?}");

    session
        .plan_group_edit(&edit(
            &[first],
            GroupIntent::ScaleDurations {
                ratio: "1/2".to_owned(),
            },
        ))
        .expect("its duration is written where the motif is");
}

#[test]
fn a_specialization_leaves_the_motif_alone() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let lead = voice_events(&facts, "lead");
    let second = lead[2].clone();

    let offered = session
        .plan_group_edit(&edit(
            std::slice::from_ref(&second),
            GroupIntent::MoveDiatonically { steps: 1 },
        ))
        .expect("the motif spells that note")
        .specializable();
    assert!(offered, "one call, one occurrence: the override says exactly this");

    let after = transform(
        &mut session,
        &GroupEdit {
            events: vec![second],
            intent: GroupIntent::MoveDiatonically { steps: 1 },
            mode: GeneratedEditMode::Specialize,
        },
    )
    .expect("an override is written onto the call");

    assert!(
        after.contains("motif sigh(root: Pitch) {\n        root/2\n        rest/4\n        c5/2"),
        "{after}"
    );
    assert!(after.contains("with {"), "{after}");
}

// ---------------------------------------------------------- the transaction

#[test]
fn a_refusal_leaves_the_session_exactly_where_it_was() {
    let mut session = session("tuplet-fixture.musa");
    let before = session.snapshot().revision();
    let facts = score_facts(&session);
    let note = facts
        .events
        .iter()
        .find(|event| event.voice == "lead" && event.bar == 1 && event.duration_spelling == "1/4")
        .map(|event| event.id.clone())
        .expect("the flute writes a quarter in the first bar");

    session
        .plan_group_edit(&edit(
            std::slice::from_ref(&note),
            GroupIntent::ScaleDurations { ratio: "2".to_owned() },
        ))
        .expect_err("a written barline says what the bar holds");
    assert_eq!(source(&session), example("tuplet-fixture.musa"));
    assert_eq!(session.snapshot().revision(), before);
}

#[test]
fn a_transformation_touches_only_the_statements_it_names() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let upper = voice_events(&facts, "upper");
    let before = example("glass-mountain.musa");

    let after = transform(&mut session, &edit(&upper, GroupIntent::ShiftAccidentals { steps: 1 }))
        .expect("four notes take a sharp");

    // Multi-byte text before the score: the spans are byte offsets, and an
    // edit computed against the wrong unit would cut the copyright in half.
    assert!(
        after.contains("copyright \"© 2026. Licensed CC BY-SA 4.0.\";"),
        "{after}"
    );
    assert!(after.contains("motif sigh(root: Pitch)"), "{after}");
    assert!(after.contains("mark pedal"), "{after}");
    assert_eq!(
        after.lines().count(),
        before.lines().count(),
        "a replacement of four spellings is not a rewrite of the file"
    );
    assert!(after.contains("c#5/1") && after.contains("g##4/1"), "{after}");
}

#[test]
fn one_transformation_is_one_undo() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let upper = voice_events(&facts, "upper");

    let after = transform(
        &mut session,
        &edit(
            &upper,
            GroupIntent::TransposeBy {
                interval: "down m3".to_owned(),
            },
        ),
    )
    .expect("a minor third down is spellable");
    assert_ne!(after, example("glass-mountain.musa"));

    session.undo().expect("the transformation is one entry");
    assert_eq!(source(&session), example("glass-mountain.musa"));
}

#[test]
fn a_plan_is_consumed_once_and_is_stale_afterwards() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let upper = voice_events(&facts, "upper");

    let plan = session
        .plan_group_edit(&edit(&upper, GroupIntent::ShiftAccidentals { steps: 1 }))
        .expect("four notes take a sharp");
    let (id, revision) = (plan.id(), plan.revision());
    session
        .apply(ProjectCommand::ApplyGroupEdit { plan: id, revision })
        .expect("the first acceptance commits");

    let error = session
        .apply(ProjectCommand::ApplyGroupEdit { plan: id, revision })
        .expect_err("the second is a replay of an edit that already happened");
    assert!(matches!(error, ProjectError::Uneditable(_)), "{error:?}");
    assert!(session.group_edit_plan(id).is_none());
}

#[test]
fn a_plan_from_an_older_revision_is_refused() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let upper = voice_events(&facts, "upper");

    let plan = session
        .plan_group_edit(&edit(&upper, GroupIntent::ShiftAccidentals { steps: 1 }))
        .expect("four notes take a sharp");
    let (id, revision) = (plan.id(), plan.revision());

    session
        .apply(ProjectCommand::SetSource(
            example("glass-mountain.musa").replace("for violin and strings", "for violin and strings, revised"),
        ))
        .expect("editing the subtitle is an ordinary edit");
    let error = session
        .apply(ProjectCommand::ApplyGroupEdit { plan: id, revision })
        .expect_err("the byte ranges describe a document that is no longer current");
    assert!(matches!(error, ProjectError::Uneditable(_)), "{error:?}");
}

#[test]
fn a_preview_states_which_bars_end_up_holding_something_else() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let upper = voice_events(&facts, "upper");

    let plan = session
        .plan_group_edit(&edit(
            &upper,
            GroupIntent::ScaleDurations {
                ratio: "1/2".to_owned(),
            },
        ))
        .expect("halving four whole notes is legal without written barlines");
    assert!(!plan.bars().is_empty(), "four bars became two");
    assert!(plan.bars().iter().all(|bar| bar.voice == "upper"), "{:?}", plan.bars());
}
