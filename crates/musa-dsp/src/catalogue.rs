//! The studio language's closed, musician-facing vocabulary.
//!
//! This is presentation data beside editable [`StudioSpec`](crate::StudioSpec),
//! not the private render graph's operational descriptors.  A checked bridge
//! in `studio` joins each public parameter to its render parameter by
//! [`ParamSpec::dsp_name`].

use crate::{ParamSpec, Processor};
use std::fmt::Write as _;

/// A stable built-in identity.  Changing a public schema requires a version
/// change rather than silently giving an old name a new meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BuiltinKey {
    /// Stable written name.
    pub name: &'static str,
    /// Public schema version.
    pub version: u16,
}

/// What kind of signal a processor produces or transforms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalRole {
    /// Transforms or combines audio.
    AudioProcessor,
    /// Transforms a control-rate signal.
    ControlProcessor,
    /// Produces audio in a patch and control at a top-level signal binding.
    AudioOrControlSource,
}

impl SignalRole {
    /// Short musician-facing label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::AudioProcessor => "audio processor",
            Self::ControlProcessor => "control processor",
            Self::AudioOrControlSource => "audio or control source",
        }
    }
}

/// A public signal port kind. These are language contracts, not private graph
/// indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfacePort {
    /// Audio samples.
    Audio,
    /// A control-rate scalar.
    Control,
    /// Scheduled note gestures supplied by the score.
    NoteEvents,
}

impl SurfacePort {
    /// Stable documentation spelling.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Audio => "Audio",
            Self::Control => "Control",
            Self::NoteEvents => "NoteEvents",
        }
    }
}

/// One valid first-order port shape for a built-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortSchema {
    /// Input kinds; one entry denotes a variadic homogeneous input for `mix`.
    pub inputs: &'static [SurfacePort],
    /// Output kind.
    pub output: SurfacePort,
}

/// A processor's signal role and valid port shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceSchema {
    /// Musician-facing classification.
    pub role: SignalRole,
    /// Context-selected, first-order alternatives.
    pub ports: &'static [PortSchema],
}

/// Complete public documentation and schema for one built-in processor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessorDoc {
    /// Versioned public identity.
    pub key: BuiltinKey,
    /// Closed implementation identity.
    pub processor: Processor,
    /// Plain first sentence.
    pub summary: &'static str,
    /// Longer technical explanation.
    pub note: &'static str,
    /// Typed call shape.
    pub signature: &'static str,
    /// Signal role.
    pub schema: SurfaceSchema,
    /// Short valid source example.
    pub example: &'static str,
    /// Public parameter schema, in positional order.
    pub params: &'static [ParamSpec],
    /// Whether the built-in has a closed native implementation.
    pub native: bool,
}

/// Documentation for a studio control type, unit, or routing term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StudioTermDoc {
    /// Written spelling.
    pub spelling: &'static str,
    /// Plain first sentence.
    pub summary: &'static str,
    /// Typed or grammatical shape.
    pub signature: &'static str,
    /// Longer explanation.
    pub note: &'static str,
    /// Short source example.
    pub example: &'static str,
}

const fn doc(
    processor: Processor,
    summary: &'static str,
    note: &'static str,
    signature: &'static str,
    schema: SurfaceSchema,
    example: &'static str,
    params: &'static [ParamSpec],
    native: bool,
) -> ProcessorDoc {
    ProcessorDoc {
        key: BuiltinKey {
            name: processor.name(),
            version: 1,
        },
        processor,
        summary,
        note,
        signature,
        schema,
        example,
        params,
        native,
    }
}

const AUDIO: SurfacePort = SurfacePort::Audio;
const CONTROL: SurfacePort = SurfacePort::Control;
const NOTES: SurfacePort = SurfacePort::NoteEvents;
const OSCILLATOR_SCHEMA: SurfaceSchema = SurfaceSchema {
    role: SignalRole::AudioOrControlSource,
    ports: &[
        PortSchema {
            inputs: &[NOTES],
            output: AUDIO,
        },
        PortSchema {
            inputs: &[],
            output: CONTROL,
        },
    ],
};
const AUDIO_PROCESSOR_SCHEMA: SurfaceSchema = SurfaceSchema {
    role: SignalRole::AudioProcessor,
    ports: &[PortSchema {
        inputs: &[AUDIO],
        output: AUDIO,
    }],
};
const CONTROL_PROCESSOR_SCHEMA: SurfaceSchema = SurfaceSchema {
    role: SignalRole::ControlProcessor,
    ports: &[PortSchema {
        inputs: &[CONTROL],
        output: CONTROL,
    }],
};

