//! The `cadence` kind: what the evidence at a potential cadence point is.
//!
//! **Abstract domain.** The finite set of *potential cadence points* — the
//! instants the source marks as phrase endings, the instants a rest begins,
//! and the end of the piece — each paired with the two slices that arrive
//! there, a key, a cadence name, and a verdict on each of the criteria OMT
//! `036` states.
//!
//! **Abstraction map.** α locates the potential cadence points, takes the last
//! slice ending at or before each one and the slice before that, reads both as
//! numerals in each key `tonal` proposes, and checks: the harmonic criterion
//! (which numerals the two chords are), the melodic criterion (whether the
//! tonic is in the top voice of the final chord), and the positional criterion
//! (whether both chords are in root position).
//!
//! **Soundness.** No cadence finding is ever a fact, and that is a claim about
//! music rather than caution about the code. A cadence is a *formal* event: it
//! is where a phrase ends, and a phrase ending is not recoverable from pitch
//! and rhythm — a subverted cadence has every harmonic and melodic feature of
//! a real one and does not end the phrase (OMT `036`). α sees pitch, rhythm,
//! and what the source wrote down; it does not see form. So γ of a cadence
//! finding contains scores in which that place is a cadence and scores in
//! which it is an evaded gesture, and the report says which criteria held so a
//! reader can tell them apart.
//!
//! What no finding here does is choose. A place with a written `phrase` ending
//! and V–I is reported with every criterion satisfied and still as a
//! candidate; a place with V–I in the middle of a phrase is reported with the
//! formal criterion unsatisfied rather than suppressed.

// Rational arithmetic on musical time is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use super::segment::Slice;
use super::tonal;
use super::{AnalysisFinding, Cadence, Evidence, Ground, Lane, Observation, Standing};
use crate::score::{Key, ScoreEventKind, ScoreSnapshot};
use crate::time::MusicalTime;

/// Where a phrase may end, and why the reading thinks so.
struct Point {
    at: MusicalTime,
    written: bool,
    rest: bool,
    ends_the_piece: bool,
}

pub(super) fn observe(
    snapshot: &ScoreSnapshot,
    lanes: &[Lane],
    slices: &[Slice],
    assumed: Option<Key>,
) -> Vec<AnalysisFinding> {
    let keys = tonal::proposed(snapshot, lanes, slices, assumed);
    points(snapshot, lanes, slices)
        .iter()
        .flat_map(|point| read(slices, point, &keys))
        .collect()
}

/// Every potential cadence point, in time order.
///
/// All three sources are things the source wrote: a `phrase` annotation, a
/// rest, or the last note. Nothing here infers a phrase from a melodic shape
/// or a metric position — a reading that guessed at form would be inventing
/// the one piece of evidence it cannot see.
fn points(snapshot: &ScoreSnapshot, lanes: &[Lane], slices: &[Slice]) -> Vec<Point> {
    let mut points: Vec<Point> = Vec::new();
    let mut mark =
        |at: MusicalTime, written: bool, rest: bool, ends: bool| match points.iter_mut().find(|point| point.at == at) {
            Some(point) => {
                point.written |= written;
                point.rest |= rest;
                point.ends_the_piece |= ends;
            }
            None => points.push(Point {
                at,
                written,
                rest,
                ends_the_piece: ends,
            }),
        };
    for lane in lanes {
        let Some(voice) = snapshot.parts().get(lane.part).and_then(|part| part.voice(lane.voice)) else {
            continue;
        };
        for phrase in snapshot.annotations().phrases() {
            if let Some(event) = voice.events().iter().find(|event| event.id == phrase.to) {
                mark(event.onset + event.notated_duration.value, true, false, false);
            }
        }
        for event in voice.events() {
            if matches!(event.kind, ScoreEventKind::Rest) {
                mark(event.onset, false, true, false);
            }
        }
    }
    if let Some(last) = slices.last() {
        mark(last.onset + last.extent, false, false, true);
    }
    points.sort_by_key(|point| point.at);
    points
}

