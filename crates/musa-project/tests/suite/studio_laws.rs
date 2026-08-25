//! The Sound and Mix workspaces as structured editors of studio source
//! (roadmap §11, §14.4).
//!
//! One contract governs every test here: the `.musa` text is the studio, and
//! a knob is a way of writing it. So an edit must produce the *smallest* text
//! change that means what the knob meant — the composer's units, argument
//! order, and comments all survive a fader move — and the facts a workspace
//! draws from must be the facts the compiler resolved, not a parallel model
//! the interface keeps in step.

// A fixture that does not contain what a test looks for is the test failing.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::path::PathBuf;

use musa_project::{ContainerKind, ProjectCommand, ProjectError, ProjectSession, StudioEdit, StudioFacts};

fn example(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn session(name: &str) -> ProjectSession {
    ProjectSession::from_text(example(name), name)
}

fn facts(session: &ProjectSession) -> StudioFacts {
    session.snapshot().studio().cloned().expect("the example compiles")
}

fn source(session: &ProjectSession) -> String {
    session.snapshot().source().to_owned()
}

/// One stage of one patch, by the processor that runs it.
fn stage_of(facts: &StudioFacts, patch: &str, processor: &str) -> usize {
    facts
        .patches
        .iter()
        .find(|container| container.name == patch)
        .and_then(|container| container.stages.iter().find(|stage| stage.processor == processor))
        .map_or_else(|| panic!("{patch} has no {processor}"), |stage| stage.index)
}

fn param(facts: &StudioFacts, patch: &str, processor: &str, name: &str) -> musa_project::ParamFacts {
    facts
        .patches
        .iter()
        .find(|container| container.name == patch)
        .and_then(|container| container.stages.iter().find(|stage| stage.processor == processor))
        .and_then(|stage| stage.params.iter().find(|param| param.name == name))
        .cloned()
        .unwrap_or_else(|| panic!("{patch}'s {processor} has no {name}"))
}

fn set(session: &mut ProjectSession, edit: StudioEdit) -> Result<(), ProjectError> {
    session.apply(ProjectCommand::EditStudio(edit)).map(|_| ())
}

/// A patch that writes a value in milliseconds and leaves a reverb's `mix`
/// unwritten, so a test can see what an edit does to each.
const CAREFUL: &str = r#"piece "Careful" {
    tempo 1/4 = 90;
    meter 4/4;
    score {
        part pad {
            voice one { c4/1 }
        }
    }
    studio {
        patch soft {
            oscillator(sine)
                |> envelope(adsr(attack: 30 ms, decay: 1 s, sustain: 0.5, release: 2 s))
                |> lowpass(cutoff: 900 Hz, resonance: 0.7)
                |> output;
        }
        bus room {
            reverb(room: 0.5, damping: 0.4);
        }
        assign pad -> soft;
        route pad -> master;
        send pad -> room at -12 dB;
        route room -> master;
    }
}
"#;

/// A studio that declares a patch but never assigns the part to it. The
/// language will not let such a part be routed or sent — an assignment is what
/// makes a part a signal — so the fixture leaves both out too.
const UNASSIGNED: &str = r#"piece "Unassigned" {
    tempo 1/4 = 90;
    meter 4/4;
    score {
        part pad {
            voice one { c4/1 }
        }
    }
    studio {
        patch soft {
            oscillator(sine) |> output;
        }
    }
}
"#;

#[test]
fn the_facts_report_what_the_compiler_resolved() {
    let studio = facts(&session("glass-mountain.musa"));

    assert!(studio.declared);
    let cutoff = param(&studio, "glass_pad", "lowpass", "cutoff");
    assert_eq!((cutoff.value.numerator, cutoff.value.denominator), (1400, 1));
    assert_eq!(cutoff.unit, "Hz");
    assert!(cutoff.written);
    assert_eq!(cutoff.summary, "Sets the boundary frequency.");
    let lowpass = studio
        .patches
        .iter()
        .find(|patch| patch.name == "glass_pad")
        .and_then(|patch| patch.stages.iter().find(|stage| stage.processor == "lowpass"))
        .expect("the filter stage");
    assert_eq!(lowpass.origin, "bundled Musa source + registered primitive");
    assert!(lowpass.summary.starts_with("Keeps frequencies"));
    assert!(lowpass.signature.contains("resonance: Ratio"));
    // The written value is what the modulation moves around, so the interface
    // has to be able to say that a knob is not the whole story (§13.7).
    assert_eq!(cutoff.modulated_by.as_deref(), Some("lfo"));
}

#[test]
fn a_parameter_the_patch_left_alone_reads_as_its_default() {
    let studio = facts(&session("glass-mountain.musa"));

    // `reverb(room: 0.82, damping: 0.55)` says nothing about `mix`.
    let mix = studio
        .buses
        .iter()
        .find(|bus| bus.name == "hall")
        .and_then(|bus| bus.stages.first())
        .and_then(|stage| stage.params.iter().find(|param| param.name == "mix"))
        .cloned()
        .expect("the hall bus is a reverb");
    assert!(!mix.written);
    assert!(mix.span.is_none());
    assert_eq!((mix.value.numerator, mix.value.denominator), (1, 1));
}

