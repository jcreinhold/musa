//! Bounded expressive MIDI evidence and the temporary legacy note-entry path.
//!
//! MIDI is edge evidence, not written music. This module preserves fixed raw
//! events, calibrates their clock, pairs note lifecycles, and freezes bounded
//! memory-only takes without editing source. Audition events retain device
//! dimensions for the checked source instrument to interpret.
//!
//! [`EntryBuffer`] and pitch spelling remain only until prompt 209 removes the
//! superseded step-entry workflow. New capture code must not depend on them.

// Every arithmetic expression below is over small integers with known
// bounds: a note number is 0–127, a natural is 0–11, an alteration is −2–2,
// and a signature is −7–7 fifths, all held in `i32`. No sum, difference, or
// product of those can leave `i32`, and the only division is by the constant
// twelve, so there is nothing here for a checked operation to report.
#![allow(clippy::arithmetic_side_effects)]

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use musa_playback::{
    AuditionEvent, AuditionInputKind, CalibratedMidiEvent, MidiClockCalibration, MidiClockCalibrator, MidiInputEvent,
    MidiInputLosses, MidiMessageKind,
};
use musa_score::{Key, Letter};

/// How close two presses must be to be one chord.
///
/// Long enough that a hand landing on a triad is never split — the spread of
/// a deliberate chord is a few milliseconds — and short enough that a fast
/// scale is never joined: at 40 ms a run would have to pass 1 500 notes per
/// minute before two of its notes collided.
const CHORD_WINDOW: Duration = Duration::from_millis(40);

/// Recent phrase memory: thirty seconds at ordinary performance density.
pub(crate) const RECENT_MIDI_MICROS: u64 = 30_000_000;
/// Event bound paired with [`RECENT_MIDI_MICROS`].
pub(crate) const RECENT_MIDI_EVENTS: usize = 4_096;
/// Explicit capture time bound.
pub(crate) const CAPTURE_MIDI_MICROS: u64 = 600_000_000;
/// Explicit capture event bound.
pub(crate) const CAPTURE_MIDI_EVENTS: usize = 65_536;

/// Stable MIDI device identity exposed by the project facade.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiDeviceFacts {
    pub id: String,
    pub name: String,
    pub selected: bool,
}

/// Deterministic note-pairing observation retained beside a raw event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MidiPairingFact {
    Matched,
    RepeatedAttack,
    MissingAttack,
    RepetitionLimit,
    NotANote,
}

/// One immutable event in recent memory or a frozen take.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapturedMidiEvent {
    pub id: u64,
    pub raw: MidiInputEvent,
    pub project_micros: u64,
    pub calibration: MidiClockCalibration,
    pub voice: Option<u32>,
    pub pairing: MidiPairingFact,
}

/// Project and performance context fixed at take start.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiTakeContext {
    pub revision: u64,
    pub part: String,
    pub voice: Option<String>,
    pub transport_playing: bool,
    pub transport_frame: u64,
    pub sample_rate: u32,
    pub tempo: Option<crate::Fraction>,
    pub meter: Option<(u32, u32)>,
    pub device_id: String,
    pub device_name: String,
    pub audition_instrument: String,
}

/// One immutable memory-only MIDI take.
#[derive(Clone, Debug)]
pub struct MidiTake {
    context: MidiTakeContext,
    events: Arc<[CapturedMidiEvent]>,
    losses: MidiInputLosses,
    calibration_at_start: MidiClockCalibration,
    calibration_at_end: MidiClockCalibration,
    connection_discontinuities: u32,
    held_notes_at_end: u16,
    overflowed: bool,
}

impl MidiTake {
    pub fn context(&self) -> &MidiTakeContext {
        &self.context
    }

    pub fn events(&self) -> &[CapturedMidiEvent] {
        &self.events
    }

    pub const fn losses(&self) -> MidiInputLosses {
        self.losses
    }

    pub const fn calibration_at_start(&self) -> MidiClockCalibration {
        self.calibration_at_start
    }

    pub const fn calibration_at_end(&self) -> MidiClockCalibration {
        self.calibration_at_end
    }

    pub const fn connection_discontinuities(&self) -> u32 {
        self.connection_discontinuities
    }

    pub const fn held_notes_at_end(&self) -> u16 {
        self.held_notes_at_end
    }

    pub const fn overflowed(&self) -> bool {
        self.overflowed
    }
}

/// Frontend-facing bounded capture state; raw takes stay project-owned.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiCaptureFacts {
    pub state: MidiCaptureState,
    pub recent_enabled: bool,
    pub recent_events: usize,
    pub recent_micros: u64,
    pub recent_truncated: bool,
    pub capture_events: usize,
    pub capture_micros: u64,
    pub recent_event_limit: usize,
    pub recent_time_limit_micros: u64,
    pub capture_event_limit: usize,
    pub capture_time_limit_micros: u64,
    pub unsupported_audition_events: u64,
    pub losses: MidiLossFacts,
}

/// Serializable cumulative callback-loss counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiLossFacts {
    pub queue_overflow: u64,
    pub refused_sysex: u64,
    pub malformed: u64,
    pub unsupported_system: u64,
}

