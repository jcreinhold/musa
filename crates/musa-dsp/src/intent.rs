//! Temporary compatibility oracle for the pre-source studio path.
//!
//! Ordinary declarations in `std::sound` own the language. This Rust mirror is
//! retained only until prompt 180a can compare every accepted legacy case with
//! the checked-source path and delete it. New semantic callers and constructs
//! must use source declarations plus the opaque checked artifact.
//!
//! ```text
//! studio {
//!     patch glass_pad {
//!         carrier = oscillator(sine);
//!         mix(carrier, shimmer) |> lowpass(cutoff: 1400 Hz) |> output;
//!     }
//!     assign violin -> glass_pad;
//!     route violin -> master;
//! }
//! ```
//!
//! While it remains, `StudioSpec` is editable intent rather than a render plan:
//! written values retain units and source spans so differential tests can prove
//! that the source-owned replacement loses neither meaning nor edit identity.

use indexmap::IndexMap;
use num_rational::Ratio;

use musa_score::origin::SourceSpan;

/// Temporary unit mirror for the pre-source studio compatibility oracle.
///
/// `std::sound::quantity::SoundUnit` is authoritative. This enum remains only
/// for the legacy production path compared and deleted by prompt 180a; new
/// semantic callers must decode a checked source quantity instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// Hertz.
    Hz,
    /// Dimensionless ratio.
    Linear,
    /// Decibels.
    Decibels,
    /// Seconds.
    Seconds,
}

impl Unit {
    /// How the unit is written in source, or `None` for a bare number.
    pub fn spelling(self) -> Option<&'static str> {
        match self {
            Self::Hz => Some("Hz"),
            Self::Linear => None,
            Self::Decibels => Some("dB"),
            Self::Seconds => Some("s"),
        }
    }
}

/// Temporary exact quantity for the pre-source studio compatibility oracle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WrittenQuantity {
    /// The exact magnitude, normalized to the unit's base (`ms` becomes
    /// seconds). Decimal syntax denotes its decimal rational exactly.
    pub magnitude: Ratio<i64>,
    /// The dimension it carries.
    pub unit: Unit,
}

impl WrittenQuantity {
    /// Construct an exact normalized quantity.
    pub const fn new(magnitude: Ratio<i64>, unit: Unit) -> Self {
        Self { magnitude, unit }
    }
}

/// Write an exact catalogue quantity without introducing a floating value.
/// Terminating rationals use decimal notation; the rest use `numerator/denominator`.
pub fn written_ratio(value: Ratio<i64>) -> String {
    let denominator = *value.denom();
    let mut reduced = denominator;
    while reduced % 2 == 0 {
        reduced /= 2;
    }
    while reduced % 5 == 0 {
        reduced /= 5;
    }
    if reduced != 1 {
        return value.to_string();
    }
    let numerator = i128::from(*value.numer());
    let denominator = i128::from(denominator);
    let negative = numerator < 0;
    let numerator = numerator.abs();
    let Some(whole) = numerator.checked_div(denominator) else {
        return value.to_string();
    };
    let Some(mut remainder) = numerator.checked_rem(denominator) else {
        return value.to_string();
    };
    if remainder == 0 {
        return format!("{}{whole}", if negative { "-" } else { "" });
    }
    let mut fraction = String::new();
    while remainder != 0 {
        let Some(scaled) = remainder.checked_mul(10) else {
            return value.to_string();
        };
        let Some(digit) = scaled.checked_div(denominator) else {
            return value.to_string();
        };
        fraction.push(char::from_digit(u32::try_from(digit).unwrap_or(0), 10).unwrap_or('0'));
        let Some(next) = scaled.checked_rem(denominator) else {
            return value.to_string();
        };
        remainder = next;
    }
    format!("{}{}.{}", if negative { "-" } else { "" }, whole, fraction)
}

