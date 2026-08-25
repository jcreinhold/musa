//! Closed native primitive registry and preparation-time resource accounting.
//!
//! The table is the audit surface: every private graph processor maps to one
//! versioned entry naming its configuration and state layout. Resource bounds
//! are conservative upper bounds checked before any processor state is built.

use crate::spec::{PortKind, ProcessorSpec, StudioGraphSpec};

/// Explicit bounds for the native one-frame machine. There is no default:
/// accepting a larger studio graph is a product decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioLimits {
    /// Greatest number of native primitive instances.
    pub max_primitives: usize,
    /// Greatest retained primitive and frame-buffer state in bytes.
    pub max_state_bytes: usize,
    /// Greatest conservative work units in one reference step.
    pub max_step_work: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AudioResources {
    pub(crate) primitives: usize,
    pub(crate) state_bytes: usize,
    pub(crate) step_work: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PrimitiveKind {
    Sine,
    Noise,
    Constant,
    Gain,
    Pan,
    Mixer,
    Splitter,
    MonoToStereo,
    StereoToMono,
    StereoGain,
    Passthrough,
    PolySine,
    Lfo,
    Scale,
    Bias,
    Clamp,
    Smooth,
    Biquad,
    Delay,
    Chorus,
    Reverb,
    Limiter,
    OnePole,
}

#[derive(Clone, Copy, Debug)]
struct Registration {
    kind: PrimitiveKind,
    name: &'static str,
    version: u32,
    configuration: &'static str,
    state_layout: &'static str,
    base_work: u64,
}

const REGISTRY: &[Registration] = &[
    registration(PrimitiveKind::Sine, "sine", "frequency:f32", "phase:f64", 20),
    registration(PrimitiveKind::Noise, "noise", "seed:u64,node:u32", "xorshift:u32", 8),
    registration(PrimitiveKind::Constant, "constant", "value:f32", "value:f32", 1),
    registration(PrimitiveKind::Gain, "gain", "gain:f32", "gain:f32", 4),
    registration(PrimitiveKind::Pan, "pan", "pan:f32", "pan:f32", 16),
    registration(PrimitiveKind::Mixer, "mixer", "inputs:u8", "unit", 16),
    registration(PrimitiveKind::Splitter, "splitter", "unit", "unit", 2),
    registration(PrimitiveKind::MonoToStereo, "mono-to-stereo", "unit", "unit", 2),
    registration(PrimitiveKind::StereoToMono, "stereo-to-mono", "unit", "unit", 3),
    registration(PrimitiveKind::StereoGain, "stereo-gain", "gain:f32", "gain:f32", 6),
    registration(PrimitiveKind::Passthrough, "passthrough", "channels:u8", "unit", 4),
    registration(
        PrimitiveKind::PolySine,
        "poly-sine",
        "voices:u8,adsr:f32x4,ratio:f32,blend:f32",
        "fixed-voice-pool",
        24,
    ),
    registration(PrimitiveKind::Lfo, "lfo", "waveform:u8,frequency:f32", "phase:f64", 24),
    registration(PrimitiveKind::Scale, "scale", "factor:f32", "factor:f32", 3),
    registration(PrimitiveKind::Bias, "bias", "offset:f32", "offset:f32", 2),
    registration(PrimitiveKind::Clamp, "clamp", "min:f32,max:f32", "bounds:f32x2", 4),
    registration(PrimitiveKind::Smooth, "smooth", "time:f32", "one-pole:f32", 5),
    registration(
        PrimitiveKind::Biquad,
        "biquad",
        "kind:u8,cutoff:f32,q:f32",
        "coefficients-and-stereo-history",
        64,
    ),
    registration(
        PrimitiveKind::Delay,
        "delay",
        "time:f32,feedback:f32,mix:f32",
        "two-fixed-delay-lines",
        48,
    ),
    registration(
        PrimitiveKind::Chorus,
        "chorus",
        "rate:f32,depth:f32,mix:f32",
        "two-fixed-delay-lines-and-phases",
        96,
    ),
    registration(
        PrimitiveKind::Reverb,
        "reverb",
        "room:f32,damping:f32,mix:f32",
        "two-fixed-comb-allpass-networks",
        640,
    ),
    registration(
        PrimitiveKind::Limiter,
        "limiter",
        "ceiling:f32",
        "stereo-lookahead-and-peak-ring",
        24,
    ),
    registration(
        PrimitiveKind::OnePole,
        "one-pole",
        "cutoff:f32",
        "stereo-history:f32x2",
        12,
    ),
];

// Construction witnesses keep the closed set visible to dead-code analysis
// in production builds as well as to the exhaustive dispatch matches. The
// parameterized cases include every waveform/configuration family whose
// start state differs.
const PROCESSOR_WITNESSES: &[ProcessorSpec] = &[
    ProcessorSpec::Sine,
    ProcessorSpec::Noise,
    ProcessorSpec::Constant,
    ProcessorSpec::Gain,
    ProcessorSpec::Pan,
    ProcessorSpec::Mixer { inputs: 1 },
    ProcessorSpec::Splitter,
    ProcessorSpec::MonoToStereo,
    ProcessorSpec::StereoToMono,
    ProcessorSpec::StereoGain,
    ProcessorSpec::Passthrough { channels: 2 },
    ProcessorSpec::PolySine { voices: 1 },
    ProcessorSpec::Lfo {
        waveform: crate::spec::Waveform::Sine,
    },
    ProcessorSpec::Lfo {
        waveform: crate::spec::Waveform::Triangle,
    },
    ProcessorSpec::Lfo {
        waveform: crate::spec::Waveform::Square,
    },
    ProcessorSpec::Scale,
    ProcessorSpec::Bias,
    ProcessorSpec::Clamp,
    ProcessorSpec::Smooth,
    ProcessorSpec::Biquad {
        kind: crate::spec::FilterKind::LowPass,
    },
    ProcessorSpec::Biquad {
        kind: crate::spec::FilterKind::HighPass,
    },
    ProcessorSpec::Delay,
    ProcessorSpec::Chorus,
    ProcessorSpec::Reverb,
    ProcessorSpec::Limiter,
    ProcessorSpec::OnePole,
];

const fn registration(
    kind: PrimitiveKind,
    name: &'static str,
    configuration: &'static str,
    state_layout: &'static str,
    base_work: u64,
) -> Registration {
    Registration {
        kind,
        name,
        version: 1,
        configuration,
        state_layout,
        base_work,
    }
}

fn kind(processor: ProcessorSpec) -> PrimitiveKind {
    match processor {
        ProcessorSpec::Sine => PrimitiveKind::Sine,
        ProcessorSpec::Noise => PrimitiveKind::Noise,
        ProcessorSpec::Constant => PrimitiveKind::Constant,
        ProcessorSpec::Gain => PrimitiveKind::Gain,
        ProcessorSpec::Pan => PrimitiveKind::Pan,
        ProcessorSpec::Mixer { .. } => PrimitiveKind::Mixer,
        ProcessorSpec::Splitter => PrimitiveKind::Splitter,
        ProcessorSpec::MonoToStereo => PrimitiveKind::MonoToStereo,
        ProcessorSpec::StereoToMono => PrimitiveKind::StereoToMono,
        ProcessorSpec::StereoGain => PrimitiveKind::StereoGain,
        ProcessorSpec::Passthrough { .. } => PrimitiveKind::Passthrough,
        ProcessorSpec::PolySine { .. } => PrimitiveKind::PolySine,
        ProcessorSpec::Lfo { .. } => PrimitiveKind::Lfo,
        ProcessorSpec::Scale => PrimitiveKind::Scale,
        ProcessorSpec::Bias => PrimitiveKind::Bias,
        ProcessorSpec::Clamp => PrimitiveKind::Clamp,
        ProcessorSpec::Smooth => PrimitiveKind::Smooth,
        ProcessorSpec::Biquad { .. } => PrimitiveKind::Biquad,
        ProcessorSpec::Delay => PrimitiveKind::Delay,
        ProcessorSpec::Chorus => PrimitiveKind::Chorus,
        ProcessorSpec::Reverb => PrimitiveKind::Reverb,
        ProcessorSpec::Limiter => PrimitiveKind::Limiter,
        ProcessorSpec::OnePole => PrimitiveKind::OnePole,
    }
}

fn registered(processor: ProcessorSpec) -> Option<&'static Registration> {
    let wanted = kind(processor);
    REGISTRY.iter().find(|entry| entry.kind == wanted)
}

