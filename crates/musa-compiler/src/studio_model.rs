//! Private CST-resolution records for the compatibility `studio` spelling.
//!
//! Written values retain units and source spans while the spelling is
//! translated to ordinary `std::sound` constructors. None of these types cross
//! the compiler facade or constitute a second sound-language API.

use indexmap::IndexMap;
use num_rational::Ratio;

use musa_score::origin::SourceSpan;

/// Unit token retained while a compatibility literal is printed as source.
/// `std::sound::quantity::SoundUnit` remains authoritative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceUnit {
    /// Hertz.
    Hz,
    /// Dimensionless ratio.
    Linear,
    /// Decibels.
    Decibels,
    /// Seconds.
    Seconds,
}

impl SurfaceUnit {
    /// How the unit is written in source, or `None` for a bare number.
    pub(crate) fn spelling(self) -> Option<&'static str> {
        match self {
            Self::Hz => Some("Hz"),
            Self::Linear => None,
            Self::Decibels => Some("dB"),
            Self::Seconds => Some("s"),
        }
    }
}

/// Exact compatibility literal retained only until checked-source construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceQuantity {
    /// The exact magnitude, normalized to the unit's base (`ms` becomes
    /// seconds). Decimal syntax denotes its decimal rational exactly.
    pub(crate) magnitude: Ratio<i64>,
    /// The dimension it carries.
    pub(crate) unit: SurfaceUnit,
}

impl SurfaceQuantity {
    /// Construct an exact normalized quantity.
    pub(crate) const fn new(magnitude: Ratio<i64>, unit: SurfaceUnit) -> Self {
        Self { magnitude, unit }
    }
}

/// A processor the studio language can name.
///
/// Deliberately a closed set: §7.2 forbids raw backend escapes, so a patch
/// can only say things the compiler understands and can check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SurfaceProcessor {
    /// `oscillator(sine, frequency: 220 Hz)`
    Oscillator,
    /// `gain(-15 dB)`
    Gain,
    /// `mix(a, b, ...)`
    Mix,
    /// `envelope(adsr(...))`
    Envelope,
    /// `lowpass(cutoff: 1400 Hz, resonance: 0.7)`
    Lowpass,
    /// `highpass(cutoff: 80 Hz, resonance: 0.7)`
    Highpass,
    /// `reverb(room: 0.82, damping: 0.55)`
    Reverb,
    /// `delay(time: 250 ms, feedback: 0.4, mix: 0.3)`
    Delay,
    /// `chorus(rate: 0.6 Hz, depth: 4 ms, mix: 0.4)`
    Chorus,
    /// `scale(250 Hz)` — multiply a control signal.
    Scale,
    /// `bias(1400 Hz)` — offset a control signal.
    Bias,
    /// `clamp(min: 200 Hz, max: 6000 Hz)` — bound a control signal.
    Clamp,
    /// `smoothing(time: 20 ms)` — slew-limit a control signal.
    Smoothing,
}

/// One declared parameter of a processor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceParam {
    /// The name it is written with.
    pub(crate) name: &'static str,
    /// Stable private render-graph key. Usually the public name; filter
    /// `resonance` deliberately maps to the conventional DSP key `q`.
    pub(crate) dsp_name: &'static str,
    /// Removed or historical spellings, for diagnostics only. These are not
    /// accepted by [`SurfaceProcessor::param`].
    pub(crate) former_names: &'static [&'static str],
    /// Plain musician-facing description.
    pub(crate) summary: &'static str,
    /// The unit it must be written in.
    pub(crate) unit: SurfaceUnit,
    /// Its value when the patch does not say.
    pub(crate) default: Ratio<i64>,
    /// The range a control may write, in the unit above — a decibel gain runs
    /// from −60 to +12, not from 0 to 1.
    ///
    /// This is the *writable* range, which is not the DSP's clamp: a graph
    /// parameter's private preparation descriptor bounds what the processor will
    /// accept in linear terms, and this bounds what a composer means by
    /// turning a knob all the way up. They answer different questions, and a
    /// slider needs this one.
    pub(crate) range: (Ratio<i64>, Ratio<i64>),
}

