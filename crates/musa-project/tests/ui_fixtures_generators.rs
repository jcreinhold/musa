//! Generates the interface's fixtures from the real types.
//!
//! The desktop prototype (prompt 20) is driven by committed fixtures rather
//! than hand-written mocks, so that the shape the UI codes against is the
//! shape the facade actually produces. This test writes them; it fails when
//! the committed copy is stale, so a change to `ProjectSnapshot` shows up as
//! a red test rather than as a UI that quietly renders the wrong thing.
//!
//! Run `UPDATE_UI_FIXTURES=1 cargo test -p musa-project` to refresh.

use std::path::{Path, PathBuf};

use musa_project::{DocumentId, ExportRequest, ProjectSession};
use serde_json::Value;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

/// A wire snapshot with no session behind it.
///
/// Document ids are minted per session and per process, so writing whichever
/// one this test happened to get would make the fixture depend on how many
/// sessions ran before it. [`DocumentId::NONE`] is the honest value for a
/// file on disk, and it is the value the UI's own fixtures then compare.
fn anonymous(mut wire: Value) -> Value {
    if let Some(document) = wire.get_mut("document") {
        *document = Value::from(DocumentId::NONE.0);
    }
    wire
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/musa-desktop/ui/fixtures")
}

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(name)
}

/// Write `contents` to `path`, or — when the fixture is committed and stale —
/// report exactly which one and how to refresh it.
fn write_or_compare(path: &Path, contents: &str) -> Result {
    let current = std::fs::read_to_string(path).ok();
    if current.as_deref() == Some(contents) {
        return Ok(());
    }
    if current.is_some() && std::env::var_os("UPDATE_UI_FIXTURES").is_none() {
        return Err(format!(
            "{} is stale — rerun with UPDATE_UI_FIXTURES=1 and commit the result",
            path.display()
        )
        .into());
    }
    std::fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))?;
    std::fs::write(path, contents)?;
    Ok(())
}

/// The annotated piece, which is what the outline pane is built against: the
/// demo score has no sections and no phrases, and a navigation pane with
/// nothing to navigate proves nothing.
#[test]
fn annotated_fixture_is_current() -> Result {
    let source = std::fs::read_to_string(example("annotated.musa"))?;
    let session = ProjectSession::from_text(source, "annotated.musa");
    assert!(session.snapshot().compiles(), "the fixture piece must compile");

    let mut json = serde_json::to_string_pretty(&anonymous(session.snapshot().to_wire()))?;
    json.push('\n');
    write_or_compare(&fixtures_dir().join("annotated.snapshot.json"), &json)
}

/// The snapshot the Compose screen is built against.
#[test]
fn snapshot_fixture_is_current() -> Result {
    // From text, not from the path, so the committed fixture does not carry
    // whichever machine generated it.
    let source = std::fs::read_to_string(example("glass-mountain.musa"))?;
    let session = ProjectSession::from_text(source, "glass-mountain.musa");
    assert!(session.snapshot().compiles(), "the fixture piece must compile");

    let mut json = serde_json::to_string_pretty(&anonymous(session.snapshot().to_wire()))?;
    json.push('\n');
    write_or_compare(&fixtures_dir().join("glass-mountain.snapshot.json"), &json)
}

/// The piece whose text is not ASCII.
///
/// Every other fixture here is, and while that is true a wire snapshot that
/// forgot to restate its spans in UTF-16 code units would look exactly like
/// one that remembered (`crate`-level: `musa_project::utf16`). This one makes
/// the difference visible: the UI's linking and highlighting tests run
/// against it, and they fail if the spans arrive in bytes.
#[test]
fn unicode_fixture_is_current() -> Result {
    let source = std::fs::read_to_string(example("unicode-fixture.musa"))?;
    assert!(!source.is_ascii(), "the point of this fixture is that it is not ASCII");
    let session = ProjectSession::from_text(source, "unicode-fixture.musa");
    assert!(session.snapshot().compiles(), "the fixture piece must compile");

    let mut json = serde_json::to_string_pretty(&anonymous(session.snapshot().to_wire()))?;
    json.push('\n');
    write_or_compare(&fixtures_dir().join("unicode-fixture.snapshot.json"), &json)
}

/// One MEI per engraving fixture (`docs/interface/02-engraving.md` §9).
#[test]
fn mei_fixtures_are_current() -> Result {
    for name in ["glass-mountain", "counterpoint", "twinkle", "annotated"] {
        let session = ProjectSession::open(example(&format!("{name}.musa")))?;
        let artifact = session.export(ExportRequest::Mei)?;
        let mei = artifact.as_text().unwrap_or_default();
        write_or_compare(&fixtures_dir().join(format!("{name}.mei")), mei)?;
    }
    Ok(())
}
