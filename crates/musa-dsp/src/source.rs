//! Read-only host projection of checked `std::sound::graph` data.
//!
//! This module knows the shape of the source package's interchange value, not
//! the meaning of a valid studio. Names, descriptors, ranges, ports, cycles,
//! defaults, and inference remain ordinary Musa definitions. The decoder only
//! refuses a mismatched or structurally incomplete checked artifact.

use musa_calculus::{CheckedSource, SourceDatum, SourceDatumKind, SourceSchema};
use num_rational::Ratio;
use std::sync::Arc;
use thiserror::Error;

const SCHEMA_NAME: &str = "std.sound.graph.StudioDescription";
const ROOT_TYPE: &str = "StudioDescription";
const SCHEMA_VERSION: u64 = 1;

/// The exact consumer contract for `std::sound::graph::StudioDescription`.
#[must_use]
pub fn studio_description_schema() -> SourceSchema {
    SourceSchema::new(SCHEMA_NAME, ROOT_TYPE, SCHEMA_VERSION)
}

/// A complete read-only projection of a checked source studio description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioDescription {
    schema_version: u64,
    declarations: Vec<StudioDeclaration>,
    exact_source: Arc<[u8]>,
}

impl StudioDescription {
    /// The version stated by the source value itself.
    #[must_use]
    pub const fn schema_version(&self) -> u64 {
        self.schema_version
    }

    /// Declarations in their written order.
    #[must_use]
    pub fn declarations(&self) -> &[StudioDeclaration] {
        &self.declarations
    }

    /// The complete exact checked-source identity this projection came from.
    #[must_use]
    pub fn exact_source_bytes(&self) -> &[u8] {
        &self.exact_source
    }
}

/// Which source constructor one projected declaration used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StudioDeclarationKind {
    /// An external graph input.
    Input,
    /// A processor or instrument node.
    Node,
    /// An external graph output.
    Output,
    /// A directed connection between named ports.
    Connect,
    /// A score-part binding to a named node.
    Bind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DeclarationFields {
    Port {
        anchor: u64,
        name: String,
        kind: PortKindProjection,
    },
    Node {
        anchor: u64,
        name: String,
        descriptor: String,
        parameters: Vec<ParameterProjection>,
    },
    Connect {
        anchor: u64,
        source: PortPathProjection,
        target: PortPathProjection,
    },
    Bind {
        anchor: u64,
        part_name: String,
        node: String,
    },
}

/// One declaration projected from checked source data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioDeclaration {
    kind: StudioDeclarationKind,
    fields: DeclarationFields,
}

impl StudioDeclaration {
    /// The source constructor used for this declaration.
    #[must_use]
    pub const fn kind(&self) -> StudioDeclarationKind {
        self.kind
    }

    /// The adapter anchor retained by every declaration.
    #[must_use]
    pub const fn anchor(&self) -> u64 {
        match &self.fields {
            DeclarationFields::Port { anchor, .. }
            | DeclarationFields::Node { anchor, .. }
            | DeclarationFields::Connect { anchor, .. }
            | DeclarationFields::Bind { anchor, .. } => *anchor,
        }
    }

    /// The declaration name, for input, node, and output declarations.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match &self.fields {
            DeclarationFields::Port { name, .. } | DeclarationFields::Node { name, .. } => Some(name),
            DeclarationFields::Connect { .. } | DeclarationFields::Bind { .. } => None,
        }
    }

    /// The exact projected port kind, for input and output declarations.
    #[must_use]
    pub const fn port_kind(&self) -> Option<&PortKindProjection> {
        match &self.fields {
            DeclarationFields::Port { kind, .. } => Some(kind),
            DeclarationFields::Node { .. } | DeclarationFields::Connect { .. } | DeclarationFields::Bind { .. } => None,
        }
    }

    /// The descriptor name, for node declarations.
    #[must_use]
    pub fn descriptor(&self) -> Option<&str> {
        match &self.fields {
            DeclarationFields::Node { descriptor, .. } => Some(descriptor),
            DeclarationFields::Port { .. } | DeclarationFields::Connect { .. } | DeclarationFields::Bind { .. } => None,
        }
    }

    /// Parameters in source order, for node declarations.
    #[must_use]
    pub fn parameters(&self) -> Option<&[ParameterProjection]> {
        match &self.fields {
            DeclarationFields::Node { parameters, .. } => Some(parameters),
            DeclarationFields::Port { .. } | DeclarationFields::Connect { .. } | DeclarationFields::Bind { .. } => None,
        }
    }

    /// The connection's source path.
    #[must_use]
    pub const fn source_path(&self) -> Option<&PortPathProjection> {
        match &self.fields {
            DeclarationFields::Connect { source, .. } => Some(source),
            DeclarationFields::Port { .. } | DeclarationFields::Node { .. } | DeclarationFields::Bind { .. } => None,
        }
    }

    /// The connection's target path.
    #[must_use]
    pub const fn target_path(&self) -> Option<&PortPathProjection> {
        match &self.fields {
            DeclarationFields::Connect { target, .. } => Some(target),
            DeclarationFields::Port { .. } | DeclarationFields::Node { .. } | DeclarationFields::Bind { .. } => None,
        }
    }

    /// The score part named by a binding.
    #[must_use]
    pub fn part_name(&self) -> Option<&str> {
        match &self.fields {
            DeclarationFields::Bind { part_name, .. } => Some(part_name),
            DeclarationFields::Port { .. } | DeclarationFields::Node { .. } | DeclarationFields::Connect { .. } => None,
        }
    }

    /// The node named by a binding.
    #[must_use]
    pub fn bound_node(&self) -> Option<&str> {
        match &self.fields {
            DeclarationFields::Bind { node, .. } => Some(node),
            DeclarationFields::Port { .. } | DeclarationFields::Node { .. } | DeclarationFields::Connect { .. } => None,
        }
    }
}

