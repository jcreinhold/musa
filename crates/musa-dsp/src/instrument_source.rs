//! Read-only preparation projection of checked source instrument contracts.
//!
//! The source declaration remains the schema and exact identity. This module
//! only extracts the fields preparation must query; none of these types has a
//! public constructor or supplies a default, coercion, or inference rule.

use std::sync::Arc;

use musa_calculus::{CheckedSource, SourceDatum, SourceDatumKind, SourceSchema};
use num_rational::Ratio;
use thiserror::Error;

const SCHEMA_NAME: &str = "std.sound.instrument.InstrumentExecutionArtifact";
const ROOT_TYPE: &str = "InstrumentExecutionArtifact";
const VERSION: u64 = 2;

/// Exact consumer schema of the source instrument package.
#[must_use]
pub fn instrument_contracts_schema() -> SourceSchema {
    SourceSchema::new(SCHEMA_NAME, ROOT_TYPE, VERSION)
}

/// One source-declared control accepted by an instrument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentControlContract {
    kind: String,
    namespace: String,
    name: String,
    update_rate: String,
    default_ratio: Option<Ratio<i64>>,
    default_symbol: Option<String>,
    default_exact: Arc<[u8]>,
}

impl InstrumentControlContract {
    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn update_rate(&self) -> &str {
        &self.update_rate
    }

    pub const fn default_ratio(&self) -> Option<Ratio<i64>> {
        self.default_ratio
    }

    pub fn default_symbol(&self) -> Option<&str> {
        self.default_symbol.as_deref()
    }

    pub fn default_exact_bytes(&self) -> &[u8] {
        &self.default_exact
    }
}

/// One read-only exact mapping projected from a checked private source body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentControlMapping {
    kind: String,
    namespace: String,
    name: String,
    node: String,
    parameter: String,
    minimum: Option<Ratio<i64>>,
    maximum: Option<Ratio<i64>>,
    inverse: bool,
    connection_values: Option<[Ratio<i64>; 3]>,
}

impl InstrumentControlMapping {
    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn node(&self) -> &str {
        &self.node
    }

    pub fn parameter(&self) -> &str {
        &self.parameter
    }

    pub const fn transfer(&self) -> Option<(Ratio<i64>, Ratio<i64>, bool)> {
        match (self.minimum, self.maximum) {
            (Some(minimum), Some(maximum)) => Some((minimum, maximum, self.inverse)),
            _ => None,
        }
    }

    pub const fn connection_values(&self) -> Option<[Ratio<i64>; 3]> {
        self.connection_values
    }
}

/// One source-declared technique and its explicit fallback policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentTechniqueContract {
    namespace: String,
    name: String,
    notation_only_warning: bool,
}

/// One bounded MIDI wire dimension named by source audition policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MidiAuditionInputKind {
    /// Note-on attack velocity.
    AttackVelocity,
    /// Note-off release velocity.
    ReleaseVelocity,
    /// Damper pedal (CC64).
    SustainPedal,
    /// Sostenuto pedal (CC66).
    SostenutoPedal,
    /// Soft pedal (CC67).
    SoftPedal,
    /// Fourteen-bit channel pitch bend.
    PitchBend,
    /// Channel pressure.
    ChannelPressure,
    /// Polyphonic key pressure.
    KeyPressure,
    /// An explicitly named seven-bit controller.
    Controller(u8),
}

/// Whether a source audition binding follows a channel or one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MidiAuditionScope {
    /// One value shared by the channel.
    PerChannel,
    /// One value associated with a key/voice.
    PerKey,
}

/// One exact, read-only MIDI audition binding projected from source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentAuditionBinding {
    input: MidiAuditionInputKind,
    scope: MidiAuditionScope,
    namespace: String,
    name: String,
    input_minimum: Ratio<i64>,
    input_maximum: Ratio<i64>,
    output_minimum: Ratio<i64>,
    output_maximum: Ratio<i64>,
    dead_zone: Ratio<i64>,
    switch_threshold: Option<Ratio<i64>>,
}

impl InstrumentAuditionBinding {
    /// Hardware dimension interpreted by this binding.
    pub const fn input(&self) -> MidiAuditionInputKind {
        self.input
    }

    /// Channel- or key-local scope stated by source.
    pub const fn scope(&self) -> MidiAuditionScope {
        self.scope
    }

