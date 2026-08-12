//! What the tonal, chord, and cadence readings may and may not say.
//!
//! `docs/rules/language/07-analysis.md` §2 admits a kind only with an abstract
//! domain, an abstraction map, and a soundness claim. These are the soundness
//! claims of those three kinds, written as tests:
//!
//! - **Evidence, not assertion.** Every classified finding cites the notes it
//!   read and carries the criteria it was judged against, including the ones
//!   that failed. A reading with nothing to argue about is a reading nobody
//!   can check.
//! - **Ambiguity survives.** Where two chords fit a sonority exactly, or two
//!   keys account for a passage, both are in the report as candidates and
//!   neither is chosen. Where nothing fits, nothing is reported — there is no
//!   low-confidence guess.
//! - **The written key is evidence, not proof.** A passage that leaves the
//!   written key is read in the key the notes support, and the source's own
//!   key appears as a criterion the reading did or did not satisfy.
//! - **Tonicization and modulation are not decided by duration.** At a change
//!   of tonic both readings appear, with the OMT 051 criteria attached.
//! - **Symbols are compared, never interpreted.** A written chord symbol that
//!   disagrees with the notes under it is a `conflict` finding and not a
//!   diagnostic; nothing is derived from the symbol.
//! - **Nothing is mutated.** Analyzing raises no diagnostic and leaves the
//!   snapshot and its identity exactly where they were.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]
// This suite picks one observation shape out of an enum and ignores the rest;
// the lint exists so a new variant is considered where it matters, and "which
// of these are numerals" is not one of those places.
#![allow(clippy::wildcard_enum_match_arm)]

use musa_compiler::{
    AnalysisKind, AnalysisProfile, AnalysisReport, AnalysisRequest, Approach, Cadence, CompileOptions, Evidence, Fit,
    Key, Mode, Observation, PitchClass, ScoreSnapshot, Segmentation, Severity, SourceDocument, Standing, analyze,
    compile,
};

const PIVOT: &str = include_str!("../../../examples/analysis/pivot-ambiguity.musa");
const EQUIVOCAL: &str = include_str!("../../../examples/analysis/equivocal-sonority.musa");
const CADENCES: &str = include_str!("../../../examples/analysis/cadence-evidence.musa");
const MIXTURE: &str = include_str!("../../../examples/analysis/mixture-and-direct.musa");
const UNKNOWN: &str = include_str!("../../../examples/analysis/unknown-passage.musa");

/// Compile a fixture and hand back the snapshot the analyses read.
fn score(source: &str, name: &str) -> ScoreSnapshot {
    let compiled = compile(&SourceDocument::new(source, name), &CompileOptions::default());
    assert!(
        !compiled.has_errors(),
        "the fixture must compile: {:?}",
        compiled
            .diagnostics()
            .iter()
            .filter(|found| found.severity == Severity::Error)
            .map(|found| found.message.clone())
            .collect::<Vec<_>>()
    );
    compiled.into_snapshot().expect("a score")
}

fn read(source: &str, name: &str, kind: AnalysisKind) -> AnalysisReport {
    analyze(&score(source, name), &AnalysisRequest::new(kind)).expect("a well-formed request")
}

/// Every numeral the report offers, as text plus the key it is in.
fn numerals(report: &AnalysisReport) -> Vec<(String, String)> {
    report
        .findings()
        .iter()
        .filter_map(|finding| match *finding.observation() {
            Observation::Numeral { ref numeral, key, .. } => Some((numeral.clone(), spell(key))),
            _ => None,
        })
        .collect()
}

fn spell(key: Key) -> String {
    format!(
        "{} {}",
        key.tonic(),
        match key.mode() {
            Mode::Major => "major",
            Mode::Minor => "minor",
        }
    )
}

