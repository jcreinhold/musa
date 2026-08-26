//! Bounded `sfz@1` adaptation into the source-owned `SampleMapArtifact`.
//!
//! This module owns foreign-text validation only. Its result is ordinary Musa
//! source, checked by the same elaborator and schema as a handwritten map.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::error::ProjectError;

/// Explicit bounds for one off-thread `sfz@1` adaptation. There is no default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SfzLimits {
    /// Greatest accepted SFZ byte count.
    pub max_file_bytes: usize,
    /// Greatest accepted number of region headers.
    pub max_regions: usize,
    /// Greatest accepted number of opcode assignments.
    pub max_opcodes: usize,
    /// Greatest accepted byte count of one opcode value.
    pub max_value_bytes: usize,
}

/// Readable facts about one imported SFZ instrument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SfzInstrumentFacts {
    /// Source instrument declaration name.
    pub instrument: String,
    /// Verified logical SFZ path.
    pub asset: String,
    /// Number of translated regions.
    pub regions: usize,
    /// Harmless metadata warnings retained by the adapter.
    pub warnings: Vec<String>,
    /// Stable adapter identity whose support matrix governs the result.
    pub adapter: &'static str,
}

pub(crate) struct AdaptedSfz {
    pub(crate) source: String,
    pub(crate) facts: SfzInstrumentFacts,
}

#[derive(Clone)]
struct Value {
    text: String,
    offset: usize,
}

struct Region {
    values: BTreeMap<String, Value>,
    sequence_scope: u64,
}

enum Item {
    Header { name: String, offset: usize },
    Opcode { name: String, value: Value },
}

