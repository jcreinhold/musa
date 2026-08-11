//! What `ProjectSession::analyze` promises its callers (docs/prompts/117).
//!
//! Two things, and they are the whole reason the operation exists at this
//! level rather than as a compiler pass. It **reads**: no revision, no
//! diagnostic, no edit. And it **resolves**: the report leaves the compiler as
//! part ids and byte offsets, and reaches a caller as names, bars, beats,
//! lines, and sentences, because `docs/interface/03-interaction.md` §7 does not
//! let a frontend compute any of those.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::arithmetic_side_effects)]
// This suite asks whether a finding has one particular evidence shape; "which
// of these are events" is not a place a new shape needs considering.
#![allow(clippy::wildcard_enum_match_arm)]

use musa_project::{
    AnalysisKind, AnalysisRequest, AnalysisScope, EvidenceFacts, MusicalTime, ProjectCommand, ProjectError,
    ProjectSession,
};

type Result = std::result::Result<(), ProjectError>;

const PIECE: &str = concat!(
    "piece \"Observed\" {\n",
    "    tempo quarter = 120;\n",
    "    meter 4/4;\n",
    "    key c major;\n\n",
    "    motif figure() { g4/4 a4/4 }\n\n",
    "    score {\n",
    "        harmony {\n",
    "            at 1:1 c;\n",
    "        }\n",
    "        part piano {\n",
    "            voice upper {\n",
    "                c4/4\n",
    "                e4/4\n",
    "                use figure();\n",
    "            }\n",
    "        }\n",
    "    }\n",
    "}\n",
);

fn session() -> ProjectSession {
    let session = ProjectSession::from_text(PIECE, "observed.musa");
    assert!(session.snapshot().compiles(), "fixture must compile");
    session
}

fn facts() -> AnalysisRequest {
    AnalysisRequest::new(AnalysisKind::Facts)
}

#[test]
fn analyzing_mints_no_revision_and_raises_no_diagnostic() -> Result {
    let session = session();
    let before = session.snapshot();
    let (revision, source, problems) = (
        before.revision(),
        before.source().to_owned(),
        before.diagnostics().len(),
    );

    let _report = session.analyze(&facts())?;

    let after = session.snapshot();
    assert_eq!(after.revision(), revision, "an analysis minted a revision");
    assert_eq!(after.source(), source, "an analysis changed the source");
    assert_eq!(after.diagnostics().len(), problems, "an analysis raised a diagnostic");
    Ok(())
}

#[test]
fn a_report_is_the_same_every_time() -> Result {
    let session = session();
    assert_eq!(session.analyze(&facts())?, session.analyze(&facts())?);
    assert_eq!(
        session.analyze(&facts())?.to_json(),
        session.analyze(&facts())?.to_json(),
        "two renderings of one report disagreed"
    );
    Ok(())
}

#[test]
fn findings_arrive_resolved_rather_than_as_ids() -> Result {
    let report = session().analyze(&facts())?;
    let sounding = report
        .findings
        .iter()
        .find(|finding| finding.code == "sounding-pitch")
        .expect("a sounding pitch");

    assert_eq!(sounding.standing, "fact");
    assert_eq!(sounding.bar, 1, "the first note is not in the first bar");
    assert!(
        sounding.summary.starts_with("C4 sounds"),
        "unresolved summary: {}",
        sounding.summary
    );
    match sounding.evidence {
        EvidenceFacts::Event {
            ref part,
            ref voice,
            line,
            ..
        } => {
            assert_eq!((part.as_str(), voice.as_str()), ("piano", "upper"));
            assert!(line > 0, "an event with no line");
        }
        ref other => panic!("a note without event evidence: {other:?}"),
    }
    Ok(())
}

#[test]
fn a_generated_note_cites_the_line_it_is_written_on() -> Result {
    let report = session().analyze(&facts())?;
    let generated = report
        .findings
        .iter()
        .find(|finding| finding.summary.starts_with("G4 sounds"))
        .expect("the motif's first note");
    let motif_line = PIECE
        .lines()
        .position(|line| line.contains("motif figure()"))
        .map_or(0, |index| u32::try_from(index).unwrap_or(0) + 1);
    match generated.evidence {
        EvidenceFacts::Event { line, .. } => assert_eq!(
            line, motif_line,
            "a generated note cited the use site instead of the motif"
        ),
        ref other => panic!("a note without event evidence: {other:?}"),
    }
    Ok(())
}

#[test]
fn a_value_in_force_has_no_place_in_the_file() -> Result {
    let report = session().analyze(&facts())?;
    let key = report
        .findings
        .iter()
        .find(|finding| finding.code == "key-in-force")
        .expect("the key");
    assert!(
        matches!(key.evidence, EvidenceFacts::InForce),
        "a key in force was given a span"
    );
    assert!(key.summary.contains("C major"), "unresolved key: {}", key.summary);
    Ok(())
}

#[test]
fn a_request_that_names_nothing_is_an_error_and_not_an_empty_report() {
    let error = session()
        .analyze(&facts().scoped(AnalysisScope::Part("strings".to_owned())))
        .expect_err("an error");
    assert!(
        matches!(error, ProjectError::Analysis(_)),
        "wrong error for a bad request: {error}"
    );
    assert!(error.to_string().contains("piano"), "unhelpful: {error}");
}

#[test]
fn a_window_narrows_the_report() -> Result {
    let session = session();
    let whole = session.analyze(&facts())?;
    let opening = session.analyze(&facts().within(MusicalTime::ZERO, MusicalTime::parse("1/4").expect("a time")))?;
    assert!(
        opening.findings.len() < whole.findings.len(),
        "a quarter-note window reported the whole piece"
    );
    assert!(
        opening.findings.iter().all(|finding| finding.bar == 1),
        "the window let a later bar through"
    );
    Ok(())
}

#[test]
fn a_piece_that_never_compiled_has_nothing_to_observe() {
    let mut session = ProjectSession::from_text("piece \"Broken\" {", "broken.musa");
    let error = session.analyze(&facts()).expect_err("an error");
    assert!(matches!(error, ProjectError::NoValidScore), "wrong error: {error}");
    // And an edit that fixes it makes the same request answerable, so the
    // refusal was about the score and not about the request.
    let _update = session.apply(ProjectCommand::SetSource(PIECE.to_owned()));
    assert!(session.analyze(&facts()).is_ok(), "the fixed piece is still unreadable");
}
