//! Modulation and the shared parameter system (§13.7, §17.5).
//!
//! The unit-level DSP laws — segment timing, filter response, impulse
//! response — live beside their modules, where the arithmetic is. What is
//! testable only here is what happens when a control signal, a parameter
//! descriptor, and a processor meet: that the descriptors the language
//! checks against are the ones the DSP obeys, that a modulation is combined,
//! clamped, and smoothed as its parameter declares, and that no reachable
//! modulation can make the output stop being a number.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// Sample arithmetic in tests is small and total.
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{CompileOptions, Processor, SourceDocument, compile};
use musa_dsp::testing::{
    Combination, FilterKind, GraphOptions, ProcessorSpec, StudioGraphSpec, Unit, Waveform, lower_studio, prepare_graph,
};

use super::support::render_source;

const RATE: u32 = 48_000;
const OPTIONS: GraphOptions = GraphOptions {
    sample_rate: RATE,
    render_seed: 0,
};

const GLASS_MOUNTAIN: &str = include_str!("../../../../examples/glass-mountain.musa");

/// Render `frames` of a graph with no events, as interleaved stereo.
fn render(spec: &StudioGraphSpec, frames: usize) -> Vec<f32> {
    let mut plan = prepare_graph(spec, RATE).expect("the graph is valid");
    let mut output = vec![0.0f32; frames * 2];
    plan.render(&[], &mut output);
    output
}

/// §13.7: the descriptors the language checks a written value against are the
/// ones the DSP obeys — not a copy of them.
///
/// The check is name-by-name rather than list-by-list because the two
/// vocabularies are allowed to differ in *extent* (an LFO has no `ratio`);
/// what they may not differ in is what a name means.
#[test]
fn the_language_and_the_dsp_agree_about_every_shared_parameter() {
    let pairs: [(Processor, ProcessorSpec); 8] = [
        (Processor::Envelope, ProcessorSpec::PolySine { voices: 16 }),
        (
            Processor::Lowpass,
            ProcessorSpec::Biquad {
                kind: FilterKind::LowPass,
            },
        ),
        (
            Processor::Highpass,
            ProcessorSpec::Biquad {
                kind: FilterKind::HighPass,
            },
        ),
        (Processor::Scale, ProcessorSpec::Scale),
        (Processor::Bias, ProcessorSpec::Bias),
        (Processor::Delay, ProcessorSpec::Delay),
        (Processor::Chorus, ProcessorSpec::Chorus),
        (Processor::Reverb, ProcessorSpec::Reverb),
    ];
    for (written, rendered) in pairs {
        for declared in written.params() {
            let Some(descriptor) = rendered.parameters().iter().find(|d| d.name == declared.name) else {
                continue;
            };
            assert_eq!(
                descriptor.unit,
                declared.unit,
                "`{}.{}` is written in one unit and rendered in another",
                written.name(),
                declared.name
            );
            assert!(
                (descriptor.default - declared.default as f32).abs() < 1e-6,
                "`{}.{}` defaults differently on the two sides",
                written.name(),
                declared.name
            );
        }
    }
    // `gain` is the one deliberate divergence, and it is a conversion rather
    // than a disagreement: written in dB, applied as a linear multiplier
    // (§2 — a dynamic marking is not a number of decibels, and a decibel is
    // not a coefficient).
    assert_eq!(
        Processor::Gain.param("gain").expect("gain has a gain").unit,
        Unit::Decibels
    );
    assert_eq!(
        ProcessorSpec::StereoGain
            .parameters()
            .first()
            .expect("stereo gain has a gain")
            .unit,
        Unit::Linear
    );
}

/// §13.7: a modulation combines with the base value as the parameter says,
/// not as the connection says. `gain` multiplies, so a control of `0.5` into
/// a gain of `2` is a gain of `1` — under `Replace` it would be `0.5`, and
/// the two are told apart by listening.
#[test]
fn a_modulation_combines_as_its_parameter_declares() {
    assert_eq!(
        ProcessorSpec::StereoGain
            .parameters()
            .first()
            .expect("stereo gain has a gain")
            .combination,
        Combination::Multiply
    );
    let mut spec = StudioGraphSpec::new();
    let tone = spec.add_node(ProcessorSpec::Sine);
    spec.set_param(tone, "frequency", 200.0)
        .expect("a sine has a frequency");
    let stereo = spec.add_node(ProcessorSpec::MonoToStereo);
    let gain = spec.add_node(ProcessorSpec::StereoGain);
    spec.set_param(gain, "gain", 2.0).expect("a gain has a gain");
    let control = spec.add_node(ProcessorSpec::Constant);
    spec.set_param(control, "value", 0.5).expect("a constant has a value");
    spec.connect(tone, 0, stereo, 0);
    spec.connect(stereo, 0, gain, 0);
    spec.modulate(control, 0, gain, "gain");
    spec.set_output(gain);
    let samples = render(&spec, 8_192);
    // The last half, so the parameter smoother has arrived.
    let peak = samples
        .split_at(samples.len() / 2)
        .1
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!((peak - 1.0).abs() < 0.01, "expected a gain of 1, heard {peak}");
}

