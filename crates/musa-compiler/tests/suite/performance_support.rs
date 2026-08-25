//! Exact performed-gesture observations shared by compiler tests.
//!
//! This deliberately stops before frames. Checked scheduling is exercised in
//! `musa-dsp`; compiler laws inspect the exact value handed to that stage.

#![expect(
    clippy::expect_used,
    reason = "a score accepted by the compiler must lower to its exact gesture boundary"
)]

use musa_score::{EventId, GestureLane, ScoreSnapshot, WrittenPitch, lower_gestures};
use num_rational::Ratio;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExactNote {
    pub(crate) on_seconds: Ratio<i64>,
    pub(crate) off_seconds: Ratio<i64>,
    pub(crate) on_performed: Ratio<i64>,
    pub(crate) off_performed: Ratio<i64>,
    pub(crate) notated_on: Ratio<i64>,
    pub(crate) notated_off: Ratio<i64>,
    pub(crate) amplitude: Ratio<i64>,
    pub(crate) pitch: WrittenPitch,
    pub(crate) event: EventId,
}

pub(crate) fn notes_of(score: &ScoreSnapshot) -> Vec<Vec<ExactNote>> {
    lower_gestures(score)
        .expect("exact gestures lower")
        .lanes()
        .iter()
        .map(notes_in)
        .collect()
}

pub(crate) fn notes_in(lane: &GestureLane) -> Vec<ExactNote> {
    let mut notes: Vec<_> = lane
        .track()
        .occurrences()
        .iter()
        .map(|occurrence| {
            let span = occurrence.span();
            let gesture = occurrence.payload();
            ExactNote {
                on_seconds: lane.physical(span.start()).as_ratio(),
                off_seconds: lane.physical(span.end()).as_ratio(),
                on_performed: span.start().as_ratio(),
                off_performed: span.end().as_ratio(),
                notated_on: gesture.notated_on().as_ratio(),
                notated_off: gesture.notated_off().as_ratio(),
                amplitude: gesture.amplitude(),
                pitch: gesture.pitch(),
                event: gesture.event(),
            }
        })
        .collect();
    notes.sort_by(|left, right| {
        (left.on_seconds, left.off_seconds, left.event, left.pitch.to_string()).cmp(&(
            right.on_seconds,
            right.off_seconds,
            right.event,
            right.pitch.to_string(),
        ))
    });
    notes
}