/// A processor the studio language can name.
///
/// Deliberately a closed set: §7.2 forbids raw backend escapes, so a patch
/// can only say things the compiler understands and can check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Processor {
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
pub struct ParamSpec {
    /// The name it is written with.
    pub name: &'static str,
    /// Stable private render-graph key. Usually the public name; filter
    /// `resonance` deliberately maps to the conventional DSP key `q`.
    pub dsp_name: &'static str,
    /// Removed or historical spellings, for diagnostics only. These are not
    /// accepted by [`Processor::param`].
    pub former_names: &'static [&'static str],
    /// Plain musician-facing description.
    pub summary: &'static str,
    /// The unit it must be written in.
    pub unit: Unit,
    /// Its value when the patch does not say.
    pub default: Ratio<i64>,
    /// The range a control may write, in the unit above — a decibel gain runs
    /// from −60 to +12, not from 0 to 1.
    ///
    /// This is the *writable* range, which is not the DSP's clamp: a graph
    /// parameter's descriptor in `musa-dsp` bounds what the processor will
    /// accept in linear terms, and this bounds what a composer means by
    /// turning a knob all the way up. They answer different questions, and a
    /// slider needs this one.
    pub range: (Ratio<i64>, Ratio<i64>),
}

impl Processor {
    /// The processor a written name denotes.
    pub fn from_name(name: &str) -> Option<Self> {
        crate::catalogue::processor(name).map(|entry| entry.processor)
    }

