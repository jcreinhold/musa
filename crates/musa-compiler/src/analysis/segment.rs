//! Cutting a score into simultaneities.
//!
//! This is the first interpretive act of any harmonic reading and it is not a
//! discovery: which notes count as sounding together is a choice, the request
//! makes it ([`Segmentation`]), and every report carries it in its
//! assumptions. Nothing here decides anything about harmony — it decides only
//! *what the next question is asked about*.
//!
//! A slice keeps every sounding note with the coordinates it came from, so a
//! finding downstream can point at the notes that produced it rather than at a
//! span somebody reconstructed.

// Rational arithmetic on musical time is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use super::{Lane, NoteRef, Segmentation, inside};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::score::{ScoreEventKind, ScoreSnapshot};
use crate::time::{MusicalDuration, MusicalTime};

/// One sounding note, kept whole: the pitch and where it came from.
pub(super) struct Voiced {
    /// The spelled pitch.
    pub(super) pitch: WrittenPitch,
    /// The note it is, by every coordinate a reader can act on.
    pub(super) note: NoteRef,
}

/// What sounds together over one span, under the request's policy.
pub(super) struct Slice {
    /// Where the slice begins.
    pub(super) onset: MusicalTime,
    /// How long it lasts.
    pub(super) extent: MusicalDuration,
    /// The sounding notes, lowest first.
    pub(super) notes: Vec<Voiced>,
}

impl Slice {
    /// The distinct spelled classes sounding, lowest first.
    ///
    /// Spelled, so an F-sharp and a G-flat sounding together are two classes
    /// and not one — a reading that collapsed them would name a chord the
    /// notation does not spell.
    pub(super) fn classes(&self) -> Vec<PitchClass> {
        let mut classes: Vec<PitchClass> = Vec::new();
        for voiced in &self.notes {
            let class = voiced.pitch.pitch_class();
            if !classes.contains(&class) {
                classes.push(class);
            }
        }
        classes
    }

    /// The class in the bass.
    pub(super) fn bass(&self) -> Option<PitchClass> {
        self.notes.first().map(|voiced| voiced.pitch.pitch_class())
    }

    /// The highest sounding pitch — the melody, for the one cadence criterion
    /// that is about melody (OMT 036).
    pub(super) fn soprano(&self) -> Option<WrittenPitch> {
        self.notes.last().map(|voiced| voiced.pitch)
    }

    /// The notes, for evidence.
    pub(super) fn refs(&self) -> Vec<NoteRef> {
        self.notes.iter().map(|voiced| voiced.note).collect()
    }
}

/// One sounding note with the span it occupies, before any slicing.
struct Held {
    pitch: WrittenPitch,
    note: NoteRef,
    onset: MusicalTime,
    end: MusicalTime,
}

/// Cut the requested lanes into simultaneities the requested way.
///
/// Slices with nothing sounding are dropped rather than reported as empty
/// chords: a rest is a `facts` observation and not a sonority with no notes
/// in it.
pub(super) fn slices(
    snapshot: &ScoreSnapshot,
    lanes: &[Lane],
    how: Segmentation,
    window: Option<(MusicalTime, MusicalTime)>,
) -> Vec<Slice> {
    let held = sounding(snapshot, lanes);
    let boundaries = match how {
        Segmentation::Attacks => attacks(&held),
        Segmentation::Beats => beats(snapshot, lanes, &held),
        Segmentation::HarmonyLane => harmony_lane(snapshot, &held),
    };
    boundaries
        .windows(2)
        .filter_map(|pair| match *pair {
            [from, to] if to > from && inside(window, from) => Some(gather(&held, from, to)),
            _ => None,
        })
        .filter(|slice| !slice.notes.is_empty())
        .collect()
}

