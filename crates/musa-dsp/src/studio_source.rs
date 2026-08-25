//! Read-only production studio data decoded from one checked source value.
//!
//! This is a consumer projection, not a construction API. The compatibility
//! surface is elaborated and normalized by the ordinary Musa checker; this
//! module verifies only the exact artifact schema needed by preparation.

use std::sync::Arc;

use musa_calculus::{CheckedSource, SourceDatum, SourceDatumKind, SourceSchema};
use num_rational::Ratio;
use thiserror::Error;

use crate::{ExactQuantityProjection, SoundDimension, SoundUnit};

const SCHEMA_NAME: &str = "std.sound.production.StudioExecutionArtifact";
const ROOT_TYPE: &str = "StudioExecutionArtifact";
const SCHEMA_VERSION: u64 = 1;

/// The exact schema consumed by production audio preparation.
#[must_use]
pub fn studio_execution_schema() -> SourceSchema {
    SourceSchema::new(SCHEMA_NAME, ROOT_TYPE, SCHEMA_VERSION)
}

/// Why checked source is not a production studio artifact.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum StudioExecutionError {
    /// Another source root or edition was supplied.
    #[error("expected checked source schema `{SCHEMA_NAME}` version {SCHEMA_VERSION}")]
    WrongSchema,
    /// Canonical data did not match the declared source schema.
    #[error("malformed StudioExecutionArtifact at {path}: {expected}")]
    Malformed {
        /// Structural position within the artifact.
        path: String,
        /// Shape required at that position.
        expected: &'static str,
    },
}

/// The complete exact, normalized production studio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioExecution {
    anchor: u64,
    declared: bool,
    patches: Vec<StudioGraphProjection>,
    buses: Vec<StudioGraphProjection>,
    signals: Vec<StudioGraphProjection>,
    assignments: Vec<StudioAssignmentProjection>,
    routes: Vec<StudioRouteProjection>,
    sends: Vec<StudioSendProjection>,
    modulations: Vec<StudioModulationProjection>,
    exact_source: Arc<[u8]>,
}

impl StudioExecution {
    /// Stable ordinal for the compatibility studio block.
    #[must_use]
    pub const fn anchor(&self) -> u64 {
        self.anchor
    }
    /// Whether the compatibility source contained a studio block.
    #[must_use]
    pub const fn declared(&self) -> bool {
        self.declared
    }
    /// Instrument graphs in source order.
    pub fn patches(&self) -> impl ExactSizeIterator<Item = &StudioGraphProjection> {
        self.patches.iter()
    }
    /// Bus graphs in source order.
    pub fn buses(&self) -> impl ExactSizeIterator<Item = &StudioGraphProjection> {
        self.buses.iter()
    }
    /// Control-signal graphs in source order.
    pub fn signals(&self) -> impl ExactSizeIterator<Item = &StudioGraphProjection> {
        self.signals.iter()
    }
    /// Part assignments in source order.
    pub fn assignments(&self) -> impl ExactSizeIterator<Item = &StudioAssignmentProjection> {
        self.assignments.iter()
    }
    /// Routes in source order.
    pub fn routes(&self) -> impl ExactSizeIterator<Item = &StudioRouteProjection> {
        self.routes.iter()
    }
    /// Sends in source order.
    pub fn sends(&self) -> impl ExactSizeIterator<Item = &StudioSendProjection> {
        self.sends.iter()
    }
    /// Modulation bindings in source order.
    pub fn modulations(&self) -> impl ExactSizeIterator<Item = &StudioModulationProjection> {
        self.modulations.iter()
    }
    /// Complete source bytes used for exact equality and identity.
    #[must_use]
    pub fn exact_source(&self) -> &[u8] {
        &self.exact_source
    }

