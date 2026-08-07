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
