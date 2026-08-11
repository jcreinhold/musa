//! The declarative graph spec: nodes, typed ports, connections, and
//! parameter descriptors (roadmap §13.4, §13.7). Editable and serializable
//! intent; the compiled plan lives in `plan.rs`.

/// A node's identity within a spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

/// The kind of a port: what flows through it (§13.4). Compatibility is
/// exact equality — conversions require explicit adapter processors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortKind {
    /// Audio-rate samples with a fixed channel count.
    Audio {
        /// Channels (1 = mono, 2 = stereo).
        channels: u8,
    },
    /// Control-rate scalar (block-rate constant).
    Control,
    /// A gate (note on/off as control).
    Gate,
    /// Scheduled note events.
    NoteEvents,
}

impl std::fmt::Display for PortKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Audio { channels } => write!(f, "audio/{channels}"),
            Self::Control => write!(f, "control"),
            Self::Gate => write!(f, "gate"),
            Self::NoteEvents => write!(f, "note-events"),
        }
    }
}

/// A parameter's physical unit (§13.7).
///
/// Re-exported from `musa-compiler` rather than declared again: the language
/// checks `1400 Hz` against the same `Hz` the DSP descriptor names, so the
/// two cannot drift apart.
pub use musa_compiler::Unit;

/// Parameter smoothing applied to value changes (§13.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Smoothing {
    /// No smoothing (step change).
    None,
    /// Linear ramp over one block.
    BlockRamp,
}

/// How a modulation signal combines with the base value (§13.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combination {
    /// Replace the base value.
    Replace,
    /// Add to the base value.
    Add,
    /// Multiply the base value.
    Multiply,
}

/// One parameter's contract (§13.7).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterDescriptor {
    /// The parameter name (language-facing).
    pub name: &'static str,
    /// Its unit.
    pub unit: Unit,
    /// Inclusive value range.
    pub range: (f32, f32),
    /// Default when unset.
    pub default: f32,
    /// Smoothing applied to changes.
    pub smoothing: Smoothing,
    /// Modulation combination rule.
    pub combination: Combination,
}

/// A low-frequency oscillator's shape (§13.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waveform {
    /// A sine, the shape a modulation reaches for unless it says otherwise.
    Sine,
    /// A linear rise and fall.
    Triangle,
    /// A two-valued alternation.
    Square,
}

/// The longest delay a `delay` stage can ask for, in seconds.
///
/// A delay line is preallocated (§13.2), so its length is a number the plan
/// has to know before it renders; making it the parameter's range is what
/// turns "the line is not that long" from a render-time surprise into
/// something the patch is told at compile time.
pub const MAX_DELAY: f32 = 2.0;

/// A biquad's response (§13.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterKind {
    /// Passes below the cutoff.
    LowPass,
    /// Passes above the cutoff.
    HighPass,
}