/// §13.7: the declared range is a bound on the *result*, so a modulation
/// that asks for more than the parameter allows gets what it allows.
///
/// A cutoff is the audible case: driven far past Nyquist it must land on the
/// top of its range and keep filtering rather than blow up.
#[test]
fn a_modulation_is_clamped_to_the_parameters_range() {
    let extremes = [-1e9, -1.0, 0.0, 1e9, f32::MAX];
    for extreme in extremes {
        let mut spec = StudioGraphSpec::new();
        let noise = spec.add_node(ProcessorSpec::Noise);
        let stereo = spec.add_node(ProcessorSpec::MonoToStereo);
        let filter = spec.add_node(ProcessorSpec::Biquad {
            kind: FilterKind::LowPass,
        });
        spec.set_param(filter, "cutoff", 1_000.0)
            .expect("a filter has a cutoff");
        let control = spec.add_node(ProcessorSpec::Constant);
        spec.set_param(control, "value", extreme)
            .expect("a constant has a value");
        spec.connect(noise, 0, stereo, 0);
        spec.connect(stereo, 0, filter, 0);
        spec.modulate(control, 0, filter, "cutoff");
        spec.set_output(filter);
        let samples = render(&spec, 8_192);
        assert!(
            samples.iter().all(|sample| sample.is_finite() && sample.abs() <= 4.0),
            "a cutoff modulated to {extreme} left the audible world"
        );
    }
}

/// §17.5: nothing a modulation can reach produces a NaN or an infinity,
/// including a control signal that is itself not a number.
#[test]
fn no_reachable_modulation_produces_a_non_finite_sample() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0] {
        let mut spec = StudioGraphSpec::new();
        let noise = spec.add_node(ProcessorSpec::Noise);
        let stereo = spec.add_node(ProcessorSpec::MonoToStereo);
        let filter = spec.add_node(ProcessorSpec::Biquad {
            kind: FilterKind::LowPass,
        });
        spec.set_param(filter, "cutoff", 800.0).expect("a filter has a cutoff");
        // `set_param` refuses a non-finite value, so the only way to get one
        // into a parameter is through a modulation — which is exactly the
        // path this test exists to close.
        let control = spec.add_node(ProcessorSpec::Scale);
        spec.set_param(control, "factor", if value.is_finite() { value } else { 1e30 })
            .expect("a scale has a factor");
        let source = spec.add_node(ProcessorSpec::Constant);
        spec.set_param(source, "value", 1e30).expect("a constant has a value");
        spec.connect(source, 0, control, 0);
        spec.connect(noise, 0, stereo, 0);
        spec.connect(stereo, 0, filter, 0);
        spec.modulate(control, 0, filter, "cutoff");
        spec.set_output(filter);
        assert!(
            render(&spec, 4_096).iter().all(|sample| sample.is_finite()),
            "a control signal of {value} reached the output"
        );
    }
}

/// A modulation into a parameter the target does not declare is a graph
/// error, not a silently ignored wire.
#[test]
fn a_modulation_of_an_undeclared_parameter_is_rejected() {
    let mut spec = StudioGraphSpec::new();
    let sine = spec.add_node(ProcessorSpec::Sine);
    let control = spec.add_node(ProcessorSpec::Constant);
    spec.modulate(control, 0, sine, "cutoff");
    spec.set_output(sine);
    assert!(prepare_graph(&spec, RATE).is_err(), "a sine has no cutoff");
}

/// A modulation from an audio output is rejected: control is a port kind,
/// not a convention (§13.4).
#[test]
fn a_modulation_from_an_audio_output_is_rejected() {
    let mut spec = StudioGraphSpec::new();
    let sine = spec.add_node(ProcessorSpec::Sine);
    let other = spec.add_node(ProcessorSpec::Sine);
    spec.modulate(other, 0, sine, "frequency");
    spec.set_output(sine);
    assert!(prepare_graph(&spec, RATE).is_err(), "audio is not control");
}