    pub(crate) fn preparation_projection(&self) -> Option<crate::intent::StudioSpec> {
        let mut studio = crate::intent::StudioSpec::default();
        for graph in &self.patches {
            let _inserted = studio.insert_patch(graph.name.clone(), preparation_graph(graph)?);
        }
        for graph in &self.buses {
            let _inserted = studio.insert_bus(graph.name.clone(), preparation_graph(graph)?);
        }
        for graph in &self.signals {
            let _inserted = studio.insert_signal(graph.name.clone(), preparation_graph(graph)?);
        }
        for assignment in &self.assignments {
            studio.assign(
                assignment.part.clone(),
                crate::intent::Assignment {
                    patch: assignment.instrument.clone(),
                    patch_span: None,
                },
            );
        }
        for route in &self.routes {
            studio.push_route(crate::intent::Route {
                source: route.source.clone(),
                destination: route.destination.clone(),
                span: None,
            });
        }
        for send in &self.sends {
            studio.push_send(crate::intent::Send {
                source: send.source.clone(),
                bus: send.bus.clone(),
                level: preparation_quantity(&send.level),
                level_span: None,
                span: None,
            });
        }
        for modulation in &self.modulations {
            studio.push_modulation(crate::intent::Modulation {
                source: modulation.source.clone(),
                patch: modulation.instrument.clone(),
                node: modulation.node,
                param: static_parameter_name(&modulation.parameter)?,
            });
        }
        Some(studio)
    }
}

fn static_parameter_name(name: &str) -> Option<&'static str> {
    match name {
        "frequency" => Some("frequency"),
        "ratio" => Some("ratio"),
        "gain" => Some("gain"),
        "attack" => Some("attack"),
        "decay" => Some("decay"),
        "sustain" => Some("sustain"),
        "release" => Some("release"),
        "cutoff" => Some("cutoff"),
        "resonance" => Some("resonance"),
        "room" => Some("room"),
        "damping" => Some("damping"),
        "mix" => Some("mix"),
        "time" => Some("time"),
        "feedback" => Some("feedback"),
        "rate" => Some("rate"),
        "depth" => Some("depth"),
        "factor" => Some("factor"),
        "offset" => Some("offset"),
        "min" => Some("min"),
        "max" => Some("max"),
        _ => None,
    }
}

fn preparation_graph(source: &StudioGraphProjection) -> Option<crate::intent::Patch> {
    let mut graph = crate::intent::Patch::default();
    for node in &source.nodes {
        let processor = crate::intent::Processor::from_name(node.processor.name)?;
        graph.push(crate::intent::StudioNode {
            processor,
            label: node.label.clone(),
            params: node
                .processor
                .parameters
                .iter()
                .map(|parameter| Some(preparation_quantity(&parameter.value)))
                .collect(),
            param_spans: node.written_parameters.iter().map(|_| None).collect(),
            span: None,
            inputs: node.inputs.clone(),
        });
    }
    graph.set_output(source.output);
    Some(graph)
}

fn preparation_quantity(source: &ExactQuantityProjection) -> crate::intent::WrittenQuantity {
    let unit = match source.unit() {
        SoundUnit::Hertz => crate::intent::Unit::Hz,
        SoundUnit::Linear => crate::intent::Unit::Linear,
        SoundUnit::Decibels => crate::intent::Unit::Decibels,
        SoundUnit::Seconds => crate::intent::Unit::Seconds,
    };
    crate::intent::WrittenQuantity::new(*source.magnitude(), unit)
}

/// One named resolved graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioGraphProjection {
    anchor: u64,
    name: String,
    nodes: Vec<StudioNodeProjection>,
    output: usize,
}

impl StudioGraphProjection {
    /// Stable ordinal resolved through the compilation's lineage table.
    #[must_use]
    pub const fn anchor(&self) -> u64 {
        self.anchor
    }
    /// Declared name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Nodes in source construction order.
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = &StudioNodeProjection> {
        self.nodes.iter()
    }
    /// Designated output node.
    #[must_use]
    pub const fn output(&self) -> usize {
        self.output
    }
}

/// One resolved node with source-filled parameter defaults.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioNodeProjection {
    anchor: u64,
    label: Option<String>,
    processor: ProcessorProjection,
    written_parameters: Vec<Option<u64>>,
    inputs: Vec<usize>,
}