/// The processor a node runs. Every variant knows its static port list and
/// parameter descriptors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessorSpec {
    /// Sine oscillator, phase-continuous (§13.5). Params: `frequency` (Hz).
    /// Output: mono audio.
    Sine,
    /// Deterministic noise (xorshift; fixed seed per node). Output: mono.
    Noise,
    /// A constant control value. Params: `value`. Output: control.
    Constant,
    /// Gain stage. Params: `gain` (linear). In: mono; out: mono.
    Gain,
    /// Constant-power panner. Params: `pan` (-1..=1). In: mono; out: stereo.
    Pan,
    /// Stereo mixer with `inputs` inputs (all stereo, at most 8). Out:
    /// stereo.
    Mixer {
        /// Number of stereo inputs (1–8).
        inputs: u8,
    },
    /// One input copied to two outputs (mono).
    Splitter,
    /// Mono → stereo adapter (duplicate) (§13.4's explicit adapters).
    MonoToStereo,
    /// Stereo → mono adapter (average).
    StereoToMono,
    /// Stereo gain stage. Params: `gain` (linear). In/out: stereo.
    ///
    /// The mono [`ProcessorSpec::Gain`] sits inside a voice; this one sits on
    /// a bus or a send, where the signal has already been panned.
    StereoGain,
    /// Identity: input copied to output, `channels` wide.
    ///
    /// This is what a studio stage whose DSP has not been written yet lowers
    /// to. The graph keeps its real shape — the node is
    /// there, connected where the patch says — so replacing it later changes
    /// one match arm and no topology.
    Passthrough {
        /// Channel count of the signal passing through.
        channels: u8,
    },
    /// Polyphonic synthesizer (§13.5: voice allocator → oscillator bank →
    /// per-voice ADSR). Params: `attack`, `decay`, `sustain`, `release`,
    /// and the bank's second partial (`ratio`, `blend`). Input: note events;
    /// output: stereo.
    PolySine {
        /// Voice-pool size.
        voices: u8,
    },
    /// Low-frequency oscillator at control rate. Params: `frequency` (Hz).
    /// Output: control.
    ///
    /// One value per block, held across it (§13.6's control rate): a
    /// modulation target reads one value per block anyway, so computing a
    /// sample-rate sine for it would buy nothing but cycles.
    Lfo {
        /// Its shape.
        waveform: Waveform,
    },
    /// Multiply a control signal. Params: `factor`. In/out: control.
    Scale,
    /// Offset a control signal. Params: `offset`. In/out: control.
    Bias,
    /// Bound a control signal. Params: `min`, `max`. In/out: control.
    Clamp,
    /// Slew-limit a control signal with a one-pole. Params: `time` (s).
    /// In/out: control.
    Smooth,
    /// Biquad filter (RBJ cookbook). Params: `cutoff` (Hz), `q`. In/out:
    /// stereo; the two channels filter independently.
    Biquad {
        /// Which response.
        kind: FilterKind,
    },
    /// Delay with feedback. Params: `time` (s, up to [`MAX_DELAY`]),
    /// `feedback` (linear), `mix` (dry/wet). In/out: stereo.
    ///
    /// This is the only processor a graph cycle may pass through (§13.3): the
    /// signal that comes back has been delayed by an amount the patch wrote,
    /// so the feedback is causal and its period is visible.
    Delay,
    /// Chorus: a short delay modulated by an LFO. Params: `rate` (Hz),
    /// `depth` (s), `mix`. In/out: stereo.
    Chorus,
    /// Algorithmic reverb (comb + allpass network). Params: `room`,
    /// `damping`, `mix`. In/out: stereo.
    Reverb,
    /// Peak limiter. Params: `ceiling` (linear, 1.0 is 0 dBFS). In/out:
    /// stereo.
    ///
    /// The last node on master, always: what leaves the graph is bounded
    /// whatever the mix asked for (§13.6).
    Limiter,
    /// One-pole low-pass. Params: `cutoff` (Hz). In/out: stereo.
    ///
    /// Gentler than the biquad (6 dB/octave, no resonance) and cheaper; it is
    /// what a patch wants when it means "take the edge off" rather than
    /// "filter".
    OnePole,
}

impl ProcessorSpec {
    /// Input port kinds, in index order.
    pub fn input_ports(&self) -> Vec<PortKind> {
        match self {
            Self::Sine | Self::Noise | Self::Constant | Self::Lfo { .. } => Vec::new(),
            Self::PolySine { .. } => vec![PortKind::NoteEvents],
            Self::Gain | Self::Splitter | Self::MonoToStereo | Self::Pan => vec![PortKind::Audio { channels: 1 }],
            Self::StereoGain
            | Self::Biquad { .. }
            | Self::OnePole
            | Self::Delay
            | Self::Chorus
            | Self::Reverb
            | Self::Limiter => vec![PortKind::Audio { channels: 2 }],
            Self::Scale | Self::Bias | Self::Clamp | Self::Smooth => vec![PortKind::Control],
            Self::Passthrough { channels } => vec![PortKind::Audio { channels: *channels }],
            Self::Mixer { inputs } => vec![PortKind::Audio { channels: 2 }; usize::from(*inputs)],
            Self::StereoToMono => vec![PortKind::Audio { channels: 2 }],
        }
    }

