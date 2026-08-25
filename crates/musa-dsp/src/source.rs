//! Read-only host projections of checked `std::sound` data.
//!
//! This module knows the shape of source package interchange values, not their
//! validation policy. Names, descriptors, ranges, units, defaults, and
//! inference remain ordinary Musa definitions. Decoders only refuse a
//! mismatched or structurally incomplete checked artifact.

use musa_calculus::{CheckedSource, SourceDatum, SourceDatumKind, SourceSchema};
use num_rational::Ratio;
use std::sync::Arc;
use thiserror::Error;

const SCHEMA_NAME: &str = "std.sound.graph.StudioDescription";
const ROOT_TYPE: &str = "StudioDescription";
const SCHEMA_VERSION: u64 = 1;
const QUANTITY_SCHEMA_NAME: &str = "std.sound.quantity.ExactQuantityArtifact";
const QUANTITY_ROOT_TYPE: &str = "ExactQuantityArtifact";
const QUANTITY_SCHEMA_VERSION: u64 = 1;
const VOCABULARY_SCHEMA_NAME: &str = "std.sound.catalogue.StudioVocabularyArtifact";
const VOCABULARY_ROOT_TYPE: &str = "StudioVocabularyArtifact";
const VOCABULARY_SCHEMA_VERSION: u64 = 1;

/// The exact consumer contract for `std::sound::graph::StudioDescription`.
#[must_use]
pub fn studio_description_schema() -> SourceSchema {
    SourceSchema::new(SCHEMA_NAME, ROOT_TYPE, SCHEMA_VERSION)
}

/// The exact consumer contract for `std::sound::quantity::ExactQuantityArtifact`.
#[must_use]
pub fn exact_quantity_schema() -> SourceSchema {
    SourceSchema::new(QUANTITY_SCHEMA_NAME, QUANTITY_ROOT_TYPE, QUANTITY_SCHEMA_VERSION)
}

/// The exact consumer contract for `std::sound::catalogue::StudioVocabularyArtifact`.
#[must_use]
pub fn studio_vocabulary_schema() -> SourceSchema {
    SourceSchema::new(VOCABULARY_SCHEMA_NAME, VOCABULARY_ROOT_TYPE, VOCABULARY_SCHEMA_VERSION)
}

/// A source-owned sound dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundDimension {
    /// Cycles per second.
    Frequency,
    /// A dimensionless linear amplitude or factor.
    LinearAmplitude,
    /// A logarithmic amplitude level.
    Level,
    /// Elapsed physical time.
    Time,
}

/// A normalized unit declared by `std::sound::quantity`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundUnit {
    /// Hertz.
    Hertz,
    /// Dimensionless linear amplitude.
    Linear,
    /// Decibels.
    Decibels,
    /// Seconds.
    Seconds,
}

impl SoundUnit {
    /// Stable source suffix, or no suffix for a linear ratio.
    #[must_use]
    pub const fn spelling(self) -> Option<&'static str> {
        match self {
            Self::Hertz => Some("Hz"),
            Self::Linear => None,
            Self::Decibels => Some("dB"),
            Self::Seconds => Some("s"),
        }
    }
}

/// An opaque exact projection of one checked source quantity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactQuantityProjection {
    dimension: SoundDimension,
    magnitude: Ratio<i64>,
    unit: SoundUnit,
}

impl ExactQuantityProjection {
    pub(crate) const fn from_checked_parts(dimension: SoundDimension, magnitude: Ratio<i64>, unit: SoundUnit) -> Self {
        Self {
            dimension,
            magnitude,
            unit,
        }
    }

    /// The source index shared by the quantity and its unit.
    #[must_use]
    pub const fn dimension(&self) -> SoundDimension {
        self.dimension
    }

    /// The exact reduced magnitude in the source base unit.
    #[must_use]
    pub const fn magnitude(&self) -> &Ratio<i64> {
        &self.magnitude
    }