/// The pivot chord is read in both keys, in one report.
///
/// This is what a pivot *is* (OMT `051`): a chord diatonic in the key being
/// left and in the key being reached. A reading that assigned it to one region
/// and stopped would have named the modulation without ever showing the chord
/// that makes it ambiguous.
#[test]
fn a_pivot_chord_is_read_in_both_keys() {
    let report = read(PIVOT, "pivot.musa", AnalysisKind::Tonal);
    let at_the_pivot: Vec<(String, String)> = report
        .findings()
        .iter()
        .filter(|finding| finding.observation().at() == musa_compiler::MusicalTime::new((2, 1).into()))
        .filter_map(|finding| match *finding.observation() {
            Observation::Numeral { ref numeral, key, .. } => Some((numeral.clone(), spell(key))),
            _ => None,
        })
        .collect();
    assert!(
        at_the_pivot.contains(&("vi".to_owned(), "c major".to_owned())),
        "the pivot is not `vi` in the key being left: {at_the_pivot:?}"
    );
    assert!(
        at_the_pivot.contains(&("ii".to_owned(), "g major".to_owned())),
        "the pivot is not `ii` in the key being reached: {at_the_pivot:?}"
    );
}

/// An applied chord is named as one in the key it is applied *in*, not only as
/// a plain dominant in the key it points at (OMT `050`).
#[test]
fn an_applied_chord_keeps_the_key_it_is_applied_in() {
    let report = read(PIVOT, "pivot.musa", AnalysisKind::Tonal);
    let readings = numerals(&report);
    assert!(
        readings.contains(&("V/V".to_owned(), "c major".to_owned())),
        "the D major chord is not read as `V/V` in C major: {readings:?}"
    );
    assert!(
        readings.contains(&("V".to_owned(), "g major".to_owned())),
        "the D major chord is not read as `V` in G major: {readings:?}"
    );
}

/// Both readings of a change of tonic arrive together, and neither is decided
/// by how long the new key lasts.
#[test]
fn a_change_of_tonic_is_reported_as_both_readings() {
    let report = read(PIVOT, "pivot.musa", AnalysisKind::Tonal);
    let modulation = report
        .findings()
        .iter()
        .find(|finding| matches!(*finding.observation(), Observation::Modulation { .. }))
        .expect("a modulation candidate");
    let tonicization = report
        .findings()
        .iter()
        .find(|finding| matches!(*finding.observation(), Observation::Tonicization { .. }))
        .expect("a tonicization candidate");
    assert_eq!(modulation.standing(), Standing::Candidate);
    assert_eq!(tonicization.standing(), Standing::Candidate);
    assert_eq!(
        modulation.observation().at(),
        tonicization.observation().at(),
        "the two readings are of the same place or they are not alternatives"
    );
    let criteria: Vec<&str> = modulation.grounds().iter().map(|ground| ground.criterion).collect();
    assert!(
        criteria.iter().all(|criterion| !criterion.contains("long")),
        "a criterion about length decided this: {criteria:?}"
    );
    assert!(
        modulation.grounds().iter().any(|ground| !ground.satisfied),
        "a modulation with every criterion satisfied would be a fact, and this cannot be one"
    );
}

/// Where the notes fit two chords exactly, both are candidates and neither is
/// promoted.
#[test]
fn two_exact_fits_are_two_candidates() {
    let report = read(EQUIVOCAL, "equivocal.musa", AnalysisKind::Chords);
    let first: Vec<(String, Standing)> = report
        .findings()
        .iter()
        .filter(|finding| finding.observation().at() == musa_compiler::MusicalTime::ZERO)
        .filter_map(|finding| match *finding.observation() {
            Observation::ChordFit {
                chord, fit: Fit::Exact, ..
            } => Some((chord.to_string(), finding.standing())),
            _ => None,
        })
        .collect();
    assert_eq!(first.len(), 2, "C-E-G-A fits two chords exactly: {first:?}");
    assert!(
        first.iter().all(|(_, standing)| *standing == Standing::Candidate),
        "one of two exact fits was reported as a fact: {first:?}"
    );
}

/// One exact fit and nothing else is a fact — the abstraction determines it.
#[test]
fn a_single_exact_fit_is_a_fact() {
    let report = read(EQUIVOCAL, "equivocal.musa", AnalysisKind::Chords);
    let last = report
        .findings()
        .iter()
        .rfind(|finding| matches!(*finding.observation(), Observation::ChordFit { .. }))
        .expect("a chord fit");
    assert_eq!(last.standing(), Standing::Fact);
}