impl StudioNodeProjection {
    /// Stable ordinal for the complete call.
    #[must_use]
    pub const fn anchor(&self) -> u64 {
        self.anchor
    }
    /// Optional source binding label.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    /// Source processor declaration name.
    #[must_use]
    pub const fn processor(&self) -> &'static str {
        self.processor.name()
    }
    /// Exact, source-defaulted parameters in declaration order.
    pub fn parameters(&self) -> impl ExactSizeIterator<Item = (&'static str, &'static str, &ExactQuantityProjection)> {
        self.processor
            .parameters()
            .iter()
            .map(|parameter| (parameter.name, parameter.dsp_name, &parameter.value))
    }
    /// Written-value anchor for each parameter, or `None` where source supplied the default.
    pub fn written_parameters(&self) -> impl ExactSizeIterator<Item = Option<u64>> + '_ {
        self.written_parameters.iter().copied()
    }
    /// Input node indices in argument order.
    pub fn inputs(&self) -> impl ExactSizeIterator<Item = usize> + '_ {
        self.inputs.iter().copied()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParameterProjection {
    name: &'static str,
    dsp_name: &'static str,
    value: ExactQuantityProjection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProcessorProjection {
    name: &'static str,
    parameters: Vec<ParameterProjection>,
}

impl ProcessorProjection {
    const fn name(&self) -> &'static str {
        self.name
    }
    fn parameters(&self) -> &[ParameterProjection] {
        &self.parameters
    }
}

/// One resolved part assignment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioAssignmentProjection {
    anchor: u64,
    part: String,
    instrument: String,
}
impl StudioAssignmentProjection {
    /// Statement anchor.
    #[must_use]
    pub const fn anchor(&self) -> u64 {
        self.anchor
    }
    /// Part name.
    #[must_use]
    pub fn part(&self) -> &str {
        &self.part
    }
    /// Instrument declaration name.
    #[must_use]
    pub fn instrument(&self) -> &str {
        &self.instrument
    }
}

/// One exact send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioSendProjection {
    anchor: u64,
    level_anchor: u64,
    source: String,
    bus: String,
    level: ExactQuantityProjection,
}
impl StudioSendProjection {
    /// Statement anchor.
    #[must_use]
    pub const fn anchor(&self) -> u64 {
        self.anchor
    }
    /// Written level anchor.
    #[must_use]
    pub const fn level_anchor(&self) -> u64 {
        self.level_anchor
    }
    /// Source part or bus.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Destination bus.
    #[must_use]
    pub fn bus(&self) -> &str {
        &self.bus
    }
    /// Exact source-default-free decibel quantity.
    #[must_use]
    pub const fn level(&self) -> &ExactQuantityProjection {
        &self.level
    }
}

/// One explicit route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioRouteProjection {
    anchor: u64,
    source: String,
    destination: String,
}
impl StudioRouteProjection {
    /// Statement anchor.
    #[must_use]
    pub const fn anchor(&self) -> u64 {
        self.anchor
    }
    /// Source part or bus.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Destination bus or master.
    #[must_use]
    pub fn destination(&self) -> &str {
        &self.destination
    }
}

/// One resolved control mapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioModulationProjection {
    anchor: u64,
    source: String,
    instrument: String,
    node: usize,
    parameter: String,
}
impl StudioModulationProjection {
    /// Statement anchor.
    #[must_use]
    pub const fn anchor(&self) -> u64 {
        self.anchor
    }
    /// Control signal name.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Instrument graph name.
    #[must_use]
    pub fn instrument(&self) -> &str {
        &self.instrument
    }
    /// Resolved target node.
    #[must_use]
    pub const fn node(&self) -> usize {
        self.node
    }
    /// Public source parameter name.
    #[must_use]
    pub fn parameter(&self) -> &str {
        &self.parameter
    }
}