    /// The normalized source unit.
    #[must_use]
    pub const fn unit(&self) -> SoundUnit {
        self.unit
    }
}

/// One standalone exact quantity together with its complete checked identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedExactQuantity {
    quantity: ExactQuantityProjection,
    exact_source: Arc<[u8]>,
}

impl CheckedExactQuantity {
    /// The exact dimensioned source quantity.
    #[must_use]
    pub const fn quantity(&self) -> &ExactQuantityProjection {
        &self.quantity
    }

    /// The complete checked source bytes this projection came from.
    #[must_use]
    pub fn exact_source_bytes(&self) -> &[u8] {
        &self.exact_source
    }
}

/// Why a checked artifact is not an exact source quantity.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ExactQuantityError {
    /// The artifact names another consumer schema or source version.
    #[error("expected checked source schema `{QUANTITY_SCHEMA_NAME}` version {QUANTITY_SCHEMA_VERSION}")]
    WrongSchema,
    /// A canonical constructor, index witness, literal, or field count does not match the source declaration.
    #[error("malformed ExactQuantityArtifact at {path}: {expected}")]
    Malformed {
        /// Structural position within the value.
        path: String,
        /// Required shape at that position.
        expected: &'static str,
    },
}

/// Decode one complete checked quantity without performing unit conversion.
///
/// # Errors
///
/// Refuses the wrong schema, malformed exact framing, and any disagreement
/// among the source dimension index, quantity witness, and unit witness.
pub fn decode_exact_quantity(source: &CheckedSource) -> Result<CheckedExactQuantity, ExactQuantityError> {
    if !source.has_valid_framing() {
        return Err(quantity_malformed("artifact", "complete exact framing"));
    }
    if source.schema() != &exact_quantity_schema() {
        return Err(ExactQuantityError::WrongSchema);
    }
    let [version, quantity] =
        quantity_case_fields(source.root(), "ExactQuantityArtifact.ExactQuantityArtifact", "root")?;
    if quantity_nat(version, "root.schema_version")? != QUANTITY_SCHEMA_VERSION {
        return Err(quantity_malformed("root.schema_version", "the checked schema version"));
    }
    Ok(CheckedExactQuantity {
        quantity: quantity_value(quantity, "root.quantity")?,
        exact_source: source.exact_bytes().into(),
    })
}

/// A source-declared processor role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalRole {
    /// Transforms or combines audio.
    AudioProcessor,
    /// Transforms a control-rate signal.
    ControlProcessor,
    /// Produces audio from gestures or control without an input.
    AudioOrControlSource,
}

impl SignalRole {
    /// Stable musician-facing label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AudioProcessor => "audio processor",
            Self::ControlProcessor => "control processor",
            Self::AudioOrControlSource => "audio or control source",
        }
    }
}

/// One source-declared public port kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfacePort {
    /// Sounding samples.
    Audio,
    /// A frame-rate control value.
    Control,
    /// Scheduled performed gestures.
    NoteEvents,
}

impl SurfacePort {
    /// Stable source spelling.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Audio => "Audio",
            Self::Control => "Control",
            Self::NoteEvents => "NoteEvents",
        }
    }
}

/// One valid first-order public port contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortContract {
    inputs: Vec<SurfacePort>,
    output: SurfacePort,
}

impl PortContract {
    /// Input ports in source order.
    #[must_use]
    pub fn inputs(&self) -> &[SurfacePort] {
        &self.inputs
    }

    /// The output port.
    #[must_use]
    pub const fn output(&self) -> SurfacePort {
        self.output
    }
}

/// One exact source-owned parameter contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioParameterContract {
    name: String,
    summary: String,
    default: ExactQuantityProjection,
    minimum: ExactQuantityProjection,
    maximum: ExactQuantityProjection,
}