    /// Namespace of the accepted semantic control.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Name of the accepted semantic control.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Inclusive exact input range.
    pub const fn input_range(&self) -> (Ratio<i64>, Ratio<i64>) {
        (self.input_minimum, self.input_maximum)
    }

    /// Exact output endpoints; their order owns direction.
    pub const fn output_range(&self) -> (Ratio<i64>, Ratio<i64>) {
        (self.output_minimum, self.output_maximum)
    }

    /// Exact symmetric dead zone in input units.
    pub const fn dead_zone(&self) -> Ratio<i64> {
        self.dead_zone
    }

    /// Optional exact switch threshold.
    pub const fn switch_threshold(&self) -> Option<Ratio<i64>> {
        self.switch_threshold
    }
}

impl InstrumentTechniqueContract {
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn notation_only_warning(&self) -> bool {
        self.notation_only_warning
    }
}

/// One complete public instrument signature projected from checked source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentContract {
    declaration_id: String,
    name: String,
    summary: String,
    controls: Vec<InstrumentControlContract>,
    mappings: Vec<InstrumentControlMapping>,
    audition_bindings: Vec<InstrumentAuditionBinding>,
    techniques: Vec<InstrumentTechniqueContract>,
    channels: u8,
    implementation_id: String,
    implementation_exact: Arc<[u8]>,
}

impl InstrumentContract {
    pub fn declaration_id(&self) -> &str {
        &self.declaration_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn controls(&self) -> &[InstrumentControlContract] {
        &self.controls
    }

    pub fn mappings(&self) -> &[InstrumentControlMapping] {
        &self.mappings
    }

    /// Source-owned interpretations of MIDI audition evidence.
    pub fn audition_bindings(&self) -> &[InstrumentAuditionBinding] {
        &self.audition_bindings
    }

    pub fn techniques(&self) -> &[InstrumentTechniqueContract] {
        &self.techniques
    }

    pub const fn channels(&self) -> u8 {
        self.channels
    }

    pub fn implementation_id(&self) -> &str {
        &self.implementation_id
    }

    pub fn implementation_exact_bytes(&self) -> &[u8] {
        &self.implementation_exact
    }
}

/// Complete opaque projection of one checked instrument artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentContracts {
    declarations: Vec<InstrumentContract>,
    exact_source: Arc<[u8]>,
}

impl InstrumentContracts {
    pub fn declarations(&self) -> &[InstrumentContract] {
        &self.declarations
    }

    pub fn declaration(&self, id: &str) -> Option<&InstrumentContract> {
        self.declarations
            .iter()
            .find(|declaration| declaration.declaration_id == id)
    }

    pub fn exact_source_bytes(&self) -> &[u8] {
        &self.exact_source
    }
}

/// Why a checked value is not the declared instrument artifact.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum InstrumentContractsError {
    #[error("expected checked source schema `{SCHEMA_NAME}` version {VERSION}")]
    WrongSchema,
    #[error("malformed InstrumentExecutionArtifact at {path}: {expected}")]
    Malformed { path: String, expected: &'static str },
}

