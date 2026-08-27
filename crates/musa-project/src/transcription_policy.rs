//! The exact offline projection of the checked `std::transcription` policy.
//!
//! This module owns the checked-source boundary between the ordinary Musa
//! declarations in `stdlib/src/transcription/mod.musa` and the bounded rhythm
//! search of prompt 204a. The projection is deliberately not independently
//! constructible: every field derives from one `CheckedSource`, the complete
//! `exact_bytes` are retained, and each policy is reached by its source name.
//! There is no hidden host default.

use std::sync::{Arc, OnceLock};

use musa_calculus::{CheckedSource, SourceDatum, SourceDatumKind, SourceSchema};
use num_rational::Ratio;
use thiserror::Error;

const SCHEMA_NAME: &str = "std.transcription.TranscriptionPolicyArtifact";
const ROOT_TYPE: &str = "TranscriptionPolicyArtifact";
const SCHEMA_VERSION: u64 = 1;

static POLICIES: OnceLock<Result<TranscriptionPolicies, String>> = OnceLock::new();

/// The consumer contract every decoded artifact must inhabit.
fn schema() -> SourceSchema {
    SourceSchema::new(SCHEMA_NAME, ROOT_TYPE, SCHEMA_VERSION)
}

/// One admitted division of a beat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Subdivision {
    beat_fraction: Ratio<i64>,
    complexity: u64,
}

impl Subdivision {
    /// The exact fraction of one beat this division occupies.
    pub(crate) const fn beat_fraction(&self) -> &Ratio<i64> {
        &self.beat_fraction
    }

    /// The declared notation-complexity rank of an onset on this division.
    pub(crate) const fn complexity(&self) -> u64 {
        self.complexity
    }
}

/// One admitted irregular division, as the fraction of a beat one member of it
/// occupies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TupletAllowance {
    member_fraction: Ratio<i64>,
    complexity: u64,
}

impl TupletAllowance {
    /// The exact member fraction; a triplet eighth is `1/3` of a beat.
    pub(crate) const fn member_fraction(&self) -> &Ratio<i64> {
        &self.member_fraction
    }

    /// The declared complexity rank of a member onset.
    pub(crate) const fn complexity(&self) -> u64 {
        self.complexity
    }
}

/// The tunable exact weights of the complete cost record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CostWeights {
    onset_residual: Ratio<i64>,
    duration_residual: Ratio<i64>,
    tempo_smoothness: Ratio<i64>,
    notation_complexity: Ratio<i64>,
    transition: Ratio<i64>,
    group_split: Ratio<i64>,
}

impl CostWeights {
    pub(crate) const fn onset_residual(&self) -> &Ratio<i64> {
        &self.onset_residual
    }

    pub(crate) const fn duration_residual(&self) -> &Ratio<i64> {
        &self.duration_residual
    }

    pub(crate) const fn tempo_smoothness(&self) -> &Ratio<i64> {
        &self.tempo_smoothness
    }

    pub(crate) const fn notation_complexity(&self) -> &Ratio<i64> {
        &self.notation_complexity
    }

    pub(crate) const fn transition(&self) -> &Ratio<i64> {
        &self.transition
    }

    pub(crate) const fn group_split(&self) -> &Ratio<i64> {
        &self.group_split
    }
}

/// The adaptive onset-group proposal window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GroupWindow {
    beat_fraction: Ratio<i64>,
    min_seconds: Ratio<i64>,
    max_seconds: Ratio<i64>,
}

impl GroupWindow {
    pub(crate) const fn beat_fraction(&self) -> &Ratio<i64> {
        &self.beat_fraction
    }

    pub(crate) const fn min_seconds(&self) -> &Ratio<i64> {
        &self.min_seconds
    }

    pub(crate) const fn max_seconds(&self) -> &Ratio<i64> {
        &self.max_seconds
    }
}

/// The published bounded-search constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SearchBounds {
    top_k: u64,
    layer_states: u64,
    max_notes: u64,
    max_storage_bytes: u64,
    max_voices: u64,
}

impl SearchBounds {
    pub(crate) const fn top_k(&self) -> u64 {
        self.top_k
    }

    pub(crate) const fn layer_states(&self) -> u64 {
        self.layer_states
    }

    pub(crate) const fn max_notes(&self) -> u64 {
        self.max_notes
    }

