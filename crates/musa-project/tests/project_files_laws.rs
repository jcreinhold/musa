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

    let edited = session.snapshot().source().replace("c4/4", "d4/4");
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

    let edited = session.snapshot().source().replace("c4/4", "d4/4");
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

    let edited = session.snapshot().source().replace("c4/4", "d4/4");
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
    let edited = session.snapshot().source().replace("c4/4", "d4/4");
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
    let edited = on_disk.replace("c4/4", "d4/4");
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
    let edited = session.snapshot().source().replace("c4/4", "d4/4");
    assert!(session.apply(ProjectCommand::SetSource(edited)).is_ok());
    assert!(!session.snapshot().autosaved());
}

// --- Directory projects (docs/prompts/36; roadmap §16) --------------------

/// The album fixture, as a session opened from its real path.
fn album_piece() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/album/pieces/01-opening.musa")
}

/// A piece inside a directory project opens, compiles, and knows what it read
/// — including the library its library imported, which nobody wrote a `use`
/// for in the piece itself.
#[test]
fn a_piece_in_a_project_compiles_with_the_libraries_it_imports() -> Result {
    let session = ProjectSession::open(album_piece())?;
    let snapshot = session.snapshot();
    assert!(snapshot.compiles(), "{:?}", snapshot.diagnostics());

    let names: Vec<String> = session
        .imports()
        .iter()
        .filter_map(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        .collect();
    assert_eq!(names, ["motifs.musa", "patches.musa"]);
    Ok(())
}

/// `musa.toml` is metadata and nothing else: the session finds it by walking
/// up from the piece, and the piece compiles the same with or without it.
#[test]
fn a_project_file_is_found_from_the_piece_and_carries_only_metadata() -> Result {
    let session = ProjectSession::open(album_piece())?;
    let project = session.project().ok_or("expected a project")?;
    assert_eq!(project.name.as_deref(), Some("Album"));
    assert!(project.root.ends_with("album"));

    // The same source, compiled with no project and no files around it, is
    // the piece that no longer resolves its imports — the project file is not
    // what made it work.
    let source = std::fs::read_to_string(album_piece())?;
    let orphan = ProjectSession::from_text(source, "01-opening.musa");
    assert!(orphan.project().is_none());
    assert!(
        !orphan.snapshot().compiles(),
        "an import that resolves nowhere is an error"
    );
    Ok(())
}

/// A library edited on disk reaches the piece the next time it compiles: the
/// session re-reads its import closure on every compile, which is what
/// "recompile on save of the library" means with one document open.
#[test]
fn a_library_edited_on_disk_reaches_the_piece_that_imports_it() -> Result {
    let dir = tempfile::tempdir()?;
    let library = dir.path().join("lib.musa");
    let piece = dir.path().join("piece.musa");
    std::fs::write(&library, "library { motif tune() { c5/4 } }")?;
    std::fs::write(
        &piece,
        "piece \"P\" { import \"lib.musa\"; tempo 1/4 = 60; meter 4/4; key c major;
         score { part p { voice v { use tune(); } } } }",
    )?;

    let mut session = ProjectSession::open(&piece)?;
    assert!(session.snapshot().compiles());

    // Rename the motif out from under the piece; the next compile says so.
    std::fs::write(&library, "library { motif other() { c5/4 } }")?;
    session.apply(ProjectCommand::SetSource(
        std::fs::read_to_string(&piece)?.replace("key c major", "key c major;"),
    ))?;
    assert!(!session.snapshot().compiles(), "the piece uses a motif that is gone");
    Ok(())
}

/// The project's composer is a fallback, not a competitor: a piece that names
/// nobody inherits it, and a piece that names somebody keeps its own answer.
///
/// The resolution happens here, once, where the snapshot is built — so every
/// backend and the GUI read one name rather than each deciding for itself.
#[test]
fn the_project_composer_fills_in_for_a_piece_that_names_none() -> Result {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("musa.toml"), "[project]\ncomposer = \"Ada\"\n")?;

    let silent = dir.path().join("silent.musa");
    std::fs::write(&silent, PIECE)?;
    let mei = ProjectSession::open(&silent)?
        .snapshot()
        .mei()
        .unwrap_or_default()
        .to_owned();
    assert!(mei.contains("<composer>Ada</composer>"), "{mei}");

    let named = dir.path().join("named.musa");
    std::fs::write(&named, PIECE.replacen("{\n", "{\n    composer \"Grace\";\n", 1))?;
    let mei = ProjectSession::open(&named)?
        .snapshot()
        .mei()
        .unwrap_or_default()
        .to_owned();
    assert!(mei.contains("<composer>Grace</composer>"), "{mei}");
    assert!(!mei.contains("Ada"), "{mei}");
    Ok(())
}