/// Decode a checked, zonked production studio without supplying any default,
/// coercion, inference, or recovery policy.
///
/// # Errors
///
/// Returns [`StudioExecutionError`] when the checked value has another schema
/// or its canonical payload does not have the production artifact's exact
/// shape.
pub fn decode_studio_execution(source: &CheckedSource) -> Result<StudioExecution, StudioExecutionError> {
    if !source.has_valid_framing() {
        return Err(malformed("artifact", "complete exact framing"));
    }
    if source.schema() != &studio_execution_schema() {
        return Err(StudioExecutionError::WrongSchema);
    }
    let [
        version,
        anchor,
        declared,
        patches,
        buses,
        signals,
        assignments,
        routes,
        sends,
        modulations,
    ] = case_fields(source.root(), "StudioExecutionArtifact.StudioExecutionArtifact", "root")?;
    if nat(version, "root.schema_version")? != SCHEMA_VERSION {
        return Err(malformed("root.schema_version", "the checked schema version"));
    }
    Ok(StudioExecution {
        anchor: nat(anchor, "root.anchor")?,
        declared: boolean(declared, "root.declared")?,
        patches: list(patches, "root.patches", graph)?,
        buses: list(buses, "root.buses", graph)?,
        signals: list(signals, "root.signals", graph)?,
        assignments: list(assignments, "root.assignments", assignment)?,
        routes: list(routes, "root.routes", route)?,
        sends: list(sends, "root.sends", send)?,
        modulations: list(modulations, "root.modulations", modulation)?,
        exact_source: source.exact_bytes().into(),
    })
}

fn graph(datum: SourceDatum<'_>, path: &str) -> Result<StudioGraphProjection, StudioExecutionError> {
    let [anchor, name, nodes, output] = case_fields(datum, "StudioGraph.StudioGraph", path)?;
    Ok(StudioGraphProjection {
        anchor: nat(anchor, &field(path, "anchor"))?,
        name: text(name, &field(path, "name"))?,
        nodes: list(nodes, &field(path, "nodes"), node)?,
        output: index(output, &field(path, "output"))?,
    })
}

fn node(datum: SourceDatum<'_>, path: &str) -> Result<StudioNodeProjection, StudioExecutionError> {
    let [anchor, label, processor_datum, written, inputs] = case_fields(datum, "StudioNode.StudioNode", path)?;
    let processor = processor(processor_datum, &field(path, "processor"))?;
    let written_parameters = list(written, &field(path, "written_parameters"), optional_nat)?;
    if written_parameters.len() != processor.parameters().len() {
        return Err(malformed(path, "one written anchor per declared parameter"));
    }
    Ok(StudioNodeProjection {
        anchor: nat(anchor, &field(path, "anchor"))?,
        label: optional_text(label, &field(path, "label"))?,
        processor,
        written_parameters,
        inputs: list(inputs, &field(path, "inputs"), index)?,
    })
}

