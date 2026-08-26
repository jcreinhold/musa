//! Immutable asset closure laws (language candidate §09, prompt 182).

#![allow(clippy::expect_used)]

use std::path::Path;

use musa_project::{AssetStatus, ProjectError, ProjectSession, asset_inventory, lock_assets};

fn write(path: &Path, contents: impl AsRef<[u8]>) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture directory");
    }
    std::fs::write(path, contents).expect("fixture file");
}

fn project(root: &Path, max_bytes: u64) {
    write(
        &root.join("musa.toml"),
        format!(
            r#"[project]
name = "Assets"

[assets."assets/tone.sfz"]
kind = "sfz"
adapter = "sfz@1"
max_bytes = {max_bytes}
license = "CC0-1.0"
source = "Recorded for the fixture"
"#,
        ),
    );
    write(
        &root.join("piece.musa"),
        r#"piece "Asset" {
    meter 4/4;
    key c major;
    instrument tone from "assets/tone.sfz" conforms note_instrument;
    score { part lead { voice one { c4/1 } } }
}
"#,
    );
}

#[test]
fn a_locked_asset_exposes_metadata_and_never_its_bytes() {
    let directory = tempfile::tempdir().expect("temporary project");
    project(directory.path(), 1024);
    write(&directory.path().join("assets/tone.sfz"), b"secret-fixture-bytes");

    let inventory = lock_assets(directory.path()).expect("lock succeeds");
    let asset = inventory.facts().first().expect("one manifest asset");
    assert_eq!(asset.status, AssetStatus::Verified);
    assert_eq!(asset.license.as_deref(), Some("CC0-1.0"));
    assert_eq!(asset.bytes, Some(20));
    assert!(
        asset
            .digest
            .as_deref()
            .is_some_and(|digest| digest.starts_with("sha256:"))
    );

    let session = ProjectSession::open(directory.path().join("piece.musa")).expect("piece opens");
    let snapshot = session.snapshot();
    let opened = snapshot.assets().first().expect("one opened asset");
    assert_eq!(opened.path, asset.path);
    assert_eq!(opened.digest, asset.digest);
    assert_eq!(opened.status, AssetStatus::Verified);
    assert!(
        opened.span.is_some(),
        "the open source contributes its exact token span"
    );
    let wire = session.snapshot().to_wire().to_string();
    assert!(wire.contains("sha256:"));
    assert!(!wire.contains("secret-fixture-bytes"));
}

#[test]
fn changing_bytes_invalidates_only_the_locked_asset_identity() {
    let directory = tempfile::tempdir().expect("temporary project");
    project(directory.path(), 1024);
    let asset = directory.path().join("assets/tone.sfz");
    write(&asset, b"first");
    let first = lock_assets(directory.path())
        .expect("first lock")
        .facts()
        .first()
        .expect("one first asset")
        .digest
        .clone()
        .expect("digest");

    write(&asset, b"other");
    let stale = asset_inventory(directory.path()).expect("inventory");
    assert_eq!(
        stale.facts().first().expect("one stale asset").status,
        AssetStatus::DigestMismatch
    );
    let second = lock_assets(directory.path())
        .expect("second lock")
        .facts()
        .first()
        .expect("one second asset")
        .digest
        .clone()
        .expect("digest");
    assert_ne!(first, second, "same path and size cannot mask changed bytes");
}

#[test]
fn relocation_changes_no_lock_bytes() {
    let first = tempfile::tempdir().expect("first project");
    let second = tempfile::tempdir().expect("second project");
    for root in [first.path(), second.path()] {
        project(root, 1024);
        write(&root.join("assets/tone.sfz"), b"one identity");
        lock_assets(root).expect("lock");
    }
    assert_eq!(
        std::fs::read(first.path().join("musa.lock")).expect("first lock"),
        std::fs::read(second.path().join("musa.lock")).expect("second lock")
    );
}