/// Decode the exact read-only projection used by preparation.
///
/// # Errors
///
/// Returns [`InstrumentContractsError`] when the artifact has the wrong
/// schema or framing, malformed source data, mismatched indices, duplicate
/// identities, or a public declaration without its private implementation
/// contract.
pub fn decode_instrument_contracts(source: &CheckedSource) -> Result<InstrumentContracts, InstrumentContractsError> {
    if source.schema() != &instrument_contracts_schema() || !source.has_valid_framing() {
        return Err(InstrumentContractsError::WrongSchema);
    }
    let [version, instruments, implementations] = fields::<3>(source.root(), ROOT_TYPE, "root")?;
    if nat(version, "root.schema_version")? != VERSION {
        return Err(malformed("root.schema_version", "schema version 2"));
    }
    let instruments = list(instruments, "root.instruments", instrument)?;
    let implementations = list(implementations, "root.implementation_contracts", implementation)?;
    if instruments.len() != implementations.len() {
        return Err(malformed(
            "root.implementation_contracts",
            "one implementation identity per instrument",
        ));
    }
    let mut declarations = Vec::with_capacity(instruments.len());
    for (mut declaration, (implementation_id, mappings, audition_bindings, implementation_exact)) in
        instruments.into_iter().zip(implementations)
    {
        if declaration.declaration_id != implementation_id {
            return Err(malformed(
                "root.implementation_contracts",
                "the private body identity matching its public declaration",
            ));
        }
        declaration.implementation_id = implementation_id;
        for mapping in &mappings {
            if !declaration.controls.iter().any(|control| {
                control.kind == mapping.kind && control.namespace == mapping.namespace && control.name == mapping.name
            }) {
                return Err(malformed(
                    "root.implementation_contracts",
                    "every private mapping to implement one exposed control at the same source index",
                ));
            }
        }
        let mut targets = std::collections::HashSet::new();
        if mappings.iter().any(|mapping| {
            !targets.insert((
                mapping.namespace.as_str(),
                mapping.name.as_str(),
                mapping.node.as_str(),
                mapping.parameter.as_str(),
            ))
        }) {
            return Err(malformed(
                "root.implementation_contracts",
                "distinct targets per mapped control",
            ));
        }
        declaration.mappings = mappings;
        for binding in &audition_bindings {
            let accepted = declaration.controls.iter().any(|control| {
                control.kind == "Normalized" && control.namespace == binding.namespace && control.name == binding.name
            });
            let mapped = declaration.mappings.iter().any(|mapping| {
                mapping.kind == "Normalized" && mapping.namespace == binding.namespace && mapping.name == binding.name
            });
            if !accepted || !mapped {
                return Err(malformed(
                    "root.implementation_contracts",
                    "every audition binding to reach one accepted and privately mapped normalized control",
                ));
            }
        }
        let mut audition_targets = std::collections::HashSet::new();
        if audition_bindings.iter().any(|binding| {
            !audition_targets.insert((
                binding.input,
                binding.scope,
                binding.namespace.as_str(),
                binding.name.as_str(),
            ))
        }) {
            return Err(malformed(
                "root.implementation_contracts",
                "distinct MIDI input/scope/control audition bindings",
            ));
        }
        declaration.audition_bindings = audition_bindings;
        declaration.implementation_exact = implementation_exact;
        declarations.push(declaration);
    }
    let mut identities = std::collections::HashSet::new();
    if declarations
        .iter()
        .any(|declaration| !identities.insert(declaration.declaration_id.as_str()))
    {
        return Err(malformed("root.instruments", "distinct declaration identities"));
    }
    Ok(InstrumentContracts {
        declarations,
        exact_source: source.exact_bytes().into(),
    })
}

fn instrument(datum: SourceDatum<'_>, path: &str) -> Result<InstrumentContract, InstrumentContractsError> {
    let [declaration_id, signature] = fields::<2>(datum, "Instrument", path)?;
    let declaration_id = text(declaration_id, &format!("{path}.declaration_id"))?;
    let [name, summary, controls, techniques, channels] = fields::<5>(signature, "InstrumentSignature", path)?;
    Ok(InstrumentContract {
        declaration_id,
        name: text(name, &format!("{path}.signature.name"))?,
        summary: text(summary, &format!("{path}.signature.summary"))?,
        controls: list(controls, &format!("{path}.signature.controls"), control)?,
        mappings: Vec::new(),
        audition_bindings: Vec::new(),
        techniques: list(techniques, &format!("{path}.signature.techniques"), technique)?,
        channels: match constructor(channels) {
            Some("Mono") => 1,
            Some("Stereo") => 2,
            _ => return Err(malformed(&format!("{path}.signature.channels"), "Mono or Stereo")),
        },
        implementation_id: String::new(),
        implementation_exact: Arc::from([]),
    })
}

fn implementation(
    datum: SourceDatum<'_>,
    path: &str,
) -> Result<
    (
        String,
        Vec<InstrumentControlMapping>,
        Vec<InstrumentAuditionBinding>,
        Arc<[u8]>,
    ),
    InstrumentContractsError,
> {
    let [declaration_id, mappings, audition_bindings] = fields::<3>(datum, "InstrumentImplementationContract", path)?;
    Ok((
        text(declaration_id, &format!("{path}.declaration_id"))?,
        list(mappings, &format!("{path}.mappings"), mapping)?,
        list(
            audition_bindings,
            &format!("{path}.audition_bindings"),
            audition_binding,
        )?,
        datum.exact_bytes().into(),
    ))
}