/// Which source `PortKind` constructor was projected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortKindTag {
    /// Audio with an exact channel count.
    Audio,
    /// A scalar control stream.
    Control,
    /// Performed note events.
    NoteEvents,
}

/// A source port kind with private fields and read-only access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortKindProjection {
    tag: PortKindTag,
    channels: Option<u64>,
}

impl PortKindProjection {
    /// The source constructor used by this kind.
    #[must_use]
    pub const fn tag(&self) -> PortKindTag {
        self.tag
    }

    /// The channel count of an audio port.
    #[must_use]
    pub const fn channels(&self) -> Option<u64> {
        self.channels
    }
}

/// A named node port projected without reparsing a dotted string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortPathProjection {
    node: String,
    port: String,
}

impl PortPathProjection {
    /// The node name.
    #[must_use]
    pub fn node(&self) -> &str {
        &self.node
    }

    /// The port name.
    #[must_use]
    pub fn port(&self) -> &str {
        &self.port
    }
}

/// Which exact source parameter-value constructor was projected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterValueKind {
    /// A natural-number count.
    Count,
    /// A dimensionless exact ratio.
    Plain,
    /// An exact ratio of seconds.
    Seconds,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParameterValue {
    Count(u64),
    Plain(Ratio<i64>),
    Seconds(Ratio<i64>),
}

/// One source parameter with its anchor, name, and exact value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterProjection {
    anchor: u64,
    name: String,
    value: ParameterValue,
}

impl ParameterProjection {
    /// The adapter anchor of the written value.
    #[must_use]
    pub const fn anchor(&self) -> u64 {
        self.anchor
    }

    /// The written parameter name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Which exact source value constructor was used.
    #[must_use]
    pub const fn value_kind(&self) -> ParameterValueKind {
        match self.value {
            ParameterValue::Count(_) => ParameterValueKind::Count,
            ParameterValue::Plain(_) => ParameterValueKind::Plain,
            ParameterValue::Seconds(_) => ParameterValueKind::Seconds,
        }
    }

    /// The count, when this is `Count`.
    #[must_use]
    pub const fn count(&self) -> Option<u64> {
        match self.value {
            ParameterValue::Count(value) => Some(value),
            ParameterValue::Plain(_) | ParameterValue::Seconds(_) => None,
        }
    }

    /// The exact ratio, when this is `Plain` or `Seconds`.
    #[must_use]
    pub const fn ratio(&self) -> Option<&Ratio<i64>> {
        match &self.value {
            ParameterValue::Plain(value) | ParameterValue::Seconds(value) => Some(value),
            ParameterValue::Count(_) => None,
        }
    }
}

/// Why a checked artifact is not this source package's studio description.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum StudioDescriptionError {
    /// The artifact names another consumer schema or source version.
    #[error("expected checked source schema `{SCHEMA_NAME}` version {SCHEMA_VERSION}")]
    WrongSchema,
    /// A canonical constructor, literal, or field count does not match the schema.
    #[error("malformed StudioDescription at {path}: {expected}")]
    Malformed {
        /// Structural position within the value.
        path: String,
        /// Required shape at that position.
        expected: &'static str,
    },
}

