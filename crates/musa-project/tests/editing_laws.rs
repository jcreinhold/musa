//! Structured score editing: provenance in, transactions out (roadmap §14.6).
//!
//! Two contracts are worth stating plainly, because everything else follows
//! from them. First, an edit rewrites exactly one statement, so a generated
//! note's edit changes every event that statement spelled — and the session
//! says how many *before* it happens, which is what makes
//! `04-provenance.md` §4's choice honest. Second, an edit is a transaction:
//! source that would not compile is refused whole, leaving the session at the
//! revision it was already at.

// A fixture that does not contain what a test looks for is the test failing.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::path::PathBuf;

use musa_project::{
    EditCommand, GeneratedEditMode, HeaderField, InsertAt, NoteSpec, ProjectCommand, ProjectError, ProjectSession,
    ScoreFacts,
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

/// The id of the nth event of a voice, in score order.
fn nth_event(facts: &ScoreFacts, voice: &str, index: usize) -> String {
    facts
        .events
        .iter()
        .filter(|event| event.voice == voice)
        .nth(index)
        .map_or_else(|| panic!("{voice} has no event {index}"), |event| event.id.clone())
}

fn source(session: &ProjectSession) -> String {
    session.snapshot().source().to_owned()
}

#[test]
fn an_authored_note_changes_only_itself() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let id = nth_event(&facts, "upper", 3); // `gs4 1;`

    let impact = session
        .edit_impact(&EditCommand::ChangePitch {
            event: id.clone(),
            pitch: "g4".to_owned(),
            mode: GeneratedEditMode::EditDefinition,
        })
        .expect("the event exists");
    assert!(!impact.generated, "the strings parts are typed out");
    assert_eq!(impact.events, vec![id.clone()]);
    assert_eq!(impact.occurrences, 0);

    let before = source(&session);
    session
        .apply(ProjectCommand::EditScore(EditCommand::ChangePitch {
            event: id,
            pitch: "g4".to_owned(),
            mode: GeneratedEditMode::EditDefinition,
        }))
        .expect("a legal respelling");
    assert_eq!(source(&session), before.replace("gs4 1;", "g4 1;"));
}

#[test]
fn a_generated_note_states_its_consequence_in_counts() {
    let session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    // The violin's third note: `c5 1/2;` in the motif body, reached through
    // the first `use sigh()`.
    let id = nth_event(&facts, "lead", 2);

    let impact = session
        .edit_impact(&EditCommand::ChangePitch {
            event: id.clone(),
            pitch: "d5".to_owned(),
            mode: GeneratedEditMode::EditDefinition,
        })
        .expect("the event exists");

    assert!(impact.generated);
    assert_eq!(impact.motif.as_deref(), Some("sigh"));
    assert_eq!(impact.occurrence.as_deref(), Some("sigh()"));
    // `sigh()` is used twice, so one statement of its body spells two notes.
    // Two, not ten: the count is what changes, not the size of the
    // expansions (`04-provenance.md` §4, as repaired by this prompt).
    assert_eq!(impact.occurrences, 2);
    assert_eq!(impact.events.len(), 2, "one note per occurrence");
    assert!(impact.events.contains(&id));
}

#[test]
fn editing_a_definition_changes_every_occurrence_at_once() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let id = nth_event(&facts, "lead", 2);

    session
        .apply(ProjectCommand::EditScore(EditCommand::ChangePitch {
            event: id,
            pitch: "d5".to_owned(),
            mode: GeneratedEditMode::EditDefinition,
        }))
        .expect("editing the definition is supported");

    let text = source(&session);
    assert!(text.contains("        d5 1/2;"), "the motif body was rewritten");
    assert!(!text.contains("        c5 1/2;"));

    // And the music followed: both occurrences now sound the new pitch.
    let after = score_facts(&session);
    let changed = after
        .events
        .iter()
        .filter(|event| event.voice == "lead" && event.pitch.as_deref() == Some("D5"))
        .count();
    assert_eq!(changed, 1, "the plain occurrence sounds D5");
    // The transposed occurrence sounds it a fifth lower, spelled as the
    // compiler spells it — the point being that it changed too.
    assert!(
        after
            .events
            .iter()
            .any(|event| event.voice == "lead" && event.pitch.as_deref() == Some("G4")),
        "the transposed occurrence changed with it"
    );
}

