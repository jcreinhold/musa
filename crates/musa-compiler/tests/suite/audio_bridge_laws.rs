//! DSP tests (roadmap §17.5): oscillator accuracy and phase continuity,
//! arithmetic correctness, validation, silence-for-disconnected,
//! determinism, NaN/infinity absence, and the allocation-free render
//! contract.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// Test arithmetic on sample indices/frames is small and total.
#![allow(clippy::arithmetic_side_effects)]

use musa_dsp::testing::{ProcessorSpec, StudioGraphSpec, prepare_graph, prepare_graph_seeded};
use musa_dsp::{AudioFormat, AudioPrepareError, ChannelLayout, prepare_audio};
use musa_score::{Tuning, lower_gestures};

use super::audio_support::{options, parts};

// --- Helpers -----------------------------------------------------------------

const RATE: u32 = 48_000;

/// sine → gain → pan → mixer(1) → output.
fn voice_spec(frequency: f32, gain: f32, pan: f32) -> StudioGraphSpec {
    let mut spec = StudioGraphSpec::new();
    let sine = spec.add_node(ProcessorSpec::Sine);
    spec.set_param(sine, "frequency", frequency).expect("param");
    let gain_node = spec.add_node(ProcessorSpec::Gain);
    spec.set_param(gain_node, "gain", gain).expect("param");
    let pan_node = spec.add_node(ProcessorSpec::Pan);
    spec.set_param(pan_node, "pan", pan).expect("param");
    let mixer = spec.add_node(ProcessorSpec::Mixer { inputs: 1 });
    spec.connect(sine, 0, gain_node, 0);
    spec.connect(gain_node, 0, pan_node, 0);
    spec.connect(pan_node, 0, mixer, 0);
    spec.set_output(mixer);
    spec
}

fn render(spec: &StudioGraphSpec, frames: usize) -> Vec<f32> {
    let mut plan = prepare_graph(spec, RATE).expect("compiles");
    let mut output = vec![0.0; frames * 2];
    plan.render(&[], &mut output);
    output
}

// --- Oscillator (§13.5) ------------------------------------------------------

#[test]
fn oscillator_frequency_accuracy() {
    let output = render(&voice_spec(440.0, 1.0, 0.0), 48_000);
    // Positive-going zero crossings of the left channel = cycles.
    let left: Vec<f32> = output.iter().step_by(2).copied().collect();
    let crossings = left
        .windows(2)
        .filter(|pair| matches!(pair, [prev, next] if *prev <= 0.0 && *next > 0.0))
        .count();
    assert!(
        (438..=442).contains(&crossings),
        "expected ~440 cycles, counted {crossings}"
    );
}

#[test]
fn oscillator_phase_continuity_across_blocks() {
    // One long render equals the same render split mid-block and at odd sizes.
    let whole = render(&voice_spec(440.0, 1.0, 0.0), 5000);
    let mut plan = prepare_graph(&voice_spec(440.0, 1.0, 0.0), RATE).expect("compiles");
    let mut split = vec![0.0; 5000 * 2];
    let mut rest = split.as_mut_slice();
    for chunk in [100usize, 128, 1, 4771] {
        let (head, tail) = rest.split_at_mut(chunk * 2);
        plan.render(&[], head);
        rest = tail;
    }
    assert_eq!(whole, split, "block boundaries must be inaudible");
}

// --- Arithmetic ---------------------------------------------------------------

#[test]
fn gain_and_pan_arithmetic() {
    // frequency = sr/4 → samples cycle exactly through 0, 1, 0, -1.
    let output = render(&voice_spec(12_000.0, 0.5, 0.0), 8);
    let expected = [0.0f32, 0.5, 0.0, -0.5, 0.0, 0.5, 0.0, -0.5];
    for (i, (frame, mono)) in output.as_chunks::<2>().0.iter().zip(expected.iter()).enumerate() {
        let value = mono * std::f32::consts::FRAC_1_SQRT_2;
        let both_ok = matches!(frame, [left, right] if (*left - value).abs() < 1e-6 && (*right - value).abs() < 1e-6);
        assert!(both_ok, "sample {i}: {frame:?} != {value}");
    }
}