impl StudioParameterContract {
    /// Written parameter name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Plain musician-facing first sentence.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Exact written default.
    #[must_use]
    pub const fn default(&self) -> &ExactQuantityProjection {
        &self.default
    }

    /// Inclusive exact written lower bound.
    #[must_use]
    pub const fn minimum(&self) -> &ExactQuantityProjection {
        &self.minimum
    }

    /// Inclusive exact written upper bound.
    #[must_use]
    pub const fn maximum(&self) -> &ExactQuantityProjection {
        &self.maximum
    }
}

/// One stable host capability required by a source wrapper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitiveRequirement {
    id: String,
    version: u64,
}

impl PrimitiveRequirement {
    /// Registered primitive identity.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Registered primitive version.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }
}

/// One source-declared standard processor contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessorContract {
    name: String,
    summary: String,
    note: String,
    signature: String,
    role: SignalRole,
    ports: Vec<PortContract>,
    example: String,
    parameters: Vec<StudioParameterContract>,
    primitives: Vec<PrimitiveRequirement>,
}

impl ProcessorContract {
    /// One accepted source spelling.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Plain first sentence.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Longer source-owned explanation.
    #[must_use]
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Typed source call shape.
    #[must_use]
    pub fn signature(&self) -> &str {
        &self.signature
    }

    /// Source-declared signal role.
    #[must_use]
    pub const fn role(&self) -> SignalRole {
        self.role
    }

    /// Valid public port contracts.
    #[must_use]
    pub fn ports(&self) -> &[PortContract] {
        &self.ports
    }

    /// Short valid source example.
    #[must_use]
    pub fn example(&self) -> &str {
        &self.example
    }

    /// Parameters in positional order.
    #[must_use]
    pub fn parameters(&self) -> &[StudioParameterContract] {
        &self.parameters
    }

    /// Registered host capabilities required by the future source wrapper.
    #[must_use]
    pub fn primitives(&self) -> &[PrimitiveRequirement] {
        &self.primitives
    }
}

/// Documentation for one source studio term that is not a processor call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioTermContract {
    spelling: String,
    summary: String,
    signature: String,
    note: String,
    example: String,
}

impl StudioTermContract {
    /// Written spelling.
    #[must_use]
    pub fn spelling(&self) -> &str {
        &self.spelling
    }

    /// Plain first sentence.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Typed or grammatical shape.
    #[must_use]
    pub fn signature(&self) -> &str {
        &self.signature
    }

    /// Longer explanation.
    #[must_use]
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Short source example.
    #[must_use]
    pub fn example(&self) -> &str {
        &self.example
    }
}

/// Complete edition-pinned standard studio vocabulary projected from source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioVocabulary {
    schema_version: u64,
    processors: Vec<ProcessorContract>,
    terms: Vec<StudioTermContract>,
    exact_source: Arc<[u8]>,
}

impl StudioVocabulary {
    /// Source schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u64 {
        self.schema_version
    }

    /// Standard processors in declaration order.
    #[must_use]
    pub fn processors(&self) -> &[ProcessorContract] {
        &self.processors
    }

    /// Studio terms in declaration order.
    #[must_use]
    pub fn terms(&self) -> &[StudioTermContract] {
        &self.terms
    }

    /// Find one accepted processor spelling.
    #[must_use]
    pub fn processor(&self, name: &str) -> Option<&ProcessorContract> {
        self.processors.iter().find(|processor| processor.name == name)
    }

    /// Find one studio term spelling.
    #[must_use]
    pub fn term(&self, spelling: &str) -> Option<&StudioTermContract> {
        self.terms.iter().find(|term| term.spelling == spelling)
    }

    /// Complete exact checked-source identity.
    #[must_use]
    pub fn exact_source_bytes(&self) -> &[u8] {
        &self.exact_source
    }
}