#[test]
fn specializing_a_note_changes_that_occurrence_and_no_other() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let id = nth_event(&facts, "lead", 2); // the `c5 1/2;` of the plain occurrence

    // The core says the choice is available before it is offered.
    let impact = session
        .edit_impact(&EditCommand::ChangePitch {
            event: id.clone(),
            pitch: "d5".to_owned(),
            mode: GeneratedEditMode::Specialize,
        })
        .expect("a generated note");
    assert!(impact.generated);
    assert!(impact.specializable, "this call runs once, so it can be specialized");

    session
        .apply(ProjectCommand::EditScore(EditCommand::ChangePitch {
            event: id,
            pitch: "d5".to_owned(),
            mode: GeneratedEditMode::Specialize,
        }))
        .expect("the specialization applies");

    let text = source(&session);
    assert!(
        text.contains("use sigh() with { note 3 = d5; }"),
        "the occurrence carries its own override:\n{text}"
    );
    assert!(text.contains("        c5 1/2;"), "and the motif is untouched");

    // One occurrence sounds the new note; the transposed one still sounds
    // what the motif says, a fifth down.
    let after = score_facts(&session);
    let voices: Vec<&str> = after
        .events
        .iter()
        .filter(|event| event.voice == "lead")
        .filter_map(|event| event.pitch.as_deref())
        .collect();
    assert_eq!(voices.iter().filter(|pitch| **pitch == "D5").count(), 1);
    assert!(
        voices.contains(&"F4"),
        "the transposed occurrence still sounds the motif's note a fifth down: {voices:?}"
    );
}

#[test]
fn a_call_that_runs_more_than_once_says_why_it_cannot_be_specialized() {
    // A `with` clause belongs to the call, so specializing a note inside a
    // `repeat` would change every run of it. That is exactly what the mode
    // promises not to do, so it is refused rather than approximated.
    let mut session = ProjectSession::from_text(
        r#"piece "Etude" {
    motif sigh() { e5 1/2; c5 1/2; }
    score { part piano { voice right { repeat 2 { use sigh(); } } } }
}
"#
        .to_owned(),
        "etude.musa",
    );
    let facts = score_facts(&session);
    let id = nth_event(&facts, "right", 0);
    let before = source(&session);

    let impact = session
        .edit_impact(&EditCommand::ChangePitch {
            event: id.clone(),
            pitch: "d5".to_owned(),
            mode: GeneratedEditMode::Specialize,
        })
        .expect("a generated note");
    assert!(!impact.specializable, "the interface is told before it offers it");

    match session.apply(ProjectCommand::EditScore(EditCommand::ChangePitch {
        event: id,
        pitch: "d5".to_owned(),
        mode: GeneratedEditMode::Specialize,
    })) {
        Err(ProjectError::Uneditable(reason)) => {
            assert!(reason.contains("more than once"), "it says why: {reason}");
        }
        Err(other) => panic!("the wrong refusal: {other}"),
        Ok(_) => panic!("specializing a repeated call must not change every run"),
    }
    assert_eq!(source(&session), before, "and nothing happened");
}

#[test]
fn a_duration_cannot_be_specialized_and_says_so() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let id = nth_event(&facts, "lead", 2);

    match session.apply(ProjectCommand::EditScore(EditCommand::ChangeDuration {
        event: id,
        duration: "1/4".to_owned(),
        mode: GeneratedEditMode::Specialize,
    })) {
        Err(ProjectError::Uneditable(reason)) => {
            assert!(reason.contains("respells"), "it says what an override does: {reason}");
        }
        Err(other) => panic!("the wrong refusal: {other}"),
        Ok(_) => panic!("an override cannot renotate a note"),
    }
}

#[test]
fn a_note_can_be_entered_at_the_end_of_a_voice_and_undone() {
    let mut session = session("glass-mountain.musa");
    let before = source(&session);
    let revision = session.snapshot().revision();

    session
        .apply(ProjectCommand::EditScore(EditCommand::InsertNote {
            at: InsertAt::EndOfVoice {
                part: "strings".to_owned(),
                voice: "bass".to_owned(),
            },
            note: NoteSpec::Note {
                pitch: "a2".to_owned(),
                duration: "1".to_owned(),
            },
        }))
        .expect("a legal insertion");

    assert!(source(&session).contains("                e2 1;\n                a2 1;"));
    assert_ne!(session.snapshot().revision(), revision);
    let after = score_facts(&session);
    assert_eq!(
        after.events.iter().filter(|event| event.voice == "bass").count(),
        5,
        "the bass gained a note"
    );

    session.undo().expect("the edit is in the history");
    assert_eq!(source(&session), before);
}

#[test]
fn insertion_after_a_generated_event_writes_after_the_use_it_came_from() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let id = nth_event(&facts, "lead", 2);

    session
        .apply(ProjectCommand::EditScore(EditCommand::InsertNote {
            at: InsertAt::After { event: id },
            note: NoteSpec::Rest {
                duration: "1/4".to_owned(),
            },
        }))
        .expect("a legal insertion");

    // Not inside the motif: the statement the composer can see at that place
    // in the score is the `use`, so that is what the new note follows.
    assert!(source(&session).contains("                use sigh();\n                rest 1/4;"));
}