    /// Output port kinds, in index order.
    pub fn output_ports(&self) -> Vec<PortKind> {
        match self {
            Self::Sine | Self::Noise | Self::Gain => vec![PortKind::Audio { channels: 1 }],
            Self::Splitter => vec![PortKind::Audio { channels: 1 }, PortKind::Audio { channels: 1 }],
            Self::Constant | Self::Lfo { .. } | Self::Scale | Self::Bias | Self::Clamp | Self::Smooth => {
                vec![PortKind::Control]
            }
            Self::Pan
            | Self::MonoToStereo
            | Self::Mixer { .. }
            | Self::PolySine { .. }
            | Self::StereoGain
            | Self::Biquad { .. }
            | Self::OnePole
            | Self::Delay
            | Self::Chorus
            | Self::Reverb
            | Self::Limiter => {
                vec![PortKind::Audio { channels: 2 }]
            }
            Self::Passthrough { channels } => vec![PortKind::Audio { channels: *channels }],
            Self::StereoToMono => vec![PortKind::Audio { channels: 1 }],
        }
    }

    /// Parameter descriptors, in declaration order.
    pub fn parameters(&self) -> &'static [ParameterDescriptor] {
        const FREQUENCY: ParameterDescriptor = ParameterDescriptor {
            name: "frequency",
            unit: Unit::Hz,
            range: (0.0, 20_000.0),
            default: 440.0,
            smoothing: Smoothing::BlockRamp,
            combination: Combination::Replace,
        };
        const VALUE: ParameterDescriptor = ParameterDescriptor {
            name: "value",
            unit: Unit::Linear,
            range: (f32::NEG_INFINITY, f32::INFINITY),
            default: 1.0,
            smoothing: Smoothing::None,
            combination: Combination::Replace,
        };
        const GAIN: ParameterDescriptor = ParameterDescriptor {
            name: "gain",
            unit: Unit::Linear,
            range: (0.0, 16.0),
            default: 1.0,
            smoothing: Smoothing::BlockRamp,
            combination: Combination::Multiply,
        };
        const PAN: ParameterDescriptor = ParameterDescriptor {
            name: "pan",
            unit: Unit::Linear,
            range: (-1.0, 1.0),
            default: 0.0,
            smoothing: Smoothing::BlockRamp,
            combination: Combination::Replace,
        };
        // The names and units here are the ones the language writes
        // (`Processor::params`): `cutoff` is `Hz` on both sides, so a
        // written `1400 Hz` needs no translation to reach this descriptor.
        // `gain` is the one deliberate exception — written in dB, held here
        // as the linear multiplier the DSP applies (§2: a marking is not a
        // number of decibels, and neither is a decibel a coefficient).
        const CUTOFF: ParameterDescriptor = ParameterDescriptor {
            name: "cutoff",
            unit: Unit::Hz,
            range: (10.0, 20_000.0),
            default: 20_000.0,
            smoothing: Smoothing::BlockRamp,
            combination: Combination::Replace,
        };
        const Q: ParameterDescriptor = ParameterDescriptor {
            name: "q",
            unit: Unit::Linear,
            range: (0.05, 20.0),
            default: std::f32::consts::FRAC_1_SQRT_2,
            smoothing: Smoothing::BlockRamp,
            combination: Combination::Replace,
        };
        const FILTER: &[ParameterDescriptor] = &[CUTOFF, Q];
        // A filter that says nothing filters nothing, which for a high-pass
        // is the bottom of its range rather than the top.
        const HIGH_PASS: &[ParameterDescriptor] = &[
            ParameterDescriptor {
                default: 20.0,
                ..CUTOFF
            },
            Q,
        ];
        const ONE_POLE: &[ParameterDescriptor] = &[CUTOFF];
        const LFO: &[ParameterDescriptor] = &[ParameterDescriptor {
            name: "frequency",
            unit: Unit::Hz,
            range: (0.0, 200.0),
            default: 1.0,
            smoothing: Smoothing::None,
            combination: Combination::Replace,
        }];
        // The oscillator bank is two partials: the note, and one at a ratio
        // of it. `blend: 0` is one sine, which is what a patch that says
        // nothing about partials gets — and what keeps the default
        // instrument the one it has always been.
        const VOICE: &[ParameterDescriptor] = &[
            ParameterDescriptor {
                name: "attack",
                unit: Unit::Seconds,
                range: (0.0, 20.0),
                default: 0.005,
                smoothing: Smoothing::None,
                combination: Combination::Replace,
            },
            ParameterDescriptor {
                name: "decay",
                unit: Unit::Seconds,
                range: (0.0, 20.0),
                default: 0.0,
                smoothing: Smoothing::None,
                combination: Combination::Replace,
            },
            ParameterDescriptor {
                name: "sustain",
                unit: Unit::Linear,
                range: (0.0, 1.0),
                default: 1.0,
                smoothing: Smoothing::None,
                combination: Combination::Replace,
            },
            ParameterDescriptor {
                name: "release",
                unit: Unit::Seconds,
                range: (0.0, 60.0),
                default: 0.05,
                smoothing: Smoothing::None,
                combination: Combination::Replace,
            },
            ParameterDescriptor {
                name: "ratio",
                unit: Unit::Linear,
                range: (0.0, 32.0),
                default: 2.0,
                smoothing: Smoothing::None,
                combination: Combination::Replace,
            },
            ParameterDescriptor {
                name: "blend",
                unit: Unit::Linear,
                range: (0.0, 1.0),
                default: 0.0,
                smoothing: Smoothing::None,
                combination: Combination::Replace,
            },
        ];
        // Control stages carry `Hz` for the same reason the language does:
        // a control signal's dimension is its target's, and every target
        // today is a cutoff (see `musa_compiler::Processor::params`).
        const FACTOR: &[ParameterDescriptor] = &[ParameterDescriptor {
            name: "factor",
            unit: Unit::Hz,
            range: (-100_000.0, 100_000.0),
            default: 1.0,
            smoothing: Smoothing::BlockRamp,
            combination: Combination::Replace,
        }];
        const OFFSET: &[ParameterDescriptor] = &[ParameterDescriptor {
            name: "offset",
            unit: Unit::Hz,
            range: (-100_000.0, 100_000.0),
            default: 0.0,
            smoothing: Smoothing::BlockRamp,
            combination: Combination::Replace,
        }];
        const BOUNDS: &[ParameterDescriptor] = &[
            ParameterDescriptor {
                name: "min",
                unit: Unit::Hz,
                range: (-100_000.0, 100_000.0),
                default: 0.0,
                smoothing: Smoothing::None,
                combination: Combination::Replace,
            },
            ParameterDescriptor {
                name: "max",
                unit: Unit::Hz,
                range: (-100_000.0, 100_000.0),
                default: 20_000.0,
                smoothing: Smoothing::None,
                combination: Combination::Replace,
            },
        ];
        // The wet/dry balance every time effect carries. Multiplying a mix
        // rather than replacing it is what lets one modulation fade an effect
        // in without the patch losing the balance it was written with.
        const MIX: ParameterDescriptor = ParameterDescriptor {
            name: "mix",
            unit: Unit::Linear,
            range: (0.0, 1.0),
            default: 0.3,
            smoothing: Smoothing::BlockRamp,
            combination: Combination::Multiply,
        };
        const DELAY: &[ParameterDescriptor] = &[
            ParameterDescriptor {
                name: "time",
                unit: Unit::Seconds,
                range: (0.0, MAX_DELAY),
                default: 0.25,
                smoothing: Smoothing::BlockRamp,
                combination: Combination::Replace,
            },
            // Feedback stops short of one: at one a delay never decays, and
            // an instrument that never stops is not an effect.
            ParameterDescriptor {
                name: "feedback",
                unit: Unit::Linear,
                range: (0.0, 0.95),
                default: 0.3,
                smoothing: Smoothing::BlockRamp,
                combination: Combination::Replace,
            },
            MIX,
        ];
        const CHORUS: &[ParameterDescriptor] = &[
            ParameterDescriptor {
                name: "rate",
                unit: Unit::Hz,
                range: (0.0, 20.0),
                default: 0.6,
                smoothing: Smoothing::BlockRamp,
                combination: Combination::Replace,
            },
            ParameterDescriptor {
                name: "depth",
                unit: Unit::Seconds,
                range: (0.0, 0.01),
                default: 0.004,
                smoothing: Smoothing::BlockRamp,
                combination: Combination::Replace,
            },
            ParameterDescriptor { default: 0.4, ..MIX },
        ];
        const REVERB: &[ParameterDescriptor] = &[
            ParameterDescriptor {
                name: "room",
                unit: Unit::Linear,
                range: (0.0, 1.0),
                default: 0.5,
                smoothing: Smoothing::BlockRamp,
                combination: Combination::Replace,
            },
            ParameterDescriptor {
                name: "damping",
                unit: Unit::Linear,
                range: (0.0, 1.0),
                default: 0.5,
                smoothing: Smoothing::BlockRamp,
                combination: Combination::Replace,
            },
            // A bus that exists to be a reverb is all reverb; a reverb in a
            // patch's chain is the one that wants a balance, and says so.
            ParameterDescriptor { default: 1.0, ..MIX },
        ];
        const LIMITER: &[ParameterDescriptor] = &[ParameterDescriptor {
            name: "ceiling",
            unit: Unit::Linear,
            range: (0.0, 1.0),
            default: 1.0,
            smoothing: Smoothing::None,
            combination: Combination::Replace,
        }];
        const TIME: &[ParameterDescriptor] = &[ParameterDescriptor {
            name: "time",
            unit: Unit::Seconds,
            range: (0.0, 10.0),
            default: 0.02,
            smoothing: Smoothing::None,
            combination: Combination::Replace,
        }];
        match self {
            Self::Sine => &[FREQUENCY],
            Self::Constant => &[VALUE],
            Self::Gain | Self::StereoGain => &[GAIN],
            Self::Pan => &[PAN],
            Self::PolySine { .. } => VOICE,
            Self::Lfo { .. } => LFO,
            Self::Biquad {
                kind: FilterKind::LowPass,
            } => FILTER,
            Self::Biquad {
                kind: FilterKind::HighPass,
            } => HIGH_PASS,
            Self::OnePole => ONE_POLE,
            Self::Scale => FACTOR,
            Self::Bias => OFFSET,
            Self::Clamp => BOUNDS,
            Self::Smooth => TIME,
            Self::Delay => DELAY,
            Self::Chorus => CHORUS,
            Self::Reverb => REVERB,
            Self::Limiter => LIMITER,
            Self::Noise
            | Self::Passthrough { .. }
            | Self::Mixer { .. }
            | Self::Splitter
            | Self::MonoToStereo
            | Self::StereoToMono => &[],
        }
    }

    /// A named parameter's contract, if it declares one.
    pub(crate) fn descriptor(self, name: &str) -> Option<ParameterDescriptor> {
        self.parameters().iter().copied().find(|p| p.name == name)
    }
}