#[test]
fn mixer_sums_inputs() {
    let mut spec = StudioGraphSpec::new();
    let a = spec.add_node(ProcessorSpec::Sine);
    spec.set_param(a, "frequency", 12_000.0).expect("param");
    let b = spec.add_node(ProcessorSpec::Sine);
    spec.set_param(b, "frequency", 12_000.0).expect("param");
    let to_stereo_a = spec.add_node(ProcessorSpec::MonoToStereo);
    let to_stereo_b = spec.add_node(ProcessorSpec::MonoToStereo);
    let mixer = spec.add_node(ProcessorSpec::Mixer { inputs: 2 });
    spec.connect(a, 0, to_stereo_a, 0);
    spec.connect(b, 0, to_stereo_b, 0);
    spec.connect(to_stereo_a, 0, mixer, 0);
    spec.connect(to_stereo_b, 0, mixer, 1);
    spec.set_output(mixer);
    let output = render(&spec, 4);
    // Two identical in-phase sr/4 sines sum to 0, 2, 0, -2.
    for (i, (frame, expected)) in output
        .as_chunks::<2>()
        .0
        .iter()
        .zip([0.0f32, 2.0, 0.0, -2.0].iter())
        .enumerate()
    {
        let left = frame.first().copied().unwrap_or(f32::NAN);
        assert!((left - expected).abs() < 1e-6, "sample {i}: {left} != {expected}");
    }
}

// --- Validation (§13.3) -------------------------------------------------------

#[test]
fn cycle_is_rejected() {
    let mut spec = StudioGraphSpec::new();
    let a = spec.add_node(ProcessorSpec::Gain);
    let b = spec.add_node(ProcessorSpec::Gain);
    spec.connect(a, 0, b, 0);
    spec.connect(b, 0, a, 0);
    spec.set_output(b);
    insta::assert_snapshot!(prepare_graph(&spec, RATE).err().expect("cycle").to_string());
}

#[test]
fn port_mismatch_is_rejected() {
    let mut spec = StudioGraphSpec::new();
    let constant = spec.add_node(ProcessorSpec::Constant);
    let gain = spec.add_node(ProcessorSpec::Gain);
    spec.connect(constant, 0, gain, 0); // control into audio
    spec.set_output(gain);
    insta::assert_snapshot!(prepare_graph(&spec, RATE).err().expect("mismatch").to_string());
}

#[test]
fn channel_mismatch_requires_adapter() {
    let mut spec = StudioGraphSpec::new();
    let sine = spec.add_node(ProcessorSpec::Sine); // mono
    let mixer = spec.add_node(ProcessorSpec::Mixer { inputs: 1 }); // stereo in
    spec.connect(sine, 0, mixer, 0);
    spec.set_output(mixer);
    insta::assert_snapshot!(prepare_graph(&spec, RATE).err().expect("channels").to_string());
}

#[test]
fn missing_output_is_rejected() {
    let mut spec = StudioGraphSpec::new();
    spec.add_node(ProcessorSpec::Sine);
    insta::assert_snapshot!(prepare_graph(&spec, RATE).err().expect("no output").to_string());
}

// --- Silence, determinism, adversarial parameters ------------------------------

#[test]
fn disconnected_graph_renders_silence() {
    let mut spec = StudioGraphSpec::new();
    let sine = spec.add_node(ProcessorSpec::Sine); // unreachable from output
    let mixer = spec.add_node(ProcessorSpec::Mixer { inputs: 1 });
    spec.set_param(sine, "frequency", 440.0).expect("param");
    spec.set_output(mixer); // input unconnected
    let output = render(&spec, 256);
    assert!(output.iter().all(|sample| *sample == 0.0));
}

#[test]
fn two_renders_are_byte_equal() {
    let spec = voice_spec(440.0, 0.8, -0.3);
    assert_eq!(render(&spec, 4096), render(&spec, 4096));
}