impl From<MidiInputLosses> for MidiLossFacts {
    fn from(losses: MidiInputLosses) -> Self {
        Self {
            queue_overflow: losses.queue_overflow,
            refused_sysex: losses.refused_sysex,
            malformed: losses.malformed,
            unsupported_system: losses.unsupported_system,
        }
    }
}

/// Listen/capture/review state owned by the project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MidiCaptureState {
    Listen,
    Capturing,
    Review,
    Disconnected,
}

#[derive(Clone, Copy)]
struct NoteStack {
    voices: [u32; 8],
    len: u8,
}

impl NoteStack {
    const EMPTY: Self = Self { voices: [0; 8], len: 0 };

    fn push(&mut self, voice: u32) -> MidiPairingFact {
        let fact = if self.len == 0 {
            MidiPairingFact::Matched
        } else {
            MidiPairingFact::RepeatedAttack
        };
        if usize::from(self.len) == self.voices.len() {
            self.voices.rotate_left(1);
            if let Some(last) = self.voices.last_mut() {
                *last = voice;
            }
            MidiPairingFact::RepetitionLimit
        } else if let Some(slot) = self.voices.get_mut(usize::from(self.len)) {
            *slot = voice;
            self.len = self.len.saturating_add(1);
            fact
        } else {
            MidiPairingFact::RepetitionLimit
        }
    }

    fn pop_oldest(&mut self) -> Option<u32> {
        if self.len == 0 {
            return None;
        }
        let voice = self.voices[0];
        let len = usize::from(self.len);
        if let Some(active) = self.voices.get_mut(..len) {
            active.rotate_left(1);
        }
        self.len = self.len.saturating_sub(1);
        Some(voice)
    }
}

struct ActiveCapture {
    context: MidiTakeContext,
    events: Vec<CapturedMidiEvent>,
    started_micros: u64,
    losses_at_start: MidiInputLosses,
    calibration_at_start: MidiClockCalibration,
    connection_discontinuities: u32,
    overflowed: bool,
}

/// Control-side pairing, clocking, recent memory, and finite takes.
pub(crate) struct MidiPerformanceBuffer {
    clock: MidiClockCalibrator,
    connection_project_micros: u64,
    next_event: u64,
    next_voice: u32,
    notes: Box<[NoteStack]>,
    recent: VecDeque<CapturedMidiEvent>,
    recent_starts: VecDeque<u64>,
    sustain: [bool; 16],
    recent_enabled: bool,
    recent_truncated: bool,
    active: Option<ActiveCapture>,
    takes: Vec<MidiTake>,
    unsupported_audition_events: u64,
    last_losses: MidiInputLosses,
    disconnected: bool,
}

impl std::fmt::Debug for MidiPerformanceBuffer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MidiPerformanceBuffer")
            .field("facts", &self.facts())
            .finish()
    }
}

impl Default for MidiPerformanceBuffer {
    fn default() -> Self {
        Self {
            clock: MidiClockCalibrator::default(),
            connection_project_micros: 0,
            next_event: 0,
            next_voice: 0,
            notes: vec![NoteStack::EMPTY; 16 * 128].into_boxed_slice(),
            recent: VecDeque::with_capacity(RECENT_MIDI_EVENTS),
            recent_starts: VecDeque::with_capacity(RECENT_MIDI_EVENTS),
            sustain: [false; 16],
            recent_enabled: true,
            recent_truncated: false,
            active: None,
            takes: Vec::new(),
            unsupported_audition_events: 0,
            last_losses: MidiInputLosses::default(),
            disconnected: false,
        }
    }
}

impl MidiPerformanceBuffer {
    pub(crate) fn begin_connection(&mut self, project_micros: u64) {
        self.clock = MidiClockCalibrator::default();
        self.connection_project_micros = project_micros;
        self.notes.fill(NoteStack::EMPTY);
        self.sustain.fill(false);
        self.disconnected = false;
    }

    pub(crate) fn ingest(&mut self, raw: MidiInputEvent) -> (CapturedMidiEvent, Option<AuditionEvent>) {
        let CalibratedMidiEvent {
            calibrated_micros,
            calibration,
            ..
        } = self.clock.observe(raw);
        let project_micros = self.connection_project_micros.saturating_add(calibrated_micros);
        let quiet_before = self.quiet();
        let (voice, pairing, audition) = self.pair_and_audition(raw);
        if raw.kind == MidiMessageKind::ControlChange
            && raw.data == 64
            && let Some(sustain) = self.sustain.get_mut(usize::from(raw.channel))
        {
            *sustain = raw.value >= 64;
        }
        let starts_phrase = quiet_before && !self.quiet();
        let captured = CapturedMidiEvent {
            id: self.next_event,
            raw,
            project_micros,
            calibration,
            voice,
            pairing,
        };
        self.next_event = self.next_event.saturating_add(1);
        if self.recent_enabled {
            // Evict before push so the preallocated deque never grows to a
            // second allocation when it reaches its event bound.
            self.trim_recent(project_micros);
            if starts_phrase {
                self.recent_starts.push_back(captured.id);
                self.recent_truncated = false;
            }
            self.recent.push_back(captured);
        }
        if let Some(active) = &mut self.active {
            let too_late = project_micros.saturating_sub(active.started_micros) > CAPTURE_MIDI_MICROS;
            if active.events.len() < CAPTURE_MIDI_EVENTS && !too_late {
                active.events.push(captured);
            } else {
                active.overflowed = true;
            }
        }
        (captured, audition)
    }

