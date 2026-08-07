//! The contracts `ProjectSession` promises its callers.

use musa_project::{ExportRequest, ProjectCommand, ProjectError, ProjectSession, Span, TextEdit, Validity};

type Result = std::result::Result<(), ProjectError>;

/// A piece small enough that compiling and rendering it is free.
const PIECE: &str = concat!(
    "piece \"Test\" {\n",
    "    tempo quarter = 120;\n",
    "    meter 4/4;\n",
    "    key c major;\n\n",
    "    score {\n",
    "        part piano {\n",
    "            voice upper {\n",
    "                c4 1/4;\n",
    "                e4 1/4;\n",
    "            }\n",
    "        }\n",
    "    }\n",
    "}\n",
);

fn session() -> ProjectSession {
    let session = ProjectSession::from_text(PIECE, "test.musa");
    assert!(session.snapshot().compiles(), "fixture must compile");
    session
}

/// Undo returns the exact previous text; redo returns the exact next one.
#[test]
fn undo_redo_restores_each_state_exactly() -> Result {
    let mut session = session();
    let original = session.snapshot().source().to_owned();
    let edited = original.replace("e4 1/4;", "g4 1/4;");

    session.apply(ProjectCommand::SetSource(edited.clone()))?;
    assert_eq!(session.snapshot().source(), edited);

    session.undo()?;
    assert_eq!(session.snapshot().source(), original);

    session.redo()?;
    assert_eq!(session.snapshot().source(), edited);
    Ok(())
}

/// A revision names a state, so returning to a state returns its revision.
#[test]
fn undo_returns_to_the_earlier_revision_rather_than_minting_one() -> Result {
    let mut session = session();
    let first = session.snapshot().revision();

    session.apply(ProjectCommand::SetSource(PIECE.replace("c4", "d4")))?;
    let second = session.snapshot().revision();
    assert!(second > first);

    session.undo()?;
    assert_eq!(session.snapshot().revision(), first);
    session.redo()?;
    assert_eq!(session.snapshot().revision(), second);
    Ok(())
}

/// An edit made after an undo abandons the redo branch.
#[test]
fn editing_after_undo_drops_the_redo_branch() -> Result {
    let mut session = session();
    session.apply(ProjectCommand::SetSource(PIECE.replace("c4", "d4")))?;
    session.undo()?;
    session.apply(ProjectCommand::SetSource(PIECE.replace("c4", "f4")))?;

    assert!(
        matches!(session.redo(), Err(ProjectError::NothingTo("redo"))),
        "the redo branch must be gone"
    );
    Ok(())
}

/// The headline of roadmap §14.7: a source that stops compiling does not take
/// the score with it.
#[test]
fn invalid_source_keeps_the_last_valid_score_and_says_so() -> Result {
    let mut session = session();
    let good_revision = session.snapshot().revision();
    let good_mei = session.snapshot().mei().unwrap_or_default().to_owned();
    assert!(!good_mei.is_empty(), "the fixture must engrave");

    let update = session.apply(ProjectCommand::SetSource("piece \"Test\" { score { part".into()))?;

    assert_eq!(update.validity, Validity::Stale);
    let snapshot = session.snapshot();
    assert!(!snapshot.compiles());
    assert!(!snapshot.diagnostics().is_empty());
    assert_eq!(snapshot.mei(), Some(good_mei.as_str()));
    assert_eq!(snapshot.score_revision(), Some(good_revision));
    assert!(snapshot.revision() > good_revision);

    // And the score stays exportable throughout.
    session.export(ExportRequest::Mei)?;
    Ok(())
}

/// Setting the source to what it already is changes nothing.
#[test]
fn setting_identical_source_is_not_an_edit() -> Result {
    let mut session = session();
    let before = session.snapshot().revision();
    let update = session.apply(ProjectCommand::SetSource(PIECE.into()))?;

    assert!(!update.source_changed);
    assert_eq!(session.snapshot().revision(), before);
    assert!(session.undo().is_err(), "no state was recorded");
    Ok(())
}

/// Text edits and whole-source replacement reach the same state.
#[test]
fn applying_edits_agrees_with_setting_the_equivalent_source() -> Result {
    let start = u32::try_from(PIECE.find("c4").unwrap_or_default()).unwrap_or_default();
    assert!(start > 0, "the fixture must contain c4");
    let edit = TextEdit::new(
        Span {
            start,
            end: start.saturating_add(2),
        },
        "d4",
    );

    let mut edited = session();
    edited.apply(ProjectCommand::ApplyEdits(vec![edit]))?;

    let mut replaced = session();
    replaced.apply(ProjectCommand::SetSource(PIECE.replacen("c4", "d4", 1)))?;

    assert_eq!(edited.snapshot().source(), replaced.snapshot().source());
    assert_eq!(edited.snapshot().mei(), replaced.snapshot().mei());
    Ok(())
}

