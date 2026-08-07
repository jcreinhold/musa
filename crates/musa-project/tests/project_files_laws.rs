//! Contracts about the file behind a session.

use musa_project::{ProjectCommand, ProjectSession, Template};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

/// A created project is a piece that already compiles, and it is on disk.
#[test]
fn create_writes_a_project_that_compiles() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("etude.musa");

    let session = ProjectSession::create(&path, Template::Piece)?;

    let snapshot = session.snapshot();
    assert!(snapshot.compiles(), "{:?}", snapshot.diagnostics());
    assert!(!snapshot.unsaved());
    assert_eq!(std::fs::read_to_string(&path)?, snapshot.source());
    // The title comes from the file name, not a placeholder.
    assert!(snapshot.source().contains("\"etude\""), "{}", snapshot.source());
    Ok(())
}

/// Editing marks the project unsaved; saving writes exactly what is in the
/// session and clears the mark.
#[test]
fn save_writes_the_current_source_and_clears_the_unsaved_mark() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("etude.musa");
    let mut session = ProjectSession::create(&path, Template::Piece)?;

    let edited = session.snapshot().source().replace("c4 1/4;", "d4 1/4;");
    session.apply(ProjectCommand::SetSource(edited.clone()))?;
    assert!(session.snapshot().unsaved());

    session.apply(ProjectCommand::Save)?;
    assert!(!session.snapshot().unsaved());
    assert_eq!(std::fs::read_to_string(&path)?, edited);
    Ok(())
}

/// Undoing back to the saved text makes the project saved again — the mark
/// tracks the text, not the number of commands.
#[test]
fn undoing_back_to_the_saved_text_clears_the_unsaved_mark() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("etude.musa");
    let mut session = ProjectSession::create(&path, Template::Piece)?;

    let edited = session.snapshot().source().replace("c4 1/4;", "d4 1/4;");
    session.apply(ProjectCommand::SetSource(edited))?;
    session.undo()?;

    assert!(!session.snapshot().unsaved());
    Ok(())
}

/// A file that does not compile still opens, with its diagnostics.
#[test]
fn open_reports_diagnostics_instead_of_failing() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("broken.musa");
    std::fs::write(&path, "piece \"broken\" { score { part")?;

    let session = ProjectSession::open(&path)?;

    let snapshot = session.snapshot();
    assert!(!snapshot.compiles());
    assert!(!snapshot.diagnostics().is_empty());
    assert!(snapshot.mei().is_none());
    Ok(())
}

/// Opening a file that is not there is an error naming the path.
#[test]
fn open_missing_file_fails_with_the_path() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("absent.musa");

    let message = ProjectSession::open(&path).err().map(|error| error.to_string());

    assert!(
        message
            .as_deref()
            .is_some_and(|message| message.contains("absent.musa")),
        "{message:?}"
    );
    Ok(())
}

/// The autosave policy, stated as one law: while the source differs from the
/// file, a recovery copy of it sits beside the file; saving removes it.
#[test]
fn unsaved_work_leaves_a_recovery_copy_beside_the_file() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("etude.musa");
    let recovery = dir.path().join("etude.musa.recovery");
    let mut session = ProjectSession::create(&path, Template::Piece)?;
    assert!(!recovery.exists(), "a saved session leaves nothing behind");

    let edited = session.snapshot().source().replace("c4 1/4;", "d4 1/4;");
    session.apply(ProjectCommand::SetSource(edited.clone()))?;

    assert!(session.snapshot().autosaved());
    assert_eq!(std::fs::read_to_string(&recovery)?, edited);

    session.apply(ProjectCommand::Save)?;
    assert!(!recovery.exists(), "the copy is gone once the file has the work");
    Ok(())
}

/// What the recovery copy is for: a session that never got to save is offered
/// its work back on the next open, and taking it makes it the source.
#[test]
fn work_lost_to_a_crash_comes_back_on_the_next_open() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("etude.musa");
    let mut session = ProjectSession::create(&path, Template::Piece)?;
    let edited = session.snapshot().source().replace("c4 1/4;", "d4 1/4;");
    session.apply(ProjectCommand::SetSource(edited.clone()))?;
    drop(session); // the crash: no save, no clean close

    let mut session = ProjectSession::open(&path)?;
    assert_eq!(session.snapshot().recovery(), Some(edited.as_str()));
    // Until it is taken, the session holds what is actually on disk.
    assert_ne!(session.snapshot().source(), edited);

    session.apply(ProjectCommand::RestoreRecovery)?;
    let snapshot = session.snapshot();
    assert_eq!(snapshot.source(), edited);
    assert_eq!(snapshot.recovery(), None);
    assert!(snapshot.compiles(), "{:?}", snapshot.diagnostics());
    Ok(())
}

/// Recovery is an offer, not an imposition: declining it keeps the file's own
/// text and removes the copy, so the next open is quiet.
#[test]
fn declining_recovery_keeps_the_file_and_forgets_the_copy() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("etude.musa");
    let mut session = ProjectSession::create(&path, Template::Piece)?;
    let on_disk = session.snapshot().source().to_owned();
    let edited = on_disk.replace("c4 1/4;", "d4 1/4;");
    session.apply(ProjectCommand::SetSource(edited))?;
    drop(session);

    let mut session = ProjectSession::open(&path)?;
    session.apply(ProjectCommand::DiscardRecovery)?;
    assert_eq!(session.snapshot().recovery(), None);
    assert_eq!(session.snapshot().source(), on_disk);

    let reopened = ProjectSession::open(&path)?;
    assert_eq!(reopened.snapshot().recovery(), None);
    Ok(())
}

/// A copy that matches the file is not a recovery — it is a save that landed
/// and a copy that outlived it, and offering it would be a false alarm.
#[test]
fn a_recovery_copy_that_matches_the_file_is_not_offered() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("etude.musa");
    let session = ProjectSession::create(&path, Template::Piece)?;
    let source = session.snapshot().source().to_owned();
    drop(session);
    std::fs::write(dir.path().join("etude.musa.recovery"), &source)?;

    let session = ProjectSession::open(&path)?;

    assert_eq!(session.snapshot().recovery(), None);
    assert!(!dir.path().join("etude.musa.recovery").exists());
    Ok(())
}

/// A session with no file behind it has nowhere to write a recovery copy, and
/// must not invent one — the desktop's scratch buffer is this case.
#[test]
fn a_piece_with_no_file_autosaves_nothing() {
    let mut session = ProjectSession::new_piece(Template::Piece, "Untitled");
    let edited = session.snapshot().source().replace("c4 1/4;", "d4 1/4;");
    assert!(session.apply(ProjectCommand::SetSource(edited)).is_ok());
    assert!(!session.snapshot().autosaved());
}