/// Why a checked artifact is not the source studio vocabulary.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum StudioVocabularyError {
    /// The artifact names another consumer schema or source version.
    #[error("expected checked source schema `{VOCABULARY_SCHEMA_NAME}` version {VOCABULARY_SCHEMA_VERSION}")]
    WrongSchema,
    /// A canonical constructor, index witness, literal, or field count is wrong.
    #[error("malformed StudioVocabularyArtifact at {path}: {expected}")]
    Malformed {
        /// Structural position within the value.
        path: String,
        /// Required source shape.
        expected: &'static str,
    },
}

/// Decode one complete, checked standard-library studio vocabulary.
///
/// # Errors
///
/// Refuses the wrong source schema, malformed exact framing, duplicate public
/// spellings, and any parameter whose three source indices disagree.
pub fn decode_studio_vocabulary(source: &CheckedSource) -> Result<StudioVocabulary, StudioVocabularyError> {
    if !source.has_valid_framing() {
        return Err(vocabulary_malformed("artifact", "complete exact framing"));
    }
    if source.schema() != &studio_vocabulary_schema() {
        return Err(StudioVocabularyError::WrongSchema);
    }
    let [version, processors, terms] = vocabulary_case_fields(
        source.root(),
        "StudioVocabularyArtifact.StudioVocabularyArtifact",
        "root",
    )?;
    let schema_version = vocabulary_nat(version, "root.schema_version")?;
    if schema_version != VOCABULARY_SCHEMA_VERSION {
        return Err(vocabulary_malformed(
            "root.schema_version",
            "the checked schema version",
        ));
    }
    let processors = vocabulary_list(processors, "root.processors", vocabulary_processor)?;
    let terms = vocabulary_list(terms, "root.terms", vocabulary_term)?;
    let mut processor_names = std::collections::HashSet::new();
    if let Some(duplicate) = processors
        .iter()
        .find(|processor| !processor_names.insert(processor.name.as_str()))
    {
        return Err(vocabulary_malformed(
            &format!("root.processors.{}", duplicate.name),
            "one declaration per processor spelling",
        ));
    }
    let mut term_names = std::collections::HashSet::new();
    if let Some(duplicate) = terms.iter().find(|term| !term_names.insert(term.spelling.as_str())) {
        return Err(vocabulary_malformed(
            &format!("root.terms.{}", duplicate.spelling),
            "one declaration per studio-term spelling",
        ));
    }
    Ok(StudioVocabulary {
        schema_version,
        processors,
        terms,
        exact_source: source.exact_bytes().into(),
    })
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

/// Which exact source parameter-value kind was projected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterValueKind {
    /// A natural-number count.
    Count,
    /// A dimensionless exact ratio.
    Plain,
    /// An exact ratio of hertz.
    Hertz,
    /// An exact ratio of decibels.
    Decibels,
    /// An exact ratio of seconds.
    Seconds,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParameterValue {
    Count(u64),
    Exact(ExactQuantityProjection),
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

    /// Which exact source value kind was used.
    #[must_use]
    pub const fn value_kind(&self) -> ParameterValueKind {
        match self.value {
            ParameterValue::Count(_) => ParameterValueKind::Count,
            ParameterValue::Exact(ref value) => match value.dimension {
                SoundDimension::Frequency => ParameterValueKind::Hertz,
                SoundDimension::LinearAmplitude => ParameterValueKind::Plain,
                SoundDimension::Level => ParameterValueKind::Decibels,
                SoundDimension::Time => ParameterValueKind::Seconds,
            },
        }
    }

    /// The count, when this is `Count`.
    #[must_use]
    pub const fn count(&self) -> Option<u64> {
        match self.value {
            ParameterValue::Count(value) => Some(value),
            ParameterValue::Exact(_) => None,
        }
    }

    /// The exact ratio, when this is `Plain` or `Seconds`.
    #[must_use]
    pub const fn ratio(&self) -> Option<&Ratio<i64>> {
        match &self.value {
            ParameterValue::Exact(value) => Some(&value.magnitude),
            ParameterValue::Count(_) => None,
        }
    }

    /// The complete exact source quantity, when this is not a count.
    #[must_use]
    pub const fn exact_quantity(&self) -> Option<&ExactQuantityProjection> {
        match &self.value {
            ParameterValue::Exact(value) => Some(value),
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
            constructor: "ParameterValue.ExactValue",
        }) => {
            let [value] = fields(datum, path)?;
            quantity_value(value, path)
                .map(ParameterValue::Exact)
                .map_err(|error| match error {
                    ExactQuantityError::WrongSchema => malformed(path, "an exact source quantity"),
                    ExactQuantityError::Malformed { path, expected } => {
                        StudioDescriptionError::Malformed { path, expected }
                    }
                })
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

fn quantity_value(datum: SourceDatum<'_>, path: &str) -> Result<ExactQuantityProjection, ExactQuantityError> {
    let [dimension, value] = quantity_case_fields(datum, "ExactQuantity.Exact", path)?;
    let dimension = quantity_dimension(dimension, &field(path, "dimension"))?;
    let [inner_dimension, magnitude, unit] =
        quantity_case_fields(value, "SoundQuantity.Written", &field(path, "value"))?;
    let inner_dimension = quantity_dimension(inner_dimension, &field(path, "value.dimension"))?;
    let unit = quantity_unit(unit, &field(path, "value.unit"))?;
    if dimension != inner_dimension || dimension != unit_dimension(unit) {
        return Err(quantity_malformed(path, "matching source dimension and unit indices"));
    }
    Ok(ExactQuantityProjection {
        dimension,
        magnitude: quantity_ratio(magnitude, &field(path, "value.magnitude"))?,
        unit,
    })
}

fn quantity_dimension(datum: SourceDatum<'_>, path: &str) -> Result<SoundDimension, ExactQuantityError> {
    let Some(SourceDatumKind::Case { constructor }) = datum.kind() else {
        return Err(quantity_malformed(path, "a SoundDimension constructor"));
    };
    let [] = quantity_fields(datum, path)?;
    match constructor {
        "SoundDimension.Frequency" => Ok(SoundDimension::Frequency),
        "SoundDimension.LinearAmplitude" => Ok(SoundDimension::LinearAmplitude),
        "SoundDimension.Level" => Ok(SoundDimension::Level),
        "SoundDimension.Time" => Ok(SoundDimension::Time),
        _ => Err(quantity_malformed(path, "a SoundDimension constructor")),
    }
}

fn quantity_unit(datum: SourceDatum<'_>, path: &str) -> Result<SoundUnit, ExactQuantityError> {
    let Some(SourceDatumKind::Case { constructor }) = datum.kind() else {
        return Err(quantity_malformed(path, "a SoundUnit constructor"));
    };
    let [] = quantity_fields(datum, path)?;
    match constructor {
        "SoundUnit.Hertz" => Ok(SoundUnit::Hertz),
        "SoundUnit.Linear" => Ok(SoundUnit::Linear),
        "SoundUnit.Decibels" => Ok(SoundUnit::Decibels),
        "SoundUnit.Seconds" => Ok(SoundUnit::Seconds),
        _ => Err(quantity_malformed(path, "a SoundUnit constructor")),
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

fn quantity_nat(datum: SourceDatum<'_>, path: &str) -> Result<u64, ExactQuantityError> {
    match datum.kind() {
        Some(SourceDatumKind::Count { family: "Nat", count }) => Ok(count),
        _ => Err(quantity_malformed(path, "a Nat")),
    }
}

fn quantity_ratio(datum: SourceDatum<'_>, path: &str) -> Result<Ratio<i64>, ExactQuantityError> {
    let Some(SourceDatumKind::Literal { type_name, bytes }) = datum.kind() else {
        return Err(quantity_malformed(path, "Ratio"));
    };
    if type_name != "Ratio" || bytes.len() != 16 {
        return Err(quantity_malformed(path, "an exact Ratio encoding"));
    }
    let (numerator, denominator) = bytes.split_at(8);
    let numerator = i64::from_be_bytes(
        numerator
            .try_into()
            .map_err(|_| quantity_malformed(path, "Ratio numerator"))?,
    );
    let denominator = i64::from_be_bytes(
        denominator
            .try_into()
            .map_err(|_| quantity_malformed(path, "Ratio denominator"))?,
    );
    if denominator <= 0 {
        return Err(quantity_malformed(path, "a normalized Ratio"));
    }
    let value = Ratio::new(numerator, denominator);
    if value.numer() != &numerator || value.denom() != &denominator {
        return Err(quantity_malformed(path, "a normalized Ratio"));
    }
    Ok(value)
}

fn quantity_case_fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    constructor: &str,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], ExactQuantityError> {
    match datum.kind() {
        Some(SourceDatumKind::Case { constructor: found }) if found == constructor => quantity_fields(datum, path),
        _ => Err(quantity_malformed(path, "the required source constructor")),
    }
}

fn quantity_fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], ExactQuantityError> {
    let Some(direct) = datum.fields() else {
        return Err(quantity_malformed(path, "valid canonical child framing"));
    };
    let Some(fields): Option<Vec<_>> = direct.collect() else {
        return Err(quantity_malformed(path, "valid canonical child indices"));
    };
    fields
        .try_into()
        .map_err(|_| quantity_malformed(path, "the exact source field count"))
}

fn quantity_malformed(path: &str, expected: &'static str) -> ExactQuantityError {
    ExactQuantityError::Malformed {
        path: path.to_owned(),
        expected,
    }
}

fn vocabulary_processor(datum: SourceDatum<'_>, path: &str) -> Result<ProcessorContract, StudioVocabularyError> {
    let [
        name,
        summary,
        note,
        signature,
        role,
        ports,
        example,
        parameters,
        primitives,
    ] = vocabulary_case_fields(datum, "ProcessorContract.ProcessorContract", path)?;
    Ok(ProcessorContract {
        name: vocabulary_text(name, &field(path, "name"))?,
        summary: vocabulary_text(summary, &field(path, "summary"))?,
        note: vocabulary_text(note, &field(path, "note"))?,
        signature: vocabulary_text(signature, &field(path, "signature"))?,
        role: vocabulary_role(role, &field(path, "role"))?,
        ports: vocabulary_list(ports, &field(path, "ports"), vocabulary_port_contract)?,
        example: vocabulary_text(example, &field(path, "example"))?,
        parameters: vocabulary_list(parameters, &field(path, "parameters"), vocabulary_parameter)?,
        primitives: vocabulary_list(primitives, &field(path, "primitives"), vocabulary_primitive)?,
    })
}

fn vocabulary_role(datum: SourceDatum<'_>, path: &str) -> Result<SignalRole, StudioVocabularyError> {
    let Some(SourceDatumKind::Case { constructor }) = datum.kind() else {
        return Err(vocabulary_malformed(path, "a SignalRole constructor"));
    };
    let [] = vocabulary_fields(datum, path)?;
    match constructor {
        "SignalRole.AudioProcessor" => Ok(SignalRole::AudioProcessor),
        "SignalRole.ControlProcessor" => Ok(SignalRole::ControlProcessor),
        "SignalRole.AudioOrControlSource" => Ok(SignalRole::AudioOrControlSource),
        _ => Err(vocabulary_malformed(path, "a SignalRole constructor")),
    }
}

fn vocabulary_port_contract(datum: SourceDatum<'_>, path: &str) -> Result<PortContract, StudioVocabularyError> {
    let [inputs, output] = vocabulary_case_fields(datum, "PortContract.PortContract", path)?;
    Ok(PortContract {
        inputs: vocabulary_list(inputs, &field(path, "inputs"), vocabulary_surface_port)?,
        output: vocabulary_surface_port(output, &field(path, "output"))?,
    })
}

fn vocabulary_surface_port(datum: SourceDatum<'_>, path: &str) -> Result<SurfacePort, StudioVocabularyError> {
    let Some(SourceDatumKind::Case { constructor }) = datum.kind() else {
        return Err(vocabulary_malformed(path, "a SurfacePort constructor"));
    };
    let [] = vocabulary_fields(datum, path)?;
    match constructor {
        "SurfacePort.Audio" => Ok(SurfacePort::Audio),
        "SurfacePort.Control" => Ok(SurfacePort::Control),
        "SurfacePort.NoteEvents" => Ok(SurfacePort::NoteEvents),
        _ => Err(vocabulary_malformed(path, "a SurfacePort constructor")),
    }
}

fn vocabulary_parameter(datum: SourceDatum<'_>, path: &str) -> Result<StudioParameterContract, StudioVocabularyError> {
    let [dimension, contract] = vocabulary_case_fields(datum, "SomeParameterContract.SomeParameter", path)?;
    let dimension = vocabulary_dimension(dimension, &field(path, "dimension"))?;
    let [inner_dimension, name, summary, default, minimum, maximum] =
        vocabulary_case_fields(contract, "ParameterContract.ParameterFields", &field(path, "contract"))?;
    let inner_dimension = vocabulary_dimension(inner_dimension, &field(path, "contract.dimension"))?;
    let default = vocabulary_quantity(default, &field(path, "default"))?;
    let minimum = vocabulary_quantity(minimum, &field(path, "minimum"))?;
    let maximum = vocabulary_quantity(maximum, &field(path, "maximum"))?;
    if dimension != inner_dimension
        || [default.dimension, minimum.dimension, maximum.dimension]
            .iter()
            .any(|found| *found != dimension)
    {
        return Err(vocabulary_malformed(
            path,
            "matching parameter and quantity dimension indices",
        ));
    }
    if minimum.magnitude > default.magnitude || default.magnitude > maximum.magnitude {
        return Err(vocabulary_malformed(
            path,
            "an exact default inside its inclusive written range",
        ));
    }
    Ok(StudioParameterContract {
        name: vocabulary_text(name, &field(path, "name"))?,
        summary: vocabulary_text(summary, &field(path, "summary"))?,
        default,
        minimum,
        maximum,
    })
}

fn vocabulary_primitive(datum: SourceDatum<'_>, path: &str) -> Result<PrimitiveRequirement, StudioVocabularyError> {
    let [id, version] = vocabulary_case_fields(datum, "PrimitiveRequirement.PrimitiveRequirement", path)?;
    Ok(PrimitiveRequirement {
        id: vocabulary_text(id, &field(path, "id"))?,
        version: vocabulary_nat(version, &field(path, "version"))?,
    })
}

fn vocabulary_term(datum: SourceDatum<'_>, path: &str) -> Result<StudioTermContract, StudioVocabularyError> {
    let [spelling, summary, signature, note, example] =
        vocabulary_case_fields(datum, "StudioTermContract.StudioTermContract", path)?;
    Ok(StudioTermContract {
        spelling: vocabulary_text(spelling, &field(path, "spelling"))?,
        summary: vocabulary_text(summary, &field(path, "summary"))?,
        signature: vocabulary_text(signature, &field(path, "signature"))?,
        note: vocabulary_text(note, &field(path, "note"))?,
        example: vocabulary_text(example, &field(path, "example"))?,
    })
}

fn vocabulary_quantity(datum: SourceDatum<'_>, path: &str) -> Result<ExactQuantityProjection, StudioVocabularyError> {
    let [inner_dimension, magnitude, unit] = vocabulary_case_fields(datum, "SoundQuantity.Written", path)?;
    let dimension = vocabulary_dimension(inner_dimension, &field(path, "dimension"))?;
    let unit = vocabulary_unit(unit, &field(path, "unit"))?;
    if dimension != unit_dimension(unit) {
        return Err(vocabulary_malformed(path, "matching source dimension and unit indices"));
    }
    Ok(ExactQuantityProjection {
        dimension,
        magnitude: vocabulary_ratio(magnitude, &field(path, "magnitude"))?,
        unit,
    })
}

fn vocabulary_dimension(datum: SourceDatum<'_>, path: &str) -> Result<SoundDimension, StudioVocabularyError> {
    quantity_dimension(datum, path).map_err(vocabulary_quantity_error)
}

fn vocabulary_unit(datum: SourceDatum<'_>, path: &str) -> Result<SoundUnit, StudioVocabularyError> {
    quantity_unit(datum, path).map_err(vocabulary_quantity_error)
}

fn vocabulary_ratio(datum: SourceDatum<'_>, path: &str) -> Result<Ratio<i64>, StudioVocabularyError> {
    quantity_ratio(datum, path).map_err(vocabulary_quantity_error)
}

fn vocabulary_quantity_error(error: ExactQuantityError) -> StudioVocabularyError {
    match error {
        ExactQuantityError::WrongSchema => vocabulary_malformed("quantity", "an exact source quantity"),
        ExactQuantityError::Malformed { path, expected } => StudioVocabularyError::Malformed { path, expected },
    }
}

fn vocabulary_list<T>(
    mut datum: SourceDatum<'_>,
    path: &str,
    decode: fn(SourceDatum<'_>, &str) -> Result<T, StudioVocabularyError>,
) -> Result<Vec<T>, StudioVocabularyError> {
    let mut values = Vec::new();
    loop {
        match datum.kind() {
            Some(SourceDatumKind::Case {
                constructor: "List.Empty",
            }) => {
                let [] = vocabulary_fields(datum, path)?;
                return Ok(values);
            }
            Some(SourceDatumKind::Case {
                constructor: "List.Cons",
            }) => {
                let [head, tail] = vocabulary_fields(datum, path)?;
                values.push(decode(head, &format!("{path}[{}]", values.len()))?);
                datum = tail;
            }
            _ => return Err(vocabulary_malformed(path, "a complete List")),
        }
    }
}

fn vocabulary_nat(datum: SourceDatum<'_>, path: &str) -> Result<u64, StudioVocabularyError> {
    match datum.kind() {
        Some(SourceDatumKind::Count { family: "Nat", count }) => Ok(count),
        _ => Err(vocabulary_malformed(path, "a Nat")),
    }
}

fn vocabulary_text(datum: SourceDatum<'_>, path: &str) -> Result<String, StudioVocabularyError> {
    match datum.kind() {
        Some(SourceDatumKind::Literal {
            type_name: "Text",
            bytes,
        }) => String::from_utf8(bytes.to_vec()).map_err(|_| vocabulary_malformed(path, "UTF-8 Text")),
        _ => Err(vocabulary_malformed(path, "Text")),
    }
}

fn vocabulary_case_fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    constructor: &str,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], StudioVocabularyError> {
    match datum.kind() {
        Some(SourceDatumKind::Case { constructor: found }) if found == constructor => vocabulary_fields(datum, path),
        _ => Err(vocabulary_malformed(path, "the required source constructor")),
    }
}

fn vocabulary_fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], StudioVocabularyError> {
    let Some(direct) = datum.fields() else {
        return Err(vocabulary_malformed(path, "valid canonical child framing"));
    };
    let Some(fields): Option<Vec<_>> = direct.collect() else {
        return Err(vocabulary_malformed(path, "valid canonical child indices"));
    };
    fields
        .try_into()
        .map_err(|_| vocabulary_malformed(path, "the exact source field count"))
}

fn vocabulary_malformed(path: &str, expected: &'static str) -> StudioVocabularyError {
    StudioVocabularyError::Malformed {
        path: path.to_owned(),
        expected,
    }
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