fn audition_binding(datum: SourceDatum<'_>, path: &str) -> Result<InstrumentAuditionBinding, InstrumentContractsError> {
    let [input, scope, key, transfer] = fields::<4>(datum, "MidiAuditionBinding", path)?;
    let input = match constructor(input) {
        Some("AttackVelocity") => MidiAuditionInputKind::AttackVelocity,
        Some("ReleaseVelocity") => MidiAuditionInputKind::ReleaseVelocity,
        Some("SustainPedal") => MidiAuditionInputKind::SustainPedal,
        Some("SostenutoPedal") => MidiAuditionInputKind::SostenutoPedal,
        Some("SoftPedal") => MidiAuditionInputKind::SoftPedal,
        Some("PitchBend") => MidiAuditionInputKind::PitchBend,
        Some("ChannelPressure") => MidiAuditionInputKind::ChannelPressure,
        Some("KeyPressure") => MidiAuditionInputKind::KeyPressure,
        Some("Controller") => {
            let [number] = fields::<1>(input, "Controller", path)?;
            let number = nat(number, &format!("{path}.input.number"))?;
            MidiAuditionInputKind::Controller(
                u8::try_from(number)
                    .ok()
                    .filter(|number| *number <= 127)
                    .ok_or_else(|| malformed(&format!("{path}.input.number"), "a seven-bit controller number"))?,
            )
        }
        _ => return Err(malformed(&format!("{path}.input"), "a MidiAuditionInput constructor")),
    };
    let scope = match constructor(scope) {
        Some("PerChannel") => MidiAuditionScope::PerChannel,
        Some("PerKey") => MidiAuditionScope::PerKey,
        _ => return Err(malformed(&format!("{path}.scope"), "PerChannel or PerKey")),
    };
    let [kind, namespace, name, _, _] = fields::<5>(key, "Key", &format!("{path}.control_key"))?;
    if constructor(kind) != Some("Normalized") {
        return Err(malformed(&format!("{path}.control_key"), "a normalized control key"));
    }
    let [
        input_minimum,
        input_maximum,
        output_minimum,
        output_maximum,
        dead_zone,
        switch_threshold,
    ] = fields::<6>(transfer, "MidiAuditionTransfer", &format!("{path}.transfer"))?;
    let input_minimum = ratio(input_minimum, path)?;
    let input_maximum = ratio(input_maximum, path)?;
    let output_minimum = ratio(output_minimum, path)?;
    let output_maximum = ratio(output_maximum, path)?;
    let dead_zone = ratio(dead_zone, path)?;
    let switch_threshold = option_ratio(switch_threshold, &format!("{path}.transfer.switch_threshold"))?;
    if input_maximum <= input_minimum || dead_zone < Ratio::ZERO {
        return Err(malformed(
            &format!("{path}.transfer"),
            "an increasing input range and nonnegative dead zone",
        ));
    }
    if !(Ratio::ZERO..=Ratio::ONE).contains(&output_minimum) || !(Ratio::ZERO..=Ratio::ONE).contains(&output_maximum) {
        return Err(malformed(&format!("{path}.transfer"), "normalized output endpoints"));
    }
    if switch_threshold.is_some_and(|threshold| threshold < input_minimum || threshold > input_maximum) {
        return Err(malformed(
            &format!("{path}.transfer.switch_threshold"),
            "a threshold inside the input range",
        ));
    }
    Ok(InstrumentAuditionBinding {
        input,
        scope,
        namespace: text(namespace, &format!("{path}.namespace"))?,
        name: text(name, &format!("{path}.name"))?,
        input_minimum,
        input_maximum,
        output_minimum,
        output_maximum,
        dead_zone,
        switch_threshold,
    })
}

fn control(datum: SourceDatum<'_>, path: &str) -> Result<InstrumentControlContract, InstrumentContractsError> {
    let [kind, requirement] = fields::<2>(datum, "AcceptsControl", path)?;
    let kind = constructor(kind)
        .ok_or_else(|| malformed(&format!("{path}.kind"), "a ControlKind constructor"))?
        .to_owned();
    let [requirement_kind, key, default] = fields::<3>(requirement, "RequiredControl", path)?;
    if constructor(requirement_kind) != Some(kind.as_str()) {
        return Err(malformed(path, "one shared control-kind index"));
    }
    let [key_kind, namespace, name, _, update_rate] = fields::<5>(key, "Key", path)?;
    if constructor(key_kind) != Some(kind.as_str()) {
        return Err(malformed(path, "a key at the requirement's control kind"));
    }
    let default_ratio = numeric_control_value(default, &kind, &format!("{path}.default_value"))?;
    let default_symbol = symbolic_control_value(default, &kind, &format!("{path}.default_value"))?;
    Ok(InstrumentControlContract {
        kind,
        namespace: text(namespace, &format!("{path}.namespace"))?,
        name: text(name, &format!("{path}.name"))?,
        update_rate: constructor(update_rate)
            .ok_or_else(|| malformed(&format!("{path}.update_rate"), "an UpdateRate constructor"))?
            .to_owned(),
        default_ratio,
        default_symbol,
        default_exact: default.exact_bytes().into(),
    })
}

