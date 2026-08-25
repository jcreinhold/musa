//! Read-only projection of the source-owned `std::sound::sample::SampleMap`.
//!
//! The checked value is the authority. These types have no public
//! constructors and retain its complete exact bytes beside the fields native
//! preparation must query.

use std::sync::Arc;

use musa_calculus::{CheckedSource, SourceDatum, SourceDatumKind, SourceSchema};
use num_rational::Ratio;
use thiserror::Error;

const SCHEMA_NAME: &str = "std.sound.sample.SampleMapArtifact";
const ROOT_TYPE: &str = "SampleMapArtifact";
const VERSION: u64 = 1;

/// Exact consumer schema for a source-declared sample map.
#[must_use]
pub fn sample_map_schema() -> SourceSchema {
    SourceSchema::new(SCHEMA_NAME, ROOT_TYPE, VERSION)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Trigger {
    Attack,
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PedalCondition {
    Any,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConnectionCondition {
    Any,
    Detached,
    Ordinary,
    Legato,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LoopMode {
    None,
    Forward,
    Alternating,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectionPolicy {
    First,
    RoundRobin,
    StableWeighted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Envelope {
    pub(crate) attack: Ratio<i64>,
    pub(crate) decay: Ratio<i64>,
    pub(crate) sustain: Ratio<i64>,
    pub(crate) release: Ratio<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Region {
    pub(crate) asset: String,
    pub(crate) key_low: u8,
    pub(crate) key_high: u8,
    pub(crate) root_key: u8,
    pub(crate) expression_low: Ratio<i64>,
    pub(crate) expression_high: Ratio<i64>,
    pub(crate) technique: String,
    pub(crate) connection: ConnectionCondition,
    pub(crate) trigger: Trigger,
    pub(crate) pedal: PedalCondition,
    pub(crate) sequence_group: u64,
    pub(crate) sequence_position: u64,
    pub(crate) sequence_length: u64,
    pub(crate) weight: u64,
    pub(crate) priority: u64,
    pub(crate) tune_cents: Ratio<i64>,
    pub(crate) gain: Ratio<i64>,
    pub(crate) pan: Ratio<i64>,
    pub(crate) start_frame: u64,
    pub(crate) end_frame: u64,
    pub(crate) loop_start: u64,
    pub(crate) loop_end: u64,
    pub(crate) loop_mode: LoopMode,
    pub(crate) envelope: Envelope,
    pub(crate) exclusive_group: u64,
}

/// An exact, read-only projection of one checked source map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleMap {
    declaration_id: String,
    pub(crate) selection: SelectionPolicy,
    pub(crate) voices: u16,
    pub(crate) regions: Vec<Region>,
    exact_source: Arc<[u8]>,
}

impl SampleMap {
    /// Stable source declaration identity.
    pub fn declaration_id(&self) -> &str {
        &self.declaration_id
    }

    /// Complete checked canonical bytes from which every projected field came.
    pub fn exact_source_bytes(&self) -> &[u8] {
        &self.exact_source
    }

    /// Number of declared regions.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }
}

/// Why a checked value is not a valid native sample map.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SampleMapError {
    /// The artifact names another source schema or version.
    #[error("expected checked source schema `{SCHEMA_NAME}` version {VERSION}")]
    WrongSchema,
    /// A constructor, field, literal, or invariant is malformed.
    #[error("malformed SampleMapArtifact at {path}: {expected}")]
    Malformed {
        /// Structural position in the checked value.
        path: String,
        /// Required shape or invariant.
        expected: &'static str,
    },
}

/// Decode a complete checked source map without decoding any audio asset.
///
/// # Errors
/// Refuses the wrong schema, incomplete framing, malformed fields, and every
/// range or sequence invariant needed by bounded native preparation.
pub fn decode_sample_map(source: &CheckedSource) -> Result<SampleMap, SampleMapError> {
    if !source.has_valid_framing() {
        return Err(malformed("artifact", "complete exact framing"));
    }
    if source.schema() != &sample_map_schema() {
        return Err(SampleMapError::WrongSchema);
    }
    let [version, map] = fields::<2>(source.root(), "SampleMapArtifact", "root")?;
    if nat(version, "root.schema_version")? != VERSION {
        return Err(malformed("root.schema_version", "the checked schema version"));
    }
    let [declaration_id, selection, voices, regions] = fields::<4>(map, "SampleMap", "root.sample_map")?;
    let voices = nat(voices, "root.sample_map.voices")?;
    let voices = u16::try_from(voices)
        .ok()
        .filter(|voices| *voices > 0)
        .ok_or_else(|| malformed("root.sample_map.voices", "a positive u16 voice count"))?;
    let regions = list(regions, "root.sample_map.regions", region)?;
    if regions.is_empty() {
        return Err(malformed("root.sample_map.regions", "at least one region"));
    }
    if regions.iter().any(|region| region.weight == 0) {
        return Err(malformed(
            "root.sample_map.regions",
            "positive deterministic selection weights",
        ));
    }
    Ok(SampleMap {
        declaration_id: text(declaration_id, "root.sample_map.declaration_id")?,
        selection: match constructor(selection) {
            Some("FirstRegion") => SelectionPolicy::First,
            Some("RoundRobin") => SelectionPolicy::RoundRobin,
            Some("StableWeighted") => SelectionPolicy::StableWeighted,
            _ => return Err(malformed("root.sample_map.selection", "a SampleSelectionPolicy")),
        },
        voices,
        regions,
        exact_source: source.exact_bytes().into(),
    })
}

fn region(datum: SourceDatum<'_>, path: &str) -> Result<Region, SampleMapError> {
    let [
        asset,
        key_low,
        key_high,
        root_key,
        expression_low,
        expression_high,
        technique,
        connection,
        trigger,
        pedal,
        sequence_group,
        sequence_position,
        sequence_length,
        weight,
        priority,
        tune_cents,
        gain,
        pan,
        start_frame,
        end_frame,
        loop_start,
        loop_end,
        loop_mode,
        envelope,
        exclusive_group,
    ] = fields::<25>(datum, "SampleRegion", path)?;
    let key_low = key(key_low, &format!("{path}.key_low"))?;
    let key_high = key(key_high, &format!("{path}.key_high"))?;
    let root_key = key(root_key, &format!("{path}.root_key"))?;
    if key_low > key_high || !(key_low..=key_high).contains(&root_key) {
        return Err(malformed(path, "an ordered key range containing its root"));
    }
    let expression_low = ratio(expression_low, &format!("{path}.expression_low"))?;
    let expression_high = ratio(expression_high, &format!("{path}.expression_high"))?;
    if expression_low < Ratio::ZERO || expression_high > Ratio::ONE || expression_low > expression_high {
        return Err(malformed(path, "an ordered expression range within zero and one"));
    }
    let sequence_group = nat(sequence_group, &format!("{path}.sequence_group"))?;
    let sequence_position = nat(sequence_position, &format!("{path}.sequence_position"))?;
    let sequence_length = nat(sequence_length, &format!("{path}.sequence_length"))?;
    if sequence_group == 0 {
        if sequence_position != 0 || sequence_length != 0 {
            return Err(malformed(path, "zero sequence fields outside a round-robin group"));
        }
    } else if sequence_length == 0 || sequence_position == 0 || sequence_position > sequence_length {
        return Err(malformed(path, "a one-based position inside its sequence length"));
    }
    let gain = ratio(gain, &format!("{path}.gain"))?;
    let pan = ratio(pan, &format!("{path}.pan"))?;
    if gain < Ratio::ZERO || pan < -Ratio::ONE || pan > Ratio::ONE {
        return Err(malformed(path, "nonnegative gain and pan in [-1,1]"));
    }
    let start_frame = nat(start_frame, &format!("{path}.start_frame"))?;
    let end_frame = nat(end_frame, &format!("{path}.end_frame"))?;
    let loop_start = nat(loop_start, &format!("{path}.loop_start"))?;
    let loop_end = nat(loop_end, &format!("{path}.loop_end"))?;
    let loop_mode = match constructor(loop_mode) {
        Some("NoLoop") => LoopMode::None,
        Some("ForwardLoop") => LoopMode::Forward,
        Some("AlternatingLoop") => LoopMode::Alternating,
        _ => return Err(malformed(&format!("{path}.loop_mode"), "a SampleLoopMode")),
    };
    if loop_mode == LoopMode::None {
        if loop_start != 0 || loop_end != 0 {
            return Err(malformed(path, "zero loop bounds when looping is disabled"));
        }
    } else if loop_start >= loop_end {
        return Err(malformed(path, "a nonempty loop interval"));
    }
    let [attack, decay, sustain, release] = fields::<4>(envelope, "SampleEnvelope", path)?;
    let envelope = Envelope {
        attack: ratio(attack, &format!("{path}.envelope.attack_seconds"))?,
        decay: ratio(decay, &format!("{path}.envelope.decay_seconds"))?,
        sustain: ratio(sustain, &format!("{path}.envelope.sustain_level"))?,
        release: ratio(release, &format!("{path}.envelope.release_seconds"))?,
    };
    if envelope.attack < Ratio::ZERO
        || envelope.decay < Ratio::ZERO
        || envelope.release < Ratio::ZERO
        || envelope.sustain < Ratio::ZERO
        || envelope.sustain > Ratio::ONE
    {
        return Err(malformed(path, "nonnegative envelope times and sustain in [0,1]"));
    }
    Ok(Region {
        asset: text(asset, &format!("{path}.asset"))?,
        key_low,
        key_high,
        root_key,
        expression_low,
        expression_high,
        technique: text(technique, &format!("{path}.technique"))?,
        connection: match constructor(connection) {
            Some("AnyConnection") => ConnectionCondition::Any,
            Some("DetachedConnection") => ConnectionCondition::Detached,
            Some("OrdinaryConnection") => ConnectionCondition::Ordinary,
            Some("LegatoConnection") => ConnectionCondition::Legato,
            _ => return Err(malformed(&format!("{path}.connection"), "a SampleConnectionCondition")),
        },
        trigger: match constructor(trigger) {
            Some("AttackTrigger") => Trigger::Attack,
            Some("ReleaseTrigger") => Trigger::Release,
            _ => return Err(malformed(&format!("{path}.trigger"), "a SampleTrigger")),
        },
        pedal: match constructor(pedal) {
            Some("AnyPedal") => PedalCondition::Any,
            Some("PedalUp") => PedalCondition::Up,
            Some("PedalDown") => PedalCondition::Down,
            _ => return Err(malformed(&format!("{path}.pedal"), "a SamplePedalCondition")),
        },
        sequence_group,
        sequence_position,
        sequence_length,
        weight: nat(weight, &format!("{path}.weight"))?,
        priority: nat(priority, &format!("{path}.priority"))?,
        tune_cents: ratio(tune_cents, &format!("{path}.tune_cents"))?,
        gain,
        pan,
        start_frame,
        end_frame,
        loop_start,
        loop_end,
        loop_mode,
        envelope,
        exclusive_group: nat(exclusive_group, &format!("{path}.exclusive_group"))?,
    })
}

fn list<T>(
    mut datum: SourceDatum<'_>,
    path: &str,
    mut decode: impl FnMut(SourceDatum<'_>, &str) -> Result<T, SampleMapError>,
) -> Result<Vec<T>, SampleMapError> {
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
) -> Result<[SourceDatum<'a>; COUNT], SampleMapError> {
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

fn key(datum: SourceDatum<'_>, path: &str) -> Result<u8, SampleMapError> {
    u8::try_from(nat(datum, path)?)
        .ok()
        .filter(|key| *key <= 127)
        .ok_or_else(|| malformed(path, "a key from 0 through 127"))
}

fn nat(datum: SourceDatum<'_>, path: &str) -> Result<u64, SampleMapError> {
    match datum.kind() {
        Some(SourceDatumKind::Count { family, count }) if family.ends_with("Nat") => Ok(count),
        _ => Err(malformed(path, "Nat")),
    }
}

fn text(datum: SourceDatum<'_>, path: &str) -> Result<String, SampleMapError> {
    match datum.kind() {
        Some(SourceDatumKind::Literal { type_name, bytes }) if type_name.ends_with("Text") => {
            std::str::from_utf8(bytes)
                .map(str::to_owned)
                .map_err(|_| malformed(path, "UTF-8 Text"))
        }
        _ => Err(malformed(path, "Text")),
    }
}

fn ratio(datum: SourceDatum<'_>, path: &str) -> Result<Ratio<i64>, SampleMapError> {
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

fn malformed(path: &str, expected: &'static str) -> SampleMapError {
    SampleMapError::Malformed {
        path: path.to_owned(),
        expected,
    }
}