    /// How it is written.
    pub const fn name(self) -> &'static str {
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
    pub const fn params(self) -> &'static [ParamSpec] {
        const fn spec(
            name: &'static str,
            dsp_name: &'static str,
            former_names: &'static [&'static str],
            unit: Unit,
            default: Ratio<i64>,
            range: (Ratio<i64>, Ratio<i64>),
            summary: &'static str,
        ) -> ParamSpec {
            ParamSpec {
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
        const OSCILLATOR: &[ParamSpec] = &[
            spec(
                "frequency",
                "frequency",
                &[],
                Unit::Hz,
                r(1, 1),
                (r(0, 1), r(200, 1)),
                "Sets a control oscillator's frequency; a patch oscillator follows score pitch.",
            ),
            spec(
                "ratio",
                "ratio",
                &[],
                Unit::Linear,
                r(1, 1),
                (r(1, 4), r(16, 1)),
                "Scales the pitch supplied by the score.",
            ),
        ];
        const GAIN: &[ParamSpec] = &[spec(
            "gain",
            "gain",
            &[],
            Unit::Decibels,
            r(0, 1),
            (r(-60, 1), r(12, 1)),
            "Sets level in decibels.",
        )];
        // Written defaults are the built-in voice envelope, so `envelope()`
        // with nothing said is not a different sound from saying nothing.
        const ENVELOPE: &[ParamSpec] = &[
            spec(
                "attack",
                "attack",
                &[],
                Unit::Seconds,
                r(1, 200),
                (r(0, 1), r(5, 1)),
                "Sets the rise time after a note begins.",
            ),
            spec(
                "decay",
                "decay",
                &[],
                Unit::Seconds,
                r(0, 1),
                (r(0, 1), r(10, 1)),
                "Sets the time to reach the sustain level.",
            ),
            spec(
                "sustain",
                "sustain",
                &[],
                Unit::Linear,
                r(1, 1),
                (r(0, 1), r(1, 1)),
                "Sets the held level while a note continues.",
            ),
            spec(
                "release",
                "release",
                &[],
                Unit::Seconds,
                r(1, 20),
                (r(0, 1), r(10, 1)),
                "Sets the fade time after a note ends.",
            ),
        ];
        const LOWPASS: &[ParamSpec] = &[
            spec(
                "cutoff",
                "cutoff",
                &[],
                Unit::Hz,
                r(20_000, 1),
                (r(20, 1), r(20_000, 1)),
                "Sets the boundary frequency.",
            ),
            spec(
                "resonance",
                "q",
                &["q"],
                Unit::Linear,
                r(1_767_766_952_966_369, 2_500_000_000_000_000),
                (r(1, 10), r(20, 1)),
                "Emphasizes the cutoff, conventionally represented by quality factor Q.",
            ),
        ];
        const HIGHPASS: &[ParamSpec] = &[
            spec(
                "cutoff",
                "cutoff",
                &[],
                Unit::Hz,
                r(20, 1),
                (r(20, 1), r(20_000, 1)),
                "Sets the boundary frequency.",
            ),
            spec(
                "resonance",
                "q",
                &["q"],
                Unit::Linear,
                r(1_767_766_952_966_369, 2_500_000_000_000_000),
                (r(1, 10), r(20, 1)),
                "Emphasizes the cutoff, conventionally represented by quality factor Q.",
            ),
        ];
        const REVERB: &[ParamSpec] = &[
            spec(
                "room",
                "room",
                &[],
                Unit::Linear,
                r(1, 2),
                (r(0, 1), r(1, 1)),
                "Sets the apparent room size.",
            ),
            spec(
                "damping",
                "damping",
                &[],
                Unit::Linear,
                r(1, 2),
                (r(0, 1), r(1, 1)),
                "Controls high-frequency absorption.",
            ),
            spec(
                "mix",
                "mix",
                &[],
                Unit::Linear,
                r(1, 1),
                (r(0, 1), r(1, 1)),
                "Balances dry and processed audio.",
            ),
        ];
        // Time effects (§13.6). `mix` is the dry/wet balance: 0 is the input
        // untouched, 1 is the effect alone. A delay's `time` is bounded by
        // the line the DSP preallocates (2 s), and a chorus's `depth` by what
        // a chorus is — a few milliseconds of wobble, not a second one.
        const DELAY: &[ParamSpec] = &[
            spec(
                "time",
                "time",
                &[],
                Unit::Seconds,
                r(1, 4),
                (r(0, 1), r(2, 1)),
                "Sets the interval before each repeat.",
            ),
            spec(
                "feedback",
                "feedback",
                &[],
                Unit::Linear,
                r(3, 10),
                (r(0, 1), r(19, 20)),
                "Sets how much delayed sound repeats.",
            ),
            spec(
                "mix",
                "mix",
                &[],
                Unit::Linear,
                r(3, 10),
                (r(0, 1), r(1, 1)),
                "Balances dry and processed audio.",
            ),
        ];
        const CHORUS: &[ParamSpec] = &[
            spec(
                "rate",
                "rate",
                &[],
                Unit::Hz,
                r(3, 5),
                (r(0, 1), r(20, 1)),
                "Sets how quickly the doubled voice moves.",
            ),
            spec(
                "depth",
                "depth",
                &[],
                Unit::Seconds,
                r(1, 250),
                (r(0, 1), r(1, 100)),
                "Sets the maximum delay variation.",
            ),
            spec(
                "mix",
                "mix",
                &[],
                Unit::Linear,
                r(2, 5),
                (r(0, 1), r(1, 1)),
                "Balances dry and processed audio.",
            ),
        ];
        const SCALE: &[ParamSpec] = &[spec(
            "factor",
            "factor",
            &[],
            Unit::Hz,
            r(1, 1),
            (r(0, 1), r(20_000, 1)),
            "Multiplies each control value.",
        )];
        const BIAS: &[ParamSpec] = &[spec(
            "offset",
            "offset",
            &[],
            Unit::Hz,
            r(0, 1),
            (r(0, 1), r(20_000, 1)),
            "Adds to each control value.",
        )];
        const CLAMP: &[ParamSpec] = &[
            spec(
                "min",
                "min",
                &[],
                Unit::Hz,
                r(0, 1),
                (r(0, 1), r(20_000, 1)),
                "Sets the lowest output value.",
            ),
            spec(
                "max",
                "max",
                &[],
                Unit::Hz,
                r(20_000, 1),
                (r(0, 1), r(20_000, 1)),
                "Sets the highest output value.",
            ),
        ];
        const SMOOTHING: &[ParamSpec] = &[spec(
            "time",
            "time",
            &[],
            Unit::Seconds,
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
    pub fn param(self, name: &str) -> Option<ParamSpec> {
        self.params().iter().copied().find(|param| param.name == name)
    }
}

/// A node's index within its patch.
pub type NodeIndex = usize;

/// One processor instance inside a patch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioNode {
    /// What it runs.
    pub processor: Processor,
    /// The name it was bound to, if it was written as `name = ...`. This is
    /// what a `modulate` path addresses.
    pub label: Option<String>,
    /// Resolved parameter values, in the processor's declaration order.
    pub params: Vec<Option<WrittenQuantity>>,
    /// Where each parameter's *written* value is, parallel to `params`, and
    /// `None` for a parameter the patch left to its default.
    ///
    /// A structured editor rewrites exactly this range and nothing else: a
    /// slider that regenerated the whole call would silently normalize
    /// `30 ms` to `0.03 s` and lose any comment inside it (§11 — the source
    /// is the document, and an edit to it should be the edit that was made).
    pub param_spans: Vec<Option<SourceSpan>>,
    /// The whole `name(args)` construction, so a parameter the patch never
    /// wrote can be *added* rather than only changed.
    pub span: Option<SourceSpan>,
    /// The nodes feeding it, in argument order.
    pub inputs: Vec<NodeIndex>,
}

/// A patch or a bus: a graph of nodes with one designated output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Patch {
    nodes: Vec<StudioNode>,
    output: Option<NodeIndex>,
}

impl Patch {
    /// Its nodes, in creation order.
    pub fn nodes(&self) -> &[StudioNode] {
        &self.nodes
    }

    /// The node whose signal leaves the patch.
    pub fn output(&self) -> Option<NodeIndex> {
        self.output
    }

    #[doc(hidden)]
    pub fn push(&mut self, node: StudioNode) -> NodeIndex {
        self.nodes.push(node);
        self.nodes.len().saturating_sub(1)
    }

    #[doc(hidden)]
    pub fn set_output(&mut self, node: NodeIndex) {
        self.output = Some(node);
    }

    #[doc(hidden)]
    pub fn nodes_mut(&mut self) -> &mut Vec<StudioNode> {
        &mut self.nodes
    }
}

/// `modulate lfo -> glass_pad.lowpass.cutoff;` — a typed control connection
/// (§13.7), resolved to the node it addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Modulation {
    /// The modulating signal's name.
    pub source: String,
    /// The patch owning the modulated node.
    pub patch: String,
    /// Which node in that patch.
    pub node: NodeIndex,
    /// Which of its parameters.
    pub param: &'static str,
}