/// Conservatively price all retained native state and one-frame work.
pub(crate) fn resources(spec: &StudioGraphSpec, sample_rate: u32, max_frame_messages: usize) -> Option<AudioResources> {
    debug_assert!(PROCESSOR_WITNESSES.iter().all(|processor| {
        let wanted = kind(*processor);
        REGISTRY.iter().any(|entry| entry.kind == wanted)
    }));
    let mut state_bytes = 4usize; // shared one-frame zero buffer
    let mut step_work = 0u64;
    for node in spec.nodes() {
        let processor = node.processor();
        let entry = registered(processor)?;
        // Fixed plan/dispatch/config storage, deliberately rounded upward.
        state_bytes = state_bytes.saturating_add(1024);
        let output_samples = processor
            .output_ports()
            .iter()
            .map(|port| match port {
                PortKind::Audio { channels } => usize::from(*channels),
                PortKind::Control | PortKind::NoteEvents => 1,
            })
            .sum::<usize>();
        state_bytes = state_bytes.saturating_add(output_samples.saturating_mul(size_of::<f32>()));
        state_bytes = state_bytes.saturating_add(dynamic_state_bytes(processor, sample_rate));
        step_work = step_work.saturating_add(entry.base_work);
        step_work = step_work.saturating_add(dynamic_step_work(processor, sample_rate));
        if let ProcessorSpec::PolySine { voices } = processor {
            // Every message may be delivered in one batch. Begin searches for
            // a free/oldest slot and End searches for its handle; price three
            // full voice-pool passes per message.
            step_work = step_work.saturating_add(
                u64::try_from(max_frame_messages)
                    .unwrap_or(u64::MAX)
                    .saturating_mul(u64::from(voices))
                    .saturating_mul(3),
            );
        }
        // Reading these fields here makes absence from the audit table a
        // compiler-visible defect rather than dead documentation.
        debug_assert!(entry.version > 0 && !entry.name.is_empty());
        debug_assert!(!entry.configuration.is_empty() && !entry.state_layout.is_empty());
    }
    // One retained modulation descriptor/state plus its combine, clamp, and
    // smoothing work. This prevents a wide fan-in from hiding under a node's
    // fixed base price.
    let modulation_count = spec.modulations().len();
    state_bytes = state_bytes.saturating_add(modulation_count.saturating_mul(128));
    step_work = step_work.saturating_add(u64::try_from(modulation_count).unwrap_or(u64::MAX).saturating_mul(24));
    Some(AudioResources {
        primitives: spec.nodes().len(),
        state_bytes,
        step_work,
    })
}

