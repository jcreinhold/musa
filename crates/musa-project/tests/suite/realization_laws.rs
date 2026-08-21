//! What the session promises about a reading of an open work.
//!
//! A realization is a compile parameter, and the language has things to
//! leave open. What the session adds is everything a composer
//! could *do* about it: which performance is in force, what it decided, how
//! to draw another one, and how to keep the one thing you liked. Those are
//! session facts, so they are asserted here rather than in the interface —
//! the desktop's tests hold the screen to what this file says.
//!
//! `loop-lengths.musa` is the piece under test throughout: one question,
//! written once in each of three voices, so a reading that let the voices
//! disagree would fail these rather than look odd on a page.

// A missing decision or a piece that failed to compile is the failure these
// tests report, so panicking on one is the assertion.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

use musa_project::{ExportRequest, ProjectCommand, ProjectSession, ScoreFacts};

type Error = Box<dyn std::error::Error>;
type Result<T = ()> = std::result::Result<T, Error>;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

/// The open work, copied where a realization may be written beside it.
fn open_work(dir: &Path) -> Result<ProjectSession> {
    let path = dir.join("loop-lengths.musa");
    std::fs::copy(example("loop-lengths.musa"), &path)?;
    Ok(ProjectSession::open(&path)?)
}

fn score(session: &ProjectSession) -> ScoreFacts {
    session.snapshot().score().cloned().expect("the example compiles")
}

/// What this reading decided, as the interface reads it out.
fn answered(session: &ProjectSession) -> Vec<String> {
    score(session)
        .decisions
        .into_iter()
        .map(|decision| decision.answered)
        .collect()
}

/// A determinate piece has no performance, and therefore no row, no control
/// and no word about one anywhere in the interface.
#[test]
fn a_piece_that_asks_nothing_says_nothing() {
    let source = std::fs::read_to_string(example("counterpoint.musa")).unwrap_or_default();
    let session = ProjectSession::from_text(source, "counterpoint.musa");

    let facts = score(&session);
    assert_eq!(facts.performance, None);
    assert!(facts.decisions.is_empty());
    assert!(facts.events.iter().all(|event| event.origin.decision.is_none()));
}

/// An open work names its performance and every question it asked.
#[test]
fn an_open_work_says_which_reading_this_is() -> Result {
    let dir = tempfile::tempdir()?;
    let mut session = open_work(dir.path())?;
    session.apply(ProjectCommand::NewPerformance { performance: 4 })?;

    let facts = score(&session);
    assert_eq!(facts.performance, Some(4));
    // One question, though three voices write it: a repeat the page can
    // draw is one repeat of the piece.
    assert_eq!(facts.decisions.len(), 1);
    let decision = facts.decisions.first().expect("the piece asked something");
    assert_eq!(decision.answered, "2 passes");
    assert!(!decision.pinned);
    // The span points at the construct that asked, so the interface can open
    // the source there.
    let asked = session.snapshot().source()[decision.span.start as usize..].to_owned();
    assert!(asked.starts_with("repeat 2 to 6"), "{asked:.20}");
    Ok(())
}

/// Every note the open repeat produced knows which decision produced it, and
/// every note outside it knows that it is outside.
#[test]
fn a_note_names_the_decision_it_was_played_under() -> Result {
    let dir = tempfile::tempdir()?;
    let mut session = open_work(dir.path())?;
    session.apply(ProjectCommand::NewPerformance { performance: 4 })?;

    let facts = score(&session);
    let under = facts
        .events
        .iter()
        .filter(|event| event.origin.decision == Some(0))
        .count();
    let outside = facts
        .events
        .iter()
        .filter(|event| event.origin.decision.is_none())
        .count();
    assert!(under > 0 && outside > 0, "{under} under, {outside} outside");
    // The exact repeats are the piece's own, and no decision made them.
    assert_eq!(under + outside, facts.events.len());
    Ok(())
}

/// A new performance is another reading of the same file.
#[test]
fn a_new_performance_is_another_reading() -> Result {
    let dir = tempfile::tempdir()?;
    let mut session = open_work(dir.path())?;
    let source = session.snapshot().source().to_owned();

    session.apply(ProjectCommand::NewPerformance { performance: 4 })?;
    assert_eq!(answered(&session), ["2 passes"]);

    session.apply(ProjectCommand::NewPerformance { performance: 8 })?;
    assert_eq!(answered(&session), ["6 passes"]);

    // And the piece is untouched. A performance is the project's, never the
    // file's (`docs/rules/events/11-realization.md`).
    assert_eq!(session.snapshot().source(), source);
    assert!(!session.snapshot().unsaved());
    Ok(())
}

