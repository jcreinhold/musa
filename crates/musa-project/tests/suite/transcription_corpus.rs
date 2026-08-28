//! The checked-in transcription corpus, and the machinery every law that
//! reads it shares.
//!
//! One fixture file feeds three surfaces — the rhythm report, the notation
//! proposal, and Review — and each of them needs the same synthesis of raw
//! MIDI from the corpus's intended notes. Written once here so the three
//! cannot drift into disagreeing about what a fixture plays.

#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]

use musa_playback::{MidiClockCalibrator, MidiInputEvent, MidiMessageKind};
use musa_project::{CapturedMidiEvent, MidiPairingFact, ProjectSession, ProposalNote, TakeClock};

pub(crate) const CORPUS: &str = include_str!("../fixtures/transcription/corpus.json");

#[derive(serde::Deserialize)]
pub(crate) struct Corpus {
    pub(crate) fixtures: Vec<Fixture>,
}

#[derive(serde::Deserialize)]
pub(crate) struct Fixture {
    pub(crate) id: String,
    pub(crate) clock: Clock,
    pub(crate) notes: Vec<Note>,
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Clock {
    Known { quarter_micros: u64, origin_micros: u64 },
    Free,
    Unmeasured,
}

#[derive(serde::Deserialize)]
pub(crate) struct Note {
    pub(crate) pitch: u8,
    pub(crate) onset_micros: u64,
    pub(crate) release_micros: u64,
    pub(crate) sounding_end_micros: u64,
    pub(crate) expected: Option<Expected>,
}

#[derive(Clone, Copy, serde::Deserialize)]
pub(crate) struct Expected {
    pub(crate) duration_ticks: u32,
    pub(crate) voice: u8,
    pub(crate) group: u16,
}

pub(crate) fn session() -> ProjectSession {
    ProjectSession::from_text(
        r#"piece "Proposal laws" {
    meter 4/4;
    key c major;
    score { part p { voice v { rest/1 } } }
}
"#,
        "proposal-laws.musa",
    )
}

pub(crate) fn raw(kind: MidiMessageKind, channel: u8, data: u8, value: i16, micros: u64) -> MidiInputEvent {
    MidiInputEvent {
        device_micros: micros,
        callback_micros: micros,
        cable: 0,
        channel,
        kind,
        data,
        value,
    }
}

pub(crate) fn captured(id: u64, kind: MidiMessageKind, data: u8, value: i16, micros: u64) -> CapturedMidiEvent {
    let raw = raw(kind, 0, data, value, micros);
    CapturedMidiEvent {
        id,
        raw,
        project_micros: micros,
        calibration: MidiClockCalibrator::default().calibration(),
        voice: None,
        pairing: MidiPairingFact::NotANote,
    }
}

/// Synthesize the raw MIDI of one corpus fixture's *expected* notes, in a time
/// order the pairing stage admits. Pedal-extended notes get one sustain pedal
/// held from the first press to the last release, so `complete()` reports the
/// key release and the pedal-extended sounding end as separate facts exactly as
/// the corpus does.
pub(crate) fn synthesize(fixture: &Fixture) -> Vec<CapturedMidiEvent> {
    let expected = fixture
        .notes
        .iter()
        .enumerate()
        .filter_map(|(index, note)| note.expected.map(|_| (index, note)))
        .collect::<Vec<_>>();

    let mut events = Vec::new();
    let mut pedal_press: Option<u64> = None;
    let mut pedal_release: u64 = 0;
    for (frame, (_, note)) in expected.iter().enumerate() {
        events.push(captured(
            u64::try_from(frame * 2).unwrap_or(u64::MAX),
            MidiMessageKind::NoteOn,
            note.pitch,
            90,
            note.onset_micros,
        ));
        events.push(captured(
            u64::try_from(frame * 2 + 1).unwrap_or(u64::MAX),
            MidiMessageKind::NoteOff,
            note.pitch,
            0,
            note.release_micros,
        ));
        if note.sounding_end_micros > note.release_micros {
            pedal_press = Some(pedal_press.map_or(note.onset_micros, |p| p.min(note.onset_micros)));
            pedal_release = pedal_release.max(note.sounding_end_micros);
        }
    }
    if let Some(press) = pedal_press {
        events.push(captured(1_000_000, MidiMessageKind::ControlChange, 64, 127, press));
        events.push(captured(
            1_000_001,
            MidiMessageKind::ControlChange,
            64,
            0,
            pedal_release,
        ));
    }
    events.sort_by_key(|event| (event.project_micros, event.id));
    events
}

pub(crate) fn clock_of(clock: &Clock) -> TakeClock {
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

/// Map a proposal note back to its corpus fixture note via the derivation.
pub(crate) fn frame_of(note: &ProposalNote) -> usize {
    usize::try_from(note.derivation[0] / 2).unwrap_or(usize::MAX)
}