fn processor(datum: SourceDatum<'_>, path: &str) -> Result<ProcessorProjection, StudioExecutionError> {
    let Some(SourceDatumKind::Case { constructor }) = datum.kind() else {
        return Err(malformed(path, "a StudioProcessor constructor"));
    };
    let (name, specs): (&'static str, &[(&'static str, &'static str, SoundDimension, SoundUnit)]) = match constructor {
        "StudioProcessor.Oscillator" => (
            "oscillator",
            &[
                ("frequency", "frequency", SoundDimension::Frequency, SoundUnit::Hertz),
                ("ratio", "ratio", SoundDimension::LinearAmplitude, SoundUnit::Linear),
            ],
        ),
        "StudioProcessor.Gain" => ("gain", &[("gain", "gain", SoundDimension::Level, SoundUnit::Decibels)]),
        "StudioProcessor.Mix" => ("mix", &[]),
        "StudioProcessor.Envelope" => (
            "envelope",
            &[
                ("attack", "attack", SoundDimension::Time, SoundUnit::Seconds),
                ("decay", "decay", SoundDimension::Time, SoundUnit::Seconds),
                ("sustain", "sustain", SoundDimension::LinearAmplitude, SoundUnit::Linear),
                ("release", "release", SoundDimension::Time, SoundUnit::Seconds),
            ],
        ),
        "StudioProcessor.Lowpass" => (
            "lowpass",
            &[
                ("cutoff", "cutoff", SoundDimension::Frequency, SoundUnit::Hertz),
                ("resonance", "q", SoundDimension::LinearAmplitude, SoundUnit::Linear),
            ],
        ),
        "StudioProcessor.Highpass" => (
            "highpass",
            &[
                ("cutoff", "cutoff", SoundDimension::Frequency, SoundUnit::Hertz),
                ("resonance", "q", SoundDimension::LinearAmplitude, SoundUnit::Linear),
            ],
        ),
        "StudioProcessor.Reverb" => (
            "reverb",
            &[
                ("room", "room", SoundDimension::LinearAmplitude, SoundUnit::Linear),
                ("damping", "damping", SoundDimension::LinearAmplitude, SoundUnit::Linear),
                ("mix", "mix", SoundDimension::LinearAmplitude, SoundUnit::Linear),
            ],
        ),
        "StudioProcessor.Delay" => (
            "delay",
            &[
                ("time", "time", SoundDimension::Time, SoundUnit::Seconds),
                (
                    "feedback",
                    "feedback",
                    SoundDimension::LinearAmplitude,
                    SoundUnit::Linear,
                ),
                ("mix", "mix", SoundDimension::LinearAmplitude, SoundUnit::Linear),
            ],
        ),
        "StudioProcessor.Chorus" => (
            "chorus",
            &[
                ("rate", "rate", SoundDimension::Frequency, SoundUnit::Hertz),
                ("depth", "depth", SoundDimension::Time, SoundUnit::Seconds),
                ("mix", "mix", SoundDimension::LinearAmplitude, SoundUnit::Linear),
            ],
        ),
        "StudioProcessor.Scale" => (
            "scale",
            &[("factor", "factor", SoundDimension::Frequency, SoundUnit::Hertz)],
        ),
        "StudioProcessor.Bias" => (
            "bias",
            &[("offset", "offset", SoundDimension::Frequency, SoundUnit::Hertz)],
        ),
        "StudioProcessor.Clamp" => (
            "clamp",
            &[
                ("min", "min", SoundDimension::Frequency, SoundUnit::Hertz),
                ("max", "max", SoundDimension::Frequency, SoundUnit::Hertz),
            ],
        ),
        "StudioProcessor.Smoothing" => (
            "smoothing",
            &[("time", "time", SoundDimension::Time, SoundUnit::Seconds)],
        ),
        _ => return Err(malformed(path, "a StudioProcessor constructor")),
    };
    let values = fields_vec(datum, path)?;
    if values.len() != specs.len() {
        return Err(malformed(path, "the exact processor field count"));
    }
    let parameters = values
        .into_iter()
        .zip(specs)
        .enumerate()
        .map(|(index, (value, (parameter, dsp_name, dimension, unit)))| {
            let value = quantity(value, &format!("{path}.{parameter}"))?;
            if value.dimension() != *dimension || value.unit() != *unit {
                return Err(malformed(
                    &format!("{path}[{index}]"),
                    "the source-indexed parameter dimension",
                ));
            }
            Ok(ParameterProjection {
                name: parameter,
                dsp_name,
                value,
            })
        })
        .collect::<Result<_, _>>()?;
    Ok(ProcessorProjection { name, parameters })
}

fn assignment(datum: SourceDatum<'_>, path: &str) -> Result<StudioAssignmentProjection, StudioExecutionError> {
    let [anchor, part, instrument] = case_fields(datum, "StudioAssignment.StudioAssignment", path)?;
    Ok(StudioAssignmentProjection {
        anchor: nat(anchor, path)?,
        part: text(part, path)?,
        instrument: text(instrument, path)?,
    })
}
fn route(datum: SourceDatum<'_>, path: &str) -> Result<StudioRouteProjection, StudioExecutionError> {
    let [anchor, source, destination] = case_fields(datum, "StudioRoute.StudioRoute", path)?;
    Ok(StudioRouteProjection {
        anchor: nat(anchor, path)?,
        source: text(source, path)?,
        destination: text(destination, path)?,
    })
}
fn send(datum: SourceDatum<'_>, path: &str) -> Result<StudioSendProjection, StudioExecutionError> {
    let [anchor, level_anchor, source, bus, level] = case_fields(datum, "StudioSend.StudioSend", path)?;
    let level = quantity(level, &field(path, "level"))?;
    if level.dimension() != SoundDimension::Level || level.unit() != SoundUnit::Decibels {
        return Err(malformed(path, "an exact decibel send level"));
    }
    Ok(StudioSendProjection {
        anchor: nat(anchor, path)?,
        level_anchor: nat(level_anchor, path)?,
        source: text(source, path)?,
        bus: text(bus, path)?,
        level,
    })
}
fn modulation(datum: SourceDatum<'_>, path: &str) -> Result<StudioModulationProjection, StudioExecutionError> {
    let [anchor, source, instrument, node, parameter] = case_fields(datum, "StudioModulation.StudioModulation", path)?;
    Ok(StudioModulationProjection {
        anchor: nat(anchor, path)?,
        source: text(source, path)?,
        instrument: text(instrument, path)?,
        node: index(node, path)?,
        parameter: text(parameter, path)?,
    })
}

