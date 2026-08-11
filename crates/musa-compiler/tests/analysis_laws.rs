//! What an analysis is and is not.
//!
//! `docs/language/05-verification.md` §3 puts analyses in the third of three
//! strengths: named services that observe, may be ambiguous, and never make
//! music. `docs/language/07-analysis.md` adds the discipline that makes the
//! third strength say something precise — every kind is an abstract
//! interpretation with a domain, an abstraction map, and a soundness claim.
//! Everything pinned down here follows from those two.
//!
//! - **Reading only.** Analyzing raises no diagnostic, changes no snapshot,
//!   and leaves the semantic hash where it was. Analyzing twice gives the same
//!   report, byte for byte.
//! - **Scope and window select.** A request names a part, a voice, and a
//!   half-open span, and the report is exactly what is inside it — no note
//!   from outside, and the seam between two adjacent windows reports each note
//!   once.
//! - **Evidence points at editable text.** A generated note's evidence is the
//!   statement inside the motif that spells it, not the `use` that placed it.
//! - **A bad request is refused, an empty one is not.** A misspelled part is
//!   an error; a window with no music in it is a report with no findings.
//! - **`facts` states only facts.** Its abstraction is exact on what it
//!   reports, so nothing it returns can be a candidate or a conflict — and
//!   nothing it reports is read into the score: spelling survives, and a chord
//!   symbol stays a symbol.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]
// Exact rational time through the `MusicalTime` operators, which are total for
// musa's magnitudes (`crates/musa-compiler/src/time.rs`).
#![allow(clippy::arithmetic_side_effects)]
// This suite picks one observation or evidence shape out of an enum and
// ignores the rest; the lint exists so a new variant is considered where it
// matters, and "which of these are sounding pitches" is not one of those
// places.
#![allow(clippy::wildcard_enum_match_arm)]

use musa_compiler::{
    AnalysisError, AnalysisKind, AnalysisReport, AnalysisRequest, AnalysisScope, Evidence, MusicalTime, Observation,
    ScoreSnapshot, Severity, SourceDocument, Standing, analyze, compile,
};

const NAME: &str = "analysis.musa";

/// Two voices in one part, four bars, with a motif and a chord symbol — enough
/// for every question below to have a wrong answer available.
const PIECE: &str = "piece \"Observed\" {\n\
     meter 4/4;\n\
     key a minor;\n\
     motif figure() { c5/4 e5/4 }\n\
     score {\n\
       harmony { at 1:1 am; }\n\
       part piano {\n\
         voice lead { a4/4 c5/4 e5/2 use figure(); rest/2 }\n\
         voice bass { a3/2 e3/2 a3/1 }\n\
       }\n\
     }\n\
   }";

fn snapshot(source: &str) -> ScoreSnapshot {
    let compiled = compile(
        &SourceDocument::new(source, NAME),
        &musa_compiler::CompileOptions::default(),
    );
    let messages: Vec<&str> = compiled
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();
    assert!(
        !compiled.diagnostics().iter().any(|d| d.severity == Severity::Error),
        "the fixture did not compile: {messages:?}"
    );
    compiled.into_snapshot().expect("a snapshot")
}

fn report(request: &AnalysisRequest) -> AnalysisReport {
    analyze(&snapshot(PIECE), request).expect("a report")
}

fn facts() -> AnalysisRequest {
    AnalysisRequest::new(AnalysisKind::Facts)
}

fn at(numerator: i64, denominator: i64) -> MusicalTime {
    MusicalTime::new(num_rational::Ratio::new(numerator, denominator))
}