/// A fit that had to explain a note away is never a fact, and says which
/// criterion it failed.
#[test]
fn an_inexact_fit_says_what_it_could_not_account_for() {
    let report = read(EQUIVOCAL, "equivocal.musa", AnalysisKind::Chords);
    for finding in report.findings() {
        let Observation::ChordFit { fit, .. } = *finding.observation() else {
            continue;
        };
        if fit == Fit::Exact {
            continue;
        }
        assert_eq!(
            finding.standing(),
            Standing::Candidate,
            "an inexact fit was held as a fact"
        );
        assert!(
            finding.grounds().iter().any(|ground| !ground.satisfied),
            "an inexact fit with nothing unsatisfied: {finding:?}"
        );
    }
}

/// Every classified finding points at the notes it read.
#[test]
fn a_classification_cites_the_notes_it_read() {
    for kind in [AnalysisKind::Chords, AnalysisKind::Tonal, AnalysisKind::Cadences] {
        let report = read(PIVOT, "pivot.musa", kind);
        for finding in report.findings() {
            match *finding.evidence() {
                Evidence::Passage { ref notes, .. } => {
                    assert!(!notes.is_empty(), "a passage citing no notes: {finding:?}");
                }
                Evidence::Annotation { .. } | Evidence::Event(_) | Evidence::InForce { .. } => {}
            }
        }
    }
}