/// Every processor the source language accepts, in reference order.
pub static PROCESSORS: &[ProcessorDoc] = &[
    doc(
        Processor::Oscillator,
        "Produces a sine wave as an instrument partial or control signal.",
        "In a patch it follows score pitch and `ratio` scales that pitch; as a top-level control signal, `frequency` sets its rate.",
        "oscillator(sine, frequency: Hz = 1 Hz, ratio: Ratio = 1)",
        OSCILLATOR_SCHEMA,
        "carrier = oscillator(sine, ratio: 1)",
        Processor::Oscillator.params(),
        true,
    ),
    doc(
        Processor::Gain,
        "Changes a signal's level.",
        "The written level is in decibels and is converted to a linear multiplier only at the render-graph boundary.",
        "gain(gain: dB = 0 dB)",
        AUDIO_PROCESSOR_SCHEMA,
        "gain(gain: -12 dB)",
        Processor::Gain.params(),
        true,
    ),
    doc(
        Processor::Mix,
        "Combines two or more audio signals.",
        "Its inputs are signal arguments rather than numeric parameters.",
        "mix(first: Audio, second: Audio, ...): Audio",
        AUDIO_PROCESSOR_SCHEMA,
        "mix(carrier, shimmer)",
        Processor::Mix.params(),
        true,
    ),
    doc(
        Processor::Envelope,
        "Shapes a note's level over time.",
        "Attack, decay, sustain, and release may be grouped inside `adsr(...)` for readability.",
        "envelope(attack: s = 5 ms, decay: s = 0 s, sustain: Ratio = 1, release: s = 50 ms)",
        AUDIO_PROCESSOR_SCHEMA,
        "envelope(adsr(attack: 30 ms, release: 400 ms))",
        Processor::Envelope.params(),
        true,
    ),
    doc(
        Processor::Lowpass,
        "Keeps frequencies below a cutoff.",
        "`resonance` is conventionally represented by quality factor Q; larger values emphasize the cutoff without changing the public spelling.",
        "lowpass(cutoff: Hz = 20000 Hz, resonance: Ratio = 0.707)",
        AUDIO_PROCESSOR_SCHEMA,
        "lowpass(cutoff: 1400 Hz, resonance: 0.7)",
        Processor::Lowpass.params(),
        true,
    ),
    doc(
        Processor::Highpass,
        "Keeps frequencies above a cutoff.",
        "`resonance` is conventionally represented by quality factor Q; larger values emphasize the cutoff without changing the public spelling.",
        "highpass(cutoff: Hz = 20 Hz, resonance: Ratio = 0.707)",
        AUDIO_PROCESSOR_SCHEMA,
        "highpass(cutoff: 80 Hz, resonance: 0.7)",
        Processor::Highpass.params(),
        true,
    ),
    doc(
        Processor::Reverb,
        "Places audio in a simulated room.",
        "`room` controls apparent size, `damping` absorbs high frequencies, and `mix` balances dry and reverberant sound.",
        "reverb(room: Ratio = 0.5, damping: Ratio = 0.5, mix: Ratio = 1)",
        AUDIO_PROCESSOR_SCHEMA,
        "reverb(room: 0.82, damping: 0.55, mix: 0.3)",
        Processor::Reverb.params(),
        true,
    ),
    doc(
        Processor::Delay,
        "Repeats audio after a written time.",
        "Feedback is bounded below unity and the two-second maximum matches the preallocated delay line.",
        "delay(time: s = 250 ms, feedback: Ratio = 0.3, mix: Ratio = 0.3)",
        AUDIO_PROCESSOR_SCHEMA,
        "delay(time: 250 ms, feedback: 0.4, mix: 0.3)",
        Processor::Delay.params(),
        true,
    ),
    doc(
        Processor::Chorus,
        "Adds a gently moving doubled voice.",
        "A control-rate oscillator varies a short delay; `mix` is the dry/wet balance.",
        "chorus(rate: Hz = 0.6 Hz, depth: s = 4 ms, mix: Ratio = 0.4)",
        AUDIO_PROCESSOR_SCHEMA,
        "chorus(rate: 0.6 Hz, depth: 4 ms, mix: 0.4)",
        Processor::Chorus.params(),
        true,
    ),
    doc(
        Processor::Scale,
        "Multiplies a control signal.",
        "The factor currently carries hertz because cutoff modulation is the implemented control target.",
        "scale(factor: Hz = 1 Hz): Control",
        CONTROL_PROCESSOR_SCHEMA,
        "scale(250 Hz)",
        Processor::Scale.params(),
        true,
    ),
    doc(
        Processor::Bias,
        "Offsets a control signal.",
        "The offset currently carries hertz because cutoff modulation is the implemented control target.",
        "bias(offset: Hz = 0 Hz): Control",
        CONTROL_PROCESSOR_SCHEMA,
        "bias(1400 Hz)",
        Processor::Bias.params(),
        true,
    ),
    doc(
        Processor::Clamp,
        "Bounds a control signal.",
        "Values below `min` or above `max` are held at the corresponding endpoint.",
        "clamp(min: Hz = 0 Hz, max: Hz = 20000 Hz): Control",
        CONTROL_PROCESSOR_SCHEMA,
        "clamp(min: 200 Hz, max: 6000 Hz)",
        Processor::Clamp.params(),
        true,
    ),
    doc(
        Processor::Smoothing,
        "Slows abrupt control changes.",
        "A one-pole slew runs once per audio frame, avoiding zipper noise without changing the authored control graph.",
        "smoothing(time: s = 20 ms): Control",
        CONTROL_PROCESSOR_SCHEMA,
        "smoothing(time: 20 ms)",
        Processor::Smoothing.params(),
        true,
    ),
];