pub(crate) fn adapt(
    instrument: &str,
    logical: &str,
    bytes: &[u8],
    limits: SfzLimits,
) -> Result<AdaptedSfz, ProjectError> {
    if bytes.len() > limits.max_file_bytes {
        return fail(format!(
            "`{logical}` has {} bytes, above the explicit SFZ limit {}",
            bytes.len(),
            limits.max_file_bytes
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProjectError::Assets(format!("SFZ `{logical}` is not UTF-8 at byte {}", error.valid_up_to()))
    })?;
    let items = scan(text, limits)?;
    let mut global = BTreeMap::new();
    let mut group = BTreeMap::new();
    let mut current: Option<Region> = None;
    let mut regions = Vec::new();
    let mut warnings = Vec::new();
    let mut header = "";
    let mut sequence_scope = 1u64;
    for item in items {
        match item {
            Item::Header { name, offset } => {
                if let Some(region) = current.take() {
                    regions.push(region);
                }
                match name.as_str() {
                    "global" => {
                        if !regions.is_empty() || !group.is_empty() {
                            return at(text, logical, offset, "`<global>` must precede groups and regions");
                        }
                        header = "global";
                    }
                    "group" => {
                        sequence_scope = sequence_scope.saturating_add(1);
                        group = global.clone();
                        header = "group";
                    }
                    "region" => {
                        current = Some(Region {
                            values: if group.is_empty() {
                                global.clone()
                            } else {
                                group.clone()
                            },
                            sequence_scope,
                        });
                        header = "region";
                    }
                    _ => return at(text, logical, offset, &format!("unsupported SFZ header `<{name}>`")),
                }
            }
            Item::Opcode { name, value } => {
                if header.is_empty() {
                    return at(
                        text,
                        logical,
                        value.offset,
                        &format!("opcode `{name}` appears before a header"),
                    );
                }
                if matches!(name.as_str(), "global_label" | "group_label" | "region_label") {
                    warnings.push(format!("{name}={} (metadata only)", value.text));
                    continue;
                }
                if !supported(&name) {
                    return at(
                        text,
                        logical,
                        value.offset,
                        &format!("unsupported sound-changing opcode `{name}`"),
                    );
                }
                let target = match header {
                    "global" => Some(&mut global),
                    "group" => Some(&mut group),
                    "region" => current.as_mut().map(|region| &mut region.values),
                    _ => None,
                }
                .ok_or_else(|| positioned(text, logical, value.offset, "opcode has no active SFZ header"))?;
                if name == "key" {
                    target.insert("lokey".to_owned(), value.clone());
                    target.insert("hikey".to_owned(), value.clone());
                    target.insert("pitch_keycenter".to_owned(), value);
                } else {
                    target.insert(name, value);
                }
            }
        }
    }
    if let Some(region) = current {
        regions.push(region);
    }
    if regions.is_empty() {
        return fail(format!("SFZ `{logical}` declares no `<region>`"));
    }
    if regions.len() > limits.max_regions {
        return fail(format!(
            "SFZ `{logical}` declares {} regions, above the explicit limit {}",
            regions.len(),
            limits.max_regions
        ));
    }
    let mut source = String::from(
        "import std::sound::sample;\nlet imported_sfz_map: SampleMapArtifact = SampleMapArtifact {\n    schema_version = 3,\n    sample_map = SampleMap {\n",
    );
    writeln!(source, "        declaration_id = \"{}@sfz@1\",", escape(instrument))
        .map_err(|_| ProjectError::Assets("cannot construct checked SFZ adapter source".to_owned()))?;
    source.push_str("        selection = RoundRobin,\n        voices = 64,\n        regions = [\n");
    for region in &regions {
        write_region(&mut source, text, logical, region)?;
    }
    source.push_str("        ]\n    }\n};\npiece \"Imported SFZ adapter result\" { meter 4/4; key c major; score { part proof { voice observed { rest/1 } } } }\n");
    Ok(AdaptedSfz {
        source,
        facts: SfzInstrumentFacts {
            instrument: instrument.to_owned(),
            asset: logical.to_owned(),
            regions: regions.len(),
            warnings,
            adapter: "sfz@1",
        },
    })
}

fn supported(name: &str) -> bool {
    matches!(
        name,
        "sample"
            | "key"
            | "lokey"
            | "hikey"
            | "pitch_keycenter"
            | "pitch_keytrack"
            | "lovel"
            | "hivel"
            | "tune"
            | "transpose"
            | "volume"
            | "pan"
            | "offset"
            | "end"
            | "loop_start"
            | "loop_end"
            | "loop_mode"
            | "loop_type"
            | "ampeg_attack"
            | "ampeg_decay"
            | "ampeg_sustain"
            | "ampeg_release"
            | "trigger"
            | "group"
            | "off_by"
            | "off_mode"
            | "seq_length"
            | "seq_position"
            | "locc64"
            | "hicc64"
    )
}

fn scan(text: &str, limits: SfzLimits) -> Result<Vec<Item>, ProjectError> {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    let mut opcodes = 0usize;
    let mut items = Vec::new();
    while index < bytes.len() {
        index = skip_space_and_comments(text, index)?;
        if index >= bytes.len() {
            break;
        }
        if matches!(bytes.get(index), Some(b'#' | b'$')) {
            return at(
                text,
                "SFZ",
                index,
                "directives, includes, macros, and substitution are unsupported",
            );
        }
        if bytes.get(index) == Some(&b'<') {
            let start = index;
            let after_open = index.saturating_add(1);
            let end = bytes
                .get(after_open..)
                .unwrap_or_default()
                .iter()
                .position(|byte| *byte == b'>')
                .map(|relative| after_open.saturating_add(relative))
                .ok_or_else(|| ProjectError::Assets(format!("SFZ header at byte {start} has no closing `>`")))?;
            let name = text
                .get(after_open..end)
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            if name.is_empty() {
                return at(text, "SFZ", start, "an SFZ header name cannot be empty");
            }
            items.push(Item::Header { name, offset: start });
            index = end.saturating_add(1);
            continue;
        }
        let name_start = index;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            index = index.saturating_add(1);
        }
        if index == name_start {
            return at(text, "SFZ", index, "expected an opcode name or header");
        }
        let name = text.get(name_start..index).unwrap_or_default().to_ascii_lowercase();
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index = index.saturating_add(1);
        }
        if bytes.get(index) != Some(&b'=') {
            return at(text, "SFZ", name_start, &format!("opcode `{name}` has no `=`"));
        }
        index = index.saturating_add(1);
        while matches!(bytes.get(index), Some(b' ' | b'\t')) {
            index = index.saturating_add(1);
        }
        let mut value_start = index;
        let value_end;
        if bytes.get(index) == Some(&b'"') {
            index = index.saturating_add(1);
            value_start = index;
            let quoted_start = index;
            while bytes.get(index).is_some_and(|byte| *byte != b'"') {
                index = index.saturating_add(1);
            }
            if index >= bytes.len() {
                return at(
                    text,
                    "SFZ",
                    value_start,
                    &format!("quoted value for `{name}` is unfinished"),
                );
            }
            value_end = index;
            index = index.saturating_add(1);
            if value_end == quoted_start {
                return at(text, "SFZ", value_start, &format!("value for `{name}` is empty"));
            }
        } else {
            while index < bytes.len() {
                if matches!(bytes.get(index), Some(b'<' | b'#' | b'$')) {
                    break;
                }
                if bytes.get(index).is_some_and(u8::is_ascii_whitespace) && begins_item(text, index) {
                    break;
                }
                index = index.saturating_add(1);
            }
            value_end = index;
        }
        let value = text.get(value_start..value_end).unwrap_or_default().trim();
        if value.is_empty() {
            return at(text, "SFZ", value_start, &format!("value for `{name}` is empty"));
        }
        if value.len() > limits.max_value_bytes {
            return at(
                text,
                "SFZ",
                value_start,
                &format!("value for `{name}` exceeds its explicit byte limit"),
            );
        }
        opcodes = opcodes.saturating_add(1);
        if opcodes > limits.max_opcodes {
            return fail(format!(
                "SFZ opcode count exceeds the explicit limit {}",
                limits.max_opcodes
            ));
        }
        items.push(Item::Opcode {
            name,
            value: Value {
                text: value.to_owned(),
                offset: value_start,
            },
        });
    }
    Ok(items)
}

