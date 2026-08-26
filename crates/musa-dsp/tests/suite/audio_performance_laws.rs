//! End-to-end preparation evidence for R1 and the complete option boundary.

#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile, lower_gestures};
use musa_dsp::{AudioFormat, AudioOptions, ChannelLayout, CollapsePolicy, FrameRounding, MessageKind, SchedulePolicy};

use super::audio_support::{options, prepare_gestures, studio};

#[derive(Debug, PartialEq, Eq)]
struct PreparationObservation {
    sample_rate: u32,
    total_frames: u64,
    decisions: Vec<String>,
    frames: Vec<[u32; 2]>,
}

fn plan(source: &str) -> (musa_score::GesturePlan, crate::intent::StudioSpec) {
    let compilation = compile(&SourceDocument::new(source, "r1.musa"), &CompileOptions::default());
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    let gestures = lower_gestures(compilation.snapshot().expect("score")).expect("gestures");
    (gestures, studio(&compilation))
}

fn observe(
    gestures: &musa_score::GesturePlan,
    studio: &crate::intent::StudioSpec,
    options: AudioOptions,
) -> PreparationObservation {
    let mut prepared = prepare_gestures(gestures, studio, options).expect("preparation succeeds");
    let decisions = prepared
        .decisions()
        .iter()
        .map(|decision| format!("{decision:?}"))
        .collect();
    let frames = (0..2_048).map(|_| prepared.step().map(f32::to_bits)).collect();
    PreparationObservation {
        sample_rate: prepared.sample_rate(),
        total_frames: prepared.total_frames(),
        decisions,
        frames,
    }
}

fn source(prefix: &str, meter: &str, key: &str, gain: &str) -> String {
    format!(
        "{prefix} piece \"R1\" {{ tempo 1/4 = 59; meter {meter}; key {key}; \
         instrument tone conforms note_instrument {{ implementation graph {{ \
         oscillator(sine) |> gain({gain}) |> output; }} }} \
         score {{ part p {{ sound tone using neutral; voice v {{ c4/384 d4/384 }} }} }} }}"
    )
}

#[test]
fn r1_equal_complete_arguments_have_equal_preparation_observations() {
    let (gestures, studio) = plan(&source("", "4/4", "c major", "-18 dB"));
    let options = options(64);
    assert_eq!(
        observe(&gestures, &studio, options),
        observe(&gestures, &studio, options)
    );
}

#[test]
fn presentation_lineage_is_separate_from_execution() {
    let (first, first_studio) = plan(&source("", "4/4", "c major", "-18 dB"));
    let (shifted, shifted_studio) = plan(&source("// presentation offset\n", "3/4", "g major", "-18 dB"));
    let first_lane = first.lanes().first().expect("lane");
    let shifted_lane = shifted.lanes().first().expect("lane");
    assert_eq!(
        first_lane.track(),
        shifted_lane.track(),
        "performed gesture tracks are equal"
    );
    let instance = first_lane
        .track()
        .occurrences()
        .first()
        .expect("gesture")
        .payload()
        .instance();
    assert_ne!(
        first_lane.lineage(instance).expect("first lineage").origin(),
        shifted_lane.lineage(instance).expect("shifted lineage").origin(),
        "presentation source positions differ"
    );
    assert_eq!(
        observe(&first, &first_studio, options(64)),
        observe(&shifted, &shifted_studio, options(64)),
        "lineage and notation-only meter/key fields do not enter execution"
    );
}

#[test]
fn every_execution_affecting_option_axis_is_explicit() {
    let (gestures, studio) = plan(&source("", "4/4", "c major", "-18 dB"));
    let baseline = options(64);

    let mut tail = baseline;
    tail.tail_frames += 1;
    tail.max_total_frames += 1;
    assert_ne!(
        observe(&gestures, &studio, baseline).total_frames,
        observe(&gestures, &studio, tail).total_frames
    );

    let mut tuning = baseline;
    tuning.tuning.concert_a = 442.0;
    assert_ne!(
        observe(&gestures, &studio, baseline).frames,
        observe(&gestures, &studio, tuning).frames
    );

    let mut rate = baseline;
    rate.format = AudioFormat::new(std::num::NonZeroU32::new(44_100).expect("rate"), ChannelLayout::Stereo);
    assert_ne!(
        observe(&gestures, &studio, baseline).sample_rate,
        observe(&gestures, &studio, rate).sample_rate
    );

    let limits = baseline.schedule.limits();
    let mut rounding = baseline;
    rounding.schedule = SchedulePolicy::new(
        baseline.schedule.version(),
        FrameRounding::Floor,
        CollapsePolicy::Ordered,
        [MessageKind::End, MessageKind::Point, MessageKind::Begin],
        limits,
    )
    .expect("rounding policy");
    assert_ne!(
        observe(&gestures, &studio, baseline).decisions,
        observe(&gestures, &studio, rounding).decisions
    );

    let mut bounded = baseline;
    bounded.limits.max_primitives = 0;
    assert!(prepare_gestures(&gestures, &studio, bounded).is_err());

    let mut extent = baseline;
    extent.max_total_frames = 0;
    assert!(prepare_gestures(&gestures, &studio, extent).is_err());
}

#[test]
fn an_instrument_binding_changes_execution_without_changing_the_gesture_track() {
    let (first, first_studio) = plan(&source("", "4/4", "c major", "-18 dB"));
    let (lower, lower_studio) = plan(&source("", "4/4", "c major", "-30 dB"));
    assert_eq!(
        first.lanes().first().expect("first lane").track(),
        lower.lanes().first().expect("lower lane").track()
    );
    assert_ne!(
        observe(&first, &first_studio, options(64)).frames,
        observe(&lower, &lower_studio, options(64)).frames
    );
}