fn mapping(datum: SourceDatum<'_>, path: &str) -> Result<InstrumentControlMapping, InstrumentContractsError> {
    let [kind, mapping] = fields::<2>(datum, "MapsControl", path)?;
    let kind = constructor(kind)
        .ok_or_else(|| malformed(&format!("{path}.kind"), "a ControlKind constructor"))?
        .to_owned();
    let (key, target, minimum, maximum, inverse, connection_values) = match constructor(mapping) {
        Some("NormalizedMapping") => {
            let [mapping_kind, key, target, transfer] = fields::<4>(mapping, "NormalizedMapping", path)?;
            if constructor(mapping_kind) != Some(kind.as_str()) || kind != "Normalized" {
                return Err(malformed(path, "one shared normalized mapping index"));
            }
            let [minimum, maximum] = transfer
                .fields()
                .ok_or_else(|| malformed(path, "a normalized transfer"))?
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| malformed(path, "a normalized transfer"))?
                .try_into()
                .map_err(|_| malformed(path, "two transfer endpoints"))?;
            let inverse = match constructor(transfer) {
                Some("LinearTransfer") => false,
                Some("InverseTransfer") => true,
                _ => return Err(malformed(path, "LinearTransfer or InverseTransfer")),
            };
            (
                key,
                target,
                Some(ratio(minimum, path)?),
                Some(ratio(maximum, path)?),
                inverse,
                None,
            )
        }
        Some("ExactRatioMapping") => {
            let [key, target] = fields::<2>(mapping, "ExactRatioMapping", path)?;
            if kind != "ExactRatio" {
                return Err(malformed(path, "one shared exact-ratio mapping index"));
            }
            (key, target, None, None, false, None)
        }
        Some("ConnectionMapping") => {
            let [key, target, transfer] = fields::<3>(mapping, "ConnectionMapping", path)?;
            if kind != "PhraseConnection" {
                return Err(malformed(path, "one shared phrase-connection mapping index"));
            }
            let [detached, ordinary, legato] = fields::<3>(transfer, "ConnectionTransfer", path)?;
            (
                key,
                target,
                None,
                None,
                false,
                Some([ratio(detached, path)?, ratio(ordinary, path)?, ratio(legato, path)?]),
            )
        }
        _ => return Err(malformed(path, "a ControlMapping constructor")),
    };
    let [key_kind, namespace, name, _, _] = fields::<5>(key, "Key", path)?;
    if constructor(key_kind) != Some(kind.as_str()) {
        return Err(malformed(path, "a key at the mapping's control kind"));
    }
    let [node, parameter] = fields::<2>(target, "ParameterTarget", path)?;
    Ok(InstrumentControlMapping {
        kind,
        namespace: text(namespace, &format!("{path}.namespace"))?,
        name: text(name, &format!("{path}.name"))?,
        node: text(node, &format!("{path}.target.node"))?,
        parameter: text(parameter, &format!("{path}.target.parameter"))?,
        minimum,
        maximum,
        inverse,
        connection_values,
    })
}

fn symbolic_control_value(
    datum: SourceDatum<'_>,
    kind: &str,
    path: &str,
) -> Result<Option<String>, InstrumentContractsError> {
    if kind != "PhraseConnection" {
        return Ok(None);
    }
    let [connection] = fields::<1>(datum, "ConnectionValue", path)?;
    constructor(connection)
        .map(str::to_owned)
        .map(Some)
        .ok_or_else(|| malformed(path, "a Connection constructor"))
}

fn numeric_control_value(
    datum: SourceDatum<'_>,
    kind: &str,
    path: &str,
) -> Result<Option<Ratio<i64>>, InstrumentContractsError> {
    match kind {
        "Normalized" => {
            let [value] = fields::<1>(datum, "NormalizedValue", path)?;
            Ok(Some(ratio(value, path)?))
        }
        "ExactRatio" => {
            let [value] = fields::<1>(datum, "ExactRatioValue", path)?;
            Ok(Some(ratio(value, path)?))
        }
        _ => Ok(None),
    }
}