/// The four cadences, each with the criteria that separate them.
#[test]
fn each_cadence_reports_the_criteria_that_name_it() {
    let report = read(CADENCES, "cadences.musa", AnalysisKind::Cadences);
    let found: Vec<Cadence> = report
        .findings()
        .iter()
        .filter_map(|finding| match *finding.observation() {
            Observation::Cadence { cadence, key, .. } if key.tonic() == PitchClass::parse("c").expect("c") => {
                Some(cadence)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        found,
        vec![
            Cadence::PerfectAuthentic,
            Cadence::ImperfectAuthentic,
            Cadence::Half,
            Cadence::Deceptive
        ],
        "the four phrase endings did not read as the four cadences"
    );
    let imperfect = report
        .findings()
        .iter()
        .find(|finding| {
            matches!(
                *finding.observation(),
                Observation::Cadence {
                    cadence: Cadence::ImperfectAuthentic,
                    ..
                }
            )
        })
        .expect("the imperfect cadence");
    assert!(
        imperfect
            .grounds()
            .iter()
            .any(|ground| ground.criterion.contains("top voice") && !ground.satisfied),
        "the imperfect cadence did not say which criterion it failed"
    );
}

/// No cadence is ever a fact: a phrase ending is a formal event, and this
/// reading sees only notes.
#[test]
fn no_cadence_is_ever_a_fact() {
    for source in [(CADENCES, "cadences.musa"), (PIVOT, "pivot.musa")] {
        let report = read(source.0, source.1, AnalysisKind::Cadences);
        for finding in report.findings() {
            assert_ne!(
                finding.standing(),
                Standing::Fact,
                "a cadence was held as a fact: {finding:?}"
            );
        }
    }
}

/// A progression that is none of the four cadences is not reported as a weak
/// one. The absence is the answer.
#[test]
fn a_progression_that_is_no_cadence_is_not_reported() {
    let report = read(PIVOT, "pivot.musa", AnalysisKind::Cadences);
    let at_the_first_phrase = report.findings().iter().any(|finding| {
        matches!(*finding.observation(), Observation::Cadence { .. })
            && finding.observation().at() < musa_compiler::MusicalTime::new((3, 1).into())
    });
    assert!(
        !at_the_first_phrase,
        "IV-vi at the end of the first phrase was called a cadence"
    );
}

/// A symbol that disagrees with the notes under it is a conflict finding, and
/// nothing more: no diagnostic, and no notes derived from the symbol.
#[test]
fn a_symbol_that_disagrees_is_a_conflict_and_not_a_diagnostic() {
    const DISAGREES: &str = "piece \"Disagreement\" {\n\
         meter 4/4;\n\
         key c major;\n\
         score {\n\
           harmony { at 1:1 f; }\n\
           part piano { voice upper { [c4 e4 g4]/1 } }\n\
         }\n\
       }\n";
    let compiled = compile(
        &SourceDocument::new(DISAGREES, "disagreement.musa"),
        &CompileOptions::default(),
    );
    assert!(!compiled.has_errors(), "the fixture must compile");
    let before = compiled.diagnostics().len();
    let snapshot = compiled.into_snapshot().expect("a score");
    let report = analyze(&snapshot, &AnalysisRequest::new(AnalysisKind::Chords)).expect("a request");
    let conflict = report
        .findings()
        .iter()
        .find(|finding| matches!(*finding.observation(), Observation::SymbolReading { .. }))
        .expect("a symbol reading");
    assert_eq!(conflict.standing(), Standing::Conflict);
    assert!(
        conflict
            .grounds()
            .iter()
            .any(|ground| ground.criterion.contains("spell") && !ground.satisfied),
        "the conflict did not say what disagreed"
    );
    let after = compile(
        &SourceDocument::new(DISAGREES, "disagreement.musa"),
        &CompileOptions::default(),
    );
    assert_eq!(
        before,
        after.diagnostics().len(),
        "analyzing changed what the compiler reported"
    );
}

/// A written key is evidence about the key and not proof of it: the reading
/// follows the notes and records whether the source agreed.
#[test]
fn the_written_key_is_a_criterion_and_not_a_conclusion() {
    let report = read(PIVOT, "pivot.musa", AnalysisKind::Tonal);
    let regions: Vec<(String, bool)> = report
        .findings()
        .iter()
        .filter_map(|finding| match *finding.observation() {
            Observation::KeyRegion { key, .. } => Some((
                spell(key),
                finding
                    .grounds()
                    .iter()
                    .any(|ground| ground.criterion.contains("source writes") && ground.satisfied),
            )),
            _ => None,
        })
        .collect();
    assert!(
        regions.iter().any(|(key, written)| key == "c major" && *written),
        "the written key was not recorded as satisfying its own criterion: {regions:?}"
    );
    assert!(
        regions.iter().any(|(key, written)| key == "g major" && !*written),
        "a region the source did not write was not marked as such: {regions:?}"
    );
}

/// An assumed key narrows the reading and does not make it true.
#[test]
fn an_assumed_key_narrows_without_promoting() {
    let snapshot = score(PIVOT, "pivot.musa");
    let request = AnalysisRequest::new(AnalysisKind::Tonal).in_key(Key::parse("g major").expect("a key"));
    let report = analyze(&snapshot, &request).expect("a request");
    let keys: Vec<String> = report
        .findings()
        .iter()
        .filter_map(|finding| match *finding.observation() {
            Observation::Numeral { key, .. } => Some(spell(key)),
            _ => None,
        })
        .collect();
    assert!(!keys.is_empty(), "an assumed key produced no reading");
    assert!(
        keys.iter().all(|key| key == "g major"),
        "an assumed key did not narrow the reading: {keys:?}"
    );
}

/// Segmentation is a policy the caller states, and the two policies genuinely
/// disagree — which is why the request has to choose one.
#[test]
fn two_segmentations_read_the_same_music_differently() {
    const OFFBEAT: &str = "piece \"Offbeat\" {\n\
         meter 4/4;\n\
         key c major;\n\
         score {\n\
           part piano {\n\
             voice upper { c5/8 d5/8 e5/8 f5/8 g5/2 }\n\
             voice lower { c3/1 }\n\
           }\n\
         }\n\
       }\n";
    let snapshot = score(OFFBEAT, "offbeat.musa");
    let count = |how| {
        analyze(&snapshot, &AnalysisRequest::new(AnalysisKind::Chords).segmenting(how))
            .expect("a request")
            .findings()
            .iter()
            .filter(|finding| matches!(*finding.observation(), Observation::Sonority { .. }))
            .count()
    };
    assert!(
        count(Segmentation::Attacks) > count(Segmentation::Beats),
        "the two segmentations read this the same way, so one of them is not a policy"
    );
}

/// Reading a score twice gives the same report, byte for byte, and leaves the
/// snapshot's identity where it was.
///
/// The kinds that read against a style are left to
/// `voice_leading_validation.rs`, which has the fixtures they need: a profile
/// is required, and a counterpoint profile also needs a two-voice score with a
/// designated cantus, which this key-and-cadence fixture is not. Which kinds
/// those are is derived from the profiles rather than listed, so a new profile
/// for a new kind moves the boundary here without anyone remembering to.
#[test]
fn a_reading_changes_nothing_and_repeats_exactly() {
    let snapshot = score(PIVOT, "pivot.musa");
    let copy = snapshot.clone();
    let styled: Vec<AnalysisKind> = AnalysisProfile::ALL.iter().map(|profile| profile.kind()).collect();
    for kind in AnalysisKind::ALL.into_iter().filter(|kind| !styled.contains(kind)) {
        let request = AnalysisRequest::new(kind);
        let first = analyze(&snapshot, &request).expect("a request");
        let second = analyze(&snapshot, &request).expect("a request");
        assert_eq!(first, second, "{} was not deterministic", kind.as_str());
    }
    assert_eq!(snapshot, copy, "analyzing changed the score");
}

/// A modulation with no chord diatonic in both keys is reported as `direct`,
/// and says so through the criterion it fails (OMT `051`).
#[test]
fn a_modulation_with_no_common_chord_is_direct() {
    let report = read(MIXTURE, "mixture.musa", AnalysisKind::Tonal);
    let approaches: Vec<Approach> = report
        .findings()
        .iter()
        .filter_map(|finding| match *finding.observation() {
            Observation::Modulation { how, .. } => Some(how),
            _ => None,
        })
        .collect();
    assert!(!approaches.is_empty(), "no modulation was found in a piece with two");
    assert!(
        approaches.iter().all(|how| *how == Approach::Direct),
        "a modulation with no common chord was called a pivot: {approaches:?}"
    );
    let modulation = report
        .findings()
        .iter()
        .find(|finding| matches!(*finding.observation(), Observation::Modulation { .. }))
        .expect("a modulation");
    assert!(
        modulation
            .grounds()
            .iter()
            .any(|ground| ground.criterion.contains("diatonic in both keys") && !ground.satisfied),
        "the direct modulation did not say what it lacked"
    );
}

/// A borrowed chord leaves the written key, and the reading says so rather
/// than absorbing it silently (OMT `061`).
#[test]
fn a_borrowed_chord_ends_the_region_it_was_borrowed_into() {
    let report = read(MIXTURE, "mixture.musa", AnalysisKind::Tonal);
    let home = report
        .findings()
        .iter()
        .find_map(|finding| match *finding.observation() {
            Observation::KeyRegion { key, to, .. } if spell(key) == "c major" => Some(to),
            _ => None,
        })
        .expect("a C major region");
    assert!(
        home < musa_compiler::MusicalTime::new((8, 1).into()),
        "the borrowed chord was absorbed into the written key: the region ran to {home}"
    );
}

/// Where nothing fits, nothing is reported. There is no nearest triad and no
/// confidence number.
#[test]
fn a_passage_nothing_accounts_for_is_reported_as_nothing() {
    for kind in [AnalysisKind::Tonal, AnalysisKind::Cadences] {
        let report = read(UNKNOWN, "unknown.musa", kind);
        assert!(
            report.findings().is_empty(),
            "{} invented a reading for music nothing accounts for: {:?}",
            kind.as_str(),
            report.findings()
        );
    }
    let chords = read(UNKNOWN, "unknown.musa", AnalysisKind::Chords);
    assert!(
        chords
            .findings()
            .iter()
            .all(|finding| matches!(*finding.observation(), Observation::Sonority { .. })),
        "a chord was named for a sonority that is not one: {:?}",
        chords.findings()
    );
    assert_eq!(
        chords.findings().len(),
        4,
        "the sonorities themselves are facts and must still be reported"
    );
}

/// Every kind states what it assumed, and no kind assumes nothing.
#[test]
fn every_kind_states_its_assumptions() {
    for kind in AnalysisKind::ALL {
        assert!(
            !kind.assumptions().is_empty(),
            "{} claims to depend on nothing",
            kind.as_str()
        );
        assert!(!kind.method().is_empty(), "{} says nothing about itself", kind.as_str());
    }
}