    fn trim_recent(&mut self, project_micros: u64) {
        while self.recent.len() >= RECENT_MIDI_EVENTS
            || self
                .recent
                .front()
                .is_some_and(|first| first.project_micros.saturating_add(RECENT_MIDI_MICROS) < project_micros)
        {
            let front = self.recent.front().map(|event| event.id);
            let next_start = front.and_then(|front| self.recent_starts.iter().copied().find(|id| *id > front));
            if let Some(next_start) = next_start {
                while self.recent.front().is_some_and(|event| event.id < next_start) {
                    self.recent.pop_front();
                }
            } else {
                // One uninterrupted phrase exceeded a hard bound. Preserve a
                // bounded suffix for inspection, but refuse to call it whole.
                self.recent.pop_front();
                self.recent_truncated = true;
            }
            let first = self.recent.front().map(|event| event.id);
            while self
                .recent_starts
                .front()
                .is_some_and(|boundary| first.is_none_or(|first| *boundary < first))
            {
                self.recent_starts.pop_front();
            }
        }
    }

    fn pair_and_audition(&mut self, raw: MidiInputEvent) -> (Option<u32>, MidiPairingFact, Option<AuditionEvent>) {
        let slot = usize::from(raw.channel) * 128 + usize::from(raw.data);
        let Some(notes) = self.notes.get_mut(slot) else {
            return (None, MidiPairingFact::NotANote, None);
        };
        match raw.kind {
            MidiMessageKind::NoteOn => {
                let voice = self.next_voice;
                self.next_voice = self.next_voice.wrapping_add(1);
                let pairing = notes.push(voice);
                (
                    Some(voice),
                    pairing,
                    Some(AuditionEvent::NoteOn {
                        voice,
                        note: raw.data,
                        velocity: u8::try_from(raw.value).unwrap_or(0),
                    }),
                )
            }
            MidiMessageKind::NoteOff => {
                let voice = notes.pop_oldest();
                (
                    voice,
                    if voice.is_some() {
                        MidiPairingFact::Matched
                    } else {
                        MidiPairingFact::MissingAttack
                    },
                    voice.map(|voice| AuditionEvent::NoteOff {
                        voice,
                        velocity: u8::try_from(raw.value).unwrap_or(0),
                    }),
                )
            }
            MidiMessageKind::ControlChange => (
                None,
                MidiPairingFact::NotANote,
                Some(AuditionEvent::Input {
                    input: match raw.data {
                        64 => AuditionInputKind::SustainPedal,
                        66 => AuditionInputKind::SostenutoPedal,
                        67 => AuditionInputKind::SoftPedal,
                        controller => AuditionInputKind::Controller(controller),
                    },
                    value: raw.value,
                    key: None,
                }),
            ),
            MidiMessageKind::PitchBend => (
                None,
                MidiPairingFact::NotANote,
                Some(AuditionEvent::Input {
                    input: AuditionInputKind::PitchBend,
                    value: raw.value,
                    key: None,
                }),
            ),
            MidiMessageKind::ChannelPressure => (
                None,
                MidiPairingFact::NotANote,
                Some(AuditionEvent::Input {
                    input: AuditionInputKind::ChannelPressure,
                    value: raw.value,
                    key: None,
                }),
            ),
            MidiMessageKind::KeyPressure => (
                None,
                MidiPairingFact::NotANote,
                Some(AuditionEvent::Input {
                    input: AuditionInputKind::KeyPressure,
                    value: raw.value,
                    key: Some(raw.data),
                }),
            ),
            MidiMessageKind::ProgramChange => (None, MidiPairingFact::NotANote, None),
        }
    }

    pub(crate) fn start_capture(&mut self, context: MidiTakeContext, losses: MidiInputLosses) -> bool {
        if self.active.is_some() {
            return false;
        }
        let started_micros = self
            .recent
            .back()
            .map_or(self.connection_project_micros, |event| event.project_micros);
        self.active = Some(ActiveCapture {
            context,
            events: Vec::with_capacity(CAPTURE_MIDI_EVENTS),
            started_micros,
            losses_at_start: losses,
            calibration_at_start: self.clock.calibration(),
            connection_discontinuities: 0,
            overflowed: false,
        });
        true
    }

    pub(crate) fn stop_capture(&mut self, losses: MidiInputLosses) -> Option<&MidiTake> {
        let active = self.active.take()?;
        let take = self.freeze(
            active.context,
            active.events,
            active.losses_at_start,
            losses,
            active.calibration_at_start,
            active.connection_discontinuities,
            active.overflowed,
        );
        self.takes.push(take);
        self.takes.last()
    }

    pub(crate) fn keep_recent(&mut self, context: MidiTakeContext, losses: MidiInputLosses) -> Option<&MidiTake> {
        let boundary = *self.recent_starts.back()?;
        let events = self
            .recent
            .iter()
            .skip_while(|event| event.id < boundary)
            .copied()
            .collect::<Vec<_>>();
        if events.is_empty() {
            return None;
        }
        let calibration_at_start = events
            .first()
            .map_or_else(|| self.clock.calibration(), |event| event.calibration);
        let take = self.freeze(context, events, losses, losses, calibration_at_start, 0, false);
        self.takes.push(take);
        self.takes.last()
    }

