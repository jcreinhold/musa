//! One directory a workstation can read (`docs/rules/across-stages/06-daw-boundary.md`).
//!
//! What is under test is that the bundle is *one* record: every artifact in
//! it comes from one compile and one render argument record, it says what it
//! could not carry, and it is installed whole or not at all. A bundle that is
//! merely present proves nothing; a bundle nobody can check is worse than no
//! bundle, because it looks like evidence.

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]
// Bundle paths are the project's own, spelled by this crate in lowercase.
#![allow(clippy::case_sensitive_file_extension_comparisons)]
// A digest is built one byte at a time; a fold reads worse than the map.
#![allow(clippy::format_collect)]

use std::path::{Path, PathBuf};

use musa_project::{DawExportOptions, DawExportReport, DawProfile, ProjectSession};
use serde_json::Value;

fn example(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn session(name: &str) -> ProjectSession {
    ProjectSession::from_text(example(name), name)
}

const ROUTED: &str = "glass-mountain.musa";

struct Exported {
    _root: tempfile::TempDir,
    bundle: PathBuf,
    report: DawExportReport,
}

impl Exported {
    fn read(&self, path: &str) -> Vec<u8> {
        std::fs::read(self.bundle.join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    fn json(&self, path: &str) -> Value {
        serde_json::from_slice(&self.read(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
    }
}

fn export(name: &str, profile: DawProfile) -> Exported {
    let root = tempfile::tempdir().expect("a place to write");
    let bundle = root.path().join("bundle");
    let report = session(name)
        .export_daw_bundle(DawExportOptions::new(profile), &bundle)
        .expect("the example compiles and exports");
    Exported {
        _root: root,
        bundle,
        report,
    }
}

/// Every path the bundle claims, relative to its root.
fn walk(root: &Path) -> Vec<String> {
    fn visit(root: &Path, at: &Path, into: &mut Vec<String>) {
        let mut entries = std::fs::read_dir(at)
            .expect("a readable directory")
            .map(|entry| entry.expect("a readable entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            if entry.is_dir() {
                visit(root, &entry, into);
            } else {
                into.push(
                    entry
                        .strip_prefix(root)
                        .expect("inside the bundle")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut paths = Vec::new();
    visit(root, root, &mut paths);
    paths
}

#[test]
fn a_logic_bundle_holds_both_midi_documents_the_notation_and_the_audio() {
    let exported = export(ROUTED, DawProfile::Logic);
    let paths = walk(&exported.bundle);
    for expected in [
        "audio/mix.wav",
        "audio/parts/violin.wav",
        "audio/parts/strings.wav",
        "audio/returns/hall.wav",
        "musa-manifest.json",
        "musa-origins.json",
        "performance.mid",
        "score.mid",
        "score.musicxml",
    ] {
        assert!(
            paths.contains(&expected.to_owned()),
            "{expected} is missing from {paths:?}"
        );
    }
    assert_eq!(paths.len(), 9, "{paths:?}");
}

#[test]
fn a_garageband_bundle_says_it_has_no_notation_rather_than_quietly_omitting_one() {
    let exported = export(ROUTED, DawProfile::GarageBand);
    let paths = walk(&exported.bundle);
    assert!(!paths.contains(&"score.musicxml".to_owned()), "{paths:?}");
    assert_eq!(exported.json("musa-manifest.json")["notation"], Value::Null);
    assert!(
        exported
            .report
            .losses()
            .iter()
            .any(|loss| loss.kind() == "notation" && loss.message().contains("MusicXML")),
        "{:?}",
        exported.report.losses()
    );
}

#[test]
fn the_profile_changes_the_packaging_and_never_the_music() {
    let logic = export(ROUTED, DawProfile::Logic);
    let garageband = export(ROUTED, DawProfile::GarageBand);
    for artifact in [
        "score.mid",
        "performance.mid",
        "audio/mix.wav",
        "audio/parts/violin.wav",
    ] {
        assert_eq!(
            logic.read(artifact),
            garageband.read(artifact),
            "`{artifact}` differs between profiles, so the profile changed the work"
        );
    }
}

#[test]
fn every_file_the_manifest_names_is_there_with_the_digest_it_claims() {
    use sha2::{Digest as _, Sha256};

    let exported = export(ROUTED, DawProfile::Logic);
    let manifest = exported.json("musa-manifest.json");
    let files = manifest["files"].as_array().expect("a file table");
    assert!(!files.is_empty());
    for file in files {
        let path = file["path"].as_str().expect("a path");
        let bytes = exported.read(path);
        assert_eq!(bytes.len() as u64, file["bytes"].as_u64().expect("a size"), "{path}");
        let digest = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(digest, file["sha256"].as_str().expect("a digest"), "{path}");
    }
    // The manifest cannot carry its own digest, and a table listing the
    // sidecar but not itself would have a rule nobody could state.
    let named = files
        .iter()
        .map(|file| file["path"].as_str().expect("a path").to_owned())
        .collect::<Vec<_>>();
    assert!(!named.contains(&"musa-manifest.json".to_owned()));
    assert!(!named.contains(&"musa-origins.json".to_owned()));
    assert_eq!(named.len(), walk(&exported.bundle).len() - 2);
}

#[test]
fn the_report_names_every_file_written_and_agrees_with_the_manifest_about_each() {
    let exported = export(ROUTED, DawProfile::Logic);
    let reported = exported
        .report
        .files()
        .iter()
        .map(|file| (file.path().to_owned(), file.digest().to_owned()))
        .collect::<Vec<_>>();
    // The report is the record of what landed on disk, so it names the
    // manifest and the sidecar too — which the manifest's own table cannot.
    let mut written = walk(&exported.bundle);
    written.sort();
    let mut named = reported.iter().map(|(path, _)| path.clone()).collect::<Vec<_>>();
    named.sort();
    assert_eq!(named, written);

    let manifest = exported.json("musa-manifest.json");
    for file in manifest["files"].as_array().expect("a file table") {
        let path = file["path"].as_str().expect("a path").to_owned();
        let digest = file["sha256"].as_str().expect("a digest").to_owned();
        assert!(
            reported.contains(&(path.clone(), digest)),
            "{path} differs from the report"
        );
    }
    assert_eq!(exported.report.version(), 1);
    assert_eq!(exported.report.destination(), exported.bundle);
}

#[test]
fn exporting_the_same_piece_twice_writes_the_same_bytes() {
    let first = export(ROUTED, DawProfile::Logic);
    let second = export(ROUTED, DawProfile::Logic);
    for path in walk(&first.bundle) {
        assert_eq!(first.read(&path), second.read(&path), "`{path}` is not reproducible");
    }
}

#[test]
fn nothing_in_the_bundle_records_when_or_where_it_was_made() {
    let exported = export(ROUTED, DawProfile::Logic);
    let home = std::env::var("HOME").unwrap_or_default();
    let root = exported.bundle.to_string_lossy().to_string();
    for path in ["musa-manifest.json", "musa-origins.json"] {
        let text = String::from_utf8(exported.read(path)).expect("json is text");
        assert!(!text.contains(&root), "{path} names the directory it was written to");
        assert!(home.is_empty() || !text.contains(&home), "{path} names the user's home");
        assert!(
            !text.contains("20") || !text.contains("T00:"),
            "{path} looks like it has a timestamp"
        );
    }
}

#[test]
fn the_manifest_states_the_render_it_was_taken_from() {
    let exported = export(ROUTED, DawProfile::Logic);
    let manifest = exported.json("musa-manifest.json");
    let render = &manifest["render"];
    assert_eq!(render["channels"], 2);
    assert_eq!(render["bitsPerSample"], 32);
    assert_eq!(render["sampleFormat"], "float");
    assert!(render["frames"].as_u64().expect("a frame count") > 0);
    assert!(render["sampleRate"].as_u64().expect("a rate") > 0);
    assert!(manifest["identity"]["music"].as_str().expect("a music identity").len() > 4);
}

#[test]
fn every_part_is_mapped_to_its_midi_track_and_its_stem() {
    let exported = export(ROUTED, DawProfile::Logic);
    let manifest = exported.json("musa-manifest.json");
    let parts = manifest["parts"].as_array().expect("a part table");
    assert_eq!(parts.len(), 2);
    for part in parts {
        let stem = part["stem"].as_str().expect("a stem path");
        assert!(walk(&exported.bundle).contains(&stem.to_owned()), "{stem}");
        assert!(part["midiTrack"].as_u64().expect("a track") > 0);
        assert!(part["midiChannel"].as_u64().is_some());
    }
    let returns = manifest["returns"].as_array().expect("a return table");
    assert_eq!(returns.len(), 1);
    assert_eq!(returns[0]["name"], "hall");
}

#[test]
fn the_routes_are_recorded_because_the_stems_do_not_sum_to_the_mix() {
    let exported = export(ROUTED, DawProfile::Logic);
    let manifest = exported.json("musa-manifest.json");
    let routes = manifest["routes"].as_array().expect("a route table");
    assert!(
        routes
            .iter()
            .any(|route| route["kind"] == "send" && route["source"] == "violin" && route["destination"] == "hall"),
        "{routes:?}"
    );
    assert!(
        manifest["additive"]
            .as_str()
            .expect("the statement")
            .contains("do not sum")
    );
}

#[test]
fn every_written_note_can_be_named_back_to_a_source_event() {
    let exported = export(ROUTED, DawProfile::Logic);
    let origins = exported.json("musa-origins.json");
    let files = origins["midi"].as_array().expect("both midi files");
    assert_eq!(files.len(), 2);
    for file in files {
        let notes = file["notes"].as_array().expect("the notes");
        assert!(!notes.is_empty());
        for note in notes {
            assert!(note["track"].as_u64().expect("a track") > 0);
            assert!(note["event"].as_u64().is_some());
        }
    }
    // The sidecar is a reading, and says so where a reader will see it.
    assert!(
        origins["canonical"]
            .as_str()
            .expect("the statement")
            .contains("canonical")
    );
}

#[test]
fn every_loss_the_bundle_reports_is_also_written_into_it() {
    let exported = export(ROUTED, DawProfile::Logic);
    let manifest = exported.json("musa-manifest.json");
    let written = manifest["losses"]
        .as_array()
        .expect("a loss table")
        .iter()
        .map(|loss| {
            (
                loss["kind"].as_str().expect("a kind").to_owned(),
                loss["message"].as_str().expect("a message").to_owned(),
            )
        })
        .collect::<Vec<_>>();
    let reported = exported
        .report
        .losses()
        .iter()
        .map(|loss| (loss.kind().to_owned(), loss.message().to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(reported, written);
    let kinds = written.iter().map(|(kind, _)| kind.as_str()).collect::<Vec<_>>();
    assert!(kinds.contains(&"tuning"), "{kinds:?}");
    assert!(kinds.contains(&"controller"), "{kinds:?}");
}

#[test]
fn an_existing_destination_is_left_alone_unless_replacement_was_asked_for() {
    let root = tempfile::tempdir().expect("a place to write");
    let bundle = root.path().join("bundle");
    std::fs::create_dir(&bundle).expect("an occupied destination");
    std::fs::write(bundle.join("mine.txt"), b"not yours").expect("a file already there");
    let session = session(ROUTED);
    assert!(
        session
            .export_daw_bundle(DawExportOptions::new(DawProfile::Logic), &bundle)
            .is_err()
    );
    assert_eq!(walk(&bundle), vec!["mine.txt".to_owned()]);

    let options = DawExportOptions::new(DawProfile::Logic).replacing();
    session.export_daw_bundle(options, &bundle).expect("a replacement");
    let paths = walk(&bundle);
    assert!(!paths.contains(&"mine.txt".to_owned()), "{paths:?}");
    assert!(paths.contains(&"score.mid".to_owned()), "{paths:?}");
    // The replaced directory is gone, not left beside the bundle.
    assert_eq!(walk(root.path()).iter().filter(|path| path.contains("mine")).count(), 0);
}

#[test]
fn an_export_that_cannot_finish_leaves_nothing_behind() {
    let root = tempfile::tempdir().expect("a place to write");
    let bundle = root.path().join("nowhere").join("bundle");
    let session = session(ROUTED);
    assert!(
        session
            .export_daw_bundle(DawExportOptions::new(DawProfile::Logic), &bundle)
            .is_err()
    );
    assert_eq!(walk(root.path()), Vec::<String>::new());
}

#[test]
fn a_piece_that_never_compiled_exports_nothing() {
    let root = tempfile::tempdir().expect("a place to write");
    let session = ProjectSession::from_text("piece \"broken\" {".to_owned(), "broken.musa");
    assert!(
        session
            .export_daw_bundle(DawExportOptions::new(DawProfile::Logic), &root.path().join("bundle"))
            .is_err()
    );
    assert_eq!(walk(root.path()), Vec::<String>::new());
}

#[test]
fn a_solo_piece_with_no_studio_still_bundles() {
    let exported = export("chant.musa", DawProfile::GarageBand);
    let paths = walk(&exported.bundle);
    assert!(paths.contains(&"audio/mix.wav".to_owned()), "{paths:?}");
    assert!(paths.iter().any(|path| path.starts_with("audio/parts/")), "{paths:?}");
    assert!(
        !paths.iter().any(|path| path.starts_with("audio/returns/")),
        "{paths:?}"
    );
}

#[test]
fn every_artifact_parses_as_what_it_claims_to_be() {
    let exported = export(ROUTED, DawProfile::Logic);
    for path in ["score.mid", "performance.mid"] {
        let bytes = exported.read(path);
        let smf = midly::Smf::parse(&bytes).unwrap_or_else(|error| panic!("{path}: {error}"));
        assert!(smf.tracks.len() > 1, "{path} has only a tempo track");
    }
    for path in walk(&exported.bundle).iter().filter(|path| path.ends_with(".wav")) {
        let bytes = exported.read(path);
        let audio =
            hound::WavReader::new(std::io::Cursor::new(bytes)).unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(audio.spec().channels, 2, "{path}");
        assert_eq!(audio.spec().bits_per_sample, 32, "{path}");
    }
    let xml = String::from_utf8(exported.read("score.musicxml")).expect("musicxml is text");
    // Read to the end rather than searched: a substring proves a prefix, and
    // a workstation reads the whole document.
    let mut reader = quick_xml::Reader::from_str(&xml);
    let mut root = None;
    let mut depth = 0usize;
    loop {
        match reader.read_event().expect("well-formed XML") {
            quick_xml::events::Event::Start(start) => {
                if depth == 0 {
                    root = Some(String::from_utf8_lossy(start.name().as_ref()).into_owned());
                }
                depth = depth.saturating_add(1);
            }
            quick_xml::events::Event::End(_) => depth = depth.saturating_sub(1),
            quick_xml::events::Event::Eof => break,
            // Everything else is content rather than structure, and this law
            // is about structure.
            quick_xml::events::Event::Empty(_)
            | quick_xml::events::Event::Text(_)
            | quick_xml::events::Event::CData(_)
            | quick_xml::events::Event::Comment(_)
            | quick_xml::events::Event::Decl(_)
            | quick_xml::events::Event::PI(_)
            | quick_xml::events::Event::DocType(_)
            | quick_xml::events::Event::GeneralRef(_) => {}
        }
    }
    assert_eq!(root.as_deref(), Some("score-partwise"));
    assert_eq!(depth, 0, "every element is closed");

    // The manifest and the sidecar are read as JSON everywhere else in this
    // file; naming them here keeps the claim in one place.
    for path in ["musa-manifest.json", "musa-origins.json"] {
        assert!(exported.json(path).is_object(), "{path} is not a JSON document");
    }
}

/// A piece whose parts keep their own clocks and their own barlines is the
/// case a Standard MIDI File cannot state, and the bundle says so twice: once
/// for the tempo track and once for the time-signature track.
///
/// It still exports. The losses are what the file *cannot* carry; the notes
/// are written at the moment they sound either way (`06-daw-boundary.md` §4).
#[test]
fn a_piece_with_two_clocks_and_two_barrings_reports_both_losses_and_still_bundles() {
    const TWO: &str = r#"piece "Two clocks" {
    tempo 1/4 = 120;
    meter 4/4;
    score {
        part fast {
            tempo 1/4 = 180;
            voice one { c5/4 d5/4 e5/4 f5/4 }
        }
        part slow {
            tempo 1/4 = 60;
            meter 3/4;
            voice one { c3/4 g3/4 c4/4 }
        }
    }
}
"#;
    let root = tempfile::tempdir().expect("a place to write");
    let bundle = root.path().join("bundle");
    let report = ProjectSession::from_text(TWO.to_owned(), "two-clocks.musa")
        .export_daw_bundle(DawExportOptions::new(DawProfile::Logic), &bundle)
        .expect("two clocks still bundle");
    let kinds = report.losses().iter().map(|loss| loss.kind()).collect::<Vec<_>>();
    assert!(kinds.contains(&"polytempo"), "{kinds:?}");
    assert!(kinds.contains(&"polymeter"), "{kinds:?}");
    // And the file the loss is about is present rather than withheld.
    assert!(bundle.join("performance.mid").is_file());
}

/// Both MIDI documents are written, and for a piece whose performance is
/// shaped they are not the same document.
///
/// This is roadmap §12.5's distinction as a file pair: `score.mid` is what is
/// written, `performance.mid` is what is played, and a bundle that emitted
/// one of them twice would quietly lose the difference.
#[test]
fn a_shaped_performance_is_written_twice_and_the_two_readings_differ() {
    let exported = export("shuffle.musa", DawProfile::Logic);
    let written = exported.read("score.mid");
    let played = exported.read("performance.mid");
    assert_ne!(written, played, "swing is a performance, not a spelling");
    for (path, bytes) in [("score.mid", &written), ("performance.mid", &played)] {
        let smf = midly::Smf::parse(bytes).unwrap_or_else(|error| panic!("{path}: {error}"));
        assert!(!smf.tracks.is_empty(), "{path} has no tracks");
    }
}

/// A locked recording reaches the mix, and lengthens every stem with it.
///
/// The tail is the point. The guide part plays one rest, so its stem is
/// silent — but it is exactly as long as the mix, because a stem is a read of
/// one frame range and not a render of its own.
#[test]
fn fixed_media_reaches_the_mix_and_every_stem_shares_its_length() {
    let project = tempfile::tempdir().expect("temporary project");
    let write = |path: std::path::PathBuf, bytes: Vec<u8>| {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("a place for it");
        }
        std::fs::write(path, bytes).expect("written");
    };
    write(
        project.path().join("musa.toml"),
        br#"[project]
name = "Bundled media"

[assets."assets/recording.wav"]
kind = "audio"
adapter = "wav@1"
max_bytes = 262144
"#
        .to_vec(),
    );
    write(
        project.path().join("piece.musa"),
        br#"piece "Bundled media" {
    meter 4/4;
    fixed_media recording from "assets/recording.wav";
    score { cue recording at 1:1; part guide { voice one { rest/4 } } }
    studio { route recording -> master; }
}
"#
        .to_vec(),
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
        for _ in 0..48_000 {
            writer.write_sample(0.25_f32).expect("recording sample");
        }
        writer.finalize().expect("recording finish");
    }
    write(project.path().join("assets/recording.wav"), recording.into_inner());
    musa_project::lock_assets(project.path()).expect("the recording locks");

    let session = ProjectSession::open(project.path().join("piece.musa")).expect("the piece opens");
    let root = tempfile::tempdir().expect("a place to write");
    let bundle = root.path().join("bundle");
    session
        .export_daw_bundle(DawExportOptions::new(DawProfile::Logic), &bundle)
        .expect("media bundles");

    let frames = |path: &str| {
        let bytes = std::fs::read(bundle.join(path)).unwrap_or_else(|error| panic!("{path}: {error}"));
        let reader =
            hound::WavReader::new(std::io::Cursor::new(bytes)).unwrap_or_else(|error| panic!("{path}: {error}"));
        reader.duration()
    };
    let mix = frames("audio/mix.wav");
    // A quarter at the default tempo is far shorter than the one-second
    // recording, so a mix that ignored the cue could not reach this length.
    assert!(mix >= 48_000, "the recording is not in the mix: {mix} frames");
    assert_eq!(
        frames("audio/parts/guide.wav"),
        mix,
        "a stem is a read of the same frames"
    );
}

/// The bundle is a directory of relative paths, and stays one wherever it is
/// asked to sit.
///
/// Two adversaries at once: a destination whose own name a shell would need
/// quoting for, and a file table that must never name a path leading out of
/// the bundle it describes.
#[test]
fn a_bundle_is_written_under_an_awkward_destination_and_never_names_a_path_outside_itself() {
    let root = tempfile::tempdir().expect("a place to write");
    let awkward = root.path().join("a folder with spaces");
    std::fs::create_dir_all(&awkward).expect("a place to write beside");
    let bundle = awkward.join("mon œuvre — v2");
    let report = session(ROUTED)
        .export_daw_bundle(DawExportOptions::new(DawProfile::Logic), &bundle)
        .expect("an awkward name is still a name");
    assert!(bundle.join("musa-manifest.json").is_file());
    for file in report.files() {
        let path = file.path();
        assert!(!path.contains(".."), "{path} climbs out of the bundle");
        assert!(!path.starts_with('/'), "{path} is not relative");
        assert!(!path.contains('\\'), "{path} is not a bundle path");
        assert!(
            path.is_ascii() && !path.chars().any(char::is_control),
            "{path} is not plainly writable"
        );
    }
    assert_eq!(walk(&bundle).len(), report.files().len());
}

/// The bundle reads the last score that compiled, and says which one it was.
///
/// A workstation export cannot wait for the source to be valid again, and it
/// must not silently re-export a stale reading as though it were the current
/// one either. The manifest's identity is what makes the difference visible.
#[test]
fn the_bundle_names_the_score_it_read_and_keeps_reading_the_last_valid_one() {
    const ONE: &str = r#"piece "Cache" { meter 4/4; score { part p { voice v { c4/4 d4/4 e4/4 f4/4 } } } }"#;
    const TWO: &str = r#"piece "Cache" { meter 4/4; score { part p { voice v { g4/4 a4/4 b4/4 c5/4 } } } }"#;
    let root = tempfile::tempdir().expect("a place to write");
    let mut session = ProjectSession::from_text(ONE.to_owned(), "cache.musa");
    let identity = |session: &ProjectSession, at: &Path| {
        session
            .export_daw_bundle(DawExportOptions::new(DawProfile::Logic).replacing(), at)
            .expect("the piece exports");
        let bytes = std::fs::read(at.join("musa-manifest.json")).expect("a manifest");
        let manifest: Value = serde_json::from_slice(&bytes).expect("valid JSON");
        manifest["identity"]["music"].as_str().expect("an identity").to_owned()
    };
    let bundle = root.path().join("bundle");
    let first = identity(&session, &bundle);

    // Broken source: the last valid score is still the one exported.
    let _update = session.apply(musa_project::ProjectCommand::SetSource("piece \"Cache\" {".to_owned()));
    assert_eq!(identity(&session, &bundle), first, "a broken edit is not a new reading");

    // Different music: a different reading, and the manifest says so.
    let _update = session.apply(musa_project::ProjectCommand::SetSource(TWO.to_owned()));
    assert_ne!(
        identity(&session, &bundle),
        first,
        "different notes are a different piece"
    );
}