#[test]
fn traversal_kind_and_size_are_rejected_before_locking() {
    let traversal = tempfile::tempdir().expect("traversal project");
    write(
        &traversal.path().join("musa.toml"),
        r#"[assets."../outside.sfz"]
kind = "sfz"
adapter = "sfz@1"
"#,
    );
    write(&traversal.path().join("musa.lock"), b"previous verified closure");
    assert!(matches!(lock_assets(traversal.path()), Err(ProjectError::Assets(message)) if message.contains("escapes")));
    assert_eq!(
        std::fs::read(traversal.path().join("musa.lock")).expect("previous lock"),
        b"previous verified closure",
        "validation failure must not destroy the previous closure"
    );

    let wrong = tempfile::tempdir().expect("kind project");
    project(wrong.path(), 1024);
    write(&wrong.path().join("assets/tone.sfz"), b"four");
    let manifest = std::fs::read_to_string(wrong.path().join("musa.toml"))
        .expect("manifest")
        .replace("kind = \"sfz\"", "kind = \"audio\"");
    write(&wrong.path().join("musa.toml"), manifest);
    assert!(matches!(lock_assets(wrong.path()), Err(ProjectError::Assets(message)) if message.contains("extension")));

    let large = tempfile::tempdir().expect("size project");
    project(large.path(), 3);
    write(&large.path().join("assets/tone.sfz"), b"four");
    assert!(matches!(lock_assets(large.path()), Err(ProjectError::Assets(message)) if message.contains("above")));

    let collision = tempfile::tempdir().expect("case collision project");
    write(
        &collision.path().join("musa.toml"),
        r#"[assets."assets/Tone.sfz"]
kind = "sfz"
adapter = "sfz@1"

[assets."assets/tone.sfz"]
kind = "sfz"
adapter = "sfz@1"
"#,
    );
    assert!(
        matches!(lock_assets(collision.path()), Err(ProjectError::Assets(message)) if message.contains("case-insensitive"))
    );
}

#[test]
fn malformed_asset_policy_invalidates_assets_without_erasing_project_metadata() {
    let directory = tempfile::tempdir().expect("temporary project");
    write(
        &directory.path().join("musa.toml"),
        r#"[project]
name = "Still a Project"

[assets."assets/tone.sfz"]
kind = "sfz"
adapter = "unversioned"
"#,
    );
    write(
        &directory.path().join("piece.musa"),
        r#"piece "Asset" {
    meter 4/4;
    score { part lead { voice one { c4/1 } } }
}
"#,
    );

    let session = ProjectSession::open(directory.path().join("piece.musa")).expect("piece opens");
    assert_eq!(
        session.project().and_then(|project| project.name.as_deref()),
        Some("Still a Project")
    );
    assert!(
        session
            .snapshot()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == "asset-manifest")
    );
}

#[test]
fn an_asset_declared_by_an_imported_instrument_keeps_the_library_origin() {
    let directory = tempfile::tempdir().expect("temporary project");
    project(directory.path(), 1024);
    write(
        &directory.path().join("piece.musa"),
        r#"piece "Asset" {
    import "library/instruments.musa";
    meter 4/4;
    score { part lead { voice one { c4/1 } } }
}
"#,
    );
    write(
        &directory.path().join("library/instruments.musa"),
        r#"instrument tone from "assets/tone.sfz" conforms note_instrument;
"#,
    );
    write(&directory.path().join("assets/tone.sfz"), b"library asset");
    lock_assets(directory.path()).expect("lock succeeds");

    let session = ProjectSession::open(directory.path().join("piece.musa")).expect("piece opens");
    let snapshot = session.snapshot();
    let asset = snapshot.assets().first().expect("imported asset fact");
    assert_eq!(asset.status, AssetStatus::Verified);
    assert!(
        asset
            .origin
            .as_deref()
            .is_some_and(|origin| origin.ends_with("library/instruments.musa"))
    );
    assert_eq!(
        asset.span, None,
        "a span cannot index a document absent from the snapshot"
    );
}

