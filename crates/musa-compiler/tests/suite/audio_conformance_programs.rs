//! Complete witnesses used by prompt 174's cross-stage audit.

#![allow(clippy::expect_used)]

use musa_dsp::{MachineValue, prepare_audio, prepare_machine};
use musa_events::{Canonical, Duration, Occurrence, PerformedTime, Position, Span, track};
use musa_score::machine::{MachineSpec, PortSchema as Port, SpecForm, SpecNode, StepTag, descriptor};
use musa_score::{Tuning, lower_gestures};
use num_rational::Ratio;

use super::audio_support as support;

#[test]
fn tonal_construction_runs_through_exact_gestures_and_one_frame_audio() {
    let source = include_str!("../../../../examples/tonal-construction.musa");
    let mut audio = support::prepare(source, 0);
    let mut frames = [0.0; 512];
    audio.render(&mut frames);
    assert!(frames.iter().all(|sample| sample.is_finite()));
    assert!(frames.iter().any(|sample| *sample != 0.0));
}

#[test]
fn unmeasured_time_runs_without_manufacturing_a_meter() {
    let source = include_str!("../../../../examples/chant.musa");
    let (score, studio) = support::parts(source);
    assert!(
        !score
            .meter_at(musa_score::Scope::Piece, musa_score::MusicalTime::ZERO)
            .is_measured()
    );
    let gestures = lower_gestures(&score).expect("exact gestures");
    let mut audio = prepare_audio(&gestures, &studio, support::options(0)).expect("one-frame audio");
    let mut frames = [0.0; 256];
    audio.render(&mut frames);
    assert!(frames.iter().all(|sample| sample.is_finite()));
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TranscriptionCandidate {
    phrase: String,
    stated_loss: String,
}

impl Canonical for TranscriptionCandidate {
    const OWNER_TYPE_ID: &'static str = "musa.audit.TranscriptionCandidate";
    const QUOTIENT_VERSION: u32 = 1;

    fn canonical_key(&self) -> String {
        format!(
            "{}:{}{}:{}",
            self.phrase.len(),
            self.phrase,
            self.stated_loss.len(),
            self.stated_loss
        )
    }
}

#[test]
fn phrase_led_transcription_keeps_ambiguity_and_states_loss() {
    let candidate = TranscriptionCandidate {
        phrase: "ascending three-note gesture".to_owned(),
        stated_loss: "source attack spectra and performer identity are not represented".to_owned(),
    };
    let span = Span::new(Position::ZERO, Position::new(Ratio::new(3, 2))).expect("ordered");
    let result = track(
        Duration::<PerformedTime>::new(Ratio::new(3, 2)).expect("positive"),
        vec![Occurrence::new(span, candidate.clone())],
    )
    .expect("bounded finite result");
    let occurrence = result.occurrences().first().expect("the candidate occurrence");
    assert_eq!(occurrence.payload(), &candidate);
    assert!(!occurrence.payload().stated_loss.is_empty());
}

#[test]
fn ensemble_tuning_is_configuration_not_a_rewritten_pitch() {
    let source = include_str!("../../../../examples/in-c.musa");
    let (score, studio) = support::parts(source);
    let gestures = lower_gestures(&score).expect("exact gestures");
    let lane = gestures.lanes().first().expect("the ensemble lane");
    let occurrence = lane.track().occurrences().first().expect("the first gesture");
    let written = occurrence.payload().pitch;
    let mut options = support::options(0);
    options.tuning = Tuning { concert_a: 432.0 };
    let mut audio = prepare_audio(&gestures, &studio, options).expect("configured instrument machine");
    assert_eq!(occurrence.payload().pitch, written);
    assert!(audio.step().iter().all(|sample| sample.is_finite()));
}

fn stored_ratio(numerator: i64, denominator: i64) -> Vec<u8> {
    let mut bytes = vec![0];
    bytes.extend_from_slice(&numerator.to_be_bytes());
    bytes.extend_from_slice(&denominator.to_be_bytes());
    bytes
}

fn stored_nat(value: u64) -> Vec<u8> {
    let mut bytes = vec![3];
    bytes.extend_from_slice(&value.to_be_bytes());
    bytes
}

fn stored_pair(first: Vec<u8>, second: Vec<u8>) -> Vec<u8> {
    let mut bytes = vec![4];
    bytes.extend(first);
    bytes.extend(second);
    bytes
}

fn primitive(id: &str, version: u32, stored: Vec<u8>) -> SpecNode {
    SpecNode::primitive(descriptor(id, version).expect("registered"), stored)
}

#[test]
fn a_finite_live_protocol_builds_a_machine_that_need_not_finish() {
    let spec = MachineSpec::new(
        StepTag::AudioFrameStep,
        Port::Unit,
        Port::Nat,
        vec![primitive("count", 1, stored_nat(0))],
    );
    let prepared = prepare_machine(&spec).expect("finite source description prepares");
    let mut running = prepared.start();
    for expected in 0..10_000 {
        assert_eq!(running.step(MachineValue::Unit), Ok(MachineValue::Nat(expected)));
    }
}

#[test]
fn audio_first_microphone_synth_and_effect_paths_compose_explicitly() {
    let pair = Port::Pair(Box::new(Port::Ratio), Box::new(Port::Ratio));
    let spec = MachineSpec::new(
        StepTag::AudioFrameStep,
        pair,
        Port::Ratio,
        vec![
            // The first input is a microphone frame; the second is a synth
            // frame. Each passes through its own explicit gain effect.
            primitive("scale", 1, stored_ratio(2, 1)),
            primitive("scale", 1, stored_ratio(3, 1)),
            SpecNode::wiring(SpecForm::Beside, vec![0, 1]),
            primitive("mix", 1, stored_pair(stored_ratio(1, 1), stored_ratio(1, 1))),
            SpecNode::wiring(SpecForm::Connect, vec![2, 3]),
        ],
    );
    let prepared = prepare_machine(&spec).expect("audio-first path prepares");
    let output = prepared
        .start()
        .step(MachineValue::pair(
            MachineValue::ratio(1, 4).expect("ratio"),
            MachineValue::ratio(1, 2).expect("ratio"),
        ))
        .expect("one frame");
    assert_eq!(output, MachineValue::ratio(2, 1).expect("mixed frame"));
}
