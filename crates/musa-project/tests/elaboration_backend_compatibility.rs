//! Backend bytes frozen before the elaboration-language migration (prompt 93).

#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use musa_project::{ExportRequest, MidiMode, ProjectSession};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const UPDATE: &str = "UPDATE_ELABORATION_BASELINE";
const SOURCE: &str = include_str!("../../../tests/fixtures/audio-bridge.musa");

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn manifest() -> Result<String> {
    let session = ProjectSession::from_text(SOURCE, "tests/fixtures/audio-bridge.musa");
    assert!(session.snapshot().compiles(), "the backend fixture must compile");
    let mut out = String::from(
        "# musa audio-bridge backend compatibility manifest v1\n\
         # Test oracle only; these are complete export bytes, not normalized summaries.\n",
    );
    for (name, request) in [
        ("mei", ExportRequest::Mei),
        ("lilypond", ExportRequest::LilyPond),
        ("musicxml", ExportRequest::MusicXml),
        ("midi-score", ExportRequest::Midi(MidiMode::Score)),
        ("midi-performance", ExportRequest::Midi(MidiMode::Performance)),
        ("wav", ExportRequest::Wav),
    ] {
        let artifact = session.export(request)?;
        let _ = writeln!(
            out,
            "{name}=bytes:{}:digest:{:016x}:warnings:{:?}",
            artifact.as_bytes().len(),
            digest(artifact.as_bytes()),
            artifact.warnings()
        );
    }
    Ok(out)
}

#[test]
fn every_existing_backend_matches_the_migration_oracle() -> Result {
    let actual = manifest()?;
    let path = repository().join("tests/fixtures/audio-bridge-backends.compat");
    let expected = std::fs::read_to_string(&path).ok();
    if expected.as_deref() == Some(actual.as_str()) {
        return Ok(());
    }
    if std::env::var_os(UPDATE).is_some() {
        std::fs::write(path, actual)?;
        return Ok(());
    }
    Err(format!(
        "{} differs — rerun in a clean worktree with {UPDATE}=1 and review each backend byte digest",
        path.display()
    )
    .into())
}
