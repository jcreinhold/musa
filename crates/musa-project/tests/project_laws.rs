//! Contracts about the volume: what a project holds, which piece is in hand,
//! and what happens to the ones that are not (prompt 84).
//!
//! `ProjectSession` is one piece, open; `Project` is the set of them. The laws
//! below are about the set — its running order, its listing, and the promise
//! that turning to another piece and back is *turning back* rather than
//! reopening.

use std::path::{Path, PathBuf};

use musa_project::{DocumentKind, Project, ProjectCommand, ProjectError};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

/// The album fixture, from wherever the test binary is run.
fn album() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/album")
}

/// A piece, its title, and nothing else — enough to be listed.
fn piece(title: &str) -> String {
    format!(
        "piece \"{title}\" {{\n    score {{\n        part p {{\n            voice v {{\n                c4/4\n            }}\n        }}\n    }}\n}}\n"
    )
}

/// Roadmap §16's first sentence: the simplest project is one file. So a loose
/// piece is not a special case with the project machinery switched off — it is
/// a project of one, and the interface shows it nothing because there is
/// nothing to choose between.
#[test]
fn a_loose_piece_is_a_project_of_one() -> Result {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("etude.musa");
    std::fs::write(&path, piece("Etude"))?;
    // A neighbour with no manifest tying them together. Opening one file must
    // not sweep in every unrelated file that happens to sit beside it.
    std::fs::write(dir.path().join("sketch.musa"), piece("Sketch"))?;

    let mut project = Project::open(&path)?;

    let snapshot = project.snapshot();
    let contents = snapshot.contents().ok_or("a project always states its contents")?;
    assert_eq!(contents.pieces.len(), 1);
    assert!(contents.material.is_empty());
    assert!(!contents.is_a_volume(), "one file is not a volume");
    Ok(())
}

/// Opening a piece filed under a `musa.toml` opens the volume it belongs to,
/// with that piece in hand — not the first piece, and not a project of one.
#[test]
fn opening_a_filed_piece_opens_the_project_around_it() -> Result {
    let mut project = Project::open(album().join("pieces/02-waltz.musa"))?;

    assert_eq!(project.showing(), "pieces/02-waltz.musa");
    let snapshot = project.snapshot();
    let contents = snapshot.contents().ok_or("a project always states its contents")?;
    assert_eq!(contents.name, "Album");
    assert_eq!(contents.composer.as_deref(), Some("musa"));
    assert!(contents.is_a_volume());
    let current: Vec<&str> = contents
        .pieces
        .iter()
        .filter(|entry| entry.current)
        .map(|entry| entry.file.as_str())
        .collect();
    assert_eq!(current, ["pieces/02-waltz.musa"], "exactly one piece is in hand");
    Ok(())
}

/// The listing prints what the composer called each piece, not what the
/// filesystem calls it — `03-interaction.md` §7: every displayed string is
/// decided in the core.
#[test]
fn the_contents_names_each_piece_as_it_names_itself() -> Result {
    let mut project = Project::open(album())?;

    let snapshot = project.snapshot();
    let contents = snapshot.contents().ok_or("a project always states its contents")?;
    let titles: Vec<&str> = contents.pieces.iter().map(|entry| entry.title.as_str()).collect();
    assert_eq!(titles, ["Opening", "Waltz"]);
    // Material has no title of its own to state, so it is listed under its
    // file name — which is what a composer would call it anyway.
    let material: Vec<&str> = contents.material.iter().map(|entry| entry.title.as_str()).collect();
    assert_eq!(material, ["motifs.musa", "patches.musa"]);
    Ok(())
}

/// The manifest sets an order, not a membership. What it names comes first, in
/// the order it names; what it does not name still appears, after — a contents
/// page that hides a file is worse than one that admits a gap.
#[test]
fn the_running_order_is_the_manifests_then_the_rest() -> Result {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir(dir.path().join("pieces"))?;
    std::fs::write(
        dir.path().join("musa.toml"),
        "[project]\nname = \"Set\"\npieces = [\"pieces/last.musa\", \"pieces/first.musa\", \"pieces/absent.musa\"]\n",
    )?;
    for name in ["first", "last", "unlisted"] {
        std::fs::write(dir.path().join("pieces").join(format!("{name}.musa")), piece(name))?;
    }

    let mut project = Project::open(dir.path())?;

    let snapshot = project.snapshot();
    let contents = snapshot.contents().ok_or("a project always states its contents")?;
    let order: Vec<&str> = contents.pieces.iter().map(|entry| entry.file.as_str()).collect();
    assert_eq!(
        order,
        ["pieces/last.musa", "pieces/first.musa", "pieces/unlisted.musa"],
        "named first, in the order named; unnamed after, in filename order"
    );
    Ok(())
}

/// You do not open a volume in order to look at its cover: a folder with no
/// piece in it is refused when it is opened, rather than opened into an empty
/// screen there is no way out of.
#[test]
fn a_folder_with_no_piece_is_refused() -> Result {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("musa.toml"), "[project]\nname = \"Empty\"\n")?;

    let opened = Project::open(dir.path());

    assert!(
        matches!(opened, Err(ProjectError::NoPieces { .. })),
        "a folder with nothing in it is not a project to open"
    );
    Ok(())
}