/// Studio concepts which are not processor calls.
pub static TERMS: &[StudioTermDoc] = &[
    StudioTermDoc {
        spelling: "Audio",
        summary: "Audio is a stream of sounding samples.",
        signature: "Audio",
        note: "It is a catalogue port type, not a source type annotation in today's studio block.",
        example: "oscillator(sine) |> gain(-6 dB)",
    },
    StudioTermDoc {
        spelling: "Control",
        summary: "Control is a frame-rate stream that changes a parameter.",
        signature: "Control",
        note: "Control signals are typed separately from audio and connect only through `modulate`.",
        example: "lfo = oscillator(sine, frequency: 0.5 Hz);",
    },
    StudioTermDoc {
        spelling: "NoteEvents",
        summary: "NoteEvents are scheduled score gestures supplied to an instrument.",
        signature: "NoteEvents",
        note: "They are a public port contract prepared from the score, not a sample stream or source keyword.",
        example: "assign violin -> glass_pad;",
    },
    StudioTermDoc {
        spelling: "Hz",
        summary: "Hertz measure cycles per second.",
        signature: "Number Hz",
        note: "Frequency parameters require this suffix; Musa never guesses a missing unit.",
        example: "cutoff: 1400 Hz",
    },
    StudioTermDoc {
        spelling: "dB",
        summary: "Decibels express an audio level logarithmically.",
        signature: "Number dB",
        note: "Conversion to a linear multiplier happens once at the private render boundary.",
        example: "gain: -12 dB",
    },
    StudioTermDoc {
        spelling: "s",
        summary: "Seconds measure studio time.",
        signature: "Number s",
        note: "Time values are normalized to seconds while preserving the written source span.",
        example: "release: 2 s",
    },
    StudioTermDoc {
        spelling: "ms",
        summary: "Milliseconds are thousandths of a second.",
        signature: "Number ms",
        note: "The compiler normalizes them to seconds without asking later stages to parse the suffix again.",
        example: "attack: 30 ms",
    },
    StudioTermDoc {
        spelling: "Ratio",
        summary: "A ratio is a dimensionless control value.",
        signature: "Ratio",
        note: "Ratio parameters are written as bare numbers and reject a unit suffix.",
        example: "resonance: 0.7",
    },
    StudioTermDoc {
        spelling: "adsr",
        summary: "ADSR groups an envelope's four stages.",
        signature: "adsr(attack: s, decay: s, sustain: Ratio, release: s)",
        note: "It is a readable argument group flattened into `envelope`, not a second processor node.",
        example: "envelope(adsr(attack: 30 ms, release: 2 s))",
    },
    StudioTermDoc {
        spelling: "sine",
        summary: "Sine selects a smooth periodic waveform.",
        signature: "sine: Waveform",
        note: "It is the oscillator waveform implemented by the current closed built-in set.",
        example: "oscillator(sine)",
    },
    StudioTermDoc {
        spelling: "studio",
        summary: "A studio block connects a score to authored sound.",
        signature: "studio { StudioItem* }",
        note: "It owns patches, buses, control signals, assignments, sends, routes, and modulations without changing score facts.",
        example: "studio { route violin -> master; }",
    },
    StudioTermDoc {
        spelling: "assign",
        summary: "An assignment chooses the patch that realizes a part.",
        signature: "assign Part -> Patch;",
        note: "It is the narrow bridge between score identity and studio sound; a part is not a synthesizer.",
        example: "assign violin -> glass_pad;",
    },
    StudioTermDoc {
        spelling: "modulate",
        summary: "A modulation connects a control signal to one parameter.",
        signature: "modulate Signal -> Patch.Stage.Parameter;",
        note: "The target path resolves statically and must name a compatible public parameter.",
        example: "modulate lfo -> glass_pad.lowpass.cutoff;",
    },
    StudioTermDoc {
        spelling: "at",
        summary: "At introduces a send's written level.",
        signature: "send Source -> Bus at dB;",
        note: "The following value is required in decibels.",
        example: "send violin -> hall at -18 dB;",
    },
    StudioTermDoc {
        spelling: "instrument",
        summary: "An instrument turns score gestures into sound.",
        signature: "Part -> Patch -> Audio",
        note: "In today's source an instrument is expressed by a `patch` assigned to a part, not by a separate declaration.",
        example: "assign violin -> glass_pad;",
    },
    StudioTermDoc {
        spelling: "patch",
        summary: "A patch describes how a part sounds.",
        signature: "patch Name { SignalChain* }",
        note: "Its chains produce audio and end at `output`; assigning a part selects the patch without making part and synthesizer the same thing.",
        example: "patch soft { oscillator(sine) |> output; }",
    },
    StudioTermDoc {
        spelling: "signal",
        summary: "A signal is a named audio or control chain.",
        signature: "Name = SignalChain;",
        note: "Signal is the concept's name, not a source keyword; a binding can feed another stage or a modulation connection.",
        example: "drift = oscillator(sine) |> scale(250 Hz);",
    },
    StudioTermDoc {
        spelling: "bus",
        summary: "A bus processes audio sent from parts or other buses.",
        signature: "bus Name { SignalChain* }",
        note: "A bus names a shared effect path; it is not a score part or a desktop-only mixer channel.",
        example: "bus hall { reverb(room: 0.8); }",
    },
    StudioTermDoc {
        spelling: "send",
        summary: "A send copies some signal to a bus.",
        signature: "send Source -> Bus at dB;",
        note: "The level is written in decibels and does not replace the source's main route.",
        example: "send violin -> hall at -18 dB;",
    },
    StudioTermDoc {
        spelling: "route",
        summary: "A route chooses where a signal goes next.",
        signature: "route Source -> Destination;",
        note: "A destination is a declared bus or the built-in `master` output.",
        example: "route violin -> master;",
    },
    StudioTermDoc {
        spelling: "master",
        summary: "The master is the studio's final audio destination.",
        signature: "master: Audio",
        note: "It is built in and cannot be redeclared; prepared audio applies the final safety boundary there.",
        example: "route hall -> master;",
    },
    StudioTermDoc {
        spelling: "room",
        summary: "Room is the apparent size of a reverb space.",
        signature: "room: Ratio = 0.5",
        note: "It ranges from 0 to 1 and belongs to the `reverb` processor.",
        example: "reverb(room: 0.82)",
    },
    StudioTermDoc {
        spelling: "output",
        summary: "Output marks the signal leaving a patch.",
        signature: "Signal |> output",
        note: "It is a terminal marker rather than a processor node.",
        example: "oscillator(sine) |> output;",
    },
];