/// Decode the complete prompt-167 source value without performing graph semantics.
///
/// # Errors
///
/// Refuses an artifact from another root/schema version and any missing,
/// additional, or wrongly typed canonical field.
pub fn decode_studio_description(source: &CheckedSource) -> Result<StudioDescription, StudioDescriptionError> {
    if !source.has_valid_framing() {
        return Err(malformed("artifact", "complete exact framing"));
    }
    let expected = studio_description_schema();
    if source.schema() != &expected {
        return Err(StudioDescriptionError::WrongSchema);
    }
    let [version, declaration_list] = case_fields(source.root(), "StudioDescription.StudioDescription", "root")?;
    let schema_version = nat(version, "root.schema_version")?;
    if schema_version != SCHEMA_VERSION {
        return Err(malformed("root.schema_version", "the checked schema version"));
    }
    let declarations = list(declaration_list, "root.declarations", declaration)?;
    Ok(StudioDescription {
        schema_version,
        declarations,
        exact_source: source.exact_bytes().into(),
    })
}

fn declaration(datum: SourceDatum<'_>, path: &str) -> Result<StudioDeclaration, StudioDescriptionError> {
    match datum.kind() {
        Some(SourceDatumKind::Case {
            constructor: "StudioDecl.Input",
        }) => {
            let [anchor, name, kind] = fields(datum, path)?;
            Ok(StudioDeclaration {
                kind: StudioDeclarationKind::Input,
                fields: DeclarationFields::Port {
                    anchor: nat(anchor, &field(path, "anchor"))?,
                    name: text(name, &field(path, "name"))?,
                    kind: port_kind(kind, &field(path, "kind"))?,
                },
            })
        }
        Some(SourceDatumKind::Case {
            constructor: "StudioDecl.Output",
        }) => {
            let [anchor, name, kind] = fields(datum, path)?;
            Ok(StudioDeclaration {
                kind: StudioDeclarationKind::Output,
                fields: DeclarationFields::Port {
                    anchor: nat(anchor, &field(path, "anchor"))?,
                    name: text(name, &field(path, "name"))?,
                    kind: port_kind(kind, &field(path, "kind"))?,
                },
            })
        }
        Some(SourceDatumKind::Case {
            constructor: "StudioDecl.Node",
        }) => {
            let [anchor, name, descriptor, parameters] = fields(datum, path)?;
            Ok(StudioDeclaration {
                kind: StudioDeclarationKind::Node,
                fields: DeclarationFields::Node {
                    anchor: nat(anchor, &field(path, "anchor"))?,
                    name: text(name, &field(path, "name"))?,
                    descriptor: text(descriptor, &field(path, "descriptor"))?,
                    parameters: list(parameters, &field(path, "parameters"), parameter)?,
                },
            })
        }
        Some(SourceDatumKind::Case {
            constructor: "StudioDecl.Connect",
        }) => {
            let [anchor, source, target] = fields(datum, path)?;
            Ok(StudioDeclaration {
                kind: StudioDeclarationKind::Connect,
                fields: DeclarationFields::Connect {
                    anchor: nat(anchor, &field(path, "anchor"))?,
                    source: port_path(source, &field(path, "source_path"))?,
                    target: port_path(target, &field(path, "target_path"))?,
                },
            })
        }
        Some(SourceDatumKind::Case {
            constructor: "StudioDecl.Bind",
        }) => {
            let [anchor, part_name, node] = fields(datum, path)?;
            Ok(StudioDeclaration {
                kind: StudioDeclarationKind::Bind,
                fields: DeclarationFields::Bind {
                    anchor: nat(anchor, &field(path, "anchor"))?,
                    part_name: text(part_name, &field(path, "part_name"))?,
                    node: text(node, &field(path, "node"))?,
                },
            })
        }
        _ => Err(malformed(path, "a StudioDecl constructor")),
    }
}

fn parameter(datum: SourceDatum<'_>, path: &str) -> Result<ParameterProjection, StudioDescriptionError> {
    let [anchor, name, value] = case_fields(datum, "Parameter.Parameter", path)?;
    Ok(ParameterProjection {
        anchor: nat(anchor, &field(path, "anchor"))?,
        name: text(name, &field(path, "name"))?,
        value: parameter_value(value, &field(path, "value"))?,
    })
}

fn parameter_value(datum: SourceDatum<'_>, path: &str) -> Result<ParameterValue, StudioDescriptionError> {
    match datum.kind() {
        Some(SourceDatumKind::Case {
            constructor: "ParameterValue.Count",
        }) => {
            let [value] = fields(datum, path)?;
            Ok(ParameterValue::Count(nat(value, path)?))
        }
        Some(SourceDatumKind::Case {
            constructor: "ParameterValue.Plain",
        }) => {
            let [value] = fields(datum, path)?;
            Ok(ParameterValue::Plain(ratio(value, path)?))
        }
        Some(SourceDatumKind::Case {
            constructor: "ParameterValue.Seconds",
        }) => {
            let [value] = fields(datum, path)?;
            Ok(ParameterValue::Seconds(ratio(value, path)?))
        }
        _ => Err(malformed(path, "a ParameterValue constructor")),
    }
}

