//! Aligned stems beside the mix (roadmap §12.6, §13.8).
//!
//! Two things make a stem export trustworthy rather than merely present: the
//! mix it comes with is the same mix the WAV export writes, and every file in
//! the set covers the same stretch of time. Both are laws here, because both
//! are what a workstation silently assumes when it lines the files up.

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::path::PathBuf;

use musa_project::{ExportRequest, ProjectSession, StemKind, StemRouteKind, StemSet};

fn example(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn session(name: &str) -> ProjectSession {
    ProjectSession::from_text(example(name), name)
}

fn stems(name: &str) -> StemSet {
    session(name).export_stems().expect("the example compiles and renders")
}

const ROUTED: &str = "glass-mountain.musa";

#[test]
fn the_set_is_the_mix_and_then_the_routes_the_source_declared() {
    let set = stems(ROUTED);
    let named = set
        .files()
        .iter()
        .map(|file| (file.kind(), file.part(), file.file_name()))
        .collect::<Vec<_>>();
    assert_eq!(
        named,
        vec![
            (StemKind::Master, "", "mix.wav"),
            (StemKind::Part, "violin", "violin.wav"),
            (StemKind::Part, "strings", "strings.wav"),
            (StemKind::Bus, "hall", "hall.wav"),
        ]
    );
}

#[test]
fn the_mix_beside_the_stems_is_the_wav_export_byte_for_byte() {
    let session = session(ROUTED);
    let exported = session.export(ExportRequest::Wav).expect("wav");
    let set = session.export_stems().expect("stems");
    let master = set
        .files()
        .iter()
        .find(|file| file.kind() == StemKind::Master)
        .expect("a set always has its mix");
    assert_eq!(
        master.bytes(),
        exported.as_bytes(),
        "a stem export that quietly re-renders the mix is a different record of the same piece"
    );
}

#[test]
fn every_file_covers_the_same_stretch_of_time() {
    let set = stems(ROUTED);
    assert!(set.frames() > 0);
    let expected = set.files().first().map(|file| file.bytes().len());
    for file in set.files() {
        assert_eq!(
            Some(file.bytes().len()),
            expected,
            "`{}` is a different length, so nothing downstream can line it up",
            file.file_name()
        );
        let audio = hound::WavReader::new(std::io::Cursor::new(file.bytes())).expect("a readable WAV");
        assert_eq!(audio.spec().channels, 2);
        assert_eq!(audio.spec().bits_per_sample, 32);
        assert_eq!(audio.spec().sample_format, hound::SampleFormat::Float);
        assert_eq!(audio.spec().sample_rate, set.sample_rate());
        assert_eq!(u64::from(audio.duration()), set.frames());
    }
}

#[test]
fn the_declared_edges_are_reported_rather_than_implied_by_the_files() {
    let set = stems(ROUTED);
    let edges = set
        .routes()
        .iter()
        .map(|route| (route.kind(), route.source(), route.destination()))
        .collect::<Vec<_>>();
    // The sends are exactly why the stems do not sum to the mix, so they are
    // the part a reader most needs stated.
    assert!(edges.contains(&(StemRouteKind::Send, "violin", "hall")), "{edges:?}");
    assert!(edges.contains(&(StemRouteKind::Send, "strings", "hall")), "{edges:?}");
    assert!(edges.contains(&(StemRouteKind::Route, "hall", "master")), "{edges:?}");
}

#[test]
fn exporting_the_same_piece_twice_writes_the_same_bytes() {
    assert_eq!(stems(ROUTED), stems(ROUTED));
}

#[test]
fn a_piece_with_no_studio_still_hands_over_its_parts() {
    let set = stems("chant.musa");
    assert_eq!(
        set.files().first().map(musa_project::StemFile::kind),
        Some(StemKind::Master)
    );
    assert!(
        set.files().iter().skip(1).all(|file| file.kind() == StemKind::Part),
        "a piece that declares no bus has no bus to tap"
    );
    assert!(set.files().len() > 1, "the parts it does have are still routes");
}

#[test]
fn a_piece_that_never_compiled_exports_nothing() {
    let session = ProjectSession::from_text("piece \"broken\" {".to_owned(), "broken.musa");
    assert!(session.export_stems().is_err());
}