    pub(crate) const fn max_storage_bytes(&self) -> u64 {
        self.max_storage_bytes
    }

    pub(crate) const fn max_voices(&self) -> u64 {
        self.max_voices
    }
}

/// One named structural policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TranscriptionPolicy {
    name: String,
    subdivisions: Vec<Subdivision>,
    tuplets: Vec<TupletAllowance>,
    unsubdivided_complexity: u64,
    weights: CostWeights,
    group_window: GroupWindow,
    bounds: SearchBounds,
}

impl TranscriptionPolicy {
    /// The source-declared policy name.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Admitted beat divisions in declared order.
    pub(crate) fn subdivisions(&self) -> &[Subdivision] {
        &self.subdivisions
    }

    /// Admitted irregular divisions in declared order.
    pub(crate) fn tuplets(&self) -> &[TupletAllowance] {
        &self.tuplets
    }

    /// The declared cost of a grid position no subdivision or tuplet names.
    pub(crate) const fn unsubdivided_complexity(&self) -> u64 {
        self.unsubdivided_complexity
    }

    pub(crate) const fn weights(&self) -> &CostWeights {
        &self.weights
    }

    pub(crate) const fn group_window(&self) -> &GroupWindow {
        &self.group_window
    }

    pub(crate) const fn bounds(&self) -> SearchBounds {
        self.bounds
    }
}

/// The complete projected artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TranscriptionPolicies {
    cost_fields: Vec<String>,
    cost_version: u64,
    policies: Vec<TranscriptionPolicy>,
    exact_source: Arc<[u8]>,
}

impl TranscriptionPolicies {
    /// The published cost-field order.
    pub(crate) fn cost_fields(&self) -> &[String] {
        &self.cost_fields
    }

    /// The published cost-record version.
    pub(crate) const fn cost_version(&self) -> u64 {
        self.cost_version
    }

    /// Look up one named policy without inventing one.
    ///
    /// # Errors
    ///
    /// Returns [`TranscriptionPolicyError::UnknownPolicy`] when no declaration
    /// carries the requested name.
    pub(crate) fn policy(&self, name: &str) -> Result<&TranscriptionPolicy, TranscriptionPolicyError> {
        self.policies
            .iter()
            .find(|policy| policy.name == name)
            .ok_or_else(|| TranscriptionPolicyError::UnknownPolicy(name.to_owned()))
    }

    /// The complete checked source identity this projection came from.
    #[cfg(test)]
    fn exact_source_bytes(&self) -> &[u8] {
        &self.exact_source
    }
}

/// Why a checked artifact is not the transcription policy.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub(crate) enum TranscriptionPolicyError {
    /// The artifact names another consumer schema or source version.
    #[error("expected checked source schema `{SCHEMA_NAME}` version {SCHEMA_VERSION}")]
    WrongSchema,
    /// No declared policy has this name.
    #[error("no transcription policy named `{0}`")]
    UnknownPolicy(String),
    /// A canonical constructor, literal, or field count disagrees with the source declaration.
    #[error("malformed TranscriptionPolicyArtifact at {path}: {expected}")]
    Malformed {
        /// Structural position within the value.
        path: String,
        /// Required shape at that position.
        expected: &'static str,
    },
}

