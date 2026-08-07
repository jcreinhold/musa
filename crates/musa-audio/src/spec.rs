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
/// two cannot drift apart (prompt 29).
pub use musa_compiler::Unit;

/// Parameter smoothing applied to value changes (§13.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Smoothing {
    /// No smoothing (step change).
    None,
    /// Linear ramp over one block.
    BlockRamp,
}

/// How a modulation signal combines with the base value (§13.7; sources
/// arrive in prompt 25).
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
    /// to (prompts 30–31). The graph keeps its real shape — the node is
    /// there, connected where the patch says — so replacing it later changes
    /// one match arm and no topology.
    Passthrough {
        /// Channel count of the signal passing through.
        channels: u8,
    },
    /// Polyphonic sine synthesizer (§13.5: voice allocator → oscillator
    /// bank → placeholder envelope). Input: note events; output: stereo.
    PolySine {
        /// Voice-pool size.
        voices: u8,
    },
}

impl ProcessorSpec {
    /// Input port kinds, in index order.
    pub fn input_ports(&self) -> Vec<PortKind> {
        match self {
            Self::Sine | Self::Noise | Self::Constant => Vec::new(),
            Self::PolySine { .. } => vec![PortKind::NoteEvents],
            Self::Gain | Self::Splitter | Self::MonoToStereo | Self::Pan => vec![PortKind::Audio { channels: 1 }],
            Self::StereoGain => vec![PortKind::Audio { channels: 2 }],
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
            Self::Constant => vec![PortKind::Control],
            Self::Pan | Self::MonoToStereo | Self::Mixer { .. } | Self::PolySine { .. } | Self::StereoGain => {
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
        match self {
            Self::Sine => &[FREQUENCY],
            Self::Constant => &[VALUE],
            Self::Gain | Self::StereoGain => &[GAIN],
            Self::Pan => &[PAN],
            Self::Noise
            | Self::Passthrough { .. }
            | Self::Mixer { .. }
            | Self::Splitter
            | Self::MonoToStereo
            | Self::StereoToMono
            | Self::PolySine { .. } => &[],
        }
    }

    /// The default value of a named parameter, if it exists.
    fn parameter_default(self, name: &str) -> Option<ParameterDescriptor> {
        self.parameters().iter().copied().find(|p| p.name == name)
    }
}

/// A declarative studio graph: nodes, connections, parameters. Editable;
/// compiled into a `RenderPlan` by `compile_graph`.
#[derive(Clone, Debug, Default)]
pub struct StudioGraphSpec {
    nodes: Vec<Node>,
    connections: Vec<Connection>,
    output: Option<NodeId>,
}

#[derive(Clone, Debug)]
pub(crate) struct Node {
    id: NodeId,
    processor: ProcessorSpec,
    params: Vec<(&'static str, f32)>,
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
        if entry.processor.parameter_default(name).is_none() || !value.is_finite() {
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