/// Every sounding pitch a report names, spelled the way the language spells
/// it, in the report's order.
fn sounding(report: &AnalysisReport) -> Vec<String> {
    report
        .findings()
        .iter()
        .filter_map(|finding| match *finding.observation() {
            Observation::Sounding { pitch, .. } => Some(format!("{pitch}")),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------- reading only

#[test]
fn analyzing_does_not_change_the_score() {
    let score = snapshot(PIECE);
    let before = score.clone();
    let _report = analyze(&score, &facts()).expect("a report");
    assert_eq!(score, before, "an analysis changed the score it read");
}

#[test]
fn the_same_request_gives_the_same_report() {
    let score = snapshot(PIECE);
    let once = analyze(&score, &facts()).expect("a report");
    let twice = analyze(&score, &facts()).expect("a report");
    assert_eq!(once, twice, "two runs of one analysis disagreed");
}

#[test]
fn findings_are_in_time_order() {
    let report = report(&facts());
    let times: Vec<MusicalTime> = report
        .findings()
        .iter()
        .map(|finding| finding.observation().at())
        .collect();
    let mut sorted = times.clone();
    sorted.sort_unstable();
    assert_eq!(times, sorted, "the report was not in time order");
}

#[test]
fn a_report_carries_its_method_and_assumptions() {
    let report = report(&facts());
    assert_eq!(report.kind(), AnalysisKind::Facts);
    assert!(!report.method().is_empty(), "an analysis with no stated method");
    assert!(
        !report.assumptions().is_empty(),
        "an analysis that assumed nothing is claiming something"
    );
}

// --------------------------------------------------------- scope selects music

#[test]
fn the_whole_score_reads_every_voice() {
    let heard = sounding(&report(&facts()));
    assert!(heard.contains(&"a4".to_owned()), "the lead's first note is missing");
    assert!(heard.contains(&"a3".to_owned()), "the bass is missing");
}

#[test]
fn a_voice_reads_only_that_voice() {
    let heard = sounding(&report(&facts().scoped(AnalysisScope::Voice {
        part: "piano".to_owned(),
        voice: "bass".to_owned(),
    })));
    assert_eq!(heard, vec!["a3", "e3", "a3"], "a voice request read something else");
}

#[test]
fn a_part_reads_all_of_its_voices() {
    let part = sounding(&report(&facts().scoped(AnalysisScope::Part("piano".to_owned()))));
    let score = sounding(&report(&facts()));
    assert_eq!(part, score, "the only part disagreed with the whole score");
}

#[test]
fn a_chord_symbol_belongs_to_the_piece_and_not_to_a_voice() {
    let has_symbol = |request: &AnalysisRequest| {
        report(request)
            .findings()
            .iter()
            .any(|finding| matches!(*finding.observation(), Observation::Written { .. }))
    };
    assert!(has_symbol(&facts()), "the piece's chord symbol was not reported");
    assert!(
        !has_symbol(&facts().scoped(AnalysisScope::Part("piano".to_owned()))),
        "a piece-wide symbol was attributed to one part"
    );
}

// -------------------------------------------------------- windows select music

#[test]
fn a_window_reads_only_what_is_inside_it() {
    let heard = sounding(&report(&facts().within(MusicalTime::ZERO, at(1, 2))));
    assert_eq!(heard, vec!["a4", "a3", "c5"], "the window let something else through");
}

#[test]
fn adjacent_windows_report_each_note_once() {
    let whole = sounding(&report(&facts().within(MusicalTime::ZERO, at(4, 1))));
    let mut split = sounding(&report(&facts().within(MusicalTime::ZERO, at(1, 1))));
    split.extend(sounding(&report(&facts().within(at(1, 1), at(4, 1)))));
    assert_eq!(whole, split, "the seam between two windows dropped or doubled a note");
}

#[test]
fn a_window_with_no_music_in_it_is_a_report_and_not_an_error() {
    let report = report(&facts().within(at(100, 1), at(101, 1)));
    assert!(report.findings().is_empty(), "found something after the piece ends");
}

// ------------------------------------------------------- evidence points home

#[test]
fn a_generated_note_cites_the_statement_that_spells_it() {
    let report = report(&facts());
    let spans: Vec<(u32, u32)> = report
        .findings()
        .iter()
        .filter_map(|finding| match (finding.observation(), finding.evidence()) {
            (&Observation::Sounding { pitch, .. }, &Evidence::Event(note)) if format!("{pitch}") == "e5" => {
                Some((note.span.start, note.span.end))
            }
            _ => None,
        })
        .collect();
    let inside_motif = PIECE.find("c5/4 e5/4").expect("the motif body");
    let motif_e = u32::try_from(inside_motif + "c5/4 ".len()).expect("a small offset");
    assert!(
        spans.iter().any(|(start, _)| *start == motif_e),
        "the generated E cited the use site instead of the motif's note: {spans:?}"
    );
}

#[test]
fn a_context_value_has_no_statement_to_point_at() {
    let report = report(&facts());
    let key = report
        .findings()
        .iter()
        .find(|finding| matches!(*finding.observation(), Observation::KeyInForce { .. }))
        .expect("the key in force");
    assert!(
        matches!(*key.evidence(), Evidence::InForce { .. }),
        "a key in force was given a source span it does not have"
    );
}

#[test]
fn an_annotation_cites_where_it_is_written() {
    let report = report(&facts());
    let symbol = report
        .findings()
        .iter()
        .find(|finding| matches!(*finding.observation(), Observation::Written { .. }))
        .expect("the chord symbol");
    match *symbol.evidence() {
        Evidence::Annotation { span } => {
            let text = PIECE.get(span.start as usize..span.end as usize).expect("the source");
            assert!(text.contains("am"), "the symbol's evidence points at `{text}`");
        }
        _ => panic!("a written symbol without a written place"),
    }
}

// ------------------------------------------------------- a bad request is bad

#[test]
fn a_misspelled_part_is_refused_rather_than_widened() {
    let error = analyze(
        &snapshot(PIECE),
        &facts().scoped(AnalysisScope::Part("piani".to_owned())),
    )
    .expect_err("an error");
    match error {
        AnalysisError::NoSuchPart {
            ref name,
            ref available,
        } => {
            assert_eq!(name, "piani");
            assert_eq!(available, &["piano".to_owned()], "the error did not say what there is");
        }
        other => panic!("wrong error: {other}"),
    }
}

#[test]
fn a_misspelled_voice_names_the_voices_that_part_has() {
    let error = analyze(
        &snapshot(PIECE),
        &facts().scoped(AnalysisScope::Voice {
            part: "piano".to_owned(),
            voice: "tenor".to_owned(),
        }),
    )
    .expect_err("an error");
    let message = error.to_string();
    assert!(
        message.contains("lead") && message.contains("bass"),
        "unhelpful: {message}"
    );
}

#[test]
fn a_backwards_window_is_refused() {
    let error = analyze(&snapshot(PIECE), &facts().within(at(2, 1), at(1, 1))).expect_err("an error");
    assert!(
        matches!(error, AnalysisError::EmptyWindow { .. }),
        "wrong error: {error}"
    );
}

#[test]
fn an_instant_is_not_a_window() {
    let error = analyze(&snapshot(PIECE), &facts().within(at(1, 1), at(1, 1))).expect_err("an error");
    assert!(
        matches!(error, AnalysisError::EmptyWindow { .. }),
        "wrong error: {error}"
    );
}

// --------------------------------------------------- `facts` states only facts

#[test]
fn every_finding_of_the_facts_kind_is_a_fact() {
    let report = report(&facts());
    assert!(
        report
            .findings()
            .iter()
            .all(|finding| finding.standing() == Standing::Fact),
        "the exact abstraction produced something it cannot separate"
    );
}

#[test]
fn spelling_survives_the_reading() {
    let sharps = snapshot(
        "piece \"Spelled\" {\n\
           meter 4/4;\n\
           score { part p { voice v { d#4/4 eb4/4 } } }\n\
         }",
    );
    let report = analyze(&sharps, &facts()).expect("a report");
    let heard: Vec<String> = report
        .findings()
        .iter()
        .filter_map(|finding| match *finding.observation() {
            Observation::Sounding { pitch, .. } => Some(format!("{pitch}")),
            _ => None,
        })
        .collect();
    assert_eq!(heard, vec!["d#4", "eb4"], "the analysis respelled the score");
}

#[test]
fn a_chord_symbol_is_reported_and_not_interpreted() {
    let report = report(&facts());
    let symbol = report
        .findings()
        .iter()
        .find_map(|finding| match *finding.observation() {
            Observation::Written { ref symbol, .. } => Some(symbol.clone()),
            _ => None,
        })
        .expect("the chord symbol");
    assert_eq!(symbol.text(), "am");
    // Nothing anywhere in the report says the notes under it are an A minor
    // triad, because deriving that would be the compiler interpreting a symbol
    // (`crates/musa-compiler/src/harmony.rs`).
    assert!(
        !report
            .findings()
            .iter()
            .any(|finding| finding.code() == "realizes" || finding.code() == "chord-tone"),
        "the analysis read notes out of a chord symbol"
    );
}

#[test]
fn a_rest_is_observed_rather_than_skipped() {
    let report = report(&facts());
    assert!(
        report
            .findings()
            .iter()
            .any(|finding| matches!(*finding.observation(), Observation::Silence { .. })),
        "the written rest is not in the report"
    );
}

#[test]
fn a_chord_is_as_many_findings_as_it_has_tones() {
    let stacked = snapshot(
        "piece \"Stacked\" {\n\
           meter 4/4;\n\
           score { part p { voice v { [c4 e4 g4]/1 } } }\n\
         }",
    );
    let report = analyze(&stacked, &facts()).expect("a report");
    let heard: Vec<String> = report
        .findings()
        .iter()
        .filter_map(|finding| match *finding.observation() {
            Observation::Sounding { pitch, .. } => Some(format!("{pitch}")),
            _ => None,
        })
        .collect();
    assert_eq!(heard, vec!["c4", "e4", "g4"], "a chord was not read as its tones");
}

#[test]
fn every_kind_owes_a_method_and_assumptions() {
    for kind in AnalysisKind::ALL {
        assert!(!kind.method().is_empty(), "{} has no stated method", kind.as_str());
        assert!(
            !kind.assumptions().is_empty(),
            "{} assumes nothing, which is itself a claim",
            kind.as_str()
        );
        assert_eq!(AnalysisKind::named(kind.as_str()), Some(kind));
    }
    assert_eq!(AnalysisKind::named("form"), None, "an unadmitted kind was accepted");
}