impl SurfaceProcessor {
    /// The processor a written name denotes.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "oscillator" => Some(Self::Oscillator),
            "gain" => Some(Self::Gain),
            "mix" => Some(Self::Mix),
            "envelope" => Some(Self::Envelope),
            "lowpass" => Some(Self::Lowpass),
            "highpass" => Some(Self::Highpass),
            "reverb" => Some(Self::Reverb),
            "delay" => Some(Self::Delay),
            "chorus" => Some(Self::Chorus),
            "scale" => Some(Self::Scale),
            "bias" => Some(Self::Bias),
            "clamp" => Some(Self::Clamp),
            "smoothing" => Some(Self::Smoothing),
            _ => None,
        }
    }

    /// How it is written.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Oscillator => "oscillator",
            Self::Gain => "gain",
            Self::Mix => "mix",
            Self::Envelope => "envelope",
            Self::Lowpass => "lowpass",
            Self::Highpass => "highpass",
            Self::Reverb => "reverb",
            Self::Delay => "delay",
            Self::Chorus => "chorus",
            Self::Scale => "scale",
            Self::Bias => "bias",
            Self::Clamp => "clamp",
            Self::Smoothing => "smoothing",
        }
    }

    /// Its parameters, in declaration order. A positional argument binds to
    /// the first parameter, so the order is part of the contract.
    ///
    /// The control stages (`scale`, `bias`, `clamp`) declare their values in
    /// `Hz` because a control signal's dimension is really its modulation
    /// target's, and today the only target anyone modulates is a cutoff.
    /// Inferring the unit from the target is a change to the unit system, not
    /// to these tables, and it waits for a second target to justify it.
    pub(crate) const fn params(self) -> &'static [SurfaceParam] {
        const fn spec(
            name: &'static str,
            dsp_name: &'static str,
            former_names: &'static [&'static str],
            unit: SurfaceUnit,
            default: Ratio<i64>,
            range: (Ratio<i64>, Ratio<i64>),
            summary: &'static str,
        ) -> SurfaceParam {
            SurfaceParam {
                name,
                dsp_name,
                former_names,
                summary,
                unit,
                default,
                range,
            }
        }
        const fn r(numerator: i64, denominator: i64) -> Ratio<i64> {
            Ratio::new_raw(numerator, denominator)
        }
        const OSCILLATOR: &[SurfaceParam] = &[
            spec(
                "frequency",
                "frequency",
                &[],
                SurfaceUnit::Hz,
                r(1, 1),
                (r(0, 1), r(200, 1)),
                "Sets a control oscillator's frequency; a patch oscillator follows score pitch.",
            ),
            spec(
                "ratio",
                "ratio",
                &[],
                SurfaceUnit::Linear,
                r(1, 1),
                (r(1, 4), r(16, 1)),
                "Scales the pitch supplied by the score.",
            ),
        ];
        const GAIN: &[SurfaceParam] = &[spec(
            "gain",
            "gain",
            &[],
            SurfaceUnit::Decibels,
            r(0, 1),
            (r(-60, 1), r(12, 1)),
            "Sets level in decibels.",
        )];
        // Written defaults are the built-in voice envelope, so `envelope()`
        // with nothing said is not a different sound from saying nothing.
        const ENVELOPE: &[SurfaceParam] = &[
            spec(
                "attack",
                "attack",
                &[],
                SurfaceUnit::Seconds,
                r(1, 200),
                (r(0, 1), r(5, 1)),
                "Sets the rise time after a note begins.",
            ),
            spec(
                "decay",
                "decay",
                &[],
                SurfaceUnit::Seconds,
                r(0, 1),
                (r(0, 1), r(10, 1)),
                "Sets the time to reach the sustain level.",
            ),
            spec(
                "sustain",
                "sustain",
                &[],
                SurfaceUnit::Linear,
                r(1, 1),
                (r(0, 1), r(1, 1)),
                "Sets the held level while a note continues.",
            ),
            spec(
                "release",
                "release",
                &[],
                SurfaceUnit::Seconds,
                r(1, 20),
                (r(0, 1), r(10, 1)),
                "Sets the fade time after a note ends.",
            ),
        ];
        const LOWPASS: &[SurfaceParam] = &[
            spec(
                "cutoff",
                "cutoff",
                &[],
                SurfaceUnit::Hz,
                r(20_000, 1),
                (r(20, 1), r(20_000, 1)),
                "Sets the boundary frequency.",
            ),
            spec(
                "resonance",
                "q",
                &["q"],
                SurfaceUnit::Linear,
                r(1_767_766_952_966_369, 2_500_000_000_000_000),
                (r(1, 10), r(20, 1)),
                "Emphasizes the cutoff, conventionally represented by quality factor Q.",
            ),
        ];
        const HIGHPASS: &[SurfaceParam] = &[
            spec(
                "cutoff",
                "cutoff",
                &[],
                SurfaceUnit::Hz,
                r(20, 1),
                (r(20, 1), r(20_000, 1)),
                "Sets the boundary frequency.",
            ),
            spec(
                "resonance",
                "q",
                &["q"],
                SurfaceUnit::Linear,
                r(1_767_766_952_966_369, 2_500_000_000_000_000),
                (r(1, 10), r(20, 1)),
                "Emphasizes the cutoff, conventionally represented by quality factor Q.",
            ),
        ];
        const REVERB: &[SurfaceParam] = &[
            spec(
                "room",
                "room",
                &[],
                SurfaceUnit::Linear,
                r(1, 2),
                (r(0, 1), r(1, 1)),
                "Sets the apparent room size.",
            ),
            spec(
                "damping",
                "damping",
                &[],
                SurfaceUnit::Linear,
                r(1, 2),
                (r(0, 1), r(1, 1)),
                "Controls high-frequency absorption.",
            ),
            spec(
                "mix",
                "mix",
                &[],
                SurfaceUnit::Linear,
                r(1, 1),
                (r(0, 1), r(1, 1)),
                "Balances dry and processed audio.",
            ),
        ];
        // Time effects (§13.6). `mix` is the dry/wet balance: 0 is the input
        // untouched, 1 is the effect alone. A delay's `time` is bounded by
        // the line the DSP preallocates (2 s), and a chorus's `depth` by what
        // a chorus is — a few milliseconds of wobble, not a second one.
        const DELAY: &[SurfaceParam] = &[
            spec(
                "time",
                "time",
                &[],
                SurfaceUnit::Seconds,
                r(1, 4),
                (r(0, 1), r(2, 1)),
                "Sets the interval before each repeat.",
            ),
            spec(
                "feedback",
                "feedback",
                &[],
                SurfaceUnit::Linear,
                r(3, 10),
                (r(0, 1), r(19, 20)),
                "Sets how much delayed sound repeats.",
            ),
            spec(
                "mix",
                "mix",
                &[],
                SurfaceUnit::Linear,
                r(3, 10),
                (r(0, 1), r(1, 1)),
                "Balances dry and processed audio.",
            ),
        ];
        const CHORUS: &[SurfaceParam] = &[
            spec(
                "rate",
                "rate",
                &[],
                SurfaceUnit::Hz,
                r(3, 5),
                (r(0, 1), r(20, 1)),
                "Sets how quickly the doubled voice moves.",
            ),
            spec(
                "depth",
                "depth",
                &[],
                SurfaceUnit::Seconds,
                r(1, 250),
                (r(0, 1), r(1, 100)),
                "Sets the maximum delay variation.",
            ),
            spec(
                "mix",
                "mix",
                &[],
                SurfaceUnit::Linear,
                r(2, 5),
                (r(0, 1), r(1, 1)),
                "Balances dry and processed audio.",
            ),
        ];
        const SCALE: &[SurfaceParam] = &[spec(
            "factor",
            "factor",
            &[],
            SurfaceUnit::Hz,
            r(1, 1),
            (r(0, 1), r(20_000, 1)),
            "Multiplies each control value.",
        )];
        const BIAS: &[SurfaceParam] = &[spec(
            "offset",
            "offset",
            &[],
            SurfaceUnit::Hz,
            r(0, 1),
            (r(0, 1), r(20_000, 1)),
            "Adds to each control value.",
        )];
        const CLAMP: &[SurfaceParam] = &[
            spec(
                "min",
                "min",
                &[],
                SurfaceUnit::Hz,
                r(0, 1),
                (r(0, 1), r(20_000, 1)),
                "Sets the lowest output value.",
            ),
            spec(
                "max",
                "max",
                &[],
                SurfaceUnit::Hz,
                r(20_000, 1),
                (r(0, 1), r(20_000, 1)),
                "Sets the highest output value.",
            ),
        ];
        const SMOOTHING: &[SurfaceParam] = &[spec(
            "time",
            "time",
            &[],
            SurfaceUnit::Seconds,
            r(1, 50),
            (r(0, 1), r(1, 1)),
            "Sets how quickly the control catches its target.",
        )];
        match self {
            Self::Oscillator => OSCILLATOR,
            Self::Gain => GAIN,
            Self::Mix => &[],
            Self::Envelope => ENVELOPE,
            Self::Lowpass => LOWPASS,
            Self::Highpass => HIGHPASS,
            Self::Reverb => REVERB,
            Self::Delay => DELAY,
            Self::Chorus => CHORUS,
            Self::Scale => SCALE,
            Self::Bias => BIAS,
            Self::Clamp => CLAMP,
            Self::Smoothing => SMOOTHING,
        }
    }

    /// A named parameter's declaration.
    pub(crate) fn param(self, name: &str) -> Option<SurfaceParam> {
        self.params().iter().copied().find(|param| param.name == name)
    }
}