fn dynamic_state_bytes(processor: ProcessorSpec, sample_rate: u32) -> usize {
    let rate = sample_rate as usize;
    match processor {
        ProcessorSpec::PolySine { voices } => usize::from(voices).saturating_mul(256),
        ProcessorSpec::Delay => rate.saturating_mul(2).saturating_mul(2).saturating_mul(4),
        ProcessorSpec::Chorus => rate
            .saturating_mul(21)
            .saturating_div(1000)
            .saturating_add(2)
            .saturating_mul(2)
            .saturating_mul(4),
        ProcessorSpec::Reverb => {
            // 12 lines per channel, conservatively rounded to the next whole
            // 44.1-kHz scale factor and including the stereo spread.
            let scale = rate.saturating_add(44_099).saturating_div(44_100).max(1);
            12_587usize
                .saturating_mul(scale)
                .saturating_add(12usize.saturating_mul(23))
                .saturating_mul(2)
                .saturating_mul(4)
        }
        ProcessorSpec::Limiter => rate
            .saturating_mul(5)
            .saturating_add(500)
            .saturating_div(1000)
            .max(1)
            .saturating_mul(3)
            .saturating_mul(4),
        ProcessorSpec::Sine
        | ProcessorSpec::Noise
        | ProcessorSpec::Constant
        | ProcessorSpec::Gain
        | ProcessorSpec::Pan
        | ProcessorSpec::Mixer { .. }
        | ProcessorSpec::Splitter
        | ProcessorSpec::MonoToStereo
        | ProcessorSpec::StereoToMono
        | ProcessorSpec::StereoGain
        | ProcessorSpec::Passthrough { .. }
        | ProcessorSpec::Lfo { .. }
        | ProcessorSpec::Scale
        | ProcessorSpec::Bias
        | ProcessorSpec::Clamp
        | ProcessorSpec::Smooth
        | ProcessorSpec::Biquad { .. }
        | ProcessorSpec::OnePole => 0,
    }
}

fn dynamic_step_work(processor: ProcessorSpec, sample_rate: u32) -> u64 {
    match processor {
        ProcessorSpec::Mixer { inputs } => u64::from(inputs).saturating_mul(4),
        ProcessorSpec::Passthrough { channels } => u64::from(channels),
        ProcessorSpec::PolySine { voices } => u64::from(voices).saturating_mul(32),
        // The current limiter scans its complete lookahead ring each frame.
        ProcessorSpec::Limiter => u64::from(sample_rate).saturating_mul(5).saturating_add(999) / 1000,
        ProcessorSpec::Sine
        | ProcessorSpec::Noise
        | ProcessorSpec::Constant
        | ProcessorSpec::Gain
        | ProcessorSpec::Pan
        | ProcessorSpec::Splitter
        | ProcessorSpec::MonoToStereo
        | ProcessorSpec::StereoToMono
        | ProcessorSpec::StereoGain
        | ProcessorSpec::Lfo { .. }
        | ProcessorSpec::Scale
        | ProcessorSpec::Bias
        | ProcessorSpec::Clamp
        | ProcessorSpec::Smooth
        | ProcessorSpec::Biquad { .. }
        | ProcessorSpec::Delay
        | ProcessorSpec::Chorus
        | ProcessorSpec::Reverb
        | ProcessorSpec::OnePole => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_names_and_kinds_are_unique_and_versioned() {
        for (index, registration) in REGISTRY.iter().enumerate() {
            assert!(registration.version > 0);
            assert!(!registration.configuration.is_empty());
            assert!(!registration.state_layout.is_empty());
            assert!(REGISTRY.iter().take(index).all(|prior| prior.kind != registration.kind));
            assert!(REGISTRY.iter().take(index).all(|prior| prior.name != registration.name));
        }
    }
}