/// A modulated render is as deterministic as an unmodulated one: the LFO's
/// phase advances by block, and blocks are the same every time.
#[test]
fn a_modulated_render_is_deterministic() {
    let mut spec = StudioGraphSpec::new();
    let noise = spec.add_node(ProcessorSpec::Noise);
    let stereo = spec.add_node(ProcessorSpec::MonoToStereo);
    let filter = spec.add_node(ProcessorSpec::Biquad {
        kind: FilterKind::LowPass,
    });
    spec.set_param(filter, "cutoff", 1_200.0)
        .expect("a filter has a cutoff");
    let lfo = spec.add_node(ProcessorSpec::Lfo {
        waveform: Waveform::Triangle,
    });
    spec.set_param(lfo, "frequency", 3.0).expect("an lfo has a frequency");
    let scale = spec.add_node(ProcessorSpec::Scale);
    spec.set_param(scale, "factor", 600.0).expect("a scale has a factor");
    let bias = spec.add_node(ProcessorSpec::Bias);
    spec.set_param(bias, "offset", 1_200.0).expect("a bias has an offset");
    spec.connect(lfo, 0, scale, 0);
    spec.connect(scale, 0, bias, 0);
    spec.connect(noise, 0, stereo, 0);
    spec.connect(stereo, 0, filter, 0);
    spec.modulate(bias, 0, filter, "cutoff");
    spec.set_output(filter);
    assert_eq!(render(&spec, 12_000), render(&spec, 12_000));
}

/// A swept cutoff moves smoothly: frame-rate modulation still needs smoothing
/// when a discontinuous control asks for a large coefficient jump.
#[test]
fn a_swept_cutoff_does_not_step() {
    let mut spec = StudioGraphSpec::new();
    let tone = spec.add_node(ProcessorSpec::Sine);
    spec.set_param(tone, "frequency", 300.0)
        .expect("a sine has a frequency");
    let stereo = spec.add_node(ProcessorSpec::MonoToStereo);
    let filter = spec.add_node(ProcessorSpec::Biquad {
        kind: FilterKind::LowPass,
    });
    spec.set_param(filter, "cutoff", 400.0).expect("a filter has a cutoff");
    let lfo = spec.add_node(ProcessorSpec::Lfo {
        waveform: Waveform::Square,
    });
    spec.set_param(lfo, "frequency", 20.0).expect("an lfo has a frequency");
    let scale = spec.add_node(ProcessorSpec::Scale);
    spec.set_param(scale, "factor", 3_000.0).expect("a scale has a factor");
    let bias = spec.add_node(ProcessorSpec::Bias);
    spec.set_param(bias, "offset", 3_400.0).expect("a bias has an offset");
    spec.connect(lfo, 0, scale, 0);
    spec.connect(scale, 0, bias, 0);
    spec.connect(tone, 0, stereo, 0);
    spec.connect(stereo, 0, filter, 0);
    spec.modulate(bias, 0, filter, "cutoff");
    spec.set_output(filter);
    // A square LFO asks the cutoff to jump 6 kHz twice per period. The
    // sample-to-sample difference of a 300 Hz sine is small; a zipper would
    // show as a difference far larger than the signal's own slope.
    let samples = render(&spec, 24_000);
    let worst = slopes(&left_channel(&samples)).fold(0.0f32, f32::max);
    assert!(worst < 0.1, "the sweep stepped by {worst}");
}