/// A node's index within its patch.
pub(crate) type SurfaceNodeIndex = usize;

/// One processor instance inside a patch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceNode {
    /// What it runs.
    pub(crate) processor: SurfaceProcessor,
    /// The name it was bound to, if it was written as `name = ...`. This is
    /// what a `modulate` path addresses.
    pub(crate) label: Option<String>,
    /// Resolved parameter values, in the processor's declaration order.
    pub(crate) params: Vec<Option<SurfaceQuantity>>,
    /// Where each parameter's *written* value is, parallel to `params`, and
    /// `None` for a parameter the patch left to its default.
    ///
    /// A structured editor rewrites exactly this range and nothing else: a
    /// slider that regenerated the whole call would silently normalize
    /// `30 ms` to `0.03 s` and lose any comment inside it (§11 — the source
    /// is the document, and an edit to it should be the edit that was made).
    pub(crate) param_spans: Vec<Option<SourceSpan>>,
    /// The whole `name(args)` construction, so a parameter the patch never
    /// wrote can be *added* rather than only changed.
    pub(crate) span: Option<SourceSpan>,
    /// The nodes feeding it, in argument order.
    pub(crate) inputs: Vec<SurfaceNodeIndex>,
}

/// A patch or a bus: a graph of nodes with one designated output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SurfaceGraph {
    nodes: Vec<SurfaceNode>,
    output: Option<SurfaceNodeIndex>,
}

