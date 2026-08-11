//! The provenance facts Origin view is drawn from
//! (`docs/interface/04-provenance.md`).
//!
//! `glass-mountain.musa` is the canonical case the specification is written
//! about: ten of the violin's notes come from two occurrences of one
//! five-note motif, one of them transposed down a fifth, and the strings
//! parts are authored. Everything the lens draws — which notes are generated,
//! which expansion made them, where that motif is declared — is asserted
//! here, because the frontend computes none of it.

// A missing occurrence or span is the failure these tests exist to report, so
// panicking on one is the assertion, not an oversight.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::path::PathBuf;

use musa_project::{OccurrenceFacts, ProjectSession, ScoreFacts};

fn example(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn facts(name: &str) -> (ScoreFacts, String) {
    let source = example(name);
    let session = ProjectSession::from_text(&source, name);
    let snapshot = session.snapshot();
    let score = snapshot.score().cloned().expect("the example compiles");
    (score, source)
}

/// The text a span points at, which is how a span is checked to be the right
/// one rather than merely a plausible pair of numbers.
fn quoted(source: &str, span: musa_project::Span) -> String {
    let range = usize::try_from(span.start).unwrap_or(0)..usize::try_from(span.end).unwrap_or(0);
    source.get(range).unwrap_or("").to_owned()
}

/// An expansion path read as the words the Origin row prints.
fn labels(entry: &OccurrenceFacts) -> Vec<&str> {
    entry.path.iter().map(|step| step.label.as_str()).collect()
}

fn occurrence<'a>(score: &'a ScoreFacts, id: &str) -> Option<&'a OccurrenceFacts> {
    score.occurrences.iter().find(|entry| entry.id == id)
}

#[test]
fn glass_mountain_has_two_occurrences_of_one_motif() {
    let (score, source) = facts("glass-mountain.musa");
    assert_eq!(
        score.occurrences.len(),
        2,
        "two `use sigh()` statements, two occurrences"
    );

    let plain = score.occurrences.first().expect("a first occurrence");
    let transposed = score.occurrences.get(1).expect("a second occurrence");

    assert_eq!(labels(plain), vec!["sigh()"]);
    assert_eq!(
        labels(transposed),
        vec!["transpose down P5", "sigh()"],
        "the path reads outside in: the `use` sits inside the transform block"
    );
    assert_eq!(transposed.label, "transpose down P5 \u{25b8} sigh()");

    for entry in [plain, transposed] {
        assert_eq!(entry.motif.as_deref(), Some("sigh"));
        assert_eq!(entry.events.len(), 5, "the motif is five notes");
        assert_eq!(quoted(&source, entry.use_site).trim(), "use sigh();");
        let declaration = entry.declaration.expect("the motif's declaration");
        assert!(
            quoted(&source, declaration).starts_with("motif sigh"),
            "the declaration span points at the motif"
        );
    }

    assert_ne!(plain.id, transposed.id, "two expansions are two occurrences");
    assert_ne!(
        plain.use_site.start, transposed.use_site.start,
        "and they are two different `use` statements"
    );
}

#[test]
fn every_generated_event_names_the_occurrence_that_made_it() {
    let (score, _) = facts("glass-mountain.musa");
    let generated: Vec<_> = score.events.iter().filter(|event| event.origin.generated).collect();
    assert_eq!(generated.len(), 10, "ten generated notes in the violin part");

    for event in &generated {
        let id = event
            .origin
            .occurrence
            .as_deref()
            .expect("a generated event has an occurrence");
        let entry = occurrence(&score, id).expect("the occurrence is in the table");
        assert!(
            entry.events.contains(&event.id),
            "the occurrence lists every event it produced"
        );
        assert_eq!(entry.path, event.origin.path, "the event and its occurrence agree");
    }
}

#[test]
fn note_index_counts_within_the_occurrence() {
    let (score, _) = facts("glass-mountain.musa");
    for entry in &score.occurrences {
        let indices: Vec<_> = entry
            .events
            .iter()
            .filter_map(|id| score.events.iter().find(|event| event.id == *id))
            .filter_map(|event| event.origin.note_index)
            .collect();
        assert_eq!(indices, vec![1, 2, 3, 4, 5], "note 1 through note 5 of {}", entry.label);
    }
}

#[test]
fn authored_events_claim_no_occurrence() {
    let (score, _) = facts("glass-mountain.musa");
    let authored: Vec<_> = score.events.iter().filter(|event| !event.origin.generated).collect();
    assert!(!authored.is_empty(), "the strings parts are typed out");
    for event in authored {
        assert_eq!(event.origin.occurrence, None);
        assert!(event.origin.path.is_empty());
        assert_eq!(event.origin.note_index, None);
    }
}

#[test]
fn a_voice_reports_whether_any_of_its_music_was_generated() {
    let (score, _) = facts("glass-mountain.musa");
    let generated: Vec<_> = score
        .parts
        .iter()
        .flat_map(|part| part.voices.iter().map(move |voice| (part.name.clone(), voice)))
        .filter(|(_, voice)| voice.generated)
        .map(|(part, voice)| format!("{part}/{}", voice.name))
        .collect();
    assert_eq!(
        generated,
        vec!["violin/lead".to_owned()],
        "only the violin's lead voice is expanded"
    );
}
