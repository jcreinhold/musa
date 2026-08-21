//! The studio layer (roadmap §6.5, §7.1): patches, buses, and the bindings
//! that connect a score to a sound.
//!
//! The bridge from score to studio is deliberately narrow — a part is
//! *assigned* to a patch and a patch output is *routed* to a bus, and that is
//! the whole of it. Nothing in this module knows what a note is, and nothing
//! in the score knows what an oscillator is (§2: part ≠ synthesizer).
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
//! A `StudioSpec` is **editable intent**, not a render plan: written values
//! keep the unit they were written in (`-15 dB` stays decibels), and the
//! conversion to whatever the DSP wants happens once, at the graph boundary
//! in `musa-audio`. That is what lets a studio UI show the user what they
//! typed rather than what the compiler made of it.

use indexmap::IndexMap;

use crate::origin::SourceSpan;

/// A parameter's physical unit (§7.2: units are part of the syntax).
///
/// This is the **one** unit declaration in the workspace: `musa-audio`
/// re-exports it rather than defining its own, so a language-level `1400 Hz`
/// and a DSP-level cutoff descriptor cannot disagree about what `Hz` is.
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

    /// The unit a written suffix denotes. `ms` is seconds, scaled at parse
    /// time: a millisecond is not a different dimension.
    fn from_suffix(suffix: &str) -> Option<Self> {
        match suffix {
            "Hz" => Some(Self::Hz),
            "dB" => Some(Self::Decibels),
            "s" | "ms" => Some(Self::Seconds),
            _ => None,
        }
    }
}

/// A written parameter value, in the unit it was written in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Value {
    /// The magnitude, normalized to the unit's base (`ms` becomes seconds).
    pub magnitude: f64,
    /// The dimension it carries.
    pub unit: Unit,
}