impl SurfaceGraph {
    /// Its nodes, in creation order.
    pub(crate) fn nodes(&self) -> &[SurfaceNode] {
        &self.nodes
    }

    /// The node whose signal leaves the patch.
    pub(crate) fn output(&self) -> Option<SurfaceNodeIndex> {
        self.output
    }

    #[doc(hidden)]
    pub(crate) fn push(&mut self, node: SurfaceNode) -> SurfaceNodeIndex {
        self.nodes.push(node);
        self.nodes.len().saturating_sub(1)
    }

    #[doc(hidden)]
    pub(crate) fn set_output(&mut self, node: SurfaceNodeIndex) {
        self.output = Some(node);
    }

    #[doc(hidden)]
    pub(crate) fn nodes_mut(&mut self) -> &mut Vec<SurfaceNode> {
        &mut self.nodes
    }
}

/// `modulate lfo -> glass_pad.lowpass.cutoff;` — a typed control connection
/// (§13.7), resolved to the node it addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceModulation {
    /// The modulating signal's name.
    pub(crate) source: String,
    /// The patch owning the modulated node.
    pub(crate) patch: String,
    /// Which node in that patch.
    pub(crate) node: SurfaceNodeIndex,
    /// Which of its parameters.
    pub(crate) param: &'static str,
}