#[test]
fn extracting_a_motif_names_a_run_and_leaves_a_use() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let run: Vec<String> = facts
        .events
        .iter()
        .filter(|event| event.voice == "bass")
        .map(|event| event.id.clone())
        .collect();

    session
        .apply(ProjectCommand::EditScore(EditCommand::ExtractMotif {
            events: run,
            name: "ground".to_owned(),
        }))
        .expect("four authored notes are extractable");

    let text = source(&session);
    assert!(text.contains("    motif ground() {\n        a2 1;\n        f2 1;\n        d2 1;\n        e2 1;\n    }"));
    assert!(text.contains("            voice bass {\n                use ground();\n            }"));

    // And the music is the same music, now with provenance.
    let after = score_facts(&session);
    let bass: Vec<_> = after.events.iter().filter(|event| event.voice == "bass").collect();
    assert_eq!(bass.len(), 4);
    assert!(bass.iter().all(|event| event.origin.generated));
}

#[test]
fn a_motif_cannot_be_extracted_out_of_generated_music() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let run = vec![nth_event(&facts, "lead", 0), nth_event(&facts, "lead", 1)];
    let before = source(&session);

    let refused = session.apply(ProjectCommand::EditScore(EditCommand::ExtractMotif {
        events: run,
        name: "nested".to_owned(),
    }));
    assert!(matches!(refused, Err(ProjectError::Uneditable(_))));
    assert_eq!(source(&session), before);
}

#[test]
fn an_edit_that_would_not_compile_is_refused_whole() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let id = nth_event(&facts, "upper", 0);
    let before = source(&session);
    let revision = session.snapshot().revision();

    // `h4` is not a pitch anyone can spell.
    let refused = session.apply(ProjectCommand::EditScore(EditCommand::ChangePitch {
        event: id,
        pitch: "h4".to_owned(),
        mode: GeneratedEditMode::EditDefinition,
    }));

    match refused {
        Err(ProjectError::RejectedEdit { intent, reason }) => {
            assert!(intent.contains("h4"), "the refusal names the attempt: {intent}");
            assert!(!reason.is_empty(), "and says why");
        }
        Err(other) => panic!("the wrong refusal: {other}"),
        Ok(_) => panic!("an edit that breaks the piece must not land"),
    }
    // The whole point: not a new revision holding broken text.
    assert_eq!(source(&session), before);
    assert_eq!(session.snapshot().revision(), revision);
    assert!(session.snapshot().compiles());
}

#[test]
fn an_event_from_a_stale_selection_is_refused_by_id() {
    let mut session = session("glass-mountain.musa");
    let refused = session.apply(ProjectCommand::EditScore(EditCommand::ChangeDuration {
        event: "event-ffff".to_owned(),
        duration: "1/8".to_owned(),
        mode: GeneratedEditMode::EditDefinition,
    }));
    assert!(matches!(refused, Err(ProjectError::NoSuchEvent(_))));
}

#[test]
fn a_duration_change_is_the_same_transaction_as_a_pitch_change() {
    let mut session = session("glass-mountain.musa");
    let facts = score_facts(&session);
    let id = nth_event(&facts, "lead", 1); // the motif's `rest 1/4;`

    let impact = session
        .edit_impact(&EditCommand::ChangeDuration {
            event: id.clone(),
            duration: "1/8".to_owned(),
            mode: GeneratedEditMode::EditDefinition,
        })
        .expect("the event exists");
    assert_eq!(impact.events.len(), 2, "a rest in a motif is two rests in the score");

    session
        .apply(ProjectCommand::EditScore(EditCommand::ChangeDuration {
            event: id,
            duration: "1/8".to_owned(),
            mode: GeneratedEditMode::EditDefinition,
        }))
        .expect("a legal renotation");
    assert!(source(&session).contains("        rest 1/8;"));
}

// --- The piece's own facts (prompt 54) -------------------------------------

/// Setting a statement the piece already has rewrites its value and nothing
/// else on the line.
#[test]
fn a_named_header_is_rewritten_in_place() {
    let mut session = session("glass-mountain.musa");
    session
        .apply(ProjectCommand::EditScore(EditCommand::SetHeader {
            field: HeaderField::Composer,
            value: "Ada Lovelace".to_owned(),
        }))
        .expect("a composer's name compiles");

    let text = source(&session);
    assert!(text.contains("composer \"Ada Lovelace\";"), "{text}");
    assert_eq!(text.matches("composer ").count(), 1, "no second statement: {text}");
    // The neighbours are untouched — this rewrote a value, not a block.
    assert!(text.contains("subtitle \"for violin and strings\";"), "{text}");
}

