//! Laws for the explicit, compiler-proved barline source action.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_in_result)]

use musa_project::{BarlineBlockerReason, ProjectCommand, ProjectError, ProjectSession};

type Result = std::result::Result<(), ProjectError>;

fn piece(body: &str) -> String {
    format!("piece \"Bars\" {{ meter 4/4; score {{ part p {{ voice v {{ {body} }} }} }} }}")
}

#[test]
fn literal_notes_rewrite_transactionally_and_idempotently() -> Result {
    let source = piece("c4/4 d4/4 e4/4 f4/4 g4/4 a4/4 b4/4 c5/4");
    let mut session = ProjectSession::from_text(&source, "bars.musa");
    let revision = session.snapshot().revision();
    let preview = session
        .barline_rewrite()
        .expect("valid plan")
        .expect("complete measured run");
    assert_eq!(preview.inserted(), 2);
    assert!(preview.source().contains("| c4/4 d4/4 e4/4 f4/4"));
    assert!(preview.source().contains("| g4/4 a4/4 b4/4 c5/4"));

    let mut applied_edits = ProjectSession::from_text(&source, "bars.musa");
    applied_edits.apply(ProjectCommand::ApplyEdits(preview.edits().to_vec()))?;
    assert_eq!(
        applied_edits.snapshot().source(),
        preview.source(),
        "the returned edit plan is the transaction the command applies"
    );

    session.apply(ProjectCommand::InsertBarlines { revision })?;
    assert_eq!(session.snapshot().source(), preview.source());
    assert_eq!(session.barline_rewrite().expect("valid already-barred result"), None);
    session.undo()?;
    assert_eq!(session.snapshot().source(), source);
    Ok(())
}

#[test]
fn incomplete_passages_and_crossing_items_are_refused() {
    let incomplete = ProjectSession::from_text(piece("c4/4 d4/4"), "short.musa");
    assert_eq!(
        incomplete.barline_rewrite().expect_err("short closing run").reason(),
        BarlineBlockerReason::EndsBetweenBarlines
    );

    let crossing = ProjectSession::from_text(
        concat!(
            "piece \"Use\" { meter 4/4; motif long() { c4/1 d4/1 } ",
            "score { part p { voice v { use long(); } } } }"
        ),
        "crossing.musa",
    );
    assert_eq!(
        crossing.barline_rewrite().expect_err("multi-bar use").reason(),
        BarlineBlockerReason::ItemCrossesBoundary
    );
}

#[test]
fn stale_preview_is_never_replayed() -> Result {
    let source = piece("c4/1");
    let mut session = ProjectSession::from_text(source, "stale.musa");
    let previewed = session.snapshot().revision();
    session.apply(ProjectCommand::SetSource(piece("d4/1")))?;
    let refused = session
        .apply(ProjectCommand::InsertBarlines { revision: previewed })
        .expect_err("old byte positions must be refused");
    assert!(refused.to_string().contains("stale"));
    Ok(())
}

#[test]
fn rests_chords_uses_and_tuplets_share_the_checked_duration_path() {
    let source = concat!(
        "piece \"Kinds\" { meter 4/4; motif whole() { c4/1 } score { part p { voice v { ",
        "rest 1/4 [c4 e4 g4] 1/4 c4/4 d4/4 use whole(); ",
        "tuplet 3/2 { c4/8 d4/8 e4/8 } tuplet 3/2 { c4/8 d4/8 e4/8 } ",
        "tuplet 3/2 { c4/8 d4/8 e4/8 } tuplet 3/2 { c4/8 d4/8 e4/8 } ",
        "} } } }"
    );
    let session = ProjectSession::from_text(source, "kinds.musa");
    assert!(
        session.snapshot().compiles(),
        "fixture: {:?}",
        session.snapshot().diagnostics()
    );
    assert_eq!(
        session
            .barline_rewrite()
            .expect("valid plan")
            .expect("three exact measures")
            .inserted(),
        3
    );
}

#[test]
fn point_facts_join_the_measure_that_follows_and_do_not_open_an_empty_one() {
    let session = ProjectSession::from_text(piece("dynamic p; c4/4 d4/4 e4/4 f4/4 clef bass;"), "points.musa");
    let rewrite = session
        .barline_rewrite()
        .expect("valid plan")
        .expect("one sounded measure");
    assert_eq!(rewrite.inserted(), 1);
    assert!(rewrite.source().contains("| dynamic p;"));
}

#[test]
fn meter_changes_polymeter_existing_bars_and_unmeasured_scopes_are_scoped() {
    let changed = ProjectSession::from_text(piece("c4/4 d4/4 e4/4 f4/4 meter 3/4; g4/4 a4/4 b4/4"), "changes.musa");
    assert_eq!(
        changed
            .barline_rewrite()
            .expect("valid plan")
            .expect("both meter stretches")
            .inserted(),
        2
    );

    let scoped = ProjectSession::from_text(
        concat!(
            "piece \"Scoped\" { meter 4/4; score { ",
            "part four { voice v { bar { c4/1 } d4/4 e4/4 f4/4 g4/4 } } ",
            "part seven { meter 7/8; voice v { c4/8 d4/8 e4/8 f4/8 g4/8 a4/8 b4/8 } } ",
            "part free { meter none; voice v { c4/4 d4/4 } } ",
            "} }"
        ),
        "scoped.musa",
    );
    assert!(
        scoped.snapshot().compiles(),
        "fixture: {:?}",
        scoped.snapshot().diagnostics()
    );
    assert_eq!(
        scoped
            .barline_rewrite()
            .expect("valid plan")
            .expect("two measured loose scopes")
            .inserted(),
        2
    );
}

#[test]
fn a_loose_run_after_partial_structured_music_is_not_called_a_pickup() {
    let session = ProjectSession::from_text(piece("repeat 1 { c4/4 } d4/4 e4/4 f4/4"), "opening.musa");
    assert_eq!(
        session
            .barline_rewrite()
            .expect_err("run starts a quarter into the measure")
            .reason(),
        BarlineBlockerReason::StartsBetweenBarlines
    );
}
