//! Laws for prompt 203's production-neutral measurement seam.

#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::path::PathBuf;

use musa_project::ProjectSession;
use musa_project::transcription_trial::{
    QwertyPerformanceDriver, TOP_K, TrialCorpus, VirtualNoteEvent, generated_corpus, run,
};

const CORPUS: &str = include_str!("../fixtures/transcription/corpus.json");
const RESULTS: &str = include_str!("../fixtures/transcription/results.json");

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/transcription")
        .join(name)
}

#[test]
fn generated_corpus_and_results_are_canonical_and_deterministic() {
    let generated = generated_corpus();
    let corpus_json = generated.canonical_json().expect("serialize corpus");
    let report = run(&generated);
    let results_json = format!(
        "{}\n",
        serde_json::to_string_pretty(&report).expect("serialize results")
    );

    if std::env::var_os("UPDATE_TRANSCRIPTION_TRIAL").is_some() {
        std::fs::write(fixture_path("corpus.json"), &corpus_json).expect("write generated corpus");
        std::fs::write(fixture_path("results.json"), &results_json).expect("write generated results");
    }

    assert_eq!(corpus_json, CORPUS);
    assert_eq!(results_json, RESULTS);
    assert_eq!(report, run(&generated));
    assert_eq!(
        TrialCorpus::read(CORPUS)
            .expect("decode corpus")
            .canonical_json()
            .expect("encode corpus"),
        CORPUS
    );
    assert_eq!(generated.fixtures().len(), 10);
    for fixture in generated.fixtures() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(fixture.source());
        let source = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let session = ProjectSession::from_text(source, fixture.source());
        assert!(
            session.snapshot().score().is_some(),
            "{} must remain an executable intended-score fixture",
            fixture.id()
        );
    }
    assert!(
        generated
            .fixtures()
            .iter()
            .any(|fixture| fixture.tags().iter().any(|tag| tag == "unmeasured"))
    );
    assert!(report.rhythm.iter().all(|model| model.peak_states <= 96));
    assert!(
        report
            .rhythm
            .iter()
            .all(|model| model.top_k_recall <= model.top_k_total)
    );
    assert_eq!(TOP_K, 5);
}

#[test]
fn qwerty_driver_is_a_note_controller_not_a_notation_mode() {
    let mut driver = QwertyPerformanceDriver::default();
    assert_eq!(
        driver.transition('a', true, 90, 10),
        Some(VirtualNoteEvent::On {
            note: 60,
            velocity: 90,
            at_micros: 10,
        })
    );
    assert_eq!(
        driver.transition('a', true, 90, 11),
        None,
        "key repeat is not a second strike"
    );
    assert_eq!(
        driver.transition('a', false, 0, 20),
        Some(VirtualNoteEvent::Off {
            note: 60,
            at_micros: 20,
        })
    );
    assert_eq!(
        driver.transition('a', false, 0, 21),
        None,
        "unmatched release is ignored"
    );
    assert_eq!(
        driver.transition('z', true, 90, 30),
        None,
        "unmapped typing keys remain ordinary UI input"
    );
}