impl Value {
    /// The value as a linear multiplier: decibels become a ratio, everything
    /// else is already one. The single place dB→linear happens.
    pub fn as_linear(self) -> f64 {
        match self.unit {
            Unit::Decibels => 10f64.powf(self.magnitude / 20.0),
            Unit::Hz | Unit::Linear | Unit::Seconds => self.magnitude,
        }
    }
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
    /// `lowpass(cutoff: 1400 Hz, q: 0.7)`
    Lowpass,
    /// `highpass(cutoff: 80 Hz, q: 0.7)`
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamSpec {
    /// The name it is written with.
    pub name: &'static str,
    /// The unit it must be written in.
    pub unit: Unit,
    /// Its value when the patch does not say.
    pub default: f64,
    /// The range a control may write, in the unit above — a decibel gain runs
    /// from −60 to +12, not from 0 to 1.
    ///
    /// This is the *writable* range, which is not the DSP's clamp: a graph
    /// parameter's descriptor in `musa-audio` bounds what the processor will
    /// accept in linear terms, and this bounds what a composer means by
    /// turning a knob all the way up. They answer different questions, and a
    /// slider needs this one.
    pub range: (f64, f64),
}

impl Processor {
    /// The processor a written name denotes.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "oscillator" => Self::Oscillator,
            "gain" => Self::Gain,
            "mix" => Self::Mix,
            "envelope" => Self::Envelope,
            "lowpass" => Self::Lowpass,
            "highpass" => Self::Highpass,
            "reverb" => Self::Reverb,
            "delay" => Self::Delay,
            "chorus" => Self::Chorus,
            "scale" => Self::Scale,
            "bias" => Self::Bias,
            "clamp" => Self::Clamp,
            "smoothing" => Self::Smoothing,
            _ => return None,
        })
    }

    /// How it is written.
    pub fn name(self) -> &'static str {
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
    pub fn params(self) -> &'static [ParamSpec] {
        const fn spec(name: &'static str, unit: Unit, default: f64, range: (f64, f64)) -> ParamSpec {
            ParamSpec {
                name,
                unit,
                default,
                range,
            }
        }
        const OSCILLATOR: &[ParamSpec] = &[
            spec("frequency", Unit::Hz, 440.0, (20.0, 20_000.0)),
            spec("ratio", Unit::Linear, 1.0, (0.25, 16.0)),
        ];
        const GAIN: &[ParamSpec] = &[spec("gain", Unit::Decibels, 0.0, (-60.0, 12.0))];
        // Written defaults are the built-in voice envelope, so `envelope()`
        // with nothing said is not a different sound from saying nothing.
        const ENVELOPE: &[ParamSpec] = &[
            spec("attack", Unit::Seconds, 0.005, (0.0, 5.0)),
            spec("decay", Unit::Seconds, 0.0, (0.0, 10.0)),
            spec("sustain", Unit::Linear, 1.0, (0.0, 1.0)),
            spec("release", Unit::Seconds, 0.05, (0.0, 10.0)),
        ];
        const LOWPASS: &[ParamSpec] = &[
            spec("cutoff", Unit::Hz, 20_000.0, (20.0, 20_000.0)),
            spec("q", Unit::Linear, std::f64::consts::FRAC_1_SQRT_2, (0.1, 20.0)),
        ];
        const HIGHPASS: &[ParamSpec] = &[
            spec("cutoff", Unit::Hz, 20.0, (20.0, 20_000.0)),
            spec("q", Unit::Linear, std::f64::consts::FRAC_1_SQRT_2, (0.1, 20.0)),
        ];
        const REVERB: &[ParamSpec] = &[
            spec("room", Unit::Linear, 0.5, (0.0, 1.0)),
            spec("damping", Unit::Linear, 0.5, (0.0, 1.0)),
            spec("mix", Unit::Linear, 1.0, (0.0, 1.0)),
        ];
        // Time effects (§13.6). `mix` is the dry/wet balance: 0 is the input
        // untouched, 1 is the effect alone. A delay's `time` is bounded by
        // the line the DSP preallocates (2 s), and a chorus's `depth` by what
        // a chorus is — a few milliseconds of wobble, not a second one.
        const DELAY: &[ParamSpec] = &[
            spec("time", Unit::Seconds, 0.25, (0.0, 2.0)),
            spec("feedback", Unit::Linear, 0.3, (0.0, 0.95)),
            spec("mix", Unit::Linear, 0.3, (0.0, 1.0)),
        ];
        const CHORUS: &[ParamSpec] = &[
            spec("rate", Unit::Hz, 0.6, (0.0, 20.0)),
            spec("depth", Unit::Seconds, 0.004, (0.0, 0.01)),
            spec("mix", Unit::Linear, 0.4, (0.0, 1.0)),
        ];
        const SCALE: &[ParamSpec] = &[spec("factor", Unit::Hz, 1.0, (0.0, 20_000.0))];
        const BIAS: &[ParamSpec] = &[spec("offset", Unit::Hz, 0.0, (0.0, 20_000.0))];
        const CLAMP: &[ParamSpec] = &[
            spec("min", Unit::Hz, 0.0, (0.0, 20_000.0)),
            spec("max", Unit::Hz, 20_000.0, (0.0, 20_000.0)),
        ];
        const SMOOTHING: &[ParamSpec] = &[spec("time", Unit::Seconds, 0.02, (0.0, 1.0))];
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
#[derive(Clone, Debug, PartialEq)]
pub struct StudioNode {
    /// What it runs.
    pub processor: Processor,
    /// The name it was bound to, if it was written as `name = ...`. This is
    /// what a `modulate` path addresses.
    pub label: Option<String>,
    /// Resolved parameter values, in the processor's declaration order.
    pub params: Vec<Value>,
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
#[derive(Clone, Debug, Default, PartialEq)]
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

    pub(crate) fn push(&mut self, node: StudioNode) -> NodeIndex {
        self.nodes.push(node);
        self.nodes.len().saturating_sub(1)
    }

    pub(crate) fn set_output(&mut self, node: NodeIndex) {
        self.output = Some(node);
    }