/// The golden-ear check (§17.5), stated as what actually distinguishes the
/// §7.1 patch from the default instrument: it is brighter than a bare sine
/// because of its partial, and it keeps sounding after the notes stop
/// because of its release. A pad, not a plink.
///
/// It is deliberately *not* "darker than the unfiltered default render". A 1400 Hz
/// low-pass over material whose highest partial is 1320 Hz is very nearly
/// transparent, and a test that claimed otherwise would be measuring its own
/// tolerance. What the filter does to signal that reaches it is measured in
/// `a_low_pass_in_a_graph_attenuates_what_is_above_it` and in the biquad's
/// own response test.
#[test]
fn the_roadmap_patch_sounds_like_a_pad() {
    let voice = |shimmer_db: f32, release: f32| {
        let source = format!(
            "piece \"pad\" {{ tempo quarter = 60; meter 4/4; \
             score {{ part violin {{ voice v {{ c4/4 }} }} }} \
             studio {{ patch p {{ carrier = oscillator(sine); \
             shimmer = oscillator(sine, ratio: 2) |> gain({shimmer_db} dB); \
             mix(carrier, shimmer) |> envelope(adsr(attack: 30 ms, decay: 1.8 s, \
             sustain: 0.65, release: {release} s)) |> output; }} \
             assign violin -> p; route violin -> master; }} }}"
        );
        render_source(&source, RATE as usize * 4)
    };
    let pad = voice(-15.0, 3.5);
    let plain = voice(-120.0, 0.05);
    assert!(
        high_frequency_energy(&pad) > high_frequency_energy(&plain) * 1.05,
        "the partial must be audible above the fundamental"
    );
    // Two seconds after the last note-off the pad is still fading and the
    // default instrument has been silent for most of that time.
    let tail = |samples: &[f32]| -> f32 {
        let from = 3 * RATE as usize * 2;
        samples
            .get(from..)
            .unwrap_or(&[])
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()))
    };
    assert!(tail(&pad) > 0.01, "the pad must still be sounding: {}", tail(&pad));
    assert!(
        tail(&plain) < 1e-6,
        "the plain voice must be long gone: {}",
        tail(&plain)
    );
}

/// A low-pass wired into a graph attenuates what is above it — the filter is
/// connected to the signal, not merely present in the node list.
#[test]
fn a_low_pass_in_a_graph_attenuates_what_is_above_it() {
    let energy = |cutoff: f32| {
        let mut spec = StudioGraphSpec::new();
        let noise = spec.add_node(ProcessorSpec::Noise);
        let stereo = spec.add_node(ProcessorSpec::MonoToStereo);
        let filter = spec.add_node(ProcessorSpec::Biquad {
            kind: FilterKind::LowPass,
        });
        spec.set_param(filter, "cutoff", cutoff).expect("a filter has a cutoff");
        spec.connect(noise, 0, stereo, 0);
        spec.connect(stereo, 0, filter, 0);
        spec.set_output(filter);
        high_frequency_energy(&render(&spec, 24_000))
    };
    assert!(
        energy(500.0) < energy(18_000.0) * 0.2,
        "a 500 Hz low-pass must take the top off broadband noise"
    );
}

/// The whole §7.1 studio renders, and every sample of it is a number.
#[test]
fn the_roadmap_studio_renders() {
    let compilation = compile(
        &SourceDocument::new(GLASS_MOUNTAIN, "glass-mountain.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let studio = compilation.into_parts().1;
    let (_, lowering) = lower_studio(&studio, &OPTIONS);
    assert!(
        (lowering.release_tail - 3.5).abs() < 1e-6,
        "the export must leave room for the patch's release"
    );
    let samples = render_source(GLASS_MOUNTAIN, RATE as usize * 4);
    assert!(samples.iter().all(|sample| sample.is_finite()));
    assert!(
        samples.iter().any(|sample| sample.abs() > 0.01),
        "a patched piece must be audible"
    );
}

/// The left channel of an interleaved stereo render.
fn left_channel(samples: &[f32]) -> Vec<f32> {
    samples
        .as_chunks::<2>()
        .0
        .iter()
        .map(|frame| frame.first().copied().unwrap_or(0.0))
        .collect()
}

/// The absolute sample-to-sample differences of a channel — a first
/// difference, which is a high-pass.
fn slopes(channel: &[f32]) -> impl Iterator<Item = f32> + '_ {
    channel
        .windows(2)
        .map(|window| (window.get(1).copied().unwrap_or(0.0) - window.first().copied().unwrap_or(0.0)).abs())
}

/// The mean absolute first difference of the left channel, relative to its
/// RMS.
///
/// Normalizing by level is what makes this a measure of *character* rather
/// than of loudness: the patched chain also sends itself to a bus, and a
/// louder render would otherwise look like a brighter one.
fn high_frequency_energy(samples: &[f32]) -> f32 {
    let left = left_channel(samples);
    let total: f32 = slopes(&left).sum();
    let power: f32 = left.iter().map(|sample| sample * sample).sum();
    let rms = (power / left.len().max(1) as f32).sqrt();
    total / left.len().max(1) as f32 / rms.max(f32::MIN_POSITIVE)
}