/// `send violin -> hall at -18 dB;`
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceSend {
    /// The part or bus sending.
    pub(crate) source: String,
    /// The bus receiving.
    pub(crate) bus: String,
    /// How much of the signal is sent.
    pub(crate) level: SurfaceQuantity,
    /// Where the level was written, for an editor that rewrites it.
    pub(crate) level_span: Option<SourceSpan>,
    /// The complete statement, retained for binding diagnostics.
    pub(crate) span: Option<SourceSpan>,
}

/// `assign violin -> glass_pad;` — which patch realizes a part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceAssignment {
    /// The patch the part is realized by.
    pub(crate) patch: String,
    /// Where the patch *name* was written, so pointing a part at a different
    /// patch replaces a name rather than rewriting a statement.
    pub(crate) patch_span: Option<SourceSpan>,
}

/// `route violin -> master;`
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceRoute {
    /// The part or bus whose output is routed.
    pub(crate) source: String,
    /// Where it goes: a bus name, or `master`.
    pub(crate) destination: String,
    /// The complete statement, retained for binding diagnostics.
    pub(crate) span: Option<SourceSpan>,
}

/// The compiled studio: everything a graph builder needs, with every name
/// already resolved (§10.6).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SurfaceStudio {
    patches: IndexMap<String, SurfaceGraph>,
    buses: IndexMap<String, SurfaceGraph>,
    signals: IndexMap<String, SurfaceGraph>,
    assignments: IndexMap<String, SurfaceAssignment>,
    routes: Vec<SurfaceRoute>,
    sends: Vec<SurfaceSend>,
    modulations: Vec<SurfaceModulation>,
    span: Option<SourceSpan>,
}

impl SurfaceStudio {
    /// Move every place this spec points at back into the composer's own text
    /// (`crate::expand`).
    #[doc(hidden)]
    pub(crate) fn remap_spans(&mut self, map: &musa_score::origin::SourceMap) {
        self.span = map.maybe(self.span);
        for patch in self
            .patches
            .values_mut()
            .chain(self.buses.values_mut())
            .chain(self.signals.values_mut())
        {
            for node in &mut patch.nodes {
                node.span = map.maybe(node.span);
                for span in &mut node.param_spans {
                    *span = map.maybe(*span);
                }
            }
        }
        for assignment in self.assignments.values_mut() {
            assignment.patch_span = map.maybe(assignment.patch_span);
        }
        for send in &mut self.sends {
            send.level_span = map.maybe(send.level_span);
            send.span = map.maybe(send.span);
        }
        for route in &mut self.routes {
            route.span = map.maybe(route.span);
        }
    }

    /// A patch by name.
    pub(crate) fn patch(&self, name: &str) -> Option<&SurfaceGraph> {
        self.patches.get(name)
    }