fn skip_space_and_comments(text: &str, mut index: usize) -> Result<usize, ProjectError> {
    let bytes = text.as_bytes();
    loop {
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index = index.saturating_add(1);
        }
        let pair_end = index.saturating_add(2);
        if bytes.get(index..pair_end) == Some(b"//") {
            index = bytes
                .get(index..)
                .unwrap_or_default()
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |relative| index.saturating_add(relative).saturating_add(1));
        } else if bytes.get(index..pair_end) == Some(b"/*") {
            let start = index;
            let body = index.saturating_add(2);
            index = text
                .get(body..)
                .unwrap_or_default()
                .find("*/")
                .map(|relative| body.saturating_add(relative).saturating_add(2))
                .ok_or_else(|| ProjectError::Assets(format!("SFZ block comment at byte {start} is unfinished")))?;
        } else {
            return Ok(index);
        }
    }
}

fn begins_item(text: &str, whitespace: usize) -> bool {
    let bytes = text.as_bytes();
    let mut index = whitespace;
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index = index.saturating_add(1);
    }
    if index >= bytes.len()
        || matches!(bytes.get(index), Some(b'<' | b'#' | b'$'))
        || bytes.get(index..index.saturating_add(2)) == Some(b"//")
    {
        return true;
    }
    let start = index;
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        index = index.saturating_add(1);
    }
    while matches!(bytes.get(index), Some(b' ' | b'\t')) {
        index = index.saturating_add(1);
    }
    index > start && bytes.get(index) == Some(&b'=')
}