/// The offline render is deterministic — the same session exports the same
/// bytes every time (roadmap §13.8).
#[test]
fn wav_export_is_deterministic() -> Result {
    let session = session();
    let first = session.export(ExportRequest::Wav)?;
    let second = session.export(ExportRequest::Wav)?;
    assert_eq!(first, second);
    assert!(first.as_bytes().starts_with(b"RIFF"));
    Ok(())
}

/// A 64-bit FNV-1a digest. Enough to pin a byte stream in a test without
/// taking a hashing dependency for one assertion.
fn digest(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The golden-audio guard for prompt 28's neutrality claim.
///
/// A piece that declares no profile must render the audio it rendered before
/// interpretation existed. Determinism alone cannot catch that: a gate applied
/// where none was written is perfectly deterministic and perfectly wrong. The
/// digest is the missing oracle — if a future change to the performance layer
/// touches an unprofiled piece by even one sample, this fails.
///
/// It has been re-pinned twice, both times for a change to the *instrument*
/// rather than to interpretation, and both times verified against the
/// previous build first.
///
/// Prompt 30 replaced the placeholder ramp with a real ADSR: the old ramp
/// compared an accumulated float against 1.0 and so spent 241 frames on a
/// 240-frame attack. 26 samples out of 576,000 changed, none by more than one
/// ulp.
///
/// Prompt 31 put the master limiter on the default instrument. This fixture
/// peaked at 3.80 — 190,000 of its samples were above full scale — so the
/// limiter engages, and most samples move. That is the point of it: sixteen
/// voices summing past 0 dBFS is a mix decision nobody made, and a file that
/// records it is a file that clips on anything that plays it. What the digest
/// still guards is unchanged: nothing here asks for a profile, and nothing in
/// the performance layer may act as though it did.
#[test]
fn an_unprofiled_piece_renders_the_golden_audio() -> Result {
    const GOLDEN: u64 = 0x1426_af4f_74c0_0489;
    let bytes = session().export(ExportRequest::Wav)?;
    assert_eq!(
        digest(bytes.as_bytes()),
        GOLDEN,
        "interpretation reached a piece that asked for none"
    );
    Ok(())
}

/// Every export target produces something from a compiled score.
#[test]
fn every_export_target_produces_an_artifact() -> Result {
    let session = session();
    for request in [
        ExportRequest::Mei,
        ExportRequest::LilyPond,
        ExportRequest::MusicXml,
        ExportRequest::Midi(musa_project::MidiMode::Score),
        ExportRequest::Midi(musa_project::MidiMode::Performance),
        ExportRequest::PerformanceDump,
        ExportRequest::NotationPlanDump,
    ] {
        let artifact = session.export(request)?;
        assert!(!artifact.as_bytes().is_empty(), "{request:?} produced nothing");
    }
    Ok(())
}

/// `MusicXML` leaves as a `score-partwise` document under the extension the
/// other programs open, and carries none of MEI's event provenance with it
/// (roadmap §12.4).
#[test]
fn musicxml_leaves_as_an_interchange_document() -> Result {
    let text = session()
        .export(ExportRequest::MusicXml)?
        .as_text()
        .unwrap_or_default()
        .to_owned();
    assert!(text.contains("<score-partwise"), "{text}");
    assert!(!text.contains("event-"), "no event ids leave in MusicXML");
    assert_eq!(ExportRequest::MusicXml.extension(), "musicxml");
    Ok(())
}

/// A piece that has never compiled has nothing to export.
#[test]
fn export_without_a_valid_score_is_refused() {
    let session = ProjectSession::from_text("piece \"broken\" { score { part", "broken.musa");
    assert!(!session.snapshot().compiles());
    assert!(matches!(
        session.export(ExportRequest::Mei),
        Err(ProjectError::NoValidScore)
    ));
}

/// Undo at the start of history is refused rather than silently ignored.
#[test]
fn undo_at_the_start_of_history_is_refused() {
    let mut session = session();
    assert!(matches!(session.undo(), Err(ProjectError::NothingTo("undo"))));
}

/// Sounding time agrees with written time and with the sample rate the
/// engine reports positions in. At 120 bpm a quarter note is half a second,
/// and the interface's playhead is only as truthful as this.
#[test]
fn event_frames_are_the_engine_s_clock() {
    let session = session();
    let snapshot = session.snapshot();
    let rate = u64::from(snapshot.playback().sample_rate);
    let events = snapshot.score().map(|score| score.events.clone()).unwrap_or_default();
    assert_eq!(events.len(), 2, "the fixture is two quarter notes");

    let first = events.first();
    let second = events.get(1);
    assert_eq!(
        first.map(|event| event.onset_frames),
        Some(0),
        "the first note starts at the start"
    );
    assert_eq!(
        first.map(|event| event.end_frames),
        rate.checked_div(2),
        "a quarter at 120 bpm lasts half a second"
    );
    assert_eq!(
        second.map(|event| event.onset_frames),
        first.map(|event| event.end_frames),
        "the second follows the first"
    );
    assert_eq!(
        second.map(|event| event.end_frames > event.onset_frames),
        Some(true),
        "an event lasts a positive time"
    );
}