/// A declarative studio graph: nodes, connections, parameters. Editable;
/// compiled into a `RenderPlan` by `compile_graph`.
#[derive(Clone, Debug, Default)]
pub struct StudioGraphSpec {
    nodes: Vec<Node>,
    connections: Vec<Connection>,
    modulations: Vec<ModulationEdge>,
    output: Option<NodeId>,
}

#[derive(Clone, Debug)]
pub(crate) struct Node {
    id: NodeId,
    processor: ProcessorSpec,
    params: Vec<(&'static str, f32)>,
}

/// A control connection into a *parameter* rather than a port (§13.7).
///
/// Modulation is typed: the source must produce control, the target must
/// declare the parameter, and what happens to the value on arrival is the
/// parameter's business (its `combination`, `range`, and `smoothing`) rather
/// than the connection's. That is what keeps `modulate` from becoming an
/// anonymous 0..1 wire into an unknown quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModulationEdge {
    pub(crate) from: NodeId,
    pub(crate) from_port: usize,
    pub(crate) to: NodeId,
    pub(crate) param: &'static str,
}

/// One connection between output and input ports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Connection {
    pub(crate) from: NodeId,
    pub(crate) from_port: usize,
    pub(crate) to: NodeId,
    pub(crate) to_port: usize,
}