/// Read the edition-pinned transcription policies compiled from
/// `std::transcription`.
///
/// Decoding happens once per process and the failure is held so repeated
/// editor requests do not recompile an unchanged broken library.
///
/// # Errors
///
/// Returns a stable description when the bundled declarations fail to check or
/// the exact artifact disagrees with the consumer schema. Either is a build
/// defect.
pub(crate) fn standard() -> Result<&'static TranscriptionPolicies, &'static str> {
    POLICIES
        .get_or_init(|| {
            let checked = musa_compiler::checked_standard_transcription_policies().map_err(|diagnostics| {
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            })?;
            decode(&checked).map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(String::as_str)
}

fn decode(source: &CheckedSource) -> Result<TranscriptionPolicies, TranscriptionPolicyError> {
    if !source.has_valid_framing() {
        return Err(malformed("artifact", "complete exact framing"));
    }
    if source.schema() != &schema() {
        return Err(TranscriptionPolicyError::WrongSchema);
    }
    let [schema_version, cost_fields, cost_version, policies] = case_fields(
        source.root(),
        "TranscriptionPolicyArtifact.TranscriptionPolicyArtifact",
        "root",
    )?;
    if nat(schema_version, "root.schema_version")? != SCHEMA_VERSION {
        return Err(malformed("root.schema_version", "the checked schema version"));
    }
    let cost_fields = list(cost_fields, "root.cost_fields", text)?;
    let cost_version = nat(cost_version, "root.cost_version")?;
    let policies = list(policies, "root.policies", decode_policy)?;
    let mut names = std::collections::HashSet::new();
    if let Some(duplicate) = policies.iter().find(|policy| !names.insert(policy.name.as_str())) {
        return Err(malformed(
            &format!("root.policies.{}", duplicate.name),
            "one declaration per policy name",
        ));
    }
    Ok(TranscriptionPolicies {
        cost_fields,
        cost_version,
        policies,
        exact_source: source.exact_bytes().into(),
    })
}

fn decode_policy(datum: SourceDatum<'_>, path: &str) -> Result<TranscriptionPolicy, TranscriptionPolicyError> {
    let [name, subdivisions, tuplets, unsubdivided, weights, window, bounds] =
        case_fields(datum, "TranscriptionPolicy.TranscriptionPolicy", path)?;
    Ok(TranscriptionPolicy {
        name: text(name, &field(path, "name"))?,
        subdivisions: list(subdivisions, &field(path, "subdivisions"), decode_subdivision)?,
        tuplets: list(tuplets, &field(path, "tuplets"), decode_tuplet)?,
        unsubdivided_complexity: nat(unsubdivided, &field(path, "unsubdivided_complexity"))?,
        weights: decode_weights(weights, &field(path, "weights"))?,
        group_window: decode_window(window, &field(path, "group_window"))?,
        bounds: decode_bounds(bounds, &field(path, "bounds"))?,
    })
}

fn decode_subdivision(datum: SourceDatum<'_>, path: &str) -> Result<Subdivision, TranscriptionPolicyError> {
    let [fraction, complexity] = case_fields(datum, "Subdivision.Subdivision", path)?;
    Ok(Subdivision {
        beat_fraction: ratio(fraction, &field(path, "beat_fraction"))?,
        complexity: nat(complexity, &field(path, "complexity"))?,
    })
}

fn decode_tuplet(datum: SourceDatum<'_>, path: &str) -> Result<TupletAllowance, TranscriptionPolicyError> {
    let [fraction, complexity] = case_fields(datum, "TupletAllowance.TupletAllowance", path)?;
    Ok(TupletAllowance {
        member_fraction: ratio(fraction, &field(path, "member_fraction"))?,
        complexity: nat(complexity, &field(path, "complexity"))?,
    })
}

fn decode_weights(datum: SourceDatum<'_>, path: &str) -> Result<CostWeights, TranscriptionPolicyError> {
    let [onset, duration, tempo, complexity, transition, group_split] =
        case_fields(datum, "CostWeights.CostWeights", path)?;
    Ok(CostWeights {
        onset_residual: ratio(onset, &field(path, "onset_residual"))?,
        duration_residual: ratio(duration, &field(path, "duration_residual"))?,
        tempo_smoothness: ratio(tempo, &field(path, "tempo_smoothness"))?,
        notation_complexity: ratio(complexity, &field(path, "notation_complexity"))?,
        transition: ratio(transition, &field(path, "transition"))?,
        group_split: ratio(group_split, &field(path, "group_split"))?,
    })
}

fn decode_window(datum: SourceDatum<'_>, path: &str) -> Result<GroupWindow, TranscriptionPolicyError> {
    let [fraction, min, max] = case_fields(datum, "GroupWindow.GroupWindow", path)?;
    Ok(GroupWindow {
        beat_fraction: ratio(fraction, &field(path, "beat_fraction"))?,
        min_seconds: ratio(min, &field(path, "min_seconds"))?,
        max_seconds: ratio(max, &field(path, "max_seconds"))?,
    })
}

fn decode_bounds(datum: SourceDatum<'_>, path: &str) -> Result<SearchBounds, TranscriptionPolicyError> {
    let [top_k, layer_states, max_notes, max_storage_bytes, max_voices] =
        case_fields(datum, "SearchBounds.SearchBounds", path)?;
    Ok(SearchBounds {
        top_k: nat(top_k, &field(path, "top_k"))?,
        layer_states: nat(layer_states, &field(path, "layer_states"))?,
        max_notes: nat(max_notes, &field(path, "max_notes"))?,
        max_storage_bytes: nat(max_storage_bytes, &field(path, "max_storage_bytes"))?,
        max_voices: nat(max_voices, &field(path, "max_voices"))?,
    })
}

fn case_fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    constructor: &str,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], TranscriptionPolicyError> {
    match datum.kind() {
        Some(SourceDatumKind::Case { constructor: found }) if found == constructor => fields(datum, path),
        _ => Err(malformed(path, "the required source constructor")),
    }
}

fn fields<'a, const COUNT: usize>(
    datum: SourceDatum<'a>,
    path: &str,
) -> Result<[SourceDatum<'a>; COUNT], TranscriptionPolicyError> {
    let Some(direct) = datum.fields() else {
        return Err(malformed(path, "valid canonical child framing"));
    };
    let Some(children): Option<Vec<_>> = direct.collect() else {
        return Err(malformed(path, "valid canonical child indices"));
    };
    if children.len() != COUNT {
        return Err(malformed(path, "the declared field count"));
    }
    children
        .try_into()
        .map_err(|_| malformed(path, "the declared field count"))
}

