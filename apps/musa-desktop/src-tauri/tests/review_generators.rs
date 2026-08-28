//! Generates the readings the Review screen is photographed against.
//!
//! The screen fixtures are the project's *real* readings of prompt 203's
//! measured corpus, not hand-written mocks: one clean take that asks nothing,
//! and one take per ambiguity class the corpus actually exhibited. A control
//! for a question the corpus never asked would have no fixture to appear in,
//! which is the point.
//!
//! Run `UPDATE_UI_FIXTURES=1 cargo test -p musa-desktop` to refresh.

#![allow(clippy::expect_used)]
#![allow(clippy::arithmetic_side_effects)]

use std::path::{Path, PathBuf};

use musa_desktop::dto::ReviewFactsDto;
use musa_playback::{MidiClockCalibrator, MidiInputEvent, MidiMessageKind};
use musa_project::{CapturedMidiEvent, MidiPairingFact, ProjectSession, TakeClock};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

/// The same corpus prompt 203 measured and `musa-project`'s review laws read.
const CORPUS: &str = include_str!("../../../../crates/musa-project/tests/fixtures/transcription/corpus.json");

/// The fixtures the screen needs: the clean case first, then one take per
/// ambiguity class those takes raise.
const WANTED: [&str; 5] = [
    "straight-known",
    "swing-known",
    "rolled-and-block-chords",
    "crossing-voices",
    "rubato-free",
];

#[derive(serde::Deserialize)]
struct Corpus {
    fixtures: Vec<Fixture>,
}

#[derive(serde::Deserialize)]
struct Fixture {
    id: String,
    clock: Clock,
    notes: Vec<Note>,
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Clock {
    Known { quarter_micros: u64, origin_micros: u64 },
    Free,
    Unmeasured,
}

#[derive(serde::Deserialize)]
struct Note {
    pitch: u8,
    onset_micros: u64,
    release_micros: u64,
    sounding_end_micros: u64,
    expected: Option<serde_json::Value>,
}

fn captured(id: u64, kind: MidiMessageKind, data: u8, value: i16, micros: u64) -> CapturedMidiEvent {
    CapturedMidiEvent {
        id,
        raw: MidiInputEvent {
            device_micros: micros,
            callback_micros: micros,
            cable: 0,
            channel: 0,
            kind,
            data,
            value,
        },
        project_micros: micros,
        calibration: MidiClockCalibrator::default().calibration(),
        voice: None,
        pairing: MidiPairingFact::NotANote,
    }
}

/// The raw MIDI of one fixture's intended notes, in a time order the pairing
/// stage admits. One sustain pedal spans every pedal-extended note, so the
/// take carries key release and sounding end as the separate facts they are.
fn synthesize(fixture: &Fixture) -> Vec<CapturedMidiEvent> {
    let played = fixture.notes.iter().filter(|note| note.expected.is_some());
    let mut events = Vec::new();
    let mut press: Option<u64> = None;
    let mut release: u64 = 0;
    for (frame, note) in played.enumerate() {
        let frame = u64::try_from(frame).unwrap_or(u64::MAX);
        events.push(captured(
            frame * 2,
            MidiMessageKind::NoteOn,
            note.pitch,
            90,
            note.onset_micros,
        ));
        events.push(captured(
            frame * 2 + 1,
            MidiMessageKind::NoteOff,
            note.pitch,
            0,
            note.release_micros,
        ));
        if note.sounding_end_micros > note.release_micros {
            press = Some(press.map_or(note.onset_micros, |at| at.min(note.onset_micros)));
            release = release.max(note.sounding_end_micros);
        }
    }
    if let Some(at) = press {
        events.push(captured(1_000_000, MidiMessageKind::ControlChange, 64, 127, at));
        events.push(captured(1_000_001, MidiMessageKind::ControlChange, 64, 0, release));
    }
    events.sort_by_key(|event| (event.project_micros, event.id));
    events
}

fn clock_of(clock: &Clock) -> TakeClock {
    match *clock {
        Clock::Known {
            quarter_micros,
            origin_micros,
        } => TakeClock::Known {
            quarter_micros,
            origin_micros,
        },
        Clock::Free => TakeClock::Free,
        Clock::Unmeasured => TakeClock::Unmeasured,
    }
}

fn session() -> ProjectSession {
    ProjectSession::from_text(
        "piece \"Review\" {\n    meter 4/4;\n    key c major;\n    score { part p { voice v { rest/1 } } }\n}\n",
        "review.musa",
    )
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/fixtures")
}

#[test]
fn review_fixtures_are_current() -> Result {
    let corpus: Corpus = serde_json::from_str(CORPUS)?;
    let mut readings = serde_json::Map::new();
    for id in WANTED {
        let fixture = corpus
            .fixtures
            .iter()
            .find(|fixture| fixture.id == id)
            .ok_or_else(|| format!("the corpus has no `{id}` fixture"))?;
        let mut session = session();
        let facts = session
            .begin_review(
                &fixture.id,
                &synthesize(fixture),
                clock_of(&fixture.clock),
                96,
                "4/4",
                None,
                "standard",
            )
            .map_err(|error| format!("`{id}` did not compose: {error:?}"))?;
        readings.insert(id.to_owned(), serde_json::to_value(ReviewFactsDto::from(&facts))?);
    }

    // The clean fixture is the one-click case and the others each ask
    // something. Committing a "clean" reading that in fact asked a question —
    // or an "ambiguous" one that asked nothing — would make the screenshots
    // claim a progressive disclosure that is not there.
    let asks = |id: &str| -> usize {
        readings
            .get(id)
            .and_then(|reading| reading.get("ambiguities"))
            .and_then(serde_json::Value::as_array)
            .map_or(0, std::vec::Vec::len)
    };
    assert_eq!(asks("straight-known"), 0, "the clean take must ask nothing");
    for id in WANTED.iter().skip(1) {
        assert!(asks(id) > 0, "`{id}` is here because it asks something");
    }

    let mut json = serde_json::to_string_pretty(&serde_json::Value::Object(readings))?;
    json.push('\n');

    let path = fixtures_dir().join("reviews.json");
    let current = std::fs::read_to_string(&path).ok();
    if current.as_deref() == Some(&json) {
        return Ok(());
    }
    if current.is_some() && std::env::var_os("UPDATE_UI_FIXTURES").is_none() {
        return Err(format!(
            "{} is stale — rerun with UPDATE_UI_FIXTURES=1 and commit the result",
            path.display()
        )
        .into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, json)?;
    Ok(())
}