/// A statement the piece does not have is written where the order says it
/// goes, not appended wherever there happened to be room.
#[test]
fn an_unnamed_header_is_inserted_in_order() {
    let mut session = session("glass-mountain.musa");
    session
        .apply(ProjectCommand::EditScore(EditCommand::SetHeader {
            field: HeaderField::Arranger,
            value: "after a folk tune".to_owned(),
        }))
        .expect("an arranger compiles");

    let text = source(&session);
    let arranger = text.find("arranger").expect("the statement was written");
    let composer = text.find("composer").expect("the fixture names a composer");
    let copyright = text.find("copyright").expect("the fixture names a copyright");
    assert!(composer < arranger && arranger < copyright, "{text}");
}

/// A piece that states nothing about itself yet gets its first statement on
/// the line after the brace.
#[test]
fn the_first_header_lands_after_the_brace() {
    let mut session = ProjectSession::from_text(
        "piece \"Bare\" {\n    score { part p { voice v { c4 1/4; } } }\n}\n".to_owned(),
        "bare.musa",
    );
    session
        .apply(ProjectCommand::EditScore(EditCommand::SetHeader {
            field: HeaderField::Composer,
            value: "Ada".to_owned(),
        }))
        .expect("a composer compiles");

    assert!(
        source(&session).starts_with("piece \"Bare\" {\n    composer \"Ada\";\n"),
        "{}",
        source(&session)
    );
}

/// An empty value removes the statement, line and all. Adding and taking back
/// a line of front matter are the same gesture.
#[test]
fn an_empty_value_removes_the_statement() {
    let mut session = session("glass-mountain.musa");
    session
        .apply(ProjectCommand::EditScore(EditCommand::SetHeader {
            field: HeaderField::Subtitle,
            value: String::new(),
        }))
        .expect("a piece without a subtitle compiles");

    let text = source(&session);
    assert!(!text.contains("subtitle"), "{text}");
    assert!(!text.contains("\n\n\n"), "no blank line left behind: {text}");
    assert!(text.contains("composer \"musa\";"), "{text}");
}

/// The title is the one field that cannot be emptied, and it says why.
#[test]
fn the_title_refuses_to_be_emptied() {
    let mut session = session("glass-mountain.musa");
    let before = source(&session);
    let error = session
        .apply(ProjectCommand::EditScore(EditCommand::SetHeader {
            field: HeaderField::Title,
            value: String::new(),
        }))
        .expect_err("a piece has to be called something");

    assert!(error.to_string().contains("called something"), "{error}");
    assert_eq!(source(&session), before);
}

/// A value that does not compile leaves the session exactly as it was — the
/// same transaction rule every other edit follows. The frontend validates
/// nothing, so this is the only thing standing between a typo and a broken
/// document.
#[test]
fn a_header_that_does_not_compile_is_refused_whole() {
    let mut session = session("glass-mountain.musa");
    let before = source(&session);
    let revision = session.snapshot().revision();

    let result = session.apply(ProjectCommand::EditScore(EditCommand::SetHeader {
        field: HeaderField::Meter,
        value: "not a meter".to_owned(),
    }));

    assert!(result.is_err(), "{result:?}");
    assert_eq!(source(&session), before);
    assert_eq!(session.snapshot().revision(), revision);
}

/// A quote a composer types into a title survives the round trip: it is
/// escaped on the way in and read back as itself.
#[test]
fn a_quotation_mark_in_a_title_round_trips() {
    let mut session = session("glass-mountain.musa");
    session
        .apply(ProjectCommand::EditScore(EditCommand::SetHeader {
            field: HeaderField::Title,
            value: "The \"Glass\" Mountain".to_owned(),
        }))
        .expect("a quoted word in a title compiles");

    assert!(
        source(&session).contains(r#"piece "The \"Glass\" Mountain""#),
        "{}",
        source(&session)
    );
    let snapshot = session.snapshot();
    let mei = snapshot.mei().unwrap_or_default();
    // The XML writer escapes it once more on its own way out; either spelling
    // means the quote survived the language.
    assert!(
        mei.contains("The &quot;Glass&quot; Mountain") || mei.contains("The \"Glass\" Mountain"),
        "{mei}"
    );
}

/// Every header edit is one revision and one undo — a composer who renames a
/// piece takes it back with ⌘Z like anything else.
#[test]
fn a_header_edit_is_one_undo() {
    let mut session = session("glass-mountain.musa");
    let before = source(&session);
    session
        .apply(ProjectCommand::EditScore(EditCommand::SetHeader {
            field: HeaderField::Tempo,
            value: "quarter = 96".to_owned(),
        }))
        .expect("a tempo compiles");
    assert!(source(&session).contains("tempo quarter = 96;"));

    session.undo().expect("there is something to undo");
    assert_eq!(source(&session), before);
}