/// Find a processor by its one accepted spelling.
pub fn processor(name: &str) -> Option<&'static ProcessorDoc> {
    PROCESSORS.iter().find(|entry| entry.key.name == name)
}

/// Find a studio concept by spelling.
pub fn term(name: &str) -> Option<&'static StudioTermDoc> {
    TERMS.iter().find(|entry| entry.spelling == name)
}

/// Generate the book's studio-vocabulary reference from the public schema.
#[must_use]
pub fn reference_markdown() -> String {
    let mut out = String::from(
        "# Studio vocabulary\n\nThis page is generated from Musa's built-in studio catalogue. The compiler, editor help, and Sound/Mix workspaces read the same facts.\n\n",
    );
    for entry in PROCESSORS {
        let _ = writeln!(
            out,
            "## `{}`\n\n{} {}\n\n- Signature: `{}`\n- Role: {}\n- Origin: `builtin`\n- Schema: version {}\n- Native process: {}\n- Example: `{}`\n",
            entry.key.name,
            entry.summary,
            entry.note,
            entry.signature,
            entry.schema.role.label(),
            entry.key.version,
            if entry.native { "available" } else { "unavailable" },
            entry.example,
        );
        let ports = entry
            .schema
            .ports
            .iter()
            .map(|ports| {
                let inputs = if ports.inputs.is_empty() {
                    "()".to_owned()
                } else {
                    ports
                        .inputs
                        .iter()
                        .map(|port| port.label())
                        .collect::<Vec<_>>()
                        .join(" × ")
                };
                format!("{inputs} -> {}", ports.output.label())
            })
            .collect::<Vec<_>>()
            .join(" or ");
        let _ = writeln!(out, "- Ports: `{ports}`\n");
        if !entry.params.is_empty() {
            out.push_str(
                "| Parameter | Meaning | Unit | Default | Written range |\n| --- | --- | --- | ---: | ---: |\n",
            );
            for parameter in entry.params {
                let _ = writeln!(
                    out,
                    "| `{}` | {} | `{}` | {} | {}–{} |",
                    parameter.name,
                    parameter.summary,
                    parameter.unit.spelling().unwrap_or("Ratio"),
                    crate::written_ratio(parameter.default),
                    crate::written_ratio(parameter.range.0),
                    crate::written_ratio(parameter.range.1),
                );
            }
            out.push('\n');
        }
    }
    out.push_str("## Studio concepts\n\n");
    for term in TERMS {
        let _ = writeln!(
            out,
            "### `{}`\n\n{} {}\n\n- Shape: `{}`\n- Origin: `builtin`\n- Example: `{}`\n",
            term.spelling, term.summary, term.note, term.signature, term.example,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use std::collections::HashSet;

    use super::*;
    use crate::spec::{FilterKind, ProcessorSpec, Waveform};

    #[test]
    fn every_accepted_processor_and_parameter_has_one_catalogue_entry() {
        let mut processors = HashSet::new();
        let mut keys = HashSet::new();
        for entry in PROCESSORS {
            assert!(processors.insert(entry.processor), "duplicate processor: {entry:?}");
            assert!(keys.insert(entry.key), "duplicate key: {:?}", entry.key);
            assert_eq!(Processor::from_name(entry.key.name), Some(entry.processor));
            assert_eq!(entry.processor.params(), entry.params);
            assert!(!entry.schema.ports.is_empty(), "missing ports: {:?}", entry.key);
            match entry.schema.role {
                SignalRole::AudioProcessor => assert!(entry.schema.ports.iter().all(|ports| {
                    !ports.inputs.is_empty()
                        && ports.inputs.iter().all(|port| *port == SurfacePort::Audio)
                        && ports.output == SurfacePort::Audio
                })),
                SignalRole::ControlProcessor => {
                    assert!(entry.schema.ports.iter().all(|ports| {
                        ports.inputs == [SurfacePort::Control] && ports.output == SurfacePort::Control
                    }));
                }
                SignalRole::AudioOrControlSource => {
                    assert!(entry.schema.ports.iter().any(|ports| {
                        ports.inputs == [SurfacePort::NoteEvents] && ports.output == SurfacePort::Audio
                    }));
                    assert!(
                        entry
                            .schema
                            .ports
                            .iter()
                            .any(|ports| { ports.inputs.is_empty() && ports.output == SurfacePort::Control })
                    );
                }
            }
            let mut names = HashSet::new();
            for parameter in entry.params {
                assert!(
                    names.insert(parameter.name),
                    "duplicate parameter in {:?}: {}",
                    entry.key,
                    parameter.name
                );
                let render = render_descriptors(entry.processor)
                    .into_iter()
                    .find_map(|render| render.descriptor(parameter.dsp_name))
                    .unwrap_or_else(|| {
                        panic!(
                            "{}.{} has no private descriptor `{}`",
                            entry.key.name, parameter.name, parameter.dsp_name,
                        )
                    });
                let converted = |value| {
                    let value = crate::quantity::ratio_to_f64(value);
                    if parameter.unit == crate::Unit::Decibels {
                        10f64.powf(value / 20.0)
                    } else {
                        value
                    }
                };
                let expected_unit = if parameter.unit == crate::Unit::Decibels {
                    crate::Unit::Linear
                } else {
                    parameter.unit
                };
                assert_eq!(
                    render.unit, expected_unit,
                    "unit of {}.{}",
                    entry.key.name, parameter.name
                );
                let public_range = (converted(parameter.range.0), converted(parameter.range.1));
                let tolerance = 1e-6;
                assert!(
                    public_range.0 + tolerance >= f64::from(render.range.0)
                        && public_range.1 <= f64::from(render.range.1) + tolerance,
                    "written range of {}.{} exceeds its render descriptor",
                    entry.key.name,
                    parameter.name,
                );
                assert!(
                    converted(parameter.default) >= public_range.0 && converted(parameter.default) <= public_range.1,
                    "default of {}.{} leaves its written range",
                    entry.key.name,
                    parameter.name,
                );
            }
        }
        for processor in ALL_PROCESSORS {
            assert!(processors.contains(processor), "missing catalogue entry: {processor:?}");
        }
    }

    fn render_descriptors(processor: Processor) -> Vec<ProcessorSpec> {
        match processor {
            Processor::Oscillator => vec![
                ProcessorSpec::PolySine { voices: 16 },
                ProcessorSpec::Lfo {
                    waveform: Waveform::Sine,
                },
            ],
            Processor::Gain => vec![ProcessorSpec::StereoGain],
            Processor::Mix => vec![ProcessorSpec::Mixer { inputs: 2 }],
            Processor::Envelope => vec![ProcessorSpec::PolySine { voices: 16 }],
            Processor::Lowpass => vec![ProcessorSpec::Biquad {
                kind: FilterKind::LowPass,
            }],
            Processor::Highpass => vec![ProcessorSpec::Biquad {
                kind: FilterKind::HighPass,
            }],
            Processor::Reverb => vec![ProcessorSpec::Reverb],
            Processor::Delay => vec![ProcessorSpec::Delay],
            Processor::Chorus => vec![ProcessorSpec::Chorus],
            Processor::Scale => vec![ProcessorSpec::Scale],
            Processor::Bias => vec![ProcessorSpec::Bias],
            Processor::Clamp => vec![ProcessorSpec::Clamp],
            Processor::Smoothing => vec![ProcessorSpec::Smooth],
        }
    }

    const ALL_PROCESSORS: &[Processor] = &[
        Processor::Oscillator,
        Processor::Gain,
        Processor::Mix,
        Processor::Envelope,
        Processor::Lowpass,
        Processor::Highpass,
        Processor::Reverb,
        Processor::Delay,
        Processor::Chorus,
        Processor::Scale,
        Processor::Bias,
        Processor::Clamp,
        Processor::Smoothing,
    ];

    #[test]
    fn checked_in_reference_is_generated_from_the_catalogue() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/book/src/reference/studio-vocabulary.md"
        );
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::write(path, reference_markdown()).expect("write studio reference");
        }
        assert_eq!(
            reference_markdown(),
            include_str!("../../../docs/book/src/reference/studio-vocabulary.md"),
            "re-run the DSP catalogue test with UPDATE_FIXTURES=1"
        );
    }
}