/// `send violin -> hall at -18 dB;`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Send {
    /// The part or bus sending.
    pub source: String,
    /// The bus receiving.
    pub bus: String,
    /// How much of the signal is sent.
    pub level: WrittenQuantity,
    /// Where the level was written, for an editor that rewrites it.
    pub level_span: Option<SourceSpan>,
}

/// `assign violin -> glass_pad;` — which patch realizes a part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    /// The patch the part is realized by.
    pub patch: String,
    /// Where the patch *name* was written, so pointing a part at a different
    /// patch replaces a name rather than rewriting a statement.
    pub patch_span: Option<SourceSpan>,
}

/// `route violin -> master;`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// The part or bus whose output is routed.
    pub source: String,
    /// Where it goes: a bus name, or `master`.
    pub destination: String,
}

/// The compiled studio: everything a graph builder needs, with every name
/// already resolved (§10.6).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StudioSpec {
    patches: IndexMap<String, Patch>,
    buses: IndexMap<String, Patch>,
    signals: IndexMap<String, Patch>,
    assignments: IndexMap<String, Assignment>,
    routes: Vec<Route>,
    sends: Vec<Send>,
    modulations: Vec<Modulation>,
    span: Option<SourceSpan>,
}

impl StudioSpec {
    /// Move every place this spec points at back into the composer's own text
    /// (`crate::expand`).
    #[doc(hidden)]
    pub fn remap_spans(&mut self, map: &musa_score::origin::SourceMap) {
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
        }
    }

    /// Whether the piece declared no studio at all, in which case the default
    /// instrument graph applies to everything (§14.8).
    pub fn is_empty(&self) -> bool {
        self.patches.is_empty() && self.buses.is_empty() && self.assignments.is_empty()
    }

    /// A patch by name.
    pub fn patch(&self, name: &str) -> Option<&Patch> {
        self.patches.get(name)
    }

    /// The declared patches, in source order.
    pub fn patches(&self) -> impl Iterator<Item = (&str, &Patch)> {
        self.patches.iter().map(|(name, patch)| (name.as_str(), patch))
    }

    /// The declared buses, in source order.
    pub fn buses(&self) -> impl Iterator<Item = (&str, &Patch)> {
        self.buses.iter().map(|(name, bus)| (name.as_str(), bus))
    }

    /// The top-level named signals (modulation sources), in source order.
    pub fn signals(&self) -> impl Iterator<Item = (&str, &Patch)> {
        self.signals.iter().map(|(name, signal)| (name.as_str(), signal))
    }

    /// Which patch realizes a part, if the studio says.
    ///
    /// A part the studio does not mention keeps the default instrument: a
    /// partial `studio` block must not silence the rest of the piece (§14.8).
    pub fn patch_for_part(&self, part: &str) -> Option<&str> {
        self.assignments.get(part).map(|assignment| assignment.patch.as_str())
    }

    /// The part→patch assignments, in source order.
    pub fn assignments(&self) -> impl Iterator<Item = (&str, &str)> {
        self.assignments
            .iter()
            .map(|(part, assignment)| (part.as_str(), assignment.patch.as_str()))
    }

    /// One part's assignment, with the span an editor would rewrite to point
    /// it at a different patch (§11's `AssignPatch`).
    pub fn assignment(&self, part: &str) -> Option<&Assignment> {
        self.assignments.get(part)
    }

    /// The declared routes, in source order.
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// The declared sends, in source order.
    pub fn sends(&self) -> &[Send] {
        &self.sends
    }

    /// The declared modulation connections, in source order.
    pub fn modulations(&self) -> &[Modulation] {
        &self.modulations
    }

    /// The `studio { … }` block itself, when the piece declared one. An
    /// editor that has to *add* a statement — assigning a part that no
    /// `assign` mentions — writes it inside this range.
    pub fn span(&self) -> Option<SourceSpan> {
        self.span
    }

    #[doc(hidden)]
    pub fn set_span(&mut self, span: Option<SourceSpan>) {
        self.span = span;
    }

    #[doc(hidden)]
    pub fn insert_patch(&mut self, name: String, patch: Patch) -> bool {
        self.patches.insert(name, patch).is_none()
    }

    #[doc(hidden)]
    pub fn insert_bus(&mut self, name: String, bus: Patch) -> bool {
        self.buses.insert(name, bus).is_none()
    }

    #[doc(hidden)]
    pub fn insert_signal(&mut self, name: String, signal: Patch) -> bool {
        self.signals.insert(name, signal).is_none()
    }

    #[doc(hidden)]
    pub fn assign(&mut self, part: String, assignment: Assignment) {
        self.assignments.insert(part, assignment);
    }

    #[doc(hidden)]
    pub fn push_route(&mut self, route: Route) {
        self.routes.push(route);
    }

    #[doc(hidden)]
    pub fn push_send(&mut self, send: Send) {
        self.sends.push(send);
    }

    #[doc(hidden)]
    pub fn push_modulation(&mut self, modulation: Modulation) {
        self.modulations.push(modulation);
    }

    /// Whether a name denotes something signal-shaped: a bus, or a part that
    /// has been assigned a patch. Used to check `route` and `send` sources.
    #[doc(hidden)]
    pub fn is_routable(&self, name: &str) -> bool {
        self.buses.contains_key(name) || self.assignments.contains_key(name)
    }

    #[doc(hidden)]
    pub fn has_bus(&self, name: &str) -> bool {
        self.buses.contains_key(name)
    }

    #[doc(hidden)]
    pub fn has_patch(&self, name: &str) -> bool {
        self.patches.contains_key(name)
    }

    #[doc(hidden)]
    pub fn has_signal(&self, name: &str) -> bool {
        self.signals.contains_key(name)
    }
}