/// Keeping a decision is what makes it stop moving.
#[test]
fn a_kept_decision_holds_across_a_new_performance() -> Result {
    let dir = tempfile::tempdir()?;
    let mut session = open_work(dir.path())?;
    session.apply(ProjectCommand::NewPerformance { performance: 4 })?;
    let path = score(&session).decisions.remove(0).path;

    session.apply(ProjectCommand::Pin(path.clone()))?;
    assert!(
        score(&session)
            .decisions
            .first()
            .is_some_and(|decision| decision.pinned)
    );

    session.apply(ProjectCommand::NewPerformance { performance: 8 })?;
    assert_eq!(answered(&session), ["2 passes"], "a kept decision is kept");
    assert_eq!(score(&session).performance, Some(8), "the performance still moved");

    // Released, it is drawn again like any other.
    session.apply(ProjectCommand::Unpin(path))?;
    assert_eq!(answered(&session), ["6 passes"]);
    Ok(())
}

/// Reading again, and keeping a decision, are moves in the one history.
#[test]
fn a_reading_is_undone_like_anything_else() -> Result {
    let dir = tempfile::tempdir()?;
    let mut session = open_work(dir.path())?;
    session.apply(ProjectCommand::NewPerformance { performance: 4 })?;
    let path = score(&session).decisions.remove(0).path;

    session.apply(ProjectCommand::Pin(path))?;
    session.apply(ProjectCommand::NewPerformance { performance: 8 })?;
    assert_eq!(score(&session).performance, Some(8));

    session.undo()?;
    assert_eq!(score(&session).performance, Some(4));
    session.undo()?;
    assert!(
        score(&session)
            .decisions
            .first()
            .is_some_and(|decision| !decision.pinned)
    );
    Ok(())
}

/// The reading survives the session, beside the piece rather than inside it.
#[test]
fn the_reading_is_still_there_tomorrow() -> Result {
    let dir = tempfile::tempdir()?;
    let path = {
        let mut session = open_work(dir.path())?;
        session.apply(ProjectCommand::NewPerformance { performance: 4 })?;
        let decision = score(&session).decisions.remove(0).path;
        session.apply(ProjectCommand::Pin(decision))?;
        dir.path().join("loop-lengths.musa")
    };

    // Beside the file, findable, and named for what it holds.
    let beside = dir.path().join("loop-lengths.musa.performance");
    assert!(beside.exists(), "the reading is kept beside the piece");
    // And not in the piece: two composers holding the same file must be able
    // to disagree about a performance, so the file is byte-for-byte the one
    // that was opened.
    assert_eq!(
        std::fs::read_to_string(&path)?,
        std::fs::read_to_string(example("loop-lengths.musa"))?
    );

    let reopened = ProjectSession::open(&path)?;
    let facts = score(&reopened);
    assert_eq!(facts.performance, Some(4));
    assert_eq!(answered(&reopened), ["2 passes"]);
    assert!(facts.decisions.first().is_some_and(|decision| decision.pinned));
    Ok(())
}

/// A piece read back to its first reading leaves nothing behind.
#[test]
fn a_piece_with_nothing_to_say_writes_no_file() -> Result {
    let dir = tempfile::tempdir()?;
    let mut session = open_work(dir.path())?;
    let beside = dir.path().join("loop-lengths.musa.performance");

    session.apply(ProjectCommand::NewPerformance { performance: 4 })?;
    assert!(beside.exists());

    session.apply(ProjectCommand::NewPerformance { performance: 0 })?;
    assert!(
        !beside.exists(),
        "seed zero with nothing kept is the absence of a reading"
    );
    Ok(())
}

/// An export of an open work says that it is one reading of it.
///
/// The file is a score, and a score of an open work that did not say so would
/// be read as the piece — which is the one thing it is not.
#[test]
fn an_exported_open_work_says_it_is_a_reading() -> Result {
    let dir = tempfile::tempdir()?;
    let mut session = open_work(dir.path())?;
    session.apply(ProjectCommand::NewPerformance { performance: 4 })?;

    for request in [ExportRequest::Mei, ExportRequest::MusicXml] {
        let artifact = session.export(request)?;
        let text = artifact.as_text().unwrap_or_default();
        assert!(text.contains("performance 4"), "{request:?} does not name the reading");
        assert!(text.contains("open work"), "{request:?} does not say what it is");
    }
    Ok(())
}