fn port_kind(datum: SourceDatum<'_>, path: &str) -> Result<PortKindProjection, StudioDescriptionError> {
    match datum.kind() {
        Some(SourceDatumKind::Case {
            constructor: "PortKind.Audio",
        }) => {
            let [channels] = fields(datum, path)?;
            Ok(PortKindProjection {
                tag: PortKindTag::Audio,
                channels: Some(nat(channels, path)?),
            })
        }
        Some(SourceDatumKind::Case {
            constructor: "PortKind.Control",
        }) => {
            let [] = fields(datum, path)?;
            Ok(PortKindProjection {
                tag: PortKindTag::Control,
                channels: None,
            })
        }
        Some(SourceDatumKind::Case {
            constructor: "PortKind.NoteEvents",
        }) => {
            let [] = fields(datum, path)?;
            Ok(PortKindProjection {
                tag: PortKindTag::NoteEvents,
                channels: None,
            })
        }
        _ => Err(malformed(path, "a PortKind constructor")),
    }
}

fn port_path(datum: SourceDatum<'_>, path: &str) -> Result<PortPathProjection, StudioDescriptionError> {
    let [node, port] = case_fields(datum, "PortPath.PortPath", path)?;
    Ok(PortPathProjection {
        node: text(node, &field(path, "node"))?,
        port: text(port, &field(path, "port"))?,
    })
}

fn list<T>(
    mut datum: SourceDatum<'_>,
    path: &str,
    decode: fn(SourceDatum<'_>, &str) -> Result<T, StudioDescriptionError>,
) -> Result<Vec<T>, StudioDescriptionError> {
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

fn nat(datum: SourceDatum<'_>, path: &str) -> Result<u64, StudioDescriptionError> {
    match datum.kind() {
        Some(SourceDatumKind::Count { family: "Nat", count }) => Ok(count),
        _ => Err(malformed(path, "a Nat")),
    }
}

fn text(datum: SourceDatum<'_>, path: &str) -> Result<String, StudioDescriptionError> {
    match datum.kind() {
        Some(SourceDatumKind::Literal {
            type_name: "Text",
            bytes,
        }) => String::from_utf8(bytes.to_vec()).map_err(|_| malformed(path, "UTF-8 Text")),
        _ => Err(malformed(path, "Text")),
    }
}

fn ratio(datum: SourceDatum<'_>, path: &str) -> Result<Ratio<i64>, StudioDescriptionError> {
    let Some(SourceDatumKind::Literal { type_name, bytes }) = datum.kind() else {
        return Err(malformed(path, "Ratio"));
    };
    if type_name != "Ratio" || bytes.len() != 16 {
        return Err(malformed(path, "an exact Ratio encoding"));
    }
    let (numerator, denominator) = bytes.split_at(8);
    let numerator = i64::from_be_bytes(numerator.try_into().map_err(|_| malformed(path, "Ratio numerator"))?);
    let denominator = i64::from_be_bytes(
        denominator
            .try_into()
            .map_err(|_| malformed(path, "Ratio denominator"))?,
    );
    if denominator <= 0 {
        return Err(malformed(path, "a normalized Ratio"));
    }
    let value = Ratio::new(numerator, denominator);
    if value.numer() != &numerator || value.denom() != &denominator {
        return Err(malformed(path, "a normalized Ratio"));
    }
    Ok(value)
}

fn case_fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    constructor: &str,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], StudioDescriptionError> {
    match datum.kind() {
        Some(SourceDatumKind::Case { constructor: found }) if found == constructor => fields(datum, path),
        _ => Err(malformed(path, "the required source constructor")),
    }
}

fn fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], StudioDescriptionError> {
    let Some(direct) = datum.fields() else {
        return Err(malformed(path, "valid canonical child framing"));
    };
    let Some(fields): Option<Vec<_>> = direct.collect() else {
        return Err(malformed(path, "valid canonical child indices"));
    };
    fields
        .try_into()
        .map_err(|_| malformed(path, "the exact source field count"))
}

fn field(path: &str, name: &str) -> String {
    format!("{path}.{name}")
}

fn malformed(path: &str, expected: &'static str) -> StudioDescriptionError {
    StudioDescriptionError::Malformed {
        path: path.to_owned(),
        expected,
    }
}