    pub(crate) fn nodes_mut(&mut self) -> &mut Vec<StudioNode> {
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
#[derive(Clone, Debug, PartialEq)]
pub struct Send {
    /// The part or bus sending.
    pub source: String,
    /// The bus receiving.
    pub bus: String,
    /// How much of the signal is sent.
    pub level: Value,
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
#[derive(Clone, Debug, Default, PartialEq)]
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
    pub(crate) fn remap_spans(&mut self, map: &crate::origin::SourceMap) {
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

    pub(crate) fn insert_patch(&mut self, name: String, patch: Patch) -> bool {
        self.patches.insert(name, patch).is_none()
    }

    pub(crate) fn insert_bus(&mut self, name: String, bus: Patch) -> bool {
        self.buses.insert(name, bus).is_none()
    }

    pub(crate) fn insert_signal(&mut self, name: String, signal: Patch) -> bool {
        self.signals.insert(name, signal).is_none()
    }

    pub(crate) fn assign(&mut self, part: String, assignment: Assignment) {
        self.assignments.insert(part, assignment);
    }

    pub(crate) fn push_route(&mut self, route: Route) {
        self.routes.push(route);
    }

    pub(crate) fn push_send(&mut self, send: Send) {
        self.sends.push(send);
    }

    pub(crate) fn push_modulation(&mut self, modulation: Modulation) {
        self.modulations.push(modulation);
    }

    /// Whether a name denotes something signal-shaped: a bus, or a part that
    /// has been assigned a patch. Used to check `route` and `send` sources.
    pub(crate) fn is_routable(&self, name: &str) -> bool {
        self.buses.contains_key(name) || self.assignments.contains_key(name)
    }

    pub(crate) fn has_bus(&self, name: &str) -> bool {
        self.buses.contains_key(name)
    }

    pub(crate) fn has_patch(&self, name: &str) -> bool {
        self.patches.contains_key(name)
    }

    pub(crate) fn has_signal(&self, name: &str) -> bool {
        self.signals.contains_key(name)
    }
}

/// Read a written number and unit suffix into a [`Value`].
///
/// `ms` is folded into seconds here, which is why the spec carries no
/// millisecond unit: the dimension is time, and the suffix is a scale.
pub(crate) fn parse_value(number: &str, suffix: Option<&str>) -> Option<Value> {
    let magnitude: f64 = number.parse().ok()?;
    match suffix {
        None => Some(Value {
            magnitude,
            unit: Unit::Linear,
        }),
        Some("ms") => Some(Value {
            magnitude: magnitude / 1000.0,
            unit: Unit::Seconds,
        }),
        Some(other) => Unit::from_suffix(other).map(|unit| Value { magnitude, unit }),
    }
}

// --- Resolution -------------------------------------------------------------

use musa_language::ast::{
    Arg, AstNode as _, BusDecl, CallExpr, PatchDecl, SendStmt, SignalChain, SignalStage, StudioDecl, StudioItem,
};

use crate::diagnose::{Code, Diagnostic};
use crate::resolve::{span_of, trimmed_span};

/// What a library's `studio` may not write.
fn complain(node: &musa_language::SyntaxNode, what: &str, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(
        Diagnostic::error(
            Code::Misplaced,
            format!("a library's `studio` declares patches and signals; `{what}` belongs to the piece"),
        )
        .at(trimmed_span(node), "not allowed in a library"),
    );
}

/// Resolve a `studio` block into a [`StudioSpec`], reporting every unresolved
/// name and mis-united value against `diagnostics`.
///
/// `parts` is the set of part names the score declared: `assign` is the one
/// place the two layers meet, so it is the one place a studio name is checked
/// against a score name.
/// `imported` holds the `studio` block of each library the piece imports. A
/// library ships building blocks — patches and signals — and nothing that
/// wires them to a particular score, because it does not know the score;
/// anything else it writes is reported rather than silently applied.
pub(crate) fn resolve(
    decl: Option<&StudioDecl>,
    imported: &[StudioDecl],
    parts: &[String],
    references: &mut crate::resolve::ReferenceIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> StudioSpec {
    let mut spec = StudioSpec {
        span: decl.map(|decl| span_of(decl.syntax())),
        ..StudioSpec::default()
    };
    let mut items: Vec<StudioItem> = Vec::new();
    for library in imported {
        for item in library.items() {
            match item {
                StudioItem::Patch(_) | StudioItem::Signal(_) => items.push(item),
                StudioItem::Bus(ref node) => complain(node.syntax(), "bus", diagnostics),
                StudioItem::Modulate(ref node) => complain(node.syntax(), "modulate", diagnostics),
                StudioItem::Assign(ref node) => complain(node.syntax(), "assign", diagnostics),
                StudioItem::Route(ref node) => complain(node.syntax(), "route", diagnostics),
                StudioItem::Send(ref node) => complain(node.syntax(), "send", diagnostics),
            }
        }
    }
    // The compiled document's own items begin here; everything before is a
    // library's, whose spans name text the reference record does not cover.
    let main_start = items.len();
    items.extend(decl.map(StudioDecl::items).unwrap_or_default());

    // Two passes: patches, buses, and signals first, so the bindings that
    // follow can be checked against them regardless of writing order. A
    // studio reads top-down, but it does not have to be written that way.
    for (index, item) in items.iter().enumerate() {
        match item {
            StudioItem::Patch(patch) => declare_patch(patch, &mut spec, diagnostics),
            StudioItem::Bus(bus) => declare_bus(bus, &mut spec, diagnostics),
            StudioItem::Signal(signal) => {
                let name = signal.name().unwrap_or_default();
                let mut built = Patch::default();
                let Some(chain) = signal.chain() else { continue };
                if lower_chain(&chain, &mut built, diagnostics).is_some() && !spec.insert_signal(name.clone(), built) {
                    diagnostics.push(
                        Diagnostic::error(Code::DuplicateName, format!("duplicate signal `{name}`"))
                            .at(trimmed_span(signal.syntax()), "declared again here"),
                    );
                }
            }
            StudioItem::Modulate(_) | StudioItem::Assign(_) | StudioItem::Route(_) | StudioItem::Send(_) => {}
        }
        // A patch that resolved is a declaration the record keeps — but only
        // the document's own; a library's patch is spelled in its own file.
        if index >= main_start
            && let StudioItem::Patch(patch) = item
        {
            let name = patch.name().unwrap_or_default();
            if !name.is_empty()
                && spec.has_patch(&name)
                && let Some(span) = crate::resolve::token_span(patch.syntax(), musa_language::SyntaxKind::Identifier)
            {
                references.declare(crate::resolve::NameKind::Patch, &name, span);
            }
        }
    }

    for item in &items {
        match item {
            StudioItem::Assign(assign) => {
                let (Some(part), Some(patch)) = (assign.source(), assign.destination()) else {
                    continue;
                };
                let span = Some(span_of(assign.syntax()));
                if !parts.contains(&part) {
                    diagnostics.push(
                        Diagnostic::error(Code::UnknownName, format!("unknown part `{part}`"))
                            .maybe_at(span, "no part with this name"),
                    );
                } else if !spec.has_patch(&patch) {
                    diagnostics.push(
                        Diagnostic::error(Code::UnknownName, format!("unknown patch `{patch}`"))
                            .maybe_at(span, "no patch with this name"),
                    );
                } else {
                    let patch_span = assign
                        .destination_token()
                        .map(|token| crate::resolve::source_span_of(&token));
                    // Both names resolved: the assign is a use of each.
                    if let Some(span) = assign
                        .source_token()
                        .map(|token| crate::resolve::source_span_of(&token))
                    {
                        references.record_use(crate::resolve::NameKind::Part, &part, span);
                    }
                    if let Some(span) = patch_span {
                        references.record_use(crate::resolve::NameKind::Patch, &patch, span);
                    }
                    spec.assign(part, Assignment { patch, patch_span });
                }
            }
            StudioItem::Route(route) => {
                let (Some(source), Some(destination)) = (route.source(), route.destination()) else {
                    continue;
                };
                let span = Some(span_of(route.syntax()));
                if !spec.is_routable(&source) {
                    diagnostics.push(
                        Diagnostic::error(
                            Code::UnknownName,
                            format!("`{source}` is not an assigned part or a bus"),
                        )
                        .maybe_at(span, "nothing sends from here"),
                    );
                } else if destination != "master" && !spec.has_bus(&destination) {
                    diagnostics.push(
                        Diagnostic::error(Code::UnknownName, format!("unknown destination `{destination}`"))
                            .maybe_at(span, "not a bus or `master`"),
                    );
                } else {
                    if parts.contains(&source)
                        && let Some(span) = route.source_token().map(|token| crate::resolve::source_span_of(&token))
                    {
                        references.record_use(crate::resolve::NameKind::Part, &source, span);
                    }
                    spec.push_route(Route { source, destination });
                }
            }
            StudioItem::Send(send) => resolve_send(send, &mut spec, parts, references, diagnostics),
            StudioItem::Modulate(modulate) => {
                let Some(source) = modulate.source() else { continue };
                let span = Some(span_of(modulate.syntax()));
                if !spec.has_signal(&source) {
                    diagnostics.push(
                        Diagnostic::error(Code::UnknownName, format!("unknown signal `{source}`"))
                            .maybe_at(span, "not declared in this studio"),
                    );
                    continue;
                }
                if let Some(modulation) = resolve_target(&source, &modulate.target(), &spec, span, diagnostics) {
                    spec.push_modulation(modulation);
                }
            }
            StudioItem::Patch(_) | StudioItem::Bus(_) | StudioItem::Signal(_) => {}
        }
    }
    spec
}

fn resolve_send(
    send: &SendStmt,
    spec: &mut StudioSpec,
    parts: &[String],
    references: &mut crate::resolve::ReferenceIndex,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (Some(source), Some(bus)) = (send.source(), send.destination()) else {
        return;
    };
    let span = Some(span_of(send.syntax()));
    if !spec.is_routable(&source) {
        diagnostics.push(
            Diagnostic::error(
                Code::UnknownName,
                format!("`{source}` is not an assigned part or a bus"),
            )
            .maybe_at(span, "nothing sends from here"),
        );
        return;
    }
    if !spec.has_bus(&bus) {
        diagnostics.push(
            Diagnostic::error(Code::UnknownName, format!("unknown bus `{bus}`"))
                .maybe_at(span, "no bus with this name"),
        );
        return;
    }
    // The source is routable and names a part: a use of it, recorded.
    if parts.contains(&source)
        && let Some(span) = send.source_token().map(|token| crate::resolve::source_span_of(&token))
    {
        references.record_use(crate::resolve::NameKind::Part, &source, span);
    }
    let level = send
        .level()
        .and_then(|literal| parse_value(&literal.number()?, literal.unit().as_deref()));
    let Some(level) = level else {
        diagnostics.push(
            Diagnostic::error(Code::NotAValue, "a send level must be a number").maybe_at(span, "expected a number"),
        );
        return;
    };
    if level.unit != Unit::Decibels {
        diagnostics
            .push(Diagnostic::error(Code::NotAValue, "a send level is written in `dB`").maybe_at(span, "missing `dB`"));
        return;
    }
    let level_span = send.level().map(|literal| trimmed_span(literal.syntax()));
    spec.push_send(Send {
        source,
        bus,
        level,
        level_span,
    });
}

/// `glass_pad.lowpass.cutoff` → the node and parameter it names.
///
/// A stage is addressed by its binding name when it has one and by its
/// processor name otherwise. Two unnamed `lowpass` stages in one patch are
/// therefore ambiguous, and saying so is better than silently picking one.
fn resolve_target(
    source: &str,
    path: &[String],
    spec: &StudioSpec,
    span: Option<crate::origin::SourceSpan>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Modulation> {
    let written = path.join(".");
    let [patch_name, stage, param] = path else {
        diagnostics.push(
            Diagnostic::error(
                Code::NotAValue,
                format!("`{written}` is not a `<patch>.<stage>.<parameter>` path"),
            )
            .maybe_at(span, "expected `<patch>.<stage>.<parameter>`"),
        );
        return None;
    };
    let Some(patch) = spec.patch(patch_name) else {
        diagnostics.push(
            Diagnostic::error(Code::UnknownName, format!("unknown patch `{patch_name}`"))
                .maybe_at(span, "no patch with this name"),
        );
        return None;
    };
    let matches: Vec<NodeIndex> = patch
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| node.label.as_deref() == Some(stage.as_str()) || node.processor.name() == stage)
        .map(|(index, _)| index)
        .collect();
    let [node] = matches.as_slice() else {
        let (message, label, help) = if matches.is_empty() {
            (
                format!("`{patch_name}` has no stage named `{stage}`"),
                "unknown stage",
                "name a stage in the patch's chain, or give this one a name with `<name> = …`",
            )
        } else {
            (
                format!("`{patch_name}` has more than one `{stage}`"),
                "which one?",
                "give the stage a name — `warm = lowpass(…)` — and address it by that",
            )
        };
        diagnostics.push(
            Diagnostic::error(Code::UnknownName, message)
                .maybe_at(span, label)
                .help(help),
        );
        return None;
    };
    let processor = patch.nodes().get(*node)?.processor;
    let Some(declared) = processor.param(param) else {
        diagnostics.push(
            Diagnostic::error(Code::UnknownName, format!("`{stage}` has no parameter `{param}`"))
                .maybe_at(span, "unknown parameter"),
        );
        return None;
    };
    Some(Modulation {
        source: source.to_owned(),
        patch: patch_name.clone(),
        node: *node,
        param: declared.name,
    })
}

fn declare_patch(decl: &PatchDecl, spec: &mut StudioSpec, diagnostics: &mut Vec<Diagnostic>) {
    let name = decl.name().unwrap_or_default();
    let Some(patch) = build_container(&decl.signals(), &decl.chains(), diagnostics) else {
        return;
    };
    if patch.output().is_none() {
        diagnostics.push(
            Diagnostic::error(Code::Studio, format!("patch `{name}` never reaches `output`"))
                .at(trimmed_span(decl.syntax()), "this chain ends nowhere"),
        );
        return;
    }
    if !spec.insert_patch(name.clone(), patch) {
        diagnostics.push(
            Diagnostic::error(Code::DuplicateName, format!("duplicate patch `{name}`"))
                .at(trimmed_span(decl.syntax()), "declared again here"),
        );
    }
}

fn declare_bus(decl: &BusDecl, spec: &mut StudioSpec, diagnostics: &mut Vec<Diagnostic>) {
    let name = decl.name().unwrap_or_default();
    // A bus's input is whatever is sent to it, so its chain needs no
    // `output` terminal: the last stage *is* the output.
    let Some(mut bus) = build_container(&decl.signals(), &decl.chains(), diagnostics) else {
        return;
    };
    if bus.output().is_none() {
        let last = bus.nodes().len().checked_sub(1);
        match last {
            Some(index) => bus.set_output(index),
            None => {
                diagnostics.push(
                    Diagnostic::error(Code::Studio, format!("bus `{name}` is empty"))
                        .at(trimmed_span(decl.syntax()), "no processors"),
                );
                return;
            }
        }
    }
    if !spec.insert_bus(name.clone(), bus) {
        diagnostics.push(
            Diagnostic::error(Code::DuplicateName, format!("duplicate bus `{name}`"))
                .at(trimmed_span(decl.syntax()), "declared again here"),
        );
    }
}

/// The shared body of a patch or a bus: named signals, then chains, with the
/// names visible to the chains that follow them.
fn build_container(
    signals: &[musa_language::ast::SignalBinding],
    chains: &[musa_language::ast::ChainStmt],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Patch> {
    let mut patch = Patch::default();
    let mut locals: IndexMap<String, NodeIndex> = IndexMap::new();
    for binding in signals {
        let Some(chain) = binding.chain() else { continue };
        let name = binding.name().unwrap_or_default();
        if let Some(node) = lower_chain_into(&chain, &mut patch, &locals, diagnostics) {
            if let Some(entry) = patch.nodes_mut().get_mut(node) {
                entry.label = Some(name.clone());
            }
            locals.insert(name, node);
        }
    }
    for statement in chains {
        let Some(chain) = statement.chain() else { continue };
        lower_chain_into(&chain, &mut patch, &locals, diagnostics);
    }
    Some(patch)
}

/// A top-level signal chain, which has no enclosing patch's local names.
fn lower_chain(chain: &SignalChain, patch: &mut Patch, diagnostics: &mut Vec<Diagnostic>) -> Option<NodeIndex> {
    let locals = IndexMap::new();
    let node = lower_chain_into(chain, patch, &locals, diagnostics)?;
    patch.set_output(node);
    Some(node)
}

/// Flatten `a |> b |> c` into nodes: each stage takes the previous stage's
/// node as its input, and `output` marks rather than adds one.
fn lower_chain_into(
    chain: &SignalChain,
    patch: &mut Patch,
    locals: &IndexMap<String, NodeIndex>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<NodeIndex> {
    let mut previous: Option<NodeIndex> = None;
    for stage in chain.stages() {
        match &stage {
            SignalStage::Name(name) => {
                let text = name.text().unwrap_or_default();
                if text == "output" {
                    match previous {
                        Some(node) => patch.set_output(node),
                        None => diagnostics.push(
                            Diagnostic::error(Code::Studio, "`output` needs a signal before it")
                                .at(trimmed_span(stage.syntax()), "nothing reaches it"),
                        ),
                    }
                    continue;
                }
                match locals.get(&text) {
                    Some(node) => previous = Some(*node),
                    None => {
                        diagnostics.push(
                            Diagnostic::error(Code::UnknownName, format!("unknown signal `{text}`"))
                                .at(trimmed_span(stage.syntax()), "not declared in this studio"),
                        );
                        return None;
                    }
                }
            }
            SignalStage::Call(call) => {
                previous = Some(lower_call(call, previous, patch, locals, diagnostics)?);
            }
            SignalStage::Literal(_) => {
                diagnostics.push(
                    Diagnostic::error(Code::NotAValue, "a number is not a signal")
                        .at(trimmed_span(stage.syntax()), "expected a signal"),
                );
                return None;
            }
        }
    }
    previous
}

/// One `name(args)` construction, with its upstream stage already lowered.
fn lower_call(
    call: &CallExpr,
    upstream: Option<NodeIndex>,
    patch: &mut Patch,
    locals: &IndexMap<String, NodeIndex>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<NodeIndex> {
    let span = Some(span_of(call.syntax()));
    let written = call.callee().unwrap_or_default();
    let Some(processor) = Processor::from_name(&written) else {
        diagnostics.push(
            Diagnostic::error(Code::UnknownWord, format!("unknown processor `{written}`"))
                .maybe_at(span, "musa has no processor by this name"),
        );
        return None;
    };
    let mut params: Vec<Value> = processor
        .params()
        .iter()
        .map(|declared| Value {
            magnitude: declared.default,
            unit: declared.unit,
        })
        .collect();
    let mut param_spans: Vec<Option<SourceSpan>> = vec![None; params.len()];
    let mut inputs: Vec<NodeIndex> = upstream.into_iter().collect();
    let mut positional = 0usize;

    for arg in call.args() {
        match arg.value() {
            // `envelope(adsr(...))`: the inner construction's arguments are
            // the outer processor's, so it flattens rather than nesting. The
            // shape exists for readability, not for a second node.
            Some(SignalStage::Call(inner)) if is_argument_group(&inner) => {
                for nested in inner.args() {
                    bind_argument(
                        &nested,
                        processor,
                        &mut params,
                        &mut param_spans,
                        &mut positional,
                        diagnostics,
                    );
                }
            }
            Some(SignalStage::Call(inner)) => {
                if let Some(node) = lower_call(&inner, None, patch, locals, diagnostics) {
                    inputs.push(node);
                }
            }
            Some(SignalStage::Name(name)) => {
                let text = name.text().unwrap_or_default();
                match locals.get(&text) {
                    Some(node) => inputs.push(*node),
                    // A bare word that names no signal is a mode selector
                    // (`oscillator(sine)`), which today has one legal value.
                    None if text == "sine" => {}
                    None => diagnostics.push(
                        Diagnostic::error(Code::UnknownName, format!("unknown signal `{text}`"))
                            .at(trimmed_span(name.syntax()), "not declared in this studio"),
                    ),
                }
            }
            Some(SignalStage::Literal(_)) | None => {
                bind_argument(
                    &arg,
                    processor,
                    &mut params,
                    &mut param_spans,
                    &mut positional,
                    diagnostics,
                );
            }
        }
    }

    Some(patch.push(StudioNode {
        processor,
        label: None,
        params,
        param_spans,
        span,
        inputs,
    }))
}

/// Whether a construction is an argument group (`adsr(...)`) rather than a
/// processor. Argument groups exist to make a long parameter list readable.
fn is_argument_group(call: &CallExpr) -> bool {
    call.callee().as_deref() == Some("adsr")
}

/// Bind one written argument to a declared parameter, checking its unit.
fn bind_argument(
    arg: &Arg,
    processor: Processor,
    params: &mut [Value],
    spans: &mut [Option<SourceSpan>],
    positional: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let span = Some(span_of(arg.syntax()));
    let index = match arg.name() {
        Some(name) => {
            let Some(found) = processor.params().iter().position(|param| param.name == name) else {
                diagnostics.push(
                    Diagnostic::error(
                        Code::UnknownName,
                        format!("`{}` has no parameter `{name}`", processor.name()),
                    )
                    .maybe_at(span, "unknown parameter"),
                );
                return;
            };
            found
        }
        None => {
            let index = *positional;
            *positional = positional.saturating_add(1);
            if index >= processor.params().len() {
                diagnostics.push(
                    Diagnostic::error(
                        Code::NotAValue,
                        format!("`{}` takes no argument in that position", processor.name()),
                    )
                    .maybe_at(span, "too many arguments"),
                );
                return;
            }
            index
        }
    };
    let Some(declared) = processor.params().get(index).copied() else {
        return;
    };
    let Some(SignalStage::Literal(literal)) = arg.value() else {
        diagnostics.push(
            Diagnostic::error(Code::NotAValue, format!("`{}` must be a number", declared.name))
                .maybe_at(span, "expected a number"),
        );
        return;
    };
    let Some(number) = literal.number() else { return };
    let Some(value) = parse_value(&number, literal.unit().as_deref()) else {
        diagnostics.push(
            Diagnostic::error(Code::NotAValue, format!("`{number}` is not a number"))
                .maybe_at(span, "expected a number"),
        );
        return;
    };
    if value.unit != declared.unit {
        // §7.2: units are syntax, so a missing one is a diagnostic and not a
        // guess. The message names the unit the parameter is declared in.
        let (message, label) = match declared.unit.spelling() {
            Some(expected) => (
                format!("`{}` is written in `{expected}`", declared.name),
                format!("expected `{expected}`"),
            ),
            None => (format!("`{}` takes no unit", declared.name), "drop the unit".to_owned()),
        };
        let mut diagnostic = Diagnostic::error(Code::NotAValue, message)
            .maybe_at(span, label)
            .note("units are part of the syntax, so musa never guesses one");
        // Writing the unit the parameter is declared in is the one repair
        // that changes no number, so it is a fix rather than a help line. A
        // *wrong* unit is not: `q: 2 Hz` might be a misplaced argument, and
        // guessing which is exactly what a fix must not do.
        if let (Some(expected), None, Some(at)) = (declared.unit.spelling(), literal.unit(), span) {
            let written = match arg.name() {
                Some(name) => format!("{name}: {number} {expected}"),
                None => format!("{number} {expected}"),
            };
            diagnostic = diagnostic.fix(format!("write `{number} {expected}`"), at, written);
        }
        diagnostics.push(diagnostic);
        return;
    }
    if let Some(slot) = params.get_mut(index) {
        *slot = value;
    }
    if let Some(slot) = spans.get_mut(index) {
        // The literal's own range, unit included: replacing it replaces what
        // was written, so `1400 Hz` becomes `900 Hz` and nothing else moves.
        *slot = Some(trimmed_span(literal.syntax()));
    }
}