/// With no project and no `composer` statement, nothing is invented: the head
/// carries a title and stops. An empty composer line is worse than none.
#[test]
fn a_piece_with_no_composer_anywhere_prints_no_composer() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("alone.musa");
    std::fs::write(&path, PIECE)?;

    let mei = ProjectSession::open(&path)?
        .snapshot()
        .mei()
        .unwrap_or_default()
        .to_owned();
    assert!(mei.contains("<title>Alone</title>"), "{mei}");
    assert!(!mei.contains("<composer>"), "{mei}");
    Ok(())
}

/// The smallest piece that engraves, for the front-matter tests above.
const PIECE: &str = "piece \"Alone\" {
    meter 4/4;
    key c major;

    score {
        part violin {
            clef treble;

            voice upper {
                c4/4
            }
        }
    }
}
";

// --- The one thing a project may say about layout (prompt 91) -------------

/// A bar in a project that asks for `proportional` is drawn to scale, and the
/// same bar in a project that says nothing is not.
///
/// End to end on purpose. The setting is only worth having if it survives the
/// whole path — manifest, session, formatter — and every intermediate hop
/// already has its own test, so the one worth adding is the one that would
/// catch a hop that was never wired.
#[test]
fn a_project_can_ask_for_its_bars_drawn_to_scale() -> Result {
    let laid_out = |manifest: Option<&str>| -> std::result::Result<String, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        if let Some(manifest) = manifest {
            std::fs::write(dir.path().join("musa.toml"), manifest)?;
        }
        let path = dir.path().join("bars.musa");
        std::fs::write(&path, BARS)?;
        Ok(ProjectSession::open(&path)?.formatted_source())
    };

    let to_scale = laid_out(Some(
        "[project]\nname = \"Album\"\n\n[format]\nbars = \"proportional\"\n",
    ))?;
    assert!(
        to_scale.contains("| c4/2                            d4/4            e4/4\n"),
        "the manifest asked for bars drawn to scale:\n{to_scale}"
    );

    // The same piece, with nothing said and with nothing above it at all.
    let compact = "| c4/2 d4/4 e4/4\n";
    assert!(laid_out(Some("[project]\nname = \"Album\"\n"))?.contains(compact));
    assert!(laid_out(None)?.contains(compact));
    Ok(())
}

/// A value this version does not know costs the project the setting and
/// nothing else.
///
/// The whole error design in one assertion: a typo in the newest and least
/// important key must not destroy the oldest and most important ones. If the
/// `[format]` section were typed as `BarSpacing`, `bars = "nonsense"` would
/// fail the manifest's parse and the project would lose its name, its composer
/// and its running order — and then `musa format` would rewrite every file to
/// the default it fell back to.
#[test]
fn an_unknown_layout_costs_the_setting_and_not_the_project() -> Result {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("musa.toml"),
        "[project]\nname = \"Album\"\ncomposer = \"Ada\"\n\n[format]\nbars = \"nonsense\"\n",
    )?;
    let path = dir.path().join("bars.musa");
    std::fs::write(&path, BARS)?;

    let session = ProjectSession::open(&path)?;
    let project = session.project().ok_or("expected a project")?;
    assert_eq!(project.bar_spacing, musa_project::BarSpacing::Compact);
    assert_eq!(project.name.as_deref(), Some("Album"));
    assert_eq!(project.composer.as_deref(), Some("Ada"));
    assert!(session.formatted_source().contains("| c4/2 d4/4 e4/4\n"));
    Ok(())
}

/// A buffer named by its path is filed under the project above it.
///
/// This is how the editor and the command line come to agree. The language
/// server holds its documents as text and names them by path so that `use`
/// resolves; naming them that way is also what lets them find the manifest, so
/// a format request in the editor is laid out the way `musa format` lays the
/// same file out. A name that is not a path is a scratch buffer, and belongs
/// to no project no matter what directory the process happens to be in.
#[test]
fn a_buffer_named_by_its_path_is_filed_under_its_project() -> Result {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("musa.toml"),
        "[project]\nname = \"Album\"\n\n[format]\nbars = \"proportional\"\n",
    )?;
    let path = dir.path().join("bars.musa");

    let held = ProjectSession::from_text(BARS, path.to_string_lossy().into_owned());
    assert_eq!(
        held.project().map(|project| project.bar_spacing),
        Some(musa_project::BarSpacing::Proportional)
    );
    assert!(
        held.formatted_source()
            .contains("| c4/2                            d4/4            e4/4\n")
    );

    let scratch = ProjectSession::from_text(BARS, "bars.musa");
    assert!(scratch.project().is_none(), "a name is not a path");
    Ok(())
}

/// A piece with one bar in it, for the three laws above.
const BARS: &str = "piece \"Bars\" {
    meter 4/4;
    key c major;

    score {
        part violin {
            clef treble;

            voice upper {
                | c4/2 d4/4 e4/4
            }
        }
    }
}
";