    fn freeze(
        &self,
        context: MidiTakeContext,
        events: Vec<CapturedMidiEvent>,
        before: MidiInputLosses,
        after: MidiInputLosses,
        calibration_at_start: MidiClockCalibration,
        connection_discontinuities: u32,
        overflowed: bool,
    ) -> MidiTake {
        MidiTake {
            context,
            calibration_at_start,
            calibration_at_end: events
                .last()
                .map_or_else(|| self.clock.calibration(), |event| event.calibration),
            connection_discontinuities,
            held_notes_at_end: self.held_notes(),
            events: events.into(),
            losses: loss_delta(before, after),
            overflowed,
        }
    }

    pub(crate) fn clear_recent(&mut self) {
        self.recent.clear();
        self.recent_starts.clear();
        self.recent_truncated = false;
    }

    pub(crate) fn set_recent_enabled(&mut self, enabled: bool) {
        self.recent_enabled = enabled;
        if !enabled {
            self.clear_recent();
        }
    }

    pub(crate) fn mark_disconnected(&mut self) {
        self.disconnected = true;
        if let Some(active) = &mut self.active {
            active.connection_discontinuities = active.connection_discontinuities.saturating_add(1);
        }
    }

    pub(crate) fn release_all(&mut self) -> Vec<AuditionEvent> {
        let mut releases = Vec::new();
        if self.sustain.iter().any(|value| *value) {
            releases.push(AuditionEvent::Input {
                input: AuditionInputKind::SustainPedal,
                value: 0,
                key: None,
            });
            self.sustain.fill(false);
        }
        for notes in &mut self.notes {
            while let Some(voice) = notes.pop_oldest() {
                releases.push(AuditionEvent::NoteOff { voice, velocity: 0 });
            }
        }
        releases
    }

    pub(crate) fn unsupported(&mut self) {
        self.unsupported_audition_events = self.unsupported_audition_events.saturating_add(1);
    }

    pub(crate) fn update_losses(&mut self, losses: MidiInputLosses) {
        self.last_losses = losses;
    }

    pub(crate) fn latest_take(&self) -> Option<&MidiTake> {
        self.takes.last()
    }

    pub(crate) fn facts(&self) -> MidiCaptureFacts {
        let recent_micros = self.recent.front().zip(self.recent.back()).map_or(0, |(first, last)| {
            last.project_micros.saturating_sub(first.project_micros)
        });
        let (capture_events, capture_micros) = self.active.as_ref().map_or((0, 0), |capture| {
            (
                capture.events.len(),
                capture
                    .events
                    .last()
                    .map_or(0, |last| last.project_micros.saturating_sub(capture.started_micros)),
            )
        });
        MidiCaptureFacts {
            state: if self.disconnected {
                MidiCaptureState::Disconnected
            } else if self.active.is_some() {
                MidiCaptureState::Capturing
            } else if !self.takes.is_empty() {
                MidiCaptureState::Review
            } else {
                MidiCaptureState::Listen
            },
            recent_enabled: self.recent_enabled,
            recent_events: self.recent.len(),
            recent_micros,
            recent_truncated: self.recent_truncated,
            capture_events,
            capture_micros,
            recent_event_limit: RECENT_MIDI_EVENTS,
            recent_time_limit_micros: RECENT_MIDI_MICROS,
            capture_event_limit: CAPTURE_MIDI_EVENTS,
            capture_time_limit_micros: CAPTURE_MIDI_MICROS,
            unsupported_audition_events: self.unsupported_audition_events,
            losses: self.last_losses.into(),
        }
    }

    fn held_notes(&self) -> u16 {
        self.notes
            .iter()
            .fold(0u16, |held, notes| held.saturating_add(u16::from(notes.len)))
    }

    fn quiet(&self) -> bool {
        self.held_notes() == 0 && !self.sustain.iter().any(|value| *value)
    }
}

fn loss_delta(before: MidiInputLosses, after: MidiInputLosses) -> MidiInputLosses {
    MidiInputLosses {
        queue_overflow: after.queue_overflow.saturating_sub(before.queue_overflow),
        refused_sysex: after.refused_sysex.saturating_sub(before.refused_sysex),
        malformed: after.malformed.saturating_sub(before.malformed),
        unsupported_system: after.unsupported_system.saturating_sub(before.unsupported_system),
    }
}

/// Notes the keyboard played, grouped as one statement to write.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiEntry {
    /// The written pitches, spelled in the piece's key, lowest first. One
    /// pitch is a note; more than one is a chord.
    pub pitches: Vec<String>,
}

/// Presses waiting to be grouped into chords.
///
/// Kept separate from the clock so the grouping rule can be tested by handing
/// it times rather than by sleeping.
#[derive(Debug, Default)]
pub(crate) struct EntryBuffer {
    /// Presses in arrival order.
    pending: Vec<(u8, Instant)>,
}

impl EntryBuffer {
    /// Record a key going down.
    pub(crate) fn press(&mut self, note: u8, at: Instant) {
        self.pending.push((note, at));
    }