fn technique(datum: SourceDatum<'_>, path: &str) -> Result<InstrumentTechniqueContract, InstrumentContractsError> {
    let [namespace, name, fallback] = fields::<3>(datum, "TechniqueSupport", path)?;
    let notation_only_warning = match constructor(fallback) {
        Some("NotationOnlyWarning") => true,
        Some("TechniqueRequired") => false,
        _ => return Err(malformed(&format!("{path}.fallback"), "a TechniqueFallback")),
    };
    Ok(InstrumentTechniqueContract {
        namespace: text(namespace, &format!("{path}.namespace"))?,
        name: text(name, &format!("{path}.name"))?,
        notation_only_warning,
    })
}

fn list<T>(
    mut datum: SourceDatum<'_>,
    path: &str,
    mut decode: impl FnMut(SourceDatum<'_>, &str) -> Result<T, InstrumentContractsError>,
) -> Result<Vec<T>, InstrumentContractsError> {
    let mut values = Vec::new();
    loop {
        match constructor(datum) {
            Some("Empty") => return Ok(values),
            Some("Cons") => {
                let [head, tail] = fields::<2>(datum, "Cons", path)?;
                values.push(decode(head, &format!("{path}.{}", values.len()))?);
                datum = tail;
            }
            _ => return Err(malformed(path, "List")),
        }
    }
}

fn fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    expected: &str,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], InstrumentContractsError> {
    if constructor(datum) != Some(expected) {
        return Err(malformed(path, "the expected source constructor"));
    }
    datum
        .fields()
        .ok_or_else(|| malformed(path, "constructor fields"))?
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| malformed(path, "canonical constructor fields"))?
        .try_into()
        .map_err(|_| malformed(path, "the expected field count"))
}

fn constructor(datum: SourceDatum<'_>) -> Option<&str> {
    match datum.kind()? {
        SourceDatumKind::Case { constructor } => constructor.rsplit('.').next(),
        SourceDatumKind::Literal { .. } | SourceDatumKind::Count { .. } => None,
    }
}

fn nat(datum: SourceDatum<'_>, path: &str) -> Result<u64, InstrumentContractsError> {
    match datum.kind() {
        Some(SourceDatumKind::Count { family, count }) if family.ends_with("Nat") => Ok(count),
        _ => Err(malformed(path, "Nat")),
    }
}

fn text(datum: SourceDatum<'_>, path: &str) -> Result<String, InstrumentContractsError> {
    match datum.kind() {
        Some(SourceDatumKind::Literal { type_name, bytes }) if type_name.ends_with("Text") => {
            std::str::from_utf8(bytes)
                .map(str::to_owned)
                .map_err(|_| malformed(path, "UTF-8 Text"))
        }
        _ => Err(malformed(path, "Text")),
    }
}

fn ratio(datum: SourceDatum<'_>, path: &str) -> Result<Ratio<i64>, InstrumentContractsError> {
    match datum.kind() {
        Some(SourceDatumKind::Literal { type_name, bytes }) if type_name.ends_with("Ratio") && bytes.len() == 16 => {
            let (numerator, denominator) = bytes.split_at(8);
            let numerator = i64::from_be_bytes(numerator.try_into().map_err(|_| malformed(path, "Ratio"))?);
            let denominator = i64::from_be_bytes(denominator.try_into().map_err(|_| malformed(path, "Ratio"))?);
            if denominator == 0 {
                return Err(malformed(path, "a nonzero Ratio denominator"));
            }
            Ok(Ratio::new(numerator, denominator))
        }
        _ => Err(malformed(path, "Ratio")),
    }
}

fn option_ratio(datum: SourceDatum<'_>, path: &str) -> Result<Option<Ratio<i64>>, InstrumentContractsError> {
    match constructor(datum) {
        Some("None") => Ok(None),
        Some("Some") => {
            let [value] = fields::<1>(datum, "Some", path)?;
            ratio(value, path).map(Some)
        }
        _ => Err(malformed(path, "Option(Ratio)")),
    }
}

fn malformed(path: &str, expected: &'static str) -> InstrumentContractsError {
    InstrumentContractsError::Malformed {
        path: path.to_owned(),
        expected,
    }
}
