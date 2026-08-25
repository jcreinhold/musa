//! Planning one staff: its measures, and the lanes inside each.
//!
//! One concern of the `plan` module; see its docs for what a plan is.

use musa_score::{BarLines, Clef, Meter, MusicalDuration, MusicalTime, Part, ScoreEvent, ScoreSnapshot, VoiceId};
use num_rational::Ratio;

use super::collect::Marks;
use super::fold::Fold;
use super::lane::plan_lane;
use super::marks::{ClefChange, KeySignature, PointMark, PositionedMark, TempoText};
use super::score::{MeasurePlan, StaffPlan, VoiceLane};

#[expect(
    clippy::too_many_arguments,
    reason = "one staff is one plan; splitting the inputs would only move the list"
)]
pub(super) fn plan_staff(
    score: &ScoreSnapshot,
    part: &Part,
    bars: &BarLines,
    key: Option<KeySignature>,
    keys: &[(MusicalTime, KeySignature)],
    clefs: &[(MusicalTime, Clef)],
    marks: &Marks,
    fold: &Fold,
    tempos: Vec<PositionedMark<TempoText>>,
) -> Result<StaffPlan, crate::NotationError> {
    let meter = bars.meter_at(MusicalTime::ZERO);
    let lanes: Vec<(VoiceId, String, Vec<ScoreEvent>)> = part
        .voices()
        .map(|(voice_id, voice)| {
            (
                voice_id,
                part.voice_name(voice_id).unwrap_or_default().to_string(),
                fold.events(voice),
            )
        })
        .collect();
    let span = lanes
        .iter()
        .filter_map(|(_, _, events)| events.last())
        .map(|event| (event.onset - MusicalTime::ZERO) + event.notated_duration.value)
        .max()
        .unwrap_or(MusicalDuration::ZERO);
    // Point marks are placed by time, so they go through the fold like clef
    // changes do: a mark inside a repeated passage stands in the measure the
    // page prints, not the one the timeline plays it in.
    let points: Vec<(VoiceId, MusicalTime, &musa_score::PointMark)> = score
        .annotations()
        .points()
        .iter()
        .filter(|point| point.part == part.id())
        .filter_map(|point| Some((point.voice, fold.at(point.at)?, point)))
        .collect();
    let mut measures = Vec::new();
    // What the last measure printed, so a measure prints a time signature
    // exactly when it says something the one before it did not.
    let mut printed: Option<Meter> = None;
    let mut printed_key: Option<KeySignature> = None;
    for measure in bars.measures_through(span) {
        let (start, end) = (measure.start, measure.end);
        let mut plans = Vec::with_capacity(lanes.len());
        for (voice_id, name, events) in &lanes {
            let lane = plan_lane(events, measure.meter, measure.duration().as_ratio(), start, end, marks)?;
            let here: Vec<PointMark> = points
                .iter()
                .filter(|(voice, at, _)| voice == voice_id && start <= *at && *at < end)
                .map(|(_, at, point)| PointMark {
                    mark: point.mark,
                    argument: point.argument.clone(),
                    onset_in_measure: MusicalDuration::new(at.as_ratio() - start.as_ratio()),
                })
                .collect();
            plans.push(VoiceLane {
                voice: *voice_id,
                name: name.clone(),
                items: lane.items,
                slurs: lane.slurs,
                phrases: lane.phrases,
                hairpins: lane.hairpins,
                points: here,
                marks: lane.marks,
            });
        }
        let lanes = plans;
        // Unmeasured music prints no time signature — there is none to print
        // — and the measured music after it prints one only if it says
        // something new. So the last *printed* meter is what a resumption is
        // compared against, which is why `printed` skips the unmeasured
        // stretch rather than recording it.
        let measured = measure.meter.is_measured();
        let changed = measured && printed != Some(measure.meter);
        if measured {
            printed = Some(measure.meter);
        }
        // The key in force where this measure opens. Read as "the latest one
        // stated at or before the barline" rather than "one stated exactly
        // here", so a modulation the compiler refused still prints somewhere
        // instead of vanishing: rendering has to stay total.
        let here_key = keys.iter().rfind(|(at, _)| *at <= start).map(|(_, key)| *key);
        let key_changed = here_key.is_some() && here_key != printed_key;
        if here_key.is_some() {
            printed_key = here_key;
        }
        measures.push(MeasurePlan {
            number: measure.number,
            meter: measure.meter,
            length: measure.duration(),
            time_signature: changed.then(|| (measure.meter.numerator(), measure.meter.denominator())),
            key: key_changed.then_some(here_key).flatten(),
            clefs: clefs
                .iter()
                .skip(1)
                .filter(|(at, _)| *at >= start && *at < end)
                .map(|(at, clef)| ClefChange {
                    onset_in_measure: MusicalDuration::new(at.as_ratio() - start.as_ratio()),
                    clef: *clef,
                })
                .collect(),
            lanes,
        });
    }
    Ok(StaffPlan {
        name: part.name().to_string(),
        clef: score.clef_at(part.id(), MusicalTime::ZERO),
        key,
        time_signature: (meter.numerator(), meter.denominator()),
        measures,
        tempos,
    })
}

/// Which beat group an onset falls in, and where that group ends.
///
/// The grouping itself comes from `beat_groups`, at the bottom of the graph,
/// so a bar of 7/8 beams 2+2+3 — the way it is counted — and beams it the same
/// way the formatter spaces it. A single beat *unit* cannot say that: the
/// groups of an irregular meter are not all the same length.
///
/// `None` for an onset past the end of the bar, which has no group.
pub(super) fn beat_group_at(meter: Meter, onset: Ratio<i64>) -> Option<(usize, Ratio<i64>)> {
    let unit = Ratio::new(1, i64::from(meter.denominator()));
    let mut end = Ratio::from_integer(0);
    for (index, group) in musa_syntax::beat_groups(meter.numerator(), meter.denominator())
        .into_iter()
        .enumerate()
    {
        end += unit * Ratio::from_integer(i64::from(group));
        if onset < end {
            return Some((index, end));
        }
    }
    None
}