fn write_region(out: &mut String, text: &str, logical: &str, region: &Region) -> Result<(), ProjectError> {
    let values = &region.values;
    let sample = required(values, text, logical, "sample")?;
    let asset = sample_path(logical, &sample.text).map_err(ProjectError::Assets)?;
    if !asset.to_ascii_lowercase().ends_with(".wav") {
        return at(text, logical, sample.offset, "`sample` must name a WAV asset in sfz@1");
    }
    let key_low = unsigned(values, text, logical, "lokey", 0, 127)?;
    let key_high = unsigned(values, text, logical, "hikey", 127, 127)?;
    let root_key = unsigned(values, text, logical, "pitch_keycenter", 60, 127)?;
    if key_low > key_high || root_key < key_low || root_key > key_high {
        return reject_value(
            values,
            text,
            logical,
            "pitch_keycenter",
            "the key range must be ordered and contain `pitch_keycenter`",
        );
    }
    if unsigned(values, text, logical, "pitch_keytrack", 100, u64::MAX)? != 100 {
        return reject_value(
            values,
            text,
            logical,
            "pitch_keytrack",
            "sfz@1 supports only `pitch_keytrack=100`",
        );
    }
    let lovel = unsigned(values, text, logical, "lovel", 0, 127)?;
    let hivel = unsigned(values, text, logical, "hivel", 127, 127)?;
    if lovel > hivel {
        return reject_value(values, text, logical, "hivel", "`lovel` must not exceed `hivel`");
    }
    let tune = signed(values, text, logical, "tune", 0)?;
    let transpose = signed(values, text, logical, "transpose", 0)?;
    let transposed_cents = transpose.checked_mul(100).ok_or_else(|| {
        values.get("transpose").map_or_else(
            || ProjectError::Assets(format!("SFZ `{logical}` transpose exceeds exact cents bounds")),
            |value| positioned(text, logical, value.offset, "`transpose` exceeds exact cents bounds"),
        )
    })?;
    let tune_cents = tune.checked_add(transposed_cents).ok_or_else(|| {
        let (name, message) = if values.contains_key("tune") {
            ("tune", "combined `tune` and `transpose` exceed exact cents bounds")
        } else {
            ("transpose", "`transpose` exceeds exact cents bounds")
        };
        values.get(name).map_or_else(
            || ProjectError::Assets(format!("SFZ `{logical}` tuning exceeds exact cents bounds")),
            |value| positioned(text, logical, value.offset, message),
        )
    })?;
    let volume = decimal(values, text, logical, "volume", "0/1")?;
    let pan = decimal(values, text, logical, "pan", "0/1")?;
    if ratio_compare(&pan, -100, 100).is_none() {
        return reject_value(values, text, logical, "pan", "`pan` must lie from -100 through 100");
    }
    let start = unsigned(values, text, logical, "offset", 0, u64::MAX)?;
    let end = inclusive_end(values, text, logical, "end")?;
    let written_loop_start = unsigned(values, text, logical, "loop_start", 0, u64::MAX)?;
    let written_loop_end = inclusive_end(values, text, logical, "loop_end")?;
    let loop_mode_name = plain(values, "loop_mode", "no_loop");
    let loop_type = plain(values, "loop_type", "forward");
    let loop_mode = match (loop_mode_name, loop_type) {
        ("no_loop", _) => "NoLoop",
        ("one_shot", _) => "OneShot",
        ("loop_sustain", "forward") => "ForwardSustainLoop",
        ("loop_sustain", "alternate") => "AlternatingSustainLoop",
        ("loop_continuous", "forward") => "ForwardContinuousLoop",
        ("loop_continuous", "alternate") => "AlternatingContinuousLoop",
        (mode, _) if !matches!(mode, "no_loop" | "one_shot" | "loop_sustain" | "loop_continuous") => {
            return reject_value(
                values,
                text,
                logical,
                "loop_mode",
                &format!("unsupported loop_mode `{mode}`"),
            );
        }
        (_, kind) => {
            return reject_value(
                values,
                text,
                logical,
                "loop_type",
                &format!("unsupported loop_type `{kind}`"),
            );
        }
    };
    if loop_mode != "NoLoop"
        && loop_mode != "OneShot"
        && (written_loop_end == 0 || written_loop_start >= written_loop_end)
    {
        return reject_value(
            values,
            text,
            logical,
            "loop_end",
            "a loop mode requires explicit ordered loop points",
        );
    }
    let (loop_start, loop_end) = if matches!(loop_mode, "NoLoop" | "OneShot") {
        (0, 0)
    } else {
        (written_loop_start, written_loop_end)
    };
    let attack = decimal(values, text, logical, "ampeg_attack", "0/1")?;
    let decay = decimal(values, text, logical, "ampeg_decay", "0/1")?;
    for (name, value) in [("ampeg_attack", &attack), ("ampeg_decay", &decay)] {
        if !ratio_nonnegative(value) {
            return reject_value(values, text, logical, name, &format!("`{name}` must not be negative"));
        }
    }
    let sustain = decimal(values, text, logical, "ampeg_sustain", "100/1")?;
    if ratio_compare(&sustain, 0, 100).is_none() {
        return reject_value(
            values,
            text,
            logical,
            "ampeg_sustain",
            "`ampeg_sustain` must lie from 0 through 100",
        );
    }
    let release = decimal(values, text, logical, "ampeg_release", "1/1000")?;
    if !ratio_nonnegative(&release) {
        return reject_value(
            values,
            text,
            logical,
            "ampeg_release",
            "`ampeg_release` must not be negative",
        );
    }
    let trigger_name = plain(values, "trigger", "attack");
    let (trigger, connection) = match trigger_name {
        "attack" => ("AttackTrigger", "AnyConnection"),
        "release" => ("ReleaseTrigger", "AnyConnection"),
        "release_key" => ("ReleaseKeyTrigger", "AnyConnection"),
        "first" => ("AttackTrigger", "FirstConnection"),
        "legato" => ("AttackTrigger", "LegatoConnection"),
        other => {
            return reject_value(
                values,
                text,
                logical,
                "trigger",
                &format!("unsupported trigger `{other}`"),
            );
        }
    };
    let locc = unsigned(values, text, logical, "locc64", 0, 127)?;
    let hicc = unsigned(values, text, logical, "hicc64", 127, 127)?;
    let pedal = match (locc, hicc) {
        (0, 127) => "AnyPedal",
        (0, 63) => "PedalUp",
        (64, 127) => "PedalDown",
        _ => {
            let opcode = if values.contains_key("hicc64") {
                "hicc64"
            } else {
                "locc64"
            };
            return reject_value(
                values,
                text,
                logical,
                opcode,
                "the CC64 range is not one exact typed pedal state",
            );
        }
    };
    let choke_group = unsigned(values, text, logical, "group", 0, u64::from(u32::MAX))?;
    let off_by = unsigned(values, text, logical, "off_by", 0, u64::from(u32::MAX))?;
    let off_mode = match plain(values, "off_mode", "fast") {
        "fast" => "FastOff",
        "normal" => "NormalOff",
        other => {
            return reject_value(
                values,
                text,
                logical,
                "off_mode",
                &format!("unsupported off_mode `{other}`"),
            );
        }
    };
    let sequence_length = unsigned(values, text, logical, "seq_length", 1, u64::MAX)?;
    let sequence_position = unsigned(values, text, logical, "seq_position", 1, u64::MAX)?;
    if sequence_position > sequence_length {
        return reject_value(
            values,
            text,
            logical,
            "seq_position",
            "`seq_position` exceeds `seq_length`",
        );
    }
    let (sequence_group, sequence_position, sequence_length) = if sequence_length == 1 && sequence_position == 1 {
        (0, 0, 0)
    } else {
        (region.sequence_scope, sequence_position, sequence_length)
    };
    write!(
        out,
        "            SampleRegion {{ asset = \"{}\", key_low = {key_low}, key_high = {key_high}, root_key = {root_key}, expression_low = {lovel}/127, expression_high = {hivel}/127, technique = \"\", connection = {connection}, trigger = {trigger}, pedal = {pedal}, sequence_group = {sequence_group}, sequence_position = {sequence_position}, sequence_length = {sequence_length}, weight = 1, priority = 0, tune_cents = {}, gain = DecibelGain({}), expression_gain = LinearExpressionGain, pan = ({}) * 1/100, start_frame = {start}, end_frame = {end}, loop_start = {loop_start}, loop_end = {loop_end}, loop_mode = {loop_mode}, envelope = SampleEnvelope {{ delay = ExactSeconds(0/1), attack = ExactSeconds({attack}), hold = ExactSeconds(0/1), decay = ExactSeconds({decay}), sustain = LinearLevel(({sustain}) * 1/100), release = ExactSeconds({release}), hold_key_timecents = 0/1, decay_key_timecents = 0/1, curve = Sfz1Envelope }}, filter = SampleFilter {{ cutoff_cents = 13500/1, resonance_centibels = 0/1 }}, modulations = [], group = {choke_group}, off_by = {off_by}, off_mode = {off_mode} }},",
        escape(&asset),
        source_integer_ratio(tune_cents),
        source_ratio(&volume),
        source_ratio(&pan),
    )
    .map_err(|_| ProjectError::Assets("cannot construct checked SFZ region source".to_owned()))?;
    Ok(())
}