    /// The groups whose window has closed by `now`, oldest first.
    ///
    /// A group is the oldest waiting press plus every press within
    /// [`CHORD_WINDOW`] of it. Nothing is released early: a chord half-played
    /// when the poll happens stays whole and arrives on the next one.
    pub(crate) fn ready(&mut self, now: Instant) -> Vec<Vec<u8>> {
        let mut groups = Vec::new();
        while let Some(&(_, first)) = self.pending.first() {
            let deadline = first.checked_add(CHORD_WINDOW).unwrap_or(first);
            if now < deadline {
                break;
            }
            let taken = self.pending.iter().take_while(|(_, at)| *at <= deadline).count();
            let mut notes: Vec<u8> = self.pending.drain(..taken).map(|(note, _)| note).collect();
            // A held key that retriggers is one note in the chord, not two.
            notes.sort_unstable();
            notes.dedup();
            groups.push(notes);
        }
        groups
    }
}

/// The natural pitch class of each letter, in the language's letter order.
const NATURALS: [(Letter, i32); 7] = [
    (Letter::C, 0),
    (Letter::D, 2),
    (Letter::E, 4),
    (Letter::F, 5),
    (Letter::G, 7),
    (Letter::A, 9),
    (Letter::B, 11),
];

/// The order accidentals are added to a signature: sharps F C G D A E B,
/// flats the same list read backwards.
const SHARP_ORDER: [Letter; 7] = [
    Letter::F,
    Letter::C,
    Letter::G,
    Letter::D,
    Letter::A,
    Letter::E,
    Letter::B,
];

/// Spell a MIDI note number the way the piece would write it.
///
/// The rule, in one sentence: **a note the key already spells is written the
/// key's way, and a note outside the key is written as an alteration of its
/// neighbour — raised in a sharp key, lowered in a flat one.**
///
/// So in G major note 66 is `f#4`, because the key spells F sharp; in F major
/// note 61 is `db4` and in D major it is `c#4`, because the two keys lean
/// opposite ways; and in B major note 65 is `e#4`, because the letter below
/// it is the one the key has. A piece with no `key` declaration is treated as
/// having none in the signature, which leans sharp — the same default every
/// notation program uses.
///
/// This is a heuristic and is meant to be: no rule can know whether a
/// composer meant an augmented fourth or a diminished fifth, and respelling
/// one note is a keystroke away. What it must never do is surprise — hence
/// the table test beside it.
/// The key's seven diatonic spellings, by the pitch class each lands on.
fn diatonic_spelling(alterations: &[i32; 7], wanted: i32) -> Option<(Letter, i32)> {
    NATURALS.iter().enumerate().find_map(|(index, &(letter, natural))| {
        let alter = alterations.get(index).copied().unwrap_or(0);
        ((natural + alter).rem_euclid(12) == wanted).then_some((letter, alter))
    })
}

pub(crate) fn spell(note: u8, key: Option<Key>) -> String {
    let fifths = key.map_or(0, Key::fifths);
    let pitch_class = i32::from(note % 12);
    let alterations = key_alterations(fifths);

    let (letter, alter) = diatonic_spelling(&alterations, pitch_class)
        .or_else(|| {
            // Outside the key: alter the neighbour the signature leans towards.
            if fifths >= 0 {
                diatonic_spelling(&alterations, (pitch_class - 1).rem_euclid(12))
                    .map(|(letter, alter)| (letter, alter + 1))
            } else {
                diatonic_spelling(&alterations, (pitch_class + 1).rem_euclid(12))
                    .map(|(letter, alter)| (letter, alter - 1))
            }
        })
        // Unreachable for any real signature: seven letters a fifth apart
        // cover every pitch class either directly or one step away.
        .unwrap_or((Letter::C, 0));

    rendered(letter, alter, note)
}

/// The enharmonic readings of a chromatic note, excluding the primary
/// [`spell`] choice: the neighbour the signature did *not* lean towards. Empty
/// for a note the key already spells unambiguously.
///
/// The enharmonic readings of a chromatic note, composed by the proposal
/// (205c) so spelling alternatives are a production fact.
pub(crate) fn spell_alternatives(note: u8, key: Option<Key>) -> Vec<String> {
    let fifths = key.map_or(0, Key::fifths);
    let pitch_class = i32::from(note % 12);
    let alterations = key_alterations(fifths);
    if diatonic_spelling(&alterations, pitch_class).is_some() {
        return Vec::new();
    }
    let primary = spell(note, key);
    let below =
        diatonic_spelling(&alterations, (pitch_class - 1).rem_euclid(12)).map(|(letter, alter)| (letter, alter + 1));
    let above =
        diatonic_spelling(&alterations, (pitch_class + 1).rem_euclid(12)).map(|(letter, alter)| (letter, alter - 1));
    [below, above]
        .into_iter()
        .flatten()
        .map(|(letter, alter)| rendered(letter, alter, note))
        .filter(|spelling| *spelling != primary)
        .collect()
}