impl StudioGraphSpec {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node; returns its identity.
    pub fn add_node(&mut self, processor: ProcessorSpec) -> NodeId {
        let id = NodeId(u32::try_from(self.nodes.len()).unwrap_or(u32::MAX));
        self.nodes.push(Node {
            id,
            processor,
            params: Vec::new(),
        });
        id
    }

    /// Set a parameter on a node (clamped to the descriptor's range at
    /// compile time).
    ///
    /// # Errors
    /// [`GraphError::UnknownNode`] / [`GraphError::InvalidParameter`].
    pub fn set_param(&mut self, node: NodeId, name: &'static str, value: f32) -> Result<(), crate::GraphError> {
        let Some(entry) = self.nodes.iter_mut().find(|entry| entry.id == node) else {
            return Err(crate::GraphError::UnknownNode(node));
        };
        if entry.processor.descriptor(name).is_none() || !value.is_finite() {
            return Err(crate::GraphError::InvalidParameter {
                node,
                name: name.to_string(),
            });
        }
        if let Some(param) = entry.params.iter_mut().find(|(key, _)| *key == name) {
            param.1 = value;
        } else {
            entry.params.push((name, value));
        }
        Ok(())
    }

    /// Connect an output port to an input port.
    pub fn connect(&mut self, from: NodeId, from_port: usize, to: NodeId, to_port: usize) {
        self.connections.push(Connection {
            from,
            from_port,
            to,
            to_port,
        });
    }