fn required<'a>(
    values: &'a BTreeMap<String, Value>,
    text: &str,
    logical: &str,
    name: &str,
) -> Result<&'a Value, ProjectError> {
    values
        .get(name)
        .ok_or_else(|| ProjectError::Assets(format!("SFZ `{logical}` region is missing `{name}`")))
        .and_then(|value| {
            if value.text.is_empty() {
                at(text, logical, value.offset, &format!("`{name}` cannot be empty"))
            } else {
                Ok(value)
            }
        })
}

fn reject_value<T>(
    values: &BTreeMap<String, Value>,
    text: &str,
    logical: &str,
    name: &str,
    message: &str,
) -> Result<T, ProjectError> {
    values.get(name).map_or_else(
        || fail(format!("SFZ `{logical}`: {message}")),
        |value| at(text, logical, value.offset, message),
    )
}

fn plain<'a>(values: &'a BTreeMap<String, Value>, name: &str, default: &'a str) -> &'a str {
    values.get(name).map_or(default, |value| value.text.as_str())
}

fn unsigned(
    values: &BTreeMap<String, Value>,
    text: &str,
    logical: &str,
    name: &str,
    default: u64,
    max: u64,
) -> Result<u64, ProjectError> {
    let Some(value) = values.get(name) else {
        return Ok(default);
    };
    value
        .text
        .parse::<u64>()
        .ok()
        .filter(|parsed| *parsed <= max)
        .ok_or_else(|| {
            positioned(
                text,
                logical,
                value.offset,
                &format!("`{name}` must be an integer from 0 through {max}"),
            )
        })
}