#[test]
fn stochastic_primitives_obey_the_explicit_render_seed() {
    let mut spec = StudioGraphSpec::new();
    let noise = spec.add_node(ProcessorSpec::Noise);
    let stereo = spec.add_node(ProcessorSpec::MonoToStereo);
    spec.connect(noise, 0, stereo, 0);
    spec.set_output(stereo);
    let render = |seed| {
        let mut plan = prepare_graph_seeded(&spec, RATE, seed).expect("noise graph");
        let mut output = vec![0.0; 512];
        plan.render(&[], &mut output);
        output
    };
    assert_eq!(render(17), render(17));
    assert_ne!(render(17), render(18));
}

#[test]
fn production_preparation_refuses_layout_tuning_and_resource_violations() {
    let (score, studio) =
        parts("piece \"bounds\" { tempo quarter = 60; meter 4/4; key c major; score { part p { voice v { c5/4 } } } }");
    let gestures = lower_gestures(&score).expect("gestures");

    let mut mono = options(0);
    mono.format = AudioFormat::new(mono.format.sample_rate(), ChannelLayout::Mono);
    assert!(matches!(
        prepare_audio(&gestures, &studio, mono),
        Err(AudioPrepareError::UnsupportedLayout(ChannelLayout::Mono))
    ));

    let mut invalid_tuning = options(0);
    invalid_tuning.tuning = Tuning { concert_a: f64::NAN };
    assert!(matches!(
        prepare_audio(&gestures, &studio, invalid_tuning),
        Err(AudioPrepareError::InvalidTuning(value)) if value.is_nan()
    ));

    let mut bounded = options(0);
    bounded.limits.max_primitives = 0;
    assert!(matches!(
        prepare_audio(&gestures, &studio, bounded),
        Err(AudioPrepareError::ResourceLimit {
            resource: "primitive count",
            ..
        })
    ));

    let mut memory = options(0);
    memory.limits.max_state_bytes = 0;
    assert!(matches!(
        prepare_audio(&gestures, &studio, memory),
        Err(AudioPrepareError::ResourceLimit {
            resource: "retained state bytes",
            ..
        })
    ));

    let mut work = options(0);
    work.limits.max_step_work = 0;
    assert!(matches!(
        prepare_audio(&gestures, &studio, work),
        Err(AudioPrepareError::ResourceLimit {
            resource: "one-frame work",
            ..
        })
    ));
}

#[test]
fn adversarial_parameters_stay_finite() {
    for (frequency, gain, pan) in [(0.0f32, 16.0f32, -1.0f32), (20_000.0, 16.0, 1.0), (0.001, 0.0, 0.0)] {
        let output = render(&voice_spec(frequency, gain, pan), 4096);
        assert!(
            output.iter().all(|sample| sample.is_finite()),
            "({frequency}, {gain}, {pan}) produced non-finite output"
        );
    }
}

#[test]
fn non_finite_parameter_is_rejected() {
    let mut spec = StudioGraphSpec::new();
    let sine = spec.add_node(ProcessorSpec::Sine);
    assert!(spec.set_param(sine, "frequency", f32::NAN).is_err());
    assert!(spec.set_param(sine, "not-a-param", 1.0).is_err());
}

// --- Offline rendering == live rendering (§13.8) ---------------------------------

#[test]
fn offline_render_writes_wav() {
    let output = render(&voice_spec(440.0, 0.5, 0.0), 4800);
    let path = std::env::temp_dir().join("musa-dsp-test.wav");
    {
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                channels: 2,
                sample_rate: RATE,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .expect("wav writer");
        for sample in &output {
            let value = (sample * f32::from(i16::MAX)).clamp(f32::from(i16::MIN), f32::from(i16::MAX));
            writer.write_sample(value as i16).expect("sample");
        }
        writer.finalize().expect("finalize");
    }
    let reader = hound::WavReader::open(&path).expect("read back");
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.duration(), 4800);
    drop(reader);
    drop(std::fs::remove_file(&path));
}