fn field(path: &str, name: &str) -> String {
    format!("{path}.{name}")
}

fn nat(datum: SourceDatum<'_>, path: &str) -> Result<u64, TranscriptionPolicyError> {
    match datum.kind() {
        Some(SourceDatumKind::Count { family: "Nat", count }) => Ok(count),
        _ => Err(malformed(path, "a Nat")),
    }
}

fn text(datum: SourceDatum<'_>, path: &str) -> Result<String, TranscriptionPolicyError> {
    match datum.kind() {
        Some(SourceDatumKind::Literal {
            type_name: "Text",
            bytes,
        }) => String::from_utf8(bytes.to_vec()).map_err(|_| malformed(path, "UTF-8 Text")),
        _ => Err(malformed(path, "Text")),
    }
}

fn ratio(datum: SourceDatum<'_>, path: &str) -> Result<Ratio<i64>, TranscriptionPolicyError> {
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

fn list<T>(
    mut datum: SourceDatum<'_>,
    path: &str,
    decode: fn(SourceDatum<'_>, &str) -> Result<T, TranscriptionPolicyError>,
) -> Result<Vec<T>, TranscriptionPolicyError> {
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

fn malformed(path: &str, expected: &'static str) -> TranscriptionPolicyError {
    TranscriptionPolicyError::Malformed {
        path: path.to_owned(),
        expected,
    }
}

#[cfg(test)]
mod laws {
    #![allow(clippy::arithmetic_side_effects)]
    #![allow(clippy::expect_used)]

    use super::*;

    const fn gcd(mut left: u64, mut right: u64) -> u64 {
        while right != 0 {
            let next = left % right;
            left = right;
            right = next;
        }
        left
    }

    const fn lcm(left: u64, right: u64) -> u64 {
        if left == 0 || right == 0 {
            return 0;
        }
        left / gcd(left, right) * right
    }

    fn ratio(numerator: i64, denominator: i64) -> Ratio<i64> {
        Ratio::new(numerator, denominator)
    }

    /// Hand-derived values from `stdlib/src/transcription/mod.musa`.
    #[test]
    fn standard_policy_declared_values_read_back_exactly() {
        let policies = standard().expect("the bundled policy must check and decode");
        assert_eq!(policies.cost_version(), 1);
        assert_eq!(
            policies.cost_fields(),
            [
                "onset-displacement",
                "duration-displacement",
                "tempo-smoothness",
                "notation-complexity",
                "rests",
                "ties",
                "tuplets",
                "syncopation-preservation",
                "user-constraints",
            ]
        );

        let policy = policies.policy("standard").expect("standard policy exists");
        assert_eq!(policy.name(), "standard");
        let subdivisions = policy.subdivisions();
        assert_eq!(subdivisions.len(), 4);
        assert_eq!(
            subdivisions
                .iter()
                .map(|subdivision| (*subdivision.beat_fraction(), subdivision.complexity()))
                .collect::<Vec<_>>(),
            [(ratio(1, 1), 0), (ratio(1, 2), 1), (ratio(1, 4), 2), (ratio(1, 8), 4),]
        );
        let tuplets = policy.tuplets();
        assert_eq!(tuplets.len(), 2);
        assert_eq!(
            tuplets
                .iter()
                .map(|tuplet| (*tuplet.member_fraction(), tuplet.complexity()))
                .collect::<Vec<_>>(),
            [(ratio(1, 3), 3), (ratio(1, 6), 5)]
        );
        assert_eq!(policy.unsubdivided_complexity(), 8);

        let weights = policy.weights();
        assert_eq!(*weights.onset_residual(), ratio(2, 1));
        assert_eq!(*weights.duration_residual(), ratio(0, 1));
        assert_eq!(*weights.tempo_smoothness(), ratio(0, 1));
        assert_eq!(*weights.notation_complexity(), ratio(2, 1));
        assert_eq!(*weights.transition(), ratio(1, 1));
        assert_eq!(*weights.group_split(), ratio(10, 1));

        let window = policy.group_window();
        assert_eq!(*window.beat_fraction(), ratio(1, 12));
        assert_eq!(*window.min_seconds(), ratio(18, 1000));
        assert_eq!(*window.max_seconds(), ratio(70, 1000));

        let bounds = policy.bounds();
        assert_eq!(bounds.top_k(), 5);
        assert_eq!(bounds.layer_states(), 96);
        assert_eq!(bounds.max_notes(), 128);
        assert_eq!(bounds.max_storage_bytes(), 131_072);
        assert_eq!(bounds.max_voices(), 4);
    }

    /// The host grid is the LCM of declared denominators; the trial's 24-tick
    /// quarter is derived, never admitted independently.
    #[test]
    fn standard_grid_is_the_least_common_multiple_of_declared_denominators() {
        let policy = standard().expect("decode").policy("standard").expect("standard exists");
        let mut grid = 1_u64;
        for subdivision in policy.subdivisions() {
            grid = lcm(
                grid,
                u64::try_from(*subdivision.beat_fraction().denom()).expect("denominator fits"),
            );
        }
        for tuplet in policy.tuplets() {
            grid = lcm(
                grid,
                u64::try_from(*tuplet.member_fraction().denom()).expect("denominator fits"),
            );
        }
        assert_eq!(
            grid, 24,
            "the policy's declared subdivisions must derive the 24-tick grid"
        );
    }

    /// The unmeasured policy admits no grid, so its only honest outcome is a
    /// host refusal rather than an invented metrical scope.
    #[test]
    fn unmeasured_policy_admits_no_subdivision() {
        let policy = standard()
            .expect("decode")
            .policy("unmeasured")
            .expect("unmeasured exists");
        assert!(policy.subdivisions().is_empty());
        assert!(policy.tuplets().is_empty());
        assert_eq!(policy.unsubdivided_complexity(), 0);
    }

    /// An unknown policy name is refused rather than falling back to a host
    /// default.
    #[test]
    fn unknown_policy_is_refused() {
        let policies = standard().expect("decode");
        assert_eq!(
            policies
                .policy("no-such-policy")
                .expect_err("unknown policy must be refused"),
            TranscriptionPolicyError::UnknownPolicy("no-such-policy".to_owned())
        );
    }

    /// The retained exact bytes are the artifact's complete identity, so two
    /// decodes agree and the framing prefix is the canonical one.
    #[test]
    fn exact_source_identity_is_retained() {
        let policies = standard().expect("decode");
        let bytes = policies.exact_source_bytes().to_vec();
        assert!(bytes.starts_with(b"musa-checked-source"));
        assert_eq!(policies.exact_source_bytes(), policies.exact_source_bytes());
    }

    /// A checked value under the same root type but a different consumer
    /// schema version is refused by the decoder, not reinterpreted.
    #[test]
    fn wrong_schema_version_is_refused() {
        let probe = musa_compiler::SourceDocument::new(
            r#"import std::transcription;
piece "Transcription policy" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#,
            "musa-stdlib:/transcription-policy.musa",
        );
        let checked = musa_compiler::checked_source_value(
            &probe,
            &musa_compiler::CompileOptions::default(),
            "transcription_policies",
            &musa_calculus::SourceSchema::new(SCHEMA_NAME, ROOT_TYPE, SCHEMA_VERSION + 1),
        )
        .expect("the same source checks under a bumped consumer version");
        assert_eq!(
            decode(&checked).expect_err("a bumped version must be refused"),
            TranscriptionPolicyError::WrongSchema
        );
    }
}