/// Render a letter, alteration, and sounding note as a written pitch: the
/// written octave is the one that makes the spelling sound at `note`, which is
/// not always the note number's own (`b#3` sounds where `c4` does).
fn rendered(letter: Letter, alter: i32, note: u8) -> String {
    let natural = NATURALS
        .iter()
        .find_map(|&(candidate, natural)| (candidate == letter).then_some(natural))
        .unwrap_or(0);
    let octave = (i32::from(note) - natural - alter).div_euclid(12) - 1;
    format!("{}{}{}", letter_name(letter), accidental(alter), octave.max(0))
}

/// The alteration each letter carries in a signature of `fifths`, indexed
/// like [`NATURALS`].
fn key_alterations(fifths: i8) -> [i32; 7] {
    let mut alterations = [0; 7];
    let count = usize::from(fifths.unsigned_abs()).min(SHARP_ORDER.len());
    let (altered, alter): (Vec<Letter>, i32) = if fifths >= 0 {
        (SHARP_ORDER.iter().copied().take(count).collect(), 1)
    } else {
        (SHARP_ORDER.iter().rev().copied().take(count).collect(), -1)
    };
    for letter in altered {
        if let Some(slot) = NATURALS
            .iter()
            .position(|&(candidate, _)| candidate == letter)
            .and_then(|index| alterations.get_mut(index))
        {
            *slot = alter;
        }
    }
    alterations
}

fn letter_name(letter: Letter) -> char {
    match letter {
        Letter::C => 'c',
        Letter::D => 'd',
        Letter::E => 'e',
        Letter::F => 'f',
        Letter::G => 'g',
        Letter::A => 'a',
        Letter::B => 'b',
    }
}

/// The language's accidental suffix (`#` sharp, `b` flat, doubled once).
fn accidental(alter: i32) -> &'static str {
    match alter {
        2 => "##",
        1 => "#",
        -1 => "b",
        -2 => "bb",
        _ => "",
    }
}

#[cfg(test)]
mod midi_laws {
    use super::*;
    use musa_score::{Accidental, Key, Letter, Mode, PitchClass};
    use std::time::{Duration, Instant};

    fn key(letter: Letter, accidental: i8, mode: Mode) -> Option<Key> {
        Some(Key::new(
            PitchClass {
                letter,
                accidental: Accidental(i32::from(accidental)),
            },
            mode,
        ))
    }

    fn raw(kind: MidiMessageKind, data: u8, value: i16, micros: u64) -> MidiInputEvent {
        MidiInputEvent {
            device_micros: 10_000 + micros,
            callback_micros: micros,
            cable: 0,
            channel: 0,
            kind,
            data,
            value,
        }
    }

    fn context() -> MidiTakeContext {
        MidiTakeContext {
            revision: 7,
            part: "piano".to_owned(),
            voice: Some("upper".to_owned()),
            transport_playing: false,
            transport_frame: 0,
            sample_rate: 48_000,
            tempo: Some(crate::Fraction {
                numerator: 120,
                denominator: 1,
            }),
            meter: Some((4, 4)),
            device_id: "fake-device".to_owned(),
            device_name: "Fake keyboard".to_owned(),
            audition_instrument: "std.sound.basic_sine@1".to_owned(),
        }
    }

    #[test]
    fn repeated_notes_pair_fifo_and_missing_releases_remain_facts() {
        let mut performance = MidiPerformanceBuffer::default();
        let first = performance.ingest(raw(MidiMessageKind::NoteOn, 60, 90, 0)).0;
        let second = performance.ingest(raw(MidiMessageKind::NoteOn, 60, 91, 1_000)).0;
        let first_release = performance.ingest(raw(MidiMessageKind::NoteOff, 60, 4, 2_000)).0;
        let second_release = performance.ingest(raw(MidiMessageKind::NoteOff, 60, 5, 3_000)).0;
        let missing = performance.ingest(raw(MidiMessageKind::NoteOff, 60, 6, 4_000)).0;
        assert_eq!(first.pairing, MidiPairingFact::Matched);
        assert_eq!(second.pairing, MidiPairingFact::RepeatedAttack);
        assert_eq!((first_release.voice, second_release.voice), (first.voice, second.voice));
        assert_eq!((missing.voice, missing.pairing), (None, MidiPairingFact::MissingAttack));
    }

    #[test]
    fn key_release_and_pedal_transition_stay_separate() {
        let mut performance = MidiPerformanceBuffer::default();
        performance.ingest(raw(MidiMessageKind::NoteOn, 60, 80, 0));
        let pedal = performance
            .ingest(raw(MidiMessageKind::ControlChange, 64, 127, 1_000))
            .0;
        let release = performance.ingest(raw(MidiMessageKind::NoteOff, 60, 32, 2_000)).0;
        assert_eq!((pedal.raw.kind, pedal.raw.data), (MidiMessageKind::ControlChange, 64));
        assert_eq!((release.raw.kind, release.raw.value), (MidiMessageKind::NoteOff, 32));
        assert_ne!(pedal.id, release.id);
    }

    #[test]
    fn disconnect_releases_latched_sustain_before_held_voices() {
        let mut performance = MidiPerformanceBuffer::default();
        performance.ingest(raw(MidiMessageKind::NoteOn, 60, 80, 0));
        performance.ingest(raw(MidiMessageKind::ControlChange, 64, 127, 1_000));

        let releases = performance.release_all();

        assert_eq!(
            releases,
            [
                AuditionEvent::Input {
                    input: AuditionInputKind::SustainPedal,
                    value: 0,
                    key: None,
                },
                AuditionEvent::NoteOff { voice: 0, velocity: 0 },
            ]
        );
        assert!(performance.quiet());
    }