#[test]
fn recorded_media_references_require_audio_assets() {
    let directory = tempfile::tempdir().expect("temporary project");
    write(
        &directory.path().join("musa.toml"),
        r#"[project]
name = "Media assets"

[assets."assets/pulse.wav"]
kind = "audio"
adapter = "wav@1"
max_bytes = 1024

[assets."assets/harbor.flac"]
kind = "audio"
adapter = "flac@1"
max_bytes = 1024
"#,
    );
    write(
        &directory.path().join("piece.musa"),
        r#"clip pulse from "assets/pulse.wav" fit 1/1 by loop;
piece "Media" {
    fixed_media harbor from "assets/harbor.flac";
    meter 4/4;
    score {
        cue pulse at 1:1;
        cue harbor at 1:1;
        part guide { voice one { c4/1 } }
    }
}
"#,
    );
    write(&directory.path().join("assets/pulse.wav"), b"wave fixture");
    write(&directory.path().join("assets/harbor.flac"), b"flac fixture");
    lock_assets(directory.path()).expect("media lock succeeds");

    let session = ProjectSession::open(directory.path().join("piece.musa")).expect("piece opens");
    let snapshot = session.snapshot();
    assert_eq!(snapshot.assets().len(), 2);
    assert!(
        snapshot
            .assets()
            .iter()
            .all(|asset| asset.status == AssetStatus::Verified)
    );
    assert_eq!(snapshot.score().expect("score facts").media.len(), 2);
}

#[test]
fn a_locked_local_recording_reaches_offline_export_deterministically() {
    let directory = tempfile::tempdir().expect("temporary project");
    write(
        &directory.path().join("musa.toml"),
        r#"[project]
name = "Rendered media"

[assets."assets/recording.wav"]
kind = "audio"
adapter = "wav@1"
max_bytes = 65536
"#,
    );
    write(
        &directory.path().join("piece.musa"),
        r#"piece "Rendered media" {
    meter 4/4;
    fixed_media recording from "assets/recording.wav";
    score { cue recording at 1:1; part guide { voice one { rest/4 } } }
    studio { route recording -> master; }
}
"#,
    );
    let mut recording = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(
            &mut recording,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .expect("recording writer");
        for _ in 0..1024 {
            writer.write_sample(0.25_f32).expect("recording sample");
        }
        writer.finalize().expect("recording finish");
    }
    write(&directory.path().join("assets/recording.wav"), recording.into_inner());
    lock_assets(directory.path()).expect("media lock succeeds");

    let session = ProjectSession::open(directory.path().join("piece.musa")).expect("piece opens");
    let snapshot = session.snapshot();
    let studio = snapshot.studio().expect("studio facts");
    assert_eq!(studio.media.len(), 1);
    let media = studio.media.first().expect("recorded-media source");
    assert_eq!(media.name, "recording");
    assert_eq!(media.kind, "fixed-media-cue");
    assert_eq!(media.occurrences, 1);
    let first = session.export(musa_project::ExportRequest::Wav).expect("first WAV");
    let second = session.export(musa_project::ExportRequest::Wav).expect("second WAV");
    assert_eq!(
        first, second,
        "offline media uses the same prepared operation each time"
    );
    let reader = hound::WavReader::new(std::io::Cursor::new(first.as_bytes())).expect("exported WAV");
    let audible = reader
        .into_samples::<f32>()
        .filter_map(Result::ok)
        .any(|sample| sample.abs() > 0.1);
    assert!(audible, "the locked recording reaches the exported master");
}

#[cfg(unix)]
#[test]
fn a_symlink_cannot_escape_the_project_root() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().expect("temporary project");
    let outside = tempfile::NamedTempFile::new().expect("outside file");
    project(directory.path(), 1024);
    std::fs::create_dir_all(directory.path().join("assets")).expect("asset directory");
    symlink(outside.path(), directory.path().join("assets/tone.sfz")).expect("symlink");
    assert!(matches!(lock_assets(directory.path()), Err(ProjectError::Assets(message)) if message.contains("symlink")));
}