    /// The declared patches, in source order.
    pub(crate) fn patches(&self) -> impl Iterator<Item = (&str, &SurfaceGraph)> {
        self.patches.iter().map(|(name, patch)| (name.as_str(), patch))
    }

    /// The declared buses, in source order.
    pub(crate) fn buses(&self) -> impl Iterator<Item = (&str, &SurfaceGraph)> {
        self.buses.iter().map(|(name, bus)| (name.as_str(), bus))
    }

    /// The top-level named signals (modulation sources), in source order.
    pub(crate) fn signals(&self) -> impl Iterator<Item = (&str, &SurfaceGraph)> {
        self.signals.iter().map(|(name, signal)| (name.as_str(), signal))
    }

    /// The part→patch assignments, in source order.
    pub(crate) fn assignments(&self) -> impl Iterator<Item = (&str, &str)> {
        self.assignments
            .iter()
            .map(|(part, assignment)| (part.as_str(), assignment.patch.as_str()))
    }

    /// One part's assignment, with the span an editor would rewrite to point
    /// it at a different patch (§11's `AssignPatch`).
    pub(crate) fn assignment(&self, part: &str) -> Option<&SurfaceAssignment> {
        self.assignments.get(part)
    }

    /// The declared routes, in source order.
    pub(crate) fn routes(&self) -> &[SurfaceRoute] {
        &self.routes
    }

    /// The declared sends, in source order.
    pub(crate) fn sends(&self) -> &[SurfaceSend] {
        &self.sends
    }

    /// The declared modulation connections, in source order.
    pub(crate) fn modulations(&self) -> &[SurfaceModulation] {
        &self.modulations
    }

    /// The `studio { … }` block itself, when the piece declared one. An
    /// editor that has to *add* a statement — assigning a part that no
    /// `assign` mentions — writes it inside this range.
    pub(crate) fn span(&self) -> Option<SourceSpan> {
        self.span
    }

    #[doc(hidden)]
    pub(crate) fn set_span(&mut self, span: Option<SourceSpan>) {
        self.span = span;
    }

    #[doc(hidden)]
    pub(crate) fn insert_patch(&mut self, name: String, patch: SurfaceGraph) -> bool {
        self.patches.insert(name, patch).is_none()
    }

    #[doc(hidden)]
    pub(crate) fn insert_bus(&mut self, name: String, bus: SurfaceGraph) -> bool {
        self.buses.insert(name, bus).is_none()
    }

    #[doc(hidden)]
    pub(crate) fn insert_signal(&mut self, name: String, signal: SurfaceGraph) -> bool {
        self.signals.insert(name, signal).is_none()
    }

    #[doc(hidden)]
    pub(crate) fn assign(&mut self, part: String, assignment: SurfaceAssignment) {
        self.assignments.insert(part, assignment);
    }

    #[doc(hidden)]
    pub(crate) fn push_route(&mut self, route: SurfaceRoute) {
        self.routes.push(route);
    }

    #[doc(hidden)]
    pub(crate) fn push_send(&mut self, send: SurfaceSend) {
        self.sends.push(send);
    }

    #[doc(hidden)]
    pub(crate) fn push_modulation(&mut self, modulation: SurfaceModulation) {
        self.modulations.push(modulation);
    }

    /// Whether a name denotes something signal-shaped: a bus, or a part that
    /// has been assigned a patch. Used to check `route` and `send` sources.
    #[doc(hidden)]
    pub(crate) fn is_routable(&self, name: &str) -> bool {
        self.buses.contains_key(name) || self.assignments.contains_key(name)
    }

    #[doc(hidden)]
    pub(crate) fn has_bus(&self, name: &str) -> bool {
        self.buses.contains_key(name)
    }

    #[doc(hidden)]
    pub(crate) fn has_patch(&self, name: &str) -> bool {
        self.patches.contains_key(name)
    }

    #[doc(hidden)]
    pub(crate) fn has_signal(&self, name: &str) -> bool {
        self.signals.contains_key(name)
    }
}