    #[test]
    fn keep_that_freezes_the_latest_complete_phrase_start() {
        let mut performance = MidiPerformanceBuffer::default();
        performance.ingest(raw(MidiMessageKind::NoteOn, 60, 80, 0));
        performance.ingest(raw(MidiMessageKind::NoteOff, 60, 0, 1_000));
        let second = performance.ingest(raw(MidiMessageKind::NoteOn, 64, 81, 2_000)).0;
        let release = performance.ingest(raw(MidiMessageKind::NoteOff, 64, 0, 3_000)).0;
        let Some(take) = performance.keep_recent(context(), MidiInputLosses::default()) else {
            std::process::abort()
        };
        assert_eq!(
            take.events().iter().map(|event| event.id).collect::<Vec<_>>(),
            [second.id, release.id]
        );
    }

    #[test]
    fn keep_that_refuses_a_suffix_with_no_observed_start_boundary() {
        let mut performance = MidiPerformanceBuffer::default();
        performance.ingest(raw(MidiMessageKind::NoteOn, 60, 80, 0));
        performance.clear_recent();
        performance.ingest(raw(MidiMessageKind::NoteOff, 60, 0, 1_000));
        performance.ingest(raw(MidiMessageKind::ChannelPressure, 0, 42, 2_000));
        assert!(performance.keep_recent(context(), MidiInputLosses::default()).is_none());
    }

    #[test]
    fn a_take_records_its_start_fit_and_connection_discontinuity() {
        let mut performance = MidiPerformanceBuffer::default();
        assert!(performance.start_capture(context(), MidiInputLosses::default()));
        performance.ingest(raw(MidiMessageKind::NoteOn, 60, 80, 0));
        performance.mark_disconnected();
        performance.begin_connection(10_000);
        performance.ingest(raw(MidiMessageKind::NoteOff, 60, 0, 1_000));
        let Some(take) = performance.stop_capture(MidiInputLosses::default()) else {
            std::process::abort()
        };
        assert_eq!(take.calibration_at_start().samples, 0);
        assert_eq!(take.calibration_at_end().samples, 1);
        assert_eq!(take.connection_discontinuities(), 1);
    }

    #[test]
    fn recent_memory_is_bounded_by_event_count_and_can_be_disabled() {
        let mut performance = MidiPerformanceBuffer::default();
        let capacity = performance.recent.capacity();
        for index in 0..RECENT_MIDI_EVENTS + 7 {
            let micros = u64::try_from(index).unwrap_or(u64::MAX).saturating_mul(1_000);
            performance.ingest(raw(MidiMessageKind::ChannelPressure, 0, 1, micros));
        }
        assert_eq!(performance.facts().recent_events, RECENT_MIDI_EVENTS);
        assert!(performance.facts().recent_truncated);
        assert_eq!(
            performance.recent.capacity(),
            capacity,
            "bounded ring must not grow at wrap"
        );
        performance.set_recent_enabled(false);
        assert_eq!(performance.facts().recent_events, 0);
        assert!(!performance.facts().recent_enabled);
    }

    #[test]
    fn the_published_recent_ring_stays_below_one_mebibyte() {
        let bytes = RECENT_MIDI_EVENTS.saturating_mul(std::mem::size_of::<CapturedMidiEvent>());
        eprintln!(
            "recent MIDI ring: {} bytes/event, {} bytes at bound",
            std::mem::size_of::<CapturedMidiEvent>(),
            bytes
        );
        assert!(bytes <= 1_048_576);
    }

    #[test]
    fn explicit_capture_refuses_growth_past_its_event_bound() {
        let mut performance = MidiPerformanceBuffer::default();
        assert!(performance.start_capture(context(), MidiInputLosses::default()));
        let captured = performance.ingest(raw(MidiMessageKind::ChannelPressure, 0, 1, 0)).0;
        let Some(active) = performance.active.as_mut() else {
            std::process::abort()
        };
        active.events.resize(CAPTURE_MIDI_EVENTS, captured);
        performance.ingest(raw(MidiMessageKind::ChannelPressure, 0, 2, 1_000));
        let Some(take) = performance.stop_capture(MidiInputLosses::default()) else {
            std::process::abort()
        };
        assert_eq!(take.events().len(), CAPTURE_MIDI_EVENTS);
        assert!(take.overflowed());
    }