fn quantity(datum: SourceDatum<'_>, path: &str) -> Result<ExactQuantityProjection, StudioExecutionError> {
    let [dimension, magnitude, unit] = case_fields(datum, "SoundQuantity.Written", path)?;
    let dimension = dimension_value(dimension, path)?;
    let unit = unit_value(unit, path)?;
    if unit_dimension(unit) != dimension {
        return Err(malformed(path, "matching source dimension and unit indices"));
    }
    Ok(ExactQuantityProjection::from_checked_parts(
        dimension,
        ratio(magnitude, path)?,
        unit,
    ))
}

fn dimension_value(datum: SourceDatum<'_>, path: &str) -> Result<SoundDimension, StudioExecutionError> {
    let Some(SourceDatumKind::Case { constructor }) = datum.kind() else {
        return Err(malformed(path, "a SoundDimension constructor"));
    };
    let [] = fields(datum, path)?;
    match constructor {
        "SoundDimension.Frequency" => Ok(SoundDimension::Frequency),
        "SoundDimension.LinearAmplitude" => Ok(SoundDimension::LinearAmplitude),
        "SoundDimension.Level" => Ok(SoundDimension::Level),
        "SoundDimension.Time" => Ok(SoundDimension::Time),
        _ => Err(malformed(path, "a SoundDimension constructor")),
    }
}
fn unit_value(datum: SourceDatum<'_>, path: &str) -> Result<SoundUnit, StudioExecutionError> {
    let Some(SourceDatumKind::Case { constructor }) = datum.kind() else {
        return Err(malformed(path, "a SoundUnit constructor"));
    };
    let [] = fields(datum, path)?;
    match constructor {
        "SoundUnit.Hertz" => Ok(SoundUnit::Hertz),
        "SoundUnit.Linear" => Ok(SoundUnit::Linear),
        "SoundUnit.Decibels" => Ok(SoundUnit::Decibels),
        "SoundUnit.Seconds" => Ok(SoundUnit::Seconds),
        _ => Err(malformed(path, "a SoundUnit constructor")),
    }
}
const fn unit_dimension(unit: SoundUnit) -> SoundDimension {
    match unit {
        SoundUnit::Hertz => SoundDimension::Frequency,
        SoundUnit::Linear => SoundDimension::LinearAmplitude,
        SoundUnit::Decibels => SoundDimension::Level,
        SoundUnit::Seconds => SoundDimension::Time,
    }
}