/// Every sounding note of the requested lanes, with its span.
fn sounding(snapshot: &ScoreSnapshot, lanes: &[Lane]) -> Vec<Held> {
    let mut held = Vec::new();
    for lane in lanes {
        let Some(voice) = snapshot.parts().get(lane.part).and_then(|part| part.voice(lane.voice)) else {
            continue;
        };
        for event in voice.events() {
            let note = NoteRef {
                part: lane.part,
                voice: lane.voice,
                id: event.id,
                span: event.origin.definition_span,
            };
            let end = event.onset + event.notated_duration.value;
            let mut push = |pitch| {
                held.push(Held {
                    pitch,
                    note,
                    onset: event.onset,
                    end,
                });
            };
            match event.kind {
                ScoreEventKind::Note { pitch } => push(pitch),
                ScoreEventKind::Chord { ref pitches } => pitches.iter().copied().for_each(push),
                ScoreEventKind::Rest => {}
            }
        }
    }
    held
}

/// The instants a slice may begin at, followed by the instant the music ends —
/// sorted, distinct, and always at least two long when anything sounds, so
/// that `windows(2)` sees every span.
fn grid(mut instants: Vec<MusicalTime>, end: MusicalTime) -> Vec<MusicalTime> {
    instants.push(end);
    instants.sort_unstable();
    instants.dedup();
    instants
}

/// Where the music stops sounding.
fn last(held: &[Held]) -> MusicalTime {
    held.iter().map(|note| note.end).max().unwrap_or(MusicalTime::ZERO)
}

/// A boundary at every attack.
fn attacks(held: &[Held]) -> Vec<MusicalTime> {
    grid(held.iter().map(|note| note.onset).collect(), last(held))
}

/// A boundary at every notated beat, reading the meter where each beat falls
/// so that a metric change moves the grid with it.
fn beats(snapshot: &ScoreSnapshot, lanes: &[Lane], held: &[Held]) -> Vec<MusicalTime> {
    let scope = lanes.first().map_or(crate::scope::Scope::Piece, Lane::scope);
    let end = last(held);
    let mut instants = Vec::new();
    let mut at = MusicalTime::ZERO;
    while at < end {
        instants.push(at);
        let meter = snapshot.meter_at(scope, at);
        let beat = MusicalDuration::new(num_rational::Ratio::new(1, i64::from(meter.denominator().max(1))));
        if beat.is_zero() {
            break;
        }
        at = at + beat;
    }
    grid(instants, end)
}

/// A boundary at every written chord symbol.
///
/// The source's own segmentation, and the only one musa did not invent. A
/// piece with no harmony lane gets no slices here rather than a silent
/// fallback to another policy: the request asked for the source's reading, and
/// the source did not write one.
fn harmony_lane(snapshot: &ScoreSnapshot, held: &[Held]) -> Vec<MusicalTime> {
    let marks: Vec<MusicalTime> = snapshot.annotations().harmony().iter().map(|mark| mark.at).collect();
    if marks.is_empty() {
        return Vec::new();
    }
    grid(marks, last(held))
}

/// What sounds over `[from, to)`.
///
/// A note is in the slice when it has begun by `from` and has not stopped by
/// then. Under [`Segmentation::Attacks`] that is every note of the slice;
/// under the other two it is what a reader hears at the boundary, which is the
/// point of choosing them.
fn gather(held: &[Held], from: MusicalTime, to: MusicalTime) -> Slice {
    let mut notes: Vec<&Held> = held
        .iter()
        .filter(|note| note.onset <= from && note.end > from)
        .collect();
    // By sounding height, not by the pitch's derived order: that one sorts by
    // letter first, which puts a C5 under an A3 and would hand every reading
    // downstream the wrong bass.
    notes.sort_by_key(|note| {
        (
            note.pitch.diatonic_height(),
            note.pitch.chromatic_height(),
            note.note.part.0,
            note.note.voice.0,
            note.note.id.0,
        )
    });
    Slice {
        onset: from,
        extent: to - from,
        notes: notes
            .into_iter()
            .map(|note| Voiced {
                pitch: note.pitch,
                note: note.note,
            })
            .collect(),
    }
}