    /// The spelling table the doc comment promises. Every row is a note a
    /// composer would play and the pitch they would expect to see.
    #[test]
    fn a_note_is_spelled_the_way_its_key_spells_it() {
        let c_major = key(Letter::C, 0, Mode::Major);
        let g_major = key(Letter::G, 0, Mode::Major);
        let f_major = key(Letter::F, 0, Mode::Major);
        let d_major = key(Letter::D, 0, Mode::Major);
        let e_flat = key(Letter::E, -1, Mode::Major);
        let b_major = key(Letter::B, 0, Mode::Major);
        let a_minor = key(Letter::A, 0, Mode::Minor);

        let rows: [(u8, Option<Key>, &str); 14] = [
            // Middle C is middle C in every key.
            (60, c_major, "c4"),
            (60, f_major, "c4"),
            (60, None, "c4"),
            // The key's own accidentals come back as the key writes them.
            (66, g_major, "f#4"),
            (70, f_major, "bb4"),
            (63, e_flat, "eb4"),
            // Outside the key, the signature decides which way to lean.
            (61, c_major, "c#4"),
            (61, d_major, "c#4"),
            (61, f_major, "db4"),
            (63, f_major, "eb4"),
            // A sharp key raises the letter below, even across a semitone
            // where the natural would have been easier to read.
            (65, b_major, "e#4"),
            // A minor key is its relative major's signature.
            (60, a_minor, "c4"),
            (61, a_minor, "c#4"),
            // Octaves follow the note number, an octave to twelve semitones.
            (48, c_major, "c3"),
        ];
        for (note, key, expected) in rows {
            assert_eq!(spell(note, key), expected, "note {note}");
        }
    }

    /// A spelling has to *sound* where the note number says, including when
    /// the letter and the number disagree about the octave (`b#3` is 60).
    #[test]
    fn every_spelling_sounds_at_the_note_it_came_from() {
        for fifths in [-6i8, -3, 0, 2, 5] {
            let key = tonic_for(fifths);
            for note in 24u8..=96 {
                let spelled = spell(note, key);
                assert_eq!(sounds_at(&spelled), Some(note), "{spelled} for note {note}");
            }
        }
    }

    /// A key whose signature has `fifths` accidentals, built by walking the
    /// circle rather than by a table the test shares with the code.
    fn tonic_for(fifths: i8) -> Option<Key> {
        const MAJORS: [(Letter, i8); 13] = [
            (Letter::G, -1),
            (Letter::D, -1),
            (Letter::A, -1),
            (Letter::E, -1),
            (Letter::B, -1),
            (Letter::F, 0),
            (Letter::C, 0),
            (Letter::G, 0),
            (Letter::D, 0),
            (Letter::A, 0),
            (Letter::E, 0),
            (Letter::B, 0),
            (Letter::F, 1),
        ];
        let index = usize::try_from(fifths + 6).ok()?;
        let &(letter, accidental) = MAJORS.get(index)?;
        key(letter, accidental, Mode::Major)
    }

    /// Read a written pitch back as the MIDI note it sounds at.
    fn sounds_at(pitch: &str) -> Option<u8> {
        let mut chars = pitch.chars();
        let natural = match chars.next()? {
            'c' => 0,
            'd' => 2,
            'e' => 4,
            'f' => 5,
            'g' => 7,
            'a' => 9,
            'b' => 11,
            _ => return None,
        };
        let rest: String = chars.collect();
        let (accidental, octave) = rest.split_at(rest.find(|c: char| c.is_ascii_digit())?);
        let alter = match accidental {
            "##" => 2,
            "#" => 1,
            "b" => -1,
            "bb" => -2,
            "" => 0,
            _ => return None,
        };
        let octave: i32 = octave.parse().ok()?;
        u8::try_from((octave + 1) * 12 + natural + alter).ok()
    }

    /// Keys pressed together are one chord; keys pressed apart are two notes.
    #[test]
    fn presses_inside_the_window_are_one_chord() {
        let start = Instant::now();
        let mut buffer = EntryBuffer::default();
        buffer.press(60, start);
        buffer.press(64, start + Duration::from_millis(5));
        buffer.press(67, start + Duration::from_millis(9));
        buffer.press(72, start + Duration::from_millis(400));

        assert_eq!(
            buffer.ready(start + Duration::from_millis(100)),
            vec![vec![60, 64, 67]],
            "the triad is one group and the later note is not in it"
        );
        assert_eq!(buffer.ready(start + Duration::from_millis(500)), vec![vec![72]]);
        assert!(buffer.ready(start + Duration::from_secs(1)).is_empty());
    }

    /// A chord still being played when the poll happens is not cut in half:
    /// it waits for its window to close.
    #[test]
    fn a_chord_is_never_released_early() {
        let start = Instant::now();
        let mut buffer = EntryBuffer::default();
        buffer.press(60, start);
        assert!(buffer.ready(start + Duration::from_millis(10)).is_empty());
        buffer.press(64, start + Duration::from_millis(20));
        assert_eq!(buffer.ready(start + Duration::from_millis(60)), vec![vec![60, 64]]);
    }

    /// A key held down retriggers on some keyboards; the chord has one of it.
    #[test]
    fn a_retriggered_key_is_one_note() {
        let start = Instant::now();
        let mut buffer = EntryBuffer::default();
        buffer.press(60, start);
        buffer.press(60, start + Duration::from_millis(3));
        assert_eq!(buffer.ready(start + Duration::from_millis(60)), vec![vec![60]]);
    }

    /// The wire shape a chord takes, so the frontend has one thing to read.
    #[test]
    fn an_entry_serializes_as_its_pitches() -> Result<(), serde_json::Error> {
        let entry = MidiEntry {
            pitches: vec!["c4".to_owned(), "e4".to_owned()],
        };
        assert_eq!(serde_json::to_string(&entry)?, r#"{"pitches":["c4","e4"]}"#);
        Ok(())
    }
}