#[test]
fn every_part_has_a_row_even_with_no_assignment() {
    let studio = facts(&ProjectSession::from_text(UNASSIGNED, "unassigned.musa"));

    let rows: Vec<(&str, Option<&str>)> = studio
        .assignments
        .iter()
        .map(|row| (row.part.as_str(), row.patch.as_deref()))
        .collect();
    assert_eq!(rows, vec![("pad", None)]);
}

#[test]
fn a_written_value_keeps_the_scale_it_was_written_in() {
    let mut session = ProjectSession::from_text(CAREFUL, "careful.musa");
    let stage = stage_of(&facts(&session), "soft", "envelope");

    set(
        &mut session,
        StudioEdit::SetParam {
            kind: ContainerKind::Patch,
            container: "soft".to_owned(),
            stage,
            param: "attack".to_owned(),
            value: 0.05,
        },
    )
    .expect("an envelope attack is editable");

    // Not `0.05 s`: the patch chose milliseconds, and an edit is not an
    // occasion to overrule that.
    assert!(source(&session).contains("attack: 50 ms"), "{}", source(&session));
    assert!(source(&session).contains("decay: 1 s"));
}

#[test]
fn setting_a_value_to_what_it_already_is_changes_nothing() {
    let mut session = ProjectSession::from_text(CAREFUL, "careful.musa");
    let before = source(&session);
    let revision = session.snapshot().revision();
    let stage = stage_of(&facts(&session), "soft", "envelope");

    set(
        &mut session,
        StudioEdit::SetParam {
            kind: ContainerKind::Patch,
            container: "soft".to_owned(),
            stage,
            param: "attack".to_owned(),
            value: 0.03,
        },
    )
    .expect("an envelope attack is editable");

    assert_eq!(source(&session), before);
    assert_eq!(session.snapshot().revision(), revision);
}

#[test]
fn a_value_the_patch_never_wrote_is_added_to_the_call() {
    let mut session = ProjectSession::from_text(CAREFUL, "careful.musa");

    set(
        &mut session,
        StudioEdit::SetParam {
            kind: ContainerKind::Bus,
            container: "room".to_owned(),
            stage: 0,
            param: "mix".to_owned(),
            value: 0.5,
        },
    )
    .expect("a reverb has a mix");

    assert!(
        source(&session).contains("reverb(room: 0.5, damping: 0.4, mix: 0.5)"),
        "{}",
        source(&session)
    );
}

#[test]
fn a_fader_rewrites_the_level_and_nothing_else() {
    let mut session = ProjectSession::from_text(CAREFUL, "careful.musa");

    set(
        &mut session,
        StudioEdit::SetSendLevel {
            source: "pad".to_owned(),
            bus: "room".to_owned(),
            decibels: -6.5,
        },
    )
    .expect("that send exists");

    assert!(
        source(&session).contains("send pad -> room at -6.5 dB;"),
        "{}",
        source(&session)
    );
}

#[test]
fn pointing_a_part_at_another_patch_replaces_the_name() {
    let mut session = ProjectSession::from_text(
        CAREFUL.replace(
            "patch soft {",
            "patch loud { oscillator(sine) |> output; }\n        patch soft {",
        ),
        "careful.musa",
    );

    set(
        &mut session,
        StudioEdit::AssignPatch {
            part: "pad".to_owned(),
            patch: "loud".to_owned(),
        },
    )
    .expect("both parts and patches exist");

    assert!(source(&session).contains("assign pad -> loud;"), "{}", source(&session));
}

#[test]
fn a_part_with_no_assignment_gets_one_written_for_it() {
    let mut session = ProjectSession::from_text(UNASSIGNED, "unassigned.musa");
    assert_eq!(
        facts(&session).assignments.first().and_then(|row| row.patch.clone()),
        None
    );

    set(
        &mut session,
        StudioEdit::AssignPatch {
            part: "pad".to_owned(),
            patch: "soft".to_owned(),
        },
    )
    .expect("the patch exists");

    assert!(source(&session).contains("assign pad -> soft;"), "{}", source(&session));
    assert_eq!(
        facts(&session).assignments.first().and_then(|row| row.patch.clone()),
        Some("soft".to_owned())
    );
}

#[test]
fn an_edit_naming_something_that_is_not_there_changes_nothing() {
    let mut session = ProjectSession::from_text(CAREFUL, "careful.musa");
    let before = source(&session);

    let refused = set(
        &mut session,
        StudioEdit::AssignPatch {
            part: "pad".to_owned(),
            patch: "nowhere".to_owned(),
        },
    );

    assert!(matches!(refused, Err(ProjectError::Uneditable(_))), "{refused:?}");
    assert_eq!(source(&session), before);
}

#[test]
fn a_studio_edit_can_be_undone_like_any_other() {
    let mut session = ProjectSession::from_text(CAREFUL, "careful.musa");
    let before = source(&session);

    set(
        &mut session,
        StudioEdit::SetSendLevel {
            source: "pad".to_owned(),
            bus: "room".to_owned(),
            decibels: 0.0,
        },
    )
    .expect("that send exists");
    assert_ne!(source(&session), before);

    session.undo().expect("there is something to undo");
    assert_eq!(source(&session), before);
}