/// What the two chords arriving at a point are, in each proposed key.
fn read(slices: &[Slice], point: &Point, keys: &[Key]) -> Vec<AnalysisFinding> {
    let Some(final_at) = slices.iter().rposition(|slice| slice.onset + slice.extent <= point.at) else {
        return Vec::new();
    };
    let (Some(last), Some(before)) = (
        slices.get(final_at),
        final_at.checked_sub(1).and_then(|at| slices.get(at)),
    ) else {
        return Vec::new();
    };
    let notes = {
        let mut notes = before.refs();
        notes.extend(last.refs());
        notes
    };
    let evidence = Evidence::Passage {
        from: before.onset,
        to: last.onset + last.extent,
        notes,
    };
    keys.iter()
        .filter_map(|key| {
            let approach = tonal::degree_of(*key, before);
            let arrival = tonal::degree_of(*key, last);
            let cadence = name(approach, arrival)?;
            let root_position = tonal::in_root_position(*key, last) && tonal::in_root_position(*key, before);
            let tonic_on_top = last.soprano().is_some_and(|pitch| pitch.pitch_class() == key.tonic());
            let cadence = match cadence {
                Cadence::PerfectAuthentic if !(root_position && tonic_on_top) => Cadence::ImperfectAuthentic,
                Cadence::PerfectAuthentic => Cadence::PerfectAuthentic,
                Cadence::ImperfectAuthentic | Cadence::Half | Cadence::Deceptive => cadence,
            };
            Some(AnalysisFinding::judged(
                "cadence",
                Standing::Candidate,
                Observation::Cadence {
                    cadence,
                    key: *key,
                    at: last.onset,
                },
                evidence.clone(),
                grounds(cadence, tonic_on_top, root_position, point),
            ))
        })
        .collect()
}

/// What the reading was judged against.
///
/// The melodic and positional criteria are attached only to the authentic
/// cadences, because they are what tells the perfect from the imperfect (OMT
/// 036). A half cadence has no tonic to put on top, and listing a criterion
/// that cannot apply as unsatisfied would report a failure where there is no
/// claim.
fn grounds(cadence: Cadence, tonic_on_top: bool, root_position: bool, point: &Point) -> Vec<Ground> {
    let mut grounds = vec![Ground {
        criterion: match cadence {
            Cadence::PerfectAuthentic | Cadence::ImperfectAuthentic => "the dominant resolves to the tonic",
            Cadence::Half => "the phrase ends on the dominant",
            Cadence::Deceptive => "the dominant resolves somewhere other than the tonic",
        },
        satisfied: true,
        cites: "OMT 036",
    }];
    if matches!(cadence, Cadence::PerfectAuthentic | Cadence::ImperfectAuthentic) {
        grounds.push(Ground {
            criterion: "the tonic is in the top voice",
            satisfied: tonic_on_top,
            cites: "OMT 036",
        });
        grounds.push(Ground {
            criterion: "both chords are in root position",
            satisfied: root_position,
            cites: "OMT 036",
        });
    }
    grounds.push(Ground {
        criterion: "a phrase ends here",
        satisfied: point.written,
        cites: "OMT 036",
    });
    grounds.push(Ground {
        criterion: "the music stops or rests here",
        satisfied: point.rest || point.ends_the_piece,
        cites: "OMT 036",
    });
    grounds
}

/// Which cadence two scale degrees make, when they make one.
///
/// Only the four the vocabulary names. A progression that is none of them is
/// not a weak cadence — it is not a cadence, and reporting it as one at low
/// confidence would be exactly the invented number this analysis refuses.
fn name(approach: Option<i64>, arrival: Option<i64>) -> Option<Cadence> {
    match (approach, arrival) {
        (Some(5), Some(1)) => Some(Cadence::PerfectAuthentic),
        (Some(5), Some(6)) => Some(Cadence::Deceptive),
        (_, Some(5)) => Some(Cadence::Half),
        _ => None,
    }
}