/// Turning away from a piece and back is turning back: its text, its unsaved
/// mark, and its undo history are where they were left. This is the whole
/// reason `Project` holds sessions rather than paths.
#[test]
fn a_piece_keeps_its_edits_and_its_history_across_a_switch() -> Result {
    let dir = tempfile::tempdir()?;
    for name in ["one", "two"] {
        std::fs::write(dir.path().join(format!("{name}.musa")), piece(name))?;
    }
    let mut project = Project::open(dir.path())?;
    assert_eq!(project.showing(), "one.musa");

    let edited = project.current().snapshot().source().replace("c4", "d4");
    project.current_mut().apply(ProjectCommand::SetSource(edited.clone()))?;

    project.show("two.musa")?;
    assert_eq!(project.current().snapshot().source(), piece("two"));
    project.show("one.musa")?;

    assert_eq!(project.current().snapshot().source(), edited, "the edit is still there");
    assert!(project.current().snapshot().unsaved(), "and still unsaved");
    project.current_mut().undo()?;
    assert_eq!(
        project.current().snapshot().source(),
        piece("one"),
        "and the history it was made in is still behind it"
    );
    Ok(())
}

/// Only the piece in hand may sound. A piece turned away from gives up the
/// audio device, which is what makes holding several of them affordable — and
/// it is stopped, not left reporting a position it is no longer playing from.
#[test]
fn the_piece_turned_away_from_is_not_playing() -> Result {
    let dir = tempfile::tempdir()?;
    for name in ["one", "two"] {
        std::fs::write(dir.path().join(format!("{name}.musa")), piece(name))?;
    }
    let mut project = Project::open(dir.path())?;

    project.show("two.musa")?;
    project.show("one.musa")?;

    assert!(!project.current().snapshot().playback().playing);
    Ok(())
}

/// A library is a legitimate musa file (roadmap §16), so opening one is not an
/// error and not a blank screen. It has no score and never will, which is a
/// different fact from "no score yet" — and `kind` is where the difference is
/// stated.
#[test]
fn material_opens_with_no_score_and_no_diagnostic() -> Result {
    let mut project = Project::open(album().join("library/motifs.musa"))?;

    let snapshot = project.snapshot();
    assert_eq!(snapshot.kind(), DocumentKind::Material);
    assert!(snapshot.score().is_none(), "a library does not sound");
    assert!(snapshot.compiles(), "and it is well-formed as what it is");
    assert_eq!(snapshot.diagnostics(), &[]);
    Ok(())
}

/// `in use` is a fact the core already has and a file browser cannot produce:
/// which material the piece in hand actually imports. It follows the piece, so
/// turning to another one restates it.
#[test]
fn material_is_marked_in_use_by_the_piece_in_hand() -> Result {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir(dir.path().join("pieces"))?;
    std::fs::create_dir(dir.path().join("library"))?;
    std::fs::write(dir.path().join("musa.toml"), "[project]\nname = \"Set\"\n")?;
    std::fs::write(
        dir.path().join("library/used.musa"),
        "library {\n    motif rise() {\n        c4/4\n    }\n}\n",
    )?;
    std::fs::write(dir.path().join("library/spare.musa"), "library {\n}\n")?;
    std::fs::write(
        dir.path().join("pieces/draws.musa"),
        "piece \"Draws\" {\n    import \"../library/used.musa\";\n    score {\n        part p {\n            voice v {\n                use rise();\n            }\n        }\n    }\n}\n",
    )?;
    std::fs::write(dir.path().join("pieces/alone.musa"), piece("Alone"))?;

    let mut project = Project::open(dir.path().join("pieces/draws.musa"))?;
    let used: Vec<String> = {
        let snapshot = project.snapshot();
        let contents = snapshot.contents().ok_or("a project always states its contents")?;
        contents
            .material
            .iter()
            .filter(|entry| entry.used)
            .map(|entry| entry.file.clone())
            .collect()
    };
    assert_eq!(used, ["library/used.musa"], "exactly what this piece imports");

    project.show("pieces/alone.musa")?;
    let snapshot = project.snapshot();
    let contents = snapshot.contents().ok_or("a project always states its contents")?;
    assert!(
        contents.material.iter().all(|entry| !entry.used),
        "a piece that imports nothing marks nothing"
    );
    Ok(())
}

/// The listing is live while someone is typing: `edited` has to be true the
/// moment the text differs from disk, without anyone having to read a
/// directory to find out.
#[test]
fn an_edited_piece_reads_as_edited_without_a_rescan() -> Result {
    let dir = tempfile::tempdir()?;
    for name in ["one", "two"] {
        std::fs::write(dir.path().join(format!("{name}.musa")), piece(name))?;
    }
    let mut project = Project::open(dir.path())?;

    let edited = project.current().snapshot().source().replace("c4", "d4");
    project.current_mut().apply(ProjectCommand::SetSource(edited))?;

    let unsaved: Vec<String> = {
        let snapshot = project.snapshot();
        let contents = snapshot.contents().ok_or("a project always states its contents")?;
        contents
            .pieces
            .iter()
            .filter(|entry| entry.unsaved)
            .map(|entry| entry.file.clone())
            .collect()
    };
    assert_eq!(unsaved, ["one.musa"]);

    project.save_all()?;
    let snapshot = project.snapshot();
    let contents = snapshot.contents().ok_or("a project always states its contents")?;
    assert!(
        contents.pieces.iter().all(|entry| !entry.unsaved),
        "saving every piece leaves none edited"
    );
    Ok(())
}