fn optional_nat(datum: SourceDatum<'_>, path: &str) -> Result<Option<u64>, StudioExecutionError> {
    match datum.kind() {
        Some(SourceDatumKind::Case {
            constructor: "Option.None",
        }) => {
            let [] = fields(datum, path)?;
            Ok(None)
        }
        Some(SourceDatumKind::Case {
            constructor: "Option.Some",
        }) => {
            let [value] = fields(datum, path)?;
            Ok(Some(nat(value, path)?))
        }
        _ => Err(malformed(path, "an Option(Nat)")),
    }
}
fn optional_text(datum: SourceDatum<'_>, path: &str) -> Result<Option<String>, StudioExecutionError> {
    match datum.kind() {
        Some(SourceDatumKind::Case {
            constructor: "Option.None",
        }) => {
            let [] = fields(datum, path)?;
            Ok(None)
        }
        Some(SourceDatumKind::Case {
            constructor: "Option.Some",
        }) => {
            let [value] = fields(datum, path)?;
            Ok(Some(text(value, path)?))
        }
        _ => Err(malformed(path, "an Option(Text)")),
    }
}
fn boolean(datum: SourceDatum<'_>, path: &str) -> Result<bool, StudioExecutionError> {
    let Some(SourceDatumKind::Case { constructor }) = datum.kind() else {
        return Err(malformed(path, "a Bool"));
    };
    let [] = fields(datum, path)?;
    match constructor {
        "Bool.True" => Ok(true),
        "Bool.False" => Ok(false),
        _ => Err(malformed(path, "a Bool")),
    }
}
fn nat(datum: SourceDatum<'_>, path: &str) -> Result<u64, StudioExecutionError> {
    match datum.kind() {
        Some(SourceDatumKind::Count { family: "Nat", count }) => Ok(count),
        _ => Err(malformed(path, "a Nat")),
    }
}
fn index(datum: SourceDatum<'_>, path: &str) -> Result<usize, StudioExecutionError> {
    usize::try_from(nat(datum, path)?).map_err(|_| malformed(path, "a host-sized Nat"))
}
fn text(datum: SourceDatum<'_>, path: &str) -> Result<String, StudioExecutionError> {
    match datum.kind() {
        Some(SourceDatumKind::Literal {
            type_name: "Text",
            bytes,
        }) => String::from_utf8(bytes.to_vec()).map_err(|_| malformed(path, "UTF-8 Text")),
        _ => Err(malformed(path, "Text")),
    }
}
fn ratio(datum: SourceDatum<'_>, path: &str) -> Result<Ratio<i64>, StudioExecutionError> {
    let Some(SourceDatumKind::Literal {
        type_name: "Ratio",
        bytes,
    }) = datum.kind()
    else {
        return Err(malformed(path, "an exact Ratio"));
    };
    if bytes.len() != 16 {
        return Err(malformed(path, "an exact Ratio encoding"));
    }
    let (n, d) = bytes.split_at(8);
    let numerator = i64::from_be_bytes(n.try_into().map_err(|_| malformed(path, "a Ratio numerator"))?);
    let denominator = i64::from_be_bytes(d.try_into().map_err(|_| malformed(path, "a Ratio denominator"))?);
    if denominator <= 0 {
        return Err(malformed(path, "a normalized Ratio"));
    }
    let value = Ratio::new(numerator, denominator);
    if value.numer() != &numerator || value.denom() != &denominator {
        return Err(malformed(path, "a normalized Ratio"));
    }
    Ok(value)
}

fn list<T>(
    mut datum: SourceDatum<'_>,
    path: &str,
    decode: impl Fn(SourceDatum<'_>, &str) -> Result<T, StudioExecutionError>,
) -> Result<Vec<T>, StudioExecutionError> {
    let mut values = Vec::new();
    loop {
        match datum.kind() {
            Some(SourceDatumKind::Case {
                constructor: "List.Empty",
            }) => {
                let [] = fields(datum, path)?;
                return Ok(values);
            }
            Some(SourceDatumKind::Case {
                constructor: "List.Cons",
            }) => {
                let [head, tail] = fields(datum, path)?;
                values.push(decode(head, &format!("{path}[{}]", values.len()))?);
                datum = tail;
            }
            _ => return Err(malformed(path, "a complete List")),
        }
    }
}
fn case_fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    constructor: &str,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], StudioExecutionError> {
    match datum.kind() {
        Some(SourceDatumKind::Case { constructor: found }) if found == constructor => fields(datum, path),
        _ => Err(malformed(path, "the required source constructor")),
    }
}
fn fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], StudioExecutionError> {
    fields_vec(datum, path)?
        .try_into()
        .map_err(|_| malformed(path, "the exact source field count"))
}
fn fields_vec<'a>(datum: SourceDatum<'a>, path: &str) -> Result<Vec<SourceDatum<'a>>, StudioExecutionError> {
    let Some(direct) = datum.fields() else {
        return Err(malformed(path, "valid canonical child framing"));
    };
    let Some(fields): Option<Vec<_>> = direct.collect() else {
        return Err(malformed(path, "valid canonical child indices"));
    };
    Ok(fields)
}
fn field(path: &str, name: &str) -> String {
    format!("{path}.{name}")
}
fn malformed(path: &str, expected: &'static str) -> StudioExecutionError {
    StudioExecutionError::Malformed {
        path: path.to_owned(),
        expected,
    }
}