    /// Connect a control output to a named parameter of another node.
    ///
    /// Both ends are checked in `compile_graph`, not here, so a spec under
    /// construction can be written in any order.
    pub fn modulate(&mut self, from: NodeId, from_port: usize, to: NodeId, param: &'static str) {
        self.modulations.push(ModulationEdge {
            from,
            from_port,
            to,
            param,
        });
    }

    /// Designate the graph's master output node (its first output port must
    /// be stereo audio, or mono audio adapted via `MonoToStereo`).
    pub fn set_output(&mut self, node: NodeId) {
        self.output = Some(node);
    }

    pub(crate) fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub(crate) fn connections(&self) -> &[Connection] {
        &self.connections
    }

    pub(crate) fn modulations(&self) -> &[ModulationEdge] {
        &self.modulations
    }

    pub(crate) fn output(&self) -> Option<NodeId> {
        self.output
    }

    pub(crate) fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|entry| entry.id == id)
    }

    pub(crate) fn processor_of(&self, id: NodeId) -> Option<ProcessorSpec> {
        self.node(id).map(|node| node.processor)
    }

    pub(crate) fn params_of(&self, id: NodeId) -> &[(&'static str, f32)] {
        self.node(id).map_or(&[], |node| node.params.as_slice())
    }

    pub(crate) fn param_value(&self, id: NodeId, descriptor: &ParameterDescriptor) -> f32 {
        self.params_of(id)
            .iter()
            .find(|(key, _)| *key == descriptor.name)
            .map_or(descriptor.default, |(_, value)| *value)
            .clamp(descriptor.range.0, descriptor.range.1)
    }
}

impl Node {
    pub(crate) fn id(&self) -> NodeId {
        self.id
    }

    pub(crate) fn processor(&self) -> ProcessorSpec {
        self.processor
    }
}

/// Options for graph compilation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphOptions {
    /// Samples per second.
    pub sample_rate: u32,
    /// Frames per processing block (render works in blocks of this size).
    pub block_size: usize,
}

impl Default for GraphOptions {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            block_size: 128,
        }
    }
}