fn signed(
    values: &BTreeMap<String, Value>,
    text: &str,
    logical: &str,
    name: &str,
    default: i64,
) -> Result<i64, ProjectError> {
    let Some(value) = values.get(name) else {
        return Ok(default);
    };
    value.text.parse::<i64>().map_err(|_| {
        positioned(
            text,
            logical,
            value.offset,
            &format!("`{name}` must be an exact integer"),
        )
    })
}

fn decimal(
    values: &BTreeMap<String, Value>,
    text: &str,
    logical: &str,
    name: &str,
    default: &str,
) -> Result<String, ProjectError> {
    let Some(value) = values.get(name) else {
        return Ok(default.to_owned());
    };
    decimal_ratio(&value.text).ok_or_else(|| {
        positioned(
            text,
            logical,
            value.offset,
            &format!("`{name}` must be a finite exact decimal"),
        )
    })
}

fn decimal_ratio(value: &str) -> Option<String> {
    let (negative, digits) = value.strip_prefix('-').map_or((false, value), |rest| (true, rest));
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let denominator = 10u64.checked_pow(u32::try_from(fraction.len()).ok()?)?;
    let whole = whole.parse::<i128>().ok()?;
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i128>().ok()?
    };
    let mut numerator = whole.checked_mul(i128::from(denominator))?.checked_add(fraction)?;
    if negative {
        numerator = numerator.checked_neg()?;
    }
    Some(format!("{numerator}/{denominator}"))
}

