//! Read-only preparation projection of checked source instrument contracts.
//!
//! The source declaration remains the schema and exact identity. This module
//! only extracts the fields preparation must query; none of these types has a
//! public constructor or supplies a default, coercion, or inference rule.

use std::sync::Arc;

use musa_calculus::{CheckedSource, SourceDatum, SourceDatumKind, SourceSchema};
use thiserror::Error;

const SCHEMA_NAME: &str = "std.sound.instrument.InstrumentExecutionArtifact";
const ROOT_TYPE: &str = "InstrumentExecutionArtifact";
const VERSION: u64 = 1;

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

    pub fn default_exact_bytes(&self) -> &[u8] {
        &self.default_exact
    }
}

/// One source-declared technique and its explicit fallback policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentTechniqueContract {
    namespace: String,
    name: String,
    notation_only_warning: bool,
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
        return Err(malformed("root.schema_version", "schema version 1"));
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
    for (mut declaration, (implementation_id, implementation_exact)) in instruments.into_iter().zip(implementations) {
        if declaration.declaration_id != implementation_id {
            return Err(malformed(
                "root.implementation_contracts",
                "the private body identity matching its public declaration",
            ));
        }
        declaration.implementation_id = implementation_id;
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

fn implementation(datum: SourceDatum<'_>, path: &str) -> Result<(String, Arc<[u8]>), InstrumentContractsError> {
    let [declaration_id, _mappings] = fields::<2>(datum, "InstrumentImplementationContract", path)?;
    Ok((
        text(declaration_id, &format!("{path}.declaration_id"))?,
        datum.exact_bytes().into(),
    ))
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
    let [key_kind, namespace, name, _, _] = fields::<5>(key, "Key", path)?;
    if constructor(key_kind) != Some(kind.as_str()) {
        return Err(malformed(path, "a key at the requirement's control kind"));
    }
    Ok(InstrumentControlContract {
        kind,
        namespace: text(namespace, &format!("{path}.namespace"))?,
        name: text(name, &format!("{path}.name"))?,
        default_exact: default.exact_bytes().into(),
    })
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

fn malformed(path: &str, expected: &'static str) -> InstrumentContractsError {
    InstrumentContractsError::Malformed {
        path: path.to_owned(),
        expected,
    }
}