fn ratio_compare(value: &str, low: i128, high: i128) -> Option<()> {
    let (numerator, denominator) = value.split_once('/')?;
    let numerator = numerator.parse::<i128>().ok()?;
    let denominator = denominator.parse::<i128>().ok()?;
    (numerator >= low.checked_mul(denominator)? && numerator <= high.checked_mul(denominator)?).then_some(())
}

fn ratio_nonnegative(value: &str) -> bool {
    value
        .split_once('/')
        .and_then(|(numerator, denominator)| {
            Some(numerator.parse::<i128>().ok()? >= 0 && denominator.parse::<i128>().ok()? > 0)
        })
        .unwrap_or(false)
}

fn inclusive_end(values: &BTreeMap<String, Value>, text: &str, logical: &str, name: &str) -> Result<u64, ProjectError> {
    let Some(value) = values.get(name) else { return Ok(0) };
    let inclusive = unsigned(values, text, logical, name, 0, u64::MAX)?;
    inclusive.checked_add(1).ok_or_else(|| {
        positioned(
            text,
            logical,
            value.offset,
            &format!("`{name}` cannot cross to a half-open frame end"),
        )
    })
}

fn sample_path(map: &str, sample: &str) -> Result<String, String> {
    let sample = sample.replace('\\', "/");
    if sample.starts_with('/') || sample.contains(':') {
        return Err(format!(
            "SFZ sample `{sample}` must be relative to its owning asset root"
        ));
    }
    let prefix = map.rsplit_once('/').map_or("", |(directory, _)| directory);
    let joined = if prefix.is_empty() {
        sample.clone()
    } else {
        format!("{prefix}/{sample}")
    };
    let floor = usize::from(map.starts_with("pkg:"));
    let mut parts: Vec<&str> = Vec::new();
    for part in joined.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.len() <= floor {
                    return Err(format!("SFZ sample `{sample}` escapes its owning asset root"));
                }
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    Ok(parts.join("/"))
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn source_ratio(value: &str) -> String {
    value
        .strip_prefix('-')
        .map_or_else(|| value.to_owned(), |magnitude| format!("(0/1 - {magnitude})"))
}

fn source_integer_ratio(value: i64) -> String {
    if value < 0 {
        format!("(0/1 - {}/1)", value.unsigned_abs())
    } else {
        format!("{value}/1")
    }
}

fn at<T>(text: &str, logical: &str, offset: usize, message: &str) -> Result<T, ProjectError> {
    Err(positioned(text, logical, offset, message))
}

fn positioned(text: &str, logical: &str, offset: usize, message: &str) -> ProjectError {
    let before = text.get(..offset).unwrap_or(text);
    let line = before.bytes().filter(|byte| *byte == b'\n').count().saturating_add(1);
    let column = before
        .rsplit_once('\n')
        .map_or(before.len(), |(_, tail)| tail.len())
        .saturating_add(1);
    ProjectError::Assets(format!("SFZ `{logical}` at {line}:{column}: {message}"))
}

fn fail<T>(message: String) -> Result<T, ProjectError> {
    Err(ProjectError::Assets(message))
}
