//! Bounded `sf2@1` adaptation into source-owned sample-map values.
//!
//! RIFF and Hydra structures remain private foreign-input machinery. The
//! semantic result is ordinary Musa source plus verified, digest-framed
//! in-memory WAV assets consumed by the existing sampler preparation path.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Cursor;
use std::sync::Arc;

use sha2::{Digest as _, Sha256};

use crate::error::ProjectError;

/// Explicit bounds for one off-thread `sf2@1` adaptation. There is no default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sf2Limits {
    /// Greatest accepted bank byte count.
    pub max_file_bytes: usize,
    /// Greatest accepted RIFF/LIST subchunk count.
    pub max_chunks: usize,
    /// Greatest accepted preset count, excluding the terminal record.
    pub max_presets: usize,
    /// Greatest accepted instrument count, excluding the terminal record.
    pub max_instruments: usize,
    /// Greatest accepted preset plus instrument zone count.
    pub max_zones: usize,
    /// Greatest accepted generator record count.
    pub max_generators: usize,
    /// Greatest accepted modulator record count.
    pub max_modulators: usize,
    /// Greatest accepted sample-header count, excluding the terminal record.
    pub max_samples: usize,
    /// Greatest retained embedded PCM byte count after 16/24-bit decoding.
    pub max_pcm_bytes: usize,
}

/// Readable facts about one imported `SoundFont` preset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sf2InstrumentFacts {
    /// Source instrument declaration name.
    pub instrument: String,
    /// Verified logical bank path without its preset fragment.
    pub asset: String,
    /// Exact selected preset name.
    pub preset_name: String,
    /// `SoundFont` bank number, without General MIDI interpretation.
    pub bank: u16,
    /// `SoundFont` preset/program number, without General MIDI interpretation.
    pub program: u16,
    /// Number of flattened audible regions.
    pub regions: usize,
    /// Stable adapter identity whose support matrix governs the result.
    pub adapter: &'static str,
}

pub(crate) struct AdaptedSf2 {
    pub(crate) source: String,
    pub(crate) samples: BTreeMap<String, Arc<[u8]>>,
    pub(crate) facts: Sf2InstrumentFacts,
}

#[derive(Clone)]
struct Chunk<'a> {
    id: [u8; 4],
    data: &'a [u8],
    offset: usize,
}

struct Riff<'a> {
    bytes: &'a [u8],
    chunks: usize,
    limit: usize,
}

impl<'a> Riff<'a> {
    fn chunk(&mut self, offset: &mut usize, end: usize) -> Result<Chunk<'a>, ProjectError> {
        self.chunks = self.chunks.saturating_add(1);
        if self.chunks > self.limit {
            return fail(format!("SoundFont chunk count exceeds explicit limit {}", self.limit));
        }
        let header_end = offset
            .checked_add(8)
            .filter(|header_end| *header_end <= end)
            .ok_or_else(|| error_at(*offset, "RIFF chunk header is truncated"))?;
        let id = self
            .bytes
            .get(*offset..offset.saturating_add(4))
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| error_at(*offset, "RIFF chunk id is truncated"))?;
        let size = u32::from_le_bytes(
            self.bytes
                .get(offset.saturating_add(4)..header_end)
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or_else(|| error_at(*offset, "RIFF chunk size is truncated"))?,
        ) as usize;
        let data_start = header_end;
        let data_end = data_start
            .checked_add(size)
            .filter(|data_end| *data_end <= end)
            .ok_or_else(|| error_at(*offset, "RIFF chunk extent exceeds its containing chunk"))?;
        let next = data_end
            .checked_add(size & 1)
            .filter(|next| *next <= end)
            .ok_or_else(|| error_at(*offset, "RIFF chunk padding exceeds its containing chunk"))?;
        let chunk = Chunk {
            id,
            data: self.bytes.get(data_start..data_end).unwrap_or_default(),
            offset: *offset,
        };
        *offset = next;
        Ok(chunk)
    }

    fn children(&mut self, list: &Chunk<'a>, kind: [u8; 4]) -> Result<Vec<Chunk<'a>>, ProjectError> {
        if list.id != *b"LIST" || list.data.get(..4) != Some(kind.as_slice()) {
            return Err(error_at(list.offset, &format!("expected LIST `{}`", fourcc(kind))));
        }
        let base = list.offset.saturating_add(12);
        let mut position = base;
        let end = list.offset.saturating_add(8).saturating_add(list.data.len());
        let mut children = Vec::new();
        while position < end {
            children.push(self.chunk(&mut position, end)?);
        }
        if position != end {
            return Err(error_at(list.offset, "LIST children do not end at the declared extent"));
        }
        Ok(children)
    }
}

#[derive(Clone)]
struct Preset {
    name: String,
    program: u16,
    bank: u16,
    bag: usize,
}

#[derive(Clone, Copy)]
struct Bag {
    generator: usize,
    modulator: usize,
}

#[derive(Clone, Copy)]
struct Generator {
    operator: u16,
    amount: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Modulator {
    source: u16,
    destination: u16,
    amount_source: u16,
    transform: u16,
    amount: i16,
}

#[derive(Clone)]
struct Instrument {
    name: String,
    bag: usize,
}

#[derive(Clone)]
struct SampleHeader {
    name: String,
    start: u32,
    end: u32,
    loop_start: u32,
    loop_end: u32,
    sample_rate: u32,
    original_key: u8,
    correction: i8,
    link: u16,
    kind: u16,
}

struct Bank<'a> {
    presets: Vec<Preset>,
    preset_bags: Vec<Bag>,
    preset_modulators: Vec<Modulator>,
    preset_generators: Vec<Generator>,
    instruments: Vec<Instrument>,
    instrument_bags: Vec<Bag>,
    instrument_modulators: Vec<Modulator>,
    instrument_generators: Vec<Generator>,
    samples: Vec<SampleHeader>,
    smpl: &'a [u8],
    sm24: Option<&'a [u8]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Range {
    low: u8,
    high: u8,
}

impl Range {
    const ALL: Self = Self { low: 0, high: 127 };

    fn intersect(self, other: Self) -> Option<Self> {
        let low = self.low.max(other.low);
        let high = self.high.min(other.high);
        (low <= high).then_some(Self { low, high })
    }
}

#[derive(Clone)]
struct Zone {
    generators: Vec<Generator>,
    modulators: Vec<Modulator>,
}

#[derive(Clone)]
struct RegionSpec {
    sample: usize,
    key: Range,
    velocity: Range,
    values: BTreeMap<u16, i32>,
    modulators: Vec<Modulator>,
}

pub(crate) fn adapt(
    instrument: &str,
    address: &str,
    bytes: &[u8],
    limits: Sf2Limits,
) -> Result<AdaptedSf2, ProjectError> {
    let (asset, selector) = split_selector(address)?;
    let bank = parse_bank(bytes, limits)?;
    let preset_index = select_preset(&bank, selector)?;
    let regions = flatten_preset(&bank, preset_index)?;
    let preset = bank
        .presets
        .get(preset_index)
        .ok_or_else(|| ProjectError::Assets("selected SoundFont preset is absent".to_owned()))?;
    if regions.is_empty() {
        return fail(format!("SoundFont preset `{}` has no audible local zone", preset.name));
    }
    let digest = format!("{:x}", Sha256::digest(bytes));
    let mut samples = BTreeMap::new();
    let mut source = String::from(
        "import std::sound::sample;\nlet imported_sf2_map: SampleMapArtifact = SampleMapArtifact {\n    schema_version = 3,\n    sample_map = SampleMap {\n",
    );
    writeln!(
        source,
        "        declaration_id = \"{}@sf2@1:{}\",",
        escape(instrument),
        digest
    )
    .map_err(|_| ProjectError::Assets("cannot construct checked SoundFont adapter source".to_owned()))?;
    source.push_str("        selection = FirstRegion,\n        voices = 64,\n        regions = [\n");
    let mut region_count = 0usize;
    for region in &regions {
        region_count = region_count.saturating_add(write_region(&mut source, &bank, region, &digest, &mut samples)?);
    }
    source.push_str("        ]\n    }\n};\npiece \"Imported SoundFont adapter result\" { meter 4/4; key c major; score { part proof { voice observed { rest/1 } } } }\n");
    Ok(AdaptedSf2 {
        source,
        samples,
        facts: Sf2InstrumentFacts {
            instrument: instrument.to_owned(),
            asset: asset.to_owned(),
            preset_name: preset.name.clone(),
            bank: preset.bank,
            program: preset.program,
            regions: region_count,
            adapter: "sf2@1",
        },
    })
}

#[derive(Clone, Copy)]
enum Selector<'a> {
    Number { bank: u16, program: u16 },
    Name(&'a str),
}

fn split_selector(address: &str) -> Result<(&str, Selector<'_>), ProjectError> {
    let (asset, fragment) = address.rsplit_once('#').ok_or_else(|| {
        ProjectError::Assets("sf2@1 requires `#preset=<bank>:<program>` or `#preset-name=<exact name>`".to_owned())
    })?;
    if asset.is_empty() || asset.contains('#') {
        return fail(format!(
            "SoundFont asset address `{address}` has a malformed preset fragment"
        ));
    }
    if let Some(number) = fragment.strip_prefix("preset=") {
        let (bank, program) = number
            .split_once(':')
            .ok_or_else(|| ProjectError::Assets("SoundFont numeric preset must be `<bank>:<program>`".to_owned()))?;
        let bank = bank
            .parse::<u16>()
            .map_err(|_| ProjectError::Assets("SoundFont bank is not an unsigned 16-bit number".to_owned()))?;
        let program = program
            .parse::<u16>()
            .map_err(|_| ProjectError::Assets("SoundFont program is not an unsigned 16-bit number".to_owned()))?;
        Ok((asset, Selector::Number { bank, program }))
    } else if let Some(name) = fragment.strip_prefix("preset-name=") {
        if name.is_empty() {
            return fail("SoundFont preset name is empty".to_owned());
        }
        Ok((asset, Selector::Name(name)))
    } else {
        fail(format!(
            "SoundFont asset address `{address}` has an unsupported fragment"
        ))
    }
}

fn select_preset(bank: &Bank<'_>, selector: Selector<'_>) -> Result<usize, ProjectError> {
    let matches: Vec<_> = bank
        .presets
        .iter()
        .take(bank.presets.len().saturating_sub(1))
        .enumerate()
        .filter(|(_, preset)| match selector {
            Selector::Number { bank, program } => preset.bank == bank && preset.program == program,
            Selector::Name(name) => preset.name == name,
        })
        .map(|(index, _)| index)
        .collect();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => fail("SoundFont preset selection matched no preset".to_owned()),
        _ => fail("SoundFont preset-name selection is ambiguous".to_owned()),
    }
}

fn parse_bank(bytes: &[u8], limits: Sf2Limits) -> Result<Bank<'_>, ProjectError> {
    if bytes.len() > limits.max_file_bytes {
        return fail(format!(
            "SoundFont has {} bytes, above the explicit limit {}",
            bytes.len(),
            limits.max_file_bytes
        ));
    }
    if bytes.get(..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"sfbk") {
        return Err(error_at(0, "expected RIFF `sfbk` SoundFont form"));
    }
    let declared = read_u32(bytes, 4)? as usize;
    if declared.checked_add(8) != Some(bytes.len()) {
        return Err(error_at(4, "RIFF size does not equal the verified bank length"));
    }
    let mut riff = Riff {
        bytes,
        chunks: 0,
        limit: limits.max_chunks,
    };
    let mut position = 12usize;
    let info = riff.chunk(&mut position, bytes.len())?;
    let sdta = riff.chunk(&mut position, bytes.len())?;
    let pdta = riff.chunk(&mut position, bytes.len())?;
    if position != bytes.len() {
        return Err(error_at(
            position,
            "SoundFont has chunks after its required three lists",
        ));
    }
    let info = riff.children(&info, *b"INFO")?;
    let version_chunk = info
        .iter()
        .find(|chunk| chunk.id == *b"ifil")
        .ok_or_else(|| error_at(12, "INFO is missing required `ifil`"))?;
    if version_chunk.data.len() != 4 {
        return Err(error_at(
            version_chunk.offset,
            "`ifil` must contain exactly one version",
        ));
    }
    let version = (read_u16(version_chunk.data, 0)?, read_u16(version_chunk.data, 2)?);
    if version.0 != 2 || version.1 > 4 {
        return Err(error_at(
            version_chunk.offset,
            "sf2@1 accepts SoundFont versions 2.00 through 2.04",
        ));
    }
    let sdta = riff.children(&sdta, *b"sdta")?;
    if !matches!(sdta.as_slice(), [first] if first.id == *b"smpl")
        && !matches!(sdta.as_slice(), [first, second] if first.id == *b"smpl" && second.id == *b"sm24")
    {
        return Err(error_at(12, "`sdta` must contain `smpl` and optional `sm24` in order"));
    }
    let smpl = sdta
        .iter()
        .find(|chunk| chunk.id == *b"smpl")
        .ok_or_else(|| error_at(12, "`sdta` is missing required sample data"))?
        .data;
    if smpl.len() % 2 != 0 {
        return Err(error_at(12, "`smpl` must contain whole signed 16-bit samples"));
    }
    let sm24 = sdta.iter().find(|chunk| chunk.id == *b"sm24").map(|chunk| chunk.data);
    if let Some(low) = sm24
        && (version.1 < 4 || low.len() != smpl.len().div_ceil(2))
    {
        return Err(error_at(
            12,
            "`sm24` does not exactly match a SoundFont 2.04 `smpl` pool",
        ));
    }
    let pdta = riff.children(&pdta, *b"pdta")?;
    let expected = [
        *b"phdr", *b"pbag", *b"pmod", *b"pgen", *b"inst", *b"ibag", *b"imod", *b"igen", *b"shdr",
    ];
    if pdta.len() != expected.len() || pdta.iter().zip(expected).any(|(chunk, id)| chunk.id != id) {
        return Err(error_at(
            12,
            "`pdta` must contain the nine Hydra chunks in specification order",
        ));
    }
    let [phdr, pbag, pmod, pgen, inst, ibag, imod, igen, shdr] = pdta.as_slice() else {
        return Err(error_at(12, "SoundFont Hydra chunk structure changed after validation"));
    };
    let presets = records(phdr, 38, limits.max_presets.saturating_add(1), |record| {
        Ok(Preset {
            name: name(record, 0)?,
            program: read_u16(record, 20)?,
            bank: read_u16(record, 22)?,
            bag: usize::from(read_u16(record, 24)?),
        })
    })?;
    let preset_bags = bags(pbag, limits.max_zones.saturating_add(1))?;
    let preset_modulators = modulators(pmod, limits.max_modulators.saturating_add(1))?;
    let preset_generators = generators(pgen, limits.max_generators.saturating_add(1))?;
    let instruments = records(inst, 22, limits.max_instruments.saturating_add(1), |record| {
        Ok(Instrument {
            name: name(record, 0)?,
            bag: usize::from(read_u16(record, 20)?),
        })
    })?;
    let instrument_bags = bags(ibag, limits.max_zones.saturating_add(1))?;
    let instrument_modulators = modulators(imod, limits.max_modulators.saturating_add(1))?;
    let instrument_generators = generators(igen, limits.max_generators.saturating_add(1))?;
    let samples = records(shdr, 46, limits.max_samples.saturating_add(1), |record| {
        Ok(SampleHeader {
            name: name(record, 0)?,
            start: read_u32(record, 20)?,
            end: read_u32(record, 24)?,
            loop_start: read_u32(record, 28)?,
            loop_end: read_u32(record, 32)?,
            sample_rate: read_u32(record, 36)?,
            original_key: match *record
                .get(40)
                .ok_or_else(|| error_at(40, "sample root key is absent"))?
            {
                key @ 0..=127 => key,
                128..=255 => 60,
            },
            correction: *record
                .get(41)
                .ok_or_else(|| error_at(41, "sample correction is absent"))? as i8,
            link: read_u16(record, 42)?,
            kind: read_u16(record, 44)?,
        })
    })?;
    validate_terminal_counts(
        &presets,
        &preset_bags,
        &preset_generators,
        &preset_modulators,
        &instruments,
        &instrument_bags,
        &instrument_generators,
        &instrument_modulators,
        &samples,
        limits.max_zones,
        smpl.len() / 2,
    )?;
    let generator_count = preset_generators
        .len()
        .saturating_sub(1)
        .saturating_add(instrument_generators.len().saturating_sub(1));
    if generator_count > limits.max_generators {
        return fail(format!(
            "SoundFont generator count {generator_count} exceeds explicit limit {}",
            limits.max_generators
        ));
    }
    let modulator_count = preset_modulators
        .len()
        .saturating_sub(1)
        .saturating_add(instrument_modulators.len().saturating_sub(1));
    if modulator_count > limits.max_modulators {
        return fail(format!(
            "SoundFont modulator count {modulator_count} exceeds explicit limit {}",
            limits.max_modulators
        ));
    }
    let pcm_bytes = smpl.len().saturating_add(sm24.map_or(0, <[u8]>::len));
    if pcm_bytes > limits.max_pcm_bytes {
        return fail(format!(
            "SoundFont embedded PCM requires {pcm_bytes} bytes, above the explicit limit {}",
            limits.max_pcm_bytes
        ));
    }
    Ok(Bank {
        presets,
        preset_bags,
        preset_modulators,
        preset_generators,
        instruments,
        instrument_bags,
        instrument_modulators,
        instrument_generators,
        samples,
        smpl,
        sm24,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_terminal_counts(
    presets: &[Preset],
    preset_bags: &[Bag],
    preset_generators: &[Generator],
    preset_modulators: &[Modulator],
    instruments: &[Instrument],
    instrument_bags: &[Bag],
    instrument_generators: &[Generator],
    instrument_modulators: &[Modulator],
    samples: &[SampleHeader],
    max_zones: usize,
    sample_points: usize,
) -> Result<(), ProjectError> {
    if presets.len() < 2 || instruments.len() < 2 || samples.len() < 2 {
        return fail("SoundFont requires at least one object plus each terminal Hydra record".to_owned());
    }
    if preset_bags.is_empty()
        || preset_generators.is_empty()
        || preset_modulators.is_empty()
        || instrument_bags.is_empty()
        || instrument_generators.is_empty()
        || instrument_modulators.is_empty()
    {
        return fail("SoundFont Hydra bag, generator, and modulator chunks require terminal records".to_owned());
    }
    if presets.last().is_none_or(|preset| preset.name != "EOP")
        || instruments.last().is_none_or(|instrument| instrument.name != "EOI")
        || samples.last().is_none_or(|sample| sample.name != "EOS")
    {
        return fail("SoundFont terminal records must be named EOP, EOI, and EOS".to_owned());
    }
    let Some(last_preset_bag) = preset_bags.len().checked_sub(1) else {
        return fail("SoundFont preset bags are absent".to_owned());
    };
    let Some(last_instrument_bag) = instrument_bags.len().checked_sub(1) else {
        return fail("SoundFont instrument bags are absent".to_owned());
    };
    if presets.last().is_none_or(|preset| preset.bag != last_preset_bag)
        || instruments
            .last()
            .is_none_or(|instrument| instrument.bag != last_instrument_bag)
    {
        return fail("SoundFont terminal object indices do not end at the terminal bags".to_owned());
    }
    let Some(last_preset_generator) = preset_generators.len().checked_sub(1) else {
        return fail("SoundFont preset generators are absent".to_owned());
    };
    let Some(last_preset_modulator) = preset_modulators.len().checked_sub(1) else {
        return fail("SoundFont preset modulators are absent".to_owned());
    };
    let Some(last_instrument_generator) = instrument_generators.len().checked_sub(1) else {
        return fail("SoundFont instrument generators are absent".to_owned());
    };
    let Some(last_instrument_modulator) = instrument_modulators.len().checked_sub(1) else {
        return fail("SoundFont instrument modulators are absent".to_owned());
    };
    if preset_bags
        .last()
        .is_none_or(|bag| bag.generator != last_preset_generator || bag.modulator != last_preset_modulator)
        || instrument_bags
            .last()
            .is_none_or(|bag| bag.generator != last_instrument_generator || bag.modulator != last_instrument_modulator)
    {
        return fail("SoundFont terminal bag indices do not end at terminal generator/modulator records".to_owned());
    }
    validate_monotonic(presets.iter().map(|preset| preset.bag), preset_bags.len(), "preset bag")?;
    validate_monotonic(
        instruments.iter().map(|instrument| instrument.bag),
        instrument_bags.len(),
        "instrument bag",
    )?;
    validate_bags(preset_bags, preset_generators.len(), preset_modulators.len(), "preset")?;
    validate_bags(
        instrument_bags,
        instrument_generators.len(),
        instrument_modulators.len(),
        "instrument",
    )?;
    let zones = last_preset_bag.saturating_add(last_instrument_bag);
    if zones > max_zones {
        return fail(format!(
            "SoundFont zone count {zones} exceeds explicit limit {max_zones}"
        ));
    }
    let pool = u32::try_from(sample_points).unwrap_or(u32::MAX);
    for (index, sample) in samples.iter().take(samples.len().saturating_sub(1)).enumerate() {
        if sample.start >= sample.end
            || sample.end > pool
            || sample.loop_start < sample.start
            || sample.loop_start >= sample.loop_end
            || sample.loop_end > sample.end
            || sample.sample_rate == 0
        {
            return fail(format!(
                "SoundFont sample header {index} has invalid bounds, rate, loop, or root key"
            ));
        }
        let base_kind = sample.kind & 0x7fff;
        if sample.kind & 0x8000 != 0 || !matches!(base_kind, 1 | 2 | 4) {
            return fail(format!(
                "SoundFont sample header {index} has unsupported ROM/linked type {}",
                sample.kind
            ));
        }
        if matches!(base_kind, 2 | 4) {
            let linked = samples.get(usize::from(sample.link)).ok_or_else(|| {
                ProjectError::Assets(format!("SoundFont sample header {index} has an absent stereo link"))
            })?;
            let expected = if base_kind == 2 { 4 } else { 2 };
            if linked.kind & 0x7fff != expected || usize::from(linked.link) != index {
                return fail(format!(
                    "SoundFont sample header {index} does not have a reciprocal stereo link"
                ));
            }
        }
    }
    Ok(())
}

fn validate_monotonic(values: impl IntoIterator<Item = usize>, bound: usize, name: &str) -> Result<(), ProjectError> {
    let mut previous = 0usize;
    for (index, value) in values.into_iter().enumerate() {
        if value < previous || value >= bound {
            return fail(format!("SoundFont {name} index {index} is not monotonic and in range"));
        }
        previous = value;
    }
    Ok(())
}

fn validate_bags(
    bags: &[Bag],
    generator_bound: usize,
    modulator_bound: usize,
    level: &str,
) -> Result<(), ProjectError> {
    validate_monotonic(
        bags.iter().map(|bag| bag.generator),
        generator_bound,
        &format!("{level} generator"),
    )?;
    validate_monotonic(
        bags.iter().map(|bag| bag.modulator),
        modulator_bound,
        &format!("{level} modulator"),
    )
}

fn object_zones(
    bags: &[Bag],
    generators: &[Generator],
    modulators: &[Modulator],
    start: usize,
    end: usize,
) -> Result<Vec<Zone>, ProjectError> {
    let mut zones = Vec::with_capacity(end.saturating_sub(start));
    for bag_index in start..end {
        let bag = bags
            .get(bag_index)
            .ok_or_else(|| ProjectError::Assets("SoundFont object references an absent bag".to_owned()))?;
        let next = bags
            .get(bag_index.saturating_add(1))
            .ok_or_else(|| ProjectError::Assets("SoundFont object zone has no terminal bag".to_owned()))?;
        zones.push(Zone {
            generators: generators
                .get(bag.generator..next.generator)
                .ok_or_else(|| ProjectError::Assets("SoundFont zone generator range is invalid".to_owned()))?
                .to_vec(),
            modulators: modulators
                .get(bag.modulator..next.modulator)
                .ok_or_else(|| ProjectError::Assets("SoundFont zone modulator range is invalid".to_owned()))?
                .iter()
                .copied()
                .filter(|modulator| !is_zero_modulator(*modulator))
                .collect(),
        });
    }
    Ok(zones)
}

fn is_zero_modulator(modulator: Modulator) -> bool {
    modulator.source == 0
        && modulator.destination == 0
        && modulator.amount_source == 0
        && modulator.transform == 0
        && modulator.amount == 0
}

fn flatten_preset(bank: &Bank<'_>, preset_index: usize) -> Result<Vec<RegionSpec>, ProjectError> {
    let preset = bank
        .presets
        .get(preset_index)
        .ok_or_else(|| ProjectError::Assets("selected SoundFont preset is absent".to_owned()))?;
    let preset_end = bank
        .presets
        .get(preset_index.saturating_add(1))
        .ok_or_else(|| ProjectError::Assets("selected SoundFont preset has no terminal record".to_owned()))?
        .bag;
    let zones = object_zones(
        &bank.preset_bags,
        &bank.preset_generators,
        &bank.preset_modulators,
        preset.bag,
        preset_end,
    )?;
    let (preset_global, preset_locals) = split_global(zones, 41, "preset")?;
    let mut output = Vec::new();
    for local in preset_locals {
        validate_zone_order(&local.generators, 41, "preset")?;
        let instrument_index = terminal_index(&local.generators, 41, "preset")?;
        let instrument = bank.instruments.get(instrument_index).ok_or_else(|| {
            ProjectError::Assets(format!(
                "SoundFont preset references absent instrument {instrument_index}"
            ))
        })?;
        let instrument_end = bank
            .instruments
            .get(instrument_index.saturating_add(1))
            .ok_or_else(|| ProjectError::Assets("SoundFont instrument has no terminal record".to_owned()))?
            .bag;
        let instrument_zones = object_zones(
            &bank.instrument_bags,
            &bank.instrument_generators,
            &bank.instrument_modulators,
            instrument.bag,
            instrument_end,
        )?;
        let (instrument_global, instrument_locals) = split_global(instrument_zones, 53, "instrument")?;
        let preset_values = combined_preset_values(preset_global.as_ref(), &local)?;
        let preset_key = combined_range(preset_global.as_ref(), &local, 43)?;
        let preset_velocity = combined_range(preset_global.as_ref(), &local, 44)?;
        let preset_modulators = combined_zone_modulators(preset_global.as_ref(), &local);
        let mut instrument_output = Vec::new();
        for instrument_local in instrument_locals {
            validate_zone_order(&instrument_local.generators, 53, "instrument")?;
            let sample = terminal_index(&instrument_local.generators, 53, "instrument")?;
            if sample >= bank.samples.len().saturating_sub(1) {
                return fail(format!("SoundFont instrument references absent sample {sample}"));
            }
            let instrument_key = combined_range(instrument_global.as_ref(), &instrument_local, 43)?;
            let instrument_velocity = combined_range(instrument_global.as_ref(), &instrument_local, 44)?;
            let Some(key) = preset_key.intersect(instrument_key) else {
                continue;
            };
            let Some(velocity) = preset_velocity.intersect(instrument_velocity) else {
                continue;
            };
            let mut values = defaults();
            apply_instrument_values(&mut values, instrument_global.as_ref())?;
            apply_instrument_values(&mut values, Some(&instrument_local))?;
            for (operator, amount) in &preset_values {
                let value = values.entry(*operator).or_insert(0);
                *value = value.checked_add(*amount).ok_or_else(|| {
                    ProjectError::Assets(format!(
                        "SoundFont generator {} overflows while combining hierarchy",
                        generator_name(*operator)
                    ))
                })?;
            }
            validate_effective_values(&values)?;
            let instrument_modulators = combined_zone_modulators(instrument_global.as_ref(), &instrument_local);
            let modulators =
                add_modulator_levels(default_modulators(), instrument_modulators, preset_modulators.clone());
            validate_modulators(&modulators)?;
            instrument_output.push(RegionSpec {
                sample,
                key,
                velocity,
                values,
                modulators,
            });
        }
        normalize_stereo_regions(bank, &mut instrument_output)?;
        output.extend(instrument_output);
    }
    Ok(output)
}

fn normalize_stereo_regions(bank: &Bank<'_>, regions: &mut [RegionSpec]) -> Result<(), ProjectError> {
    for index in 0..regions.len() {
        let region = regions
            .get(index)
            .ok_or_else(|| ProjectError::Assets("SoundFont stereo region is absent".to_owned()))?;
        let sample_index = region.sample;
        let key = region.key;
        let velocity = region.velocity;
        let sample = bank
            .samples
            .get(sample_index)
            .ok_or_else(|| ProjectError::Assets("SoundFont stereo region sample is absent".to_owned()))?;
        if !matches!(sample.kind & 0x7fff, 2 | 4) {
            continue;
        }
        let partner = regions.iter().position(|candidate| {
            candidate.sample == usize::from(sample.link) && candidate.key == key && candidate.velocity == velocity
        });
        let partner = partner.ok_or_else(|| {
            ProjectError::Assets(format!(
                "SoundFont stereo sample {sample_index} lacks its synchronized partner in the same instrument ranges"
            ))
        })?;
        if sample.kind & 0x7fff == 4 {
            for operator in [51, 52, 58] {
                let right = regions
                    .get(partner)
                    .and_then(|region| region.values.get(&operator))
                    .copied();
                let left = regions
                    .get_mut(index)
                    .ok_or_else(|| ProjectError::Assets("SoundFont left stereo region is absent".to_owned()))?;
                match right {
                    Some(value) => {
                        left.values.insert(operator, value);
                    }
                    None => {
                        left.values.remove(&operator);
                    }
                }
            }
        }
    }
    Ok(())
}

fn split_global(mut zones: Vec<Zone>, terminal: u16, level: &str) -> Result<(Option<Zone>, Vec<Zone>), ProjectError> {
    let global = zones
        .first()
        .is_some_and(|zone| !zone.generators.iter().any(|generator| generator.operator == terminal));
    let global = global.then(|| zones.remove(0));
    if let Some(global) = &global {
        validate_global_zone_order(&global.generators, level)?;
    }
    if zones.is_empty() {
        return fail(format!("SoundFont {level} declares no local zone"));
    }
    Ok((global, zones))
}

fn validate_global_zone_order(generators: &[Generator], level: &str) -> Result<(), ProjectError> {
    for (index, generator) in generators.iter().enumerate() {
        if generator.operator == 43 && index != 0 {
            return fail(format!("SoundFont {level} global keyRange is not first"));
        }
        if generator.operator == 44
            && generators
                .get(..index)
                .is_none_or(|before| before.iter().any(|row| row.operator != 43))
        {
            return fail(format!(
                "SoundFont {level} global velRange is not first or immediately after keyRange"
            ));
        }
    }
    Ok(())
}

fn validate_zone_order(generators: &[Generator], terminal: u16, level: &str) -> Result<(), ProjectError> {
    let terminal_position = generators.iter().rposition(|generator| generator.operator == terminal);
    if terminal_position != generators.len().checked_sub(1) {
        return fail(format!(
            "SoundFont {level} local zone does not end with {}",
            generator_name(terminal)
        ));
    }
    for (index, generator) in generators.iter().enumerate() {
        if generator.operator == 43 && index != 0 {
            return fail(format!("SoundFont {level} keyRange is not first"));
        }
        if generator.operator == 44
            && generators
                .get(..index)
                .is_none_or(|before| before.iter().any(|row| row.operator != 43))
        {
            return fail(format!(
                "SoundFont {level} velRange is not first or immediately after keyRange"
            ));
        }
        if generator.operator == terminal && index.checked_add(1) != Some(generators.len()) {
            return fail(format!("SoundFont {level} index generator is not terminal"));
        }
    }
    Ok(())
}

fn terminal_index(generators: &[Generator], operator: u16, level: &str) -> Result<usize, ProjectError> {
    generators
        .last()
        .filter(|generator| generator.operator == operator)
        .map(|generator| usize::from(generator.amount))
        .ok_or_else(|| {
            ProjectError::Assets(format!(
                "SoundFont {level} local zone lacks {}",
                generator_name(operator)
            ))
        })
}

fn combined_range(global: Option<&Zone>, local: &Zone, operator: u16) -> Result<Range, ProjectError> {
    let global = global.and_then(|zone| last_generator(&zone.generators, operator));
    let local = last_generator(&local.generators, operator);
    match local.or(global) {
        Some(generator) => decode_range(generator.amount, generator_name(operator)),
        None => Ok(Range::ALL),
    }
}

fn decode_range(amount: u16, name: &str) -> Result<Range, ProjectError> {
    let low = (amount & 0xff) as u8;
    let high = (amount >> 8) as u8;
    if low > high || high > 127 {
        return fail(format!("SoundFont {name} is not an ordered 0..127 range"));
    }
    Ok(Range { low, high })
}

fn last_generator(generators: &[Generator], operator: u16) -> Option<Generator> {
    generators
        .iter()
        .rev()
        .find(|generator| generator.operator == operator)
        .copied()
}

fn defaults() -> BTreeMap<u16, i32> {
    [
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 0),
        (4, 0),
        (8, 13_500),
        (9, 0),
        (12, 0),
        (17, 0),
        (33, -12_000),
        (34, -12_000),
        (35, -12_000),
        (36, -12_000),
        (37, 0),
        (38, -12_000),
        (39, 0),
        (40, 0),
        (45, 0),
        (48, 0),
        (50, 0),
        (51, 0),
        (52, 0),
        (54, 0),
        (56, 100),
        (57, 0),
        (58, -1),
    ]
    .into_iter()
    .collect()
}

fn combined_preset_values(global: Option<&Zone>, local: &Zone) -> Result<BTreeMap<u16, i32>, ProjectError> {
    let mut values = BTreeMap::new();
    apply_preset_values(&mut values, global)?;
    apply_preset_values(&mut values, Some(local))?;
    Ok(values)
}

fn apply_preset_values(values: &mut BTreeMap<u16, i32>, zone: Option<&Zone>) -> Result<(), ProjectError> {
    let Some(zone) = zone else {
        return Ok(());
    };
    for generator in &zone.generators {
        match generator.operator {
            14 | 18..=20 | 42 | 49 | 55 | 59 | 60 | 41 | 43 | 44 => {}
            8 | 9 | 17 | 33..=40 | 48 | 51 | 52 => {
                values.insert(generator.operator, signed(generator.amount));
            }
            5..=7 | 10..=11 | 13 | 15..=16 | 21..=32 => {
                let amount = signed(generator.amount);
                if amount != 0 {
                    return unsupported_generator(generator.operator, amount);
                }
            }
            0..=4 | 12 | 45..=47 | 50 | 53..=54 | 56..=58 => {
                return fail(format!(
                    "SoundFont generator {} is illegal at preset level",
                    generator_name(generator.operator)
                ));
            }
            operator => return fail(format!("SoundFont has unknown generator {operator}")),
        }
    }
    Ok(())
}

fn apply_instrument_values(values: &mut BTreeMap<u16, i32>, zone: Option<&Zone>) -> Result<(), ProjectError> {
    let Some(zone) = zone else {
        return Ok(());
    };
    for generator in &zone.generators {
        match generator.operator {
            14 | 18..=20 | 42 | 49 | 55 | 59 | 60 | 43 | 44 | 53 => {}
            0..=4 | 8..=9 | 12 | 17 | 33..=40 | 45 | 48 | 50..=52 | 54 | 56..=58 => {
                values.insert(generator.operator, signed(generator.amount));
            }
            46 | 47 => {
                let amount = signed(generator.amount);
                if amount != -1 {
                    return unsupported_generator(generator.operator, amount);
                }
            }
            5..=7 | 10..=11 | 13 | 15..=16 | 21..=32 => {
                let amount = signed(generator.amount);
                let harmless_default =
                    matches!(generator.operator, 21 | 23 | 25..=28 | 30 | 33..=36 | 38) && amount == -12_000;
                if amount != 0 && !harmless_default {
                    return unsupported_generator(generator.operator, amount);
                }
            }
            41 => return fail("SoundFont instrument generator is illegal at instrument level".to_owned()),
            operator => return fail(format!("SoundFont has unknown generator {operator}")),
        }
    }
    Ok(())
}

fn validate_effective_values(values: &BTreeMap<u16, i32>) -> Result<(), ProjectError> {
    let scale = value(values, 56);
    if scale != 100 {
        return fail(format!(
            "SoundFont generator scaleTuning has unsupported effective value {scale}"
        ));
    }
    let pan = value(values, 17);
    let attenuation = value(values, 48);
    let resonance = value(values, 9);
    let sustain = value(values, 37);
    let sample_mode = value(values, 54);
    let exclusive = value(values, 57);
    let root = value(values, 58);
    if !(-500..=500).contains(&pan)
        || !(0..=1_440).contains(&attenuation)
        || !(0..=960).contains(&resonance)
        || !(0..=1_440).contains(&sustain)
        || !matches!(sample_mode & 3, 0..=3)
        || sample_mode & !3 != 0
        || !(0..=127).contains(&exclusive)
        || !(-1..=127).contains(&root)
    {
        return fail("SoundFont effective generator value is outside the 2.04 range".to_owned());
    }
    Ok(())
}

fn value(values: &BTreeMap<u16, i32>, operator: u16) -> i32 {
    values.get(&operator).copied().unwrap_or(0)
}

fn signed(amount: u16) -> i32 {
    i32::from(amount as i16)
}

fn unsupported_generator<T>(operator: u16, amount: i32) -> Result<T, ProjectError> {
    fail(format!(
        "SoundFont generator {} has unsupported sound-changing value {amount}",
        generator_name(operator)
    ))
}

fn generator_name(operator: u16) -> &'static str {
    match operator {
        0 => "startAddrsOffset",
        1 => "endAddrsOffset",
        2 => "startloopAddrsOffset",
        3 => "endloopAddrsOffset",
        4 => "startAddrsCoarseOffset",
        5 => "modLfoToPitch",
        6 => "vibLfoToPitch",
        7 => "modEnvToPitch",
        8 => "initialFilterFc",
        9 => "initialFilterQ",
        10 => "modLfoToFilterFc",
        11 => "modEnvToFilterFc",
        12 => "endAddrsCoarseOffset",
        13 => "modLfoToVolume",
        14 => "unused1",
        15 => "chorusEffectsSend",
        16 => "reverbEffectsSend",
        17 => "pan",
        18 => "unused2",
        19 => "unused3",
        20 => "unused4",
        21 => "delayModLFO",
        22 => "freqModLFO",
        23 => "delayVibLFO",
        24 => "freqVibLFO",
        25 => "delayModEnv",
        26 => "attackModEnv",
        27 => "holdModEnv",
        28 => "decayModEnv",
        29 => "sustainModEnv",
        30 => "releaseModEnv",
        31 => "keynumToModEnvHold",
        32 => "keynumToModEnvDecay",
        33 => "delayVolEnv",
        34 => "attackVolEnv",
        35 => "holdVolEnv",
        36 => "decayVolEnv",
        37 => "sustainVolEnv",
        38 => "releaseVolEnv",
        39 => "keynumToVolEnvHold",
        40 => "keynumToVolEnvDecay",
        41 => "instrument",
        42 => "reserved1",
        43 => "keyRange",
        44 => "velRange",
        45 => "startloopAddrsCoarseOffset",
        46 => "keynum",
        47 => "velocity",
        48 => "initialAttenuation",
        49 => "reserved2",
        50 => "endloopAddrsCoarseOffset",
        51 => "coarseTune",
        52 => "fineTune",
        53 => "sampleID",
        54 => "sampleModes",
        55 => "reserved3",
        56 => "scaleTuning",
        57 => "exclusiveClass",
        58 => "overridingRootKey",
        59 => "unused5",
        60 => "endOper",
        _ => "unknown",
    }
}

type ModulatorKey = (u16, u16, u16, u16);

fn modulator_key(modulator: Modulator) -> ModulatorKey {
    (
        modulator.source,
        modulator.destination,
        modulator.amount_source,
        modulator.transform,
    )
}

fn combined_zone_modulators(global: Option<&Zone>, local: &Zone) -> BTreeMap<ModulatorKey, Modulator> {
    let mut combined = BTreeMap::new();
    if let Some(global) = global {
        for modulator in &global.modulators {
            combined.insert(modulator_key(*modulator), *modulator);
        }
    }
    for modulator in &local.modulators {
        combined.insert(modulator_key(*modulator), *modulator);
    }
    combined
}

fn default_modulators() -> BTreeMap<ModulatorKey, Modulator> {
    // The two implicit note-on-velocity routes admitted by sf2@1. Other
    // SoundFont defaults depend on ambient MIDI state absent from Musa's
    // typed note-instrument signature and are intentionally outside sf2@1.
    let velocity_attenuation = Modulator {
        source: 0x0502,
        destination: 48,
        amount_source: 0,
        transform: 0,
        amount: 960,
    };
    let velocity_filter = Modulator {
        source: 0x0102,
        destination: 8,
        amount_source: 0,
        transform: 0,
        amount: -2_400,
    };
    [velocity_attenuation, velocity_filter]
        .into_iter()
        .map(|modulator| (modulator_key(modulator), modulator))
        .collect()
}

fn add_modulator_levels(
    mut defaults: BTreeMap<ModulatorKey, Modulator>,
    instrument: BTreeMap<ModulatorKey, Modulator>,
    preset: BTreeMap<ModulatorKey, Modulator>,
) -> Vec<Modulator> {
    for (key, modulator) in instrument {
        defaults.insert(key, modulator);
    }
    for (key, modulator) in preset {
        defaults
            .entry(key)
            .and_modify(|existing| existing.amount = existing.amount.saturating_add(modulator.amount))
            .or_insert(modulator);
    }
    defaults
        .into_values()
        .filter(|modulator| modulator.amount != 0)
        .collect()
}

fn validate_modulators(modulators: &[Modulator]) -> Result<(), ProjectError> {
    for (index, modulator) in modulators.iter().enumerate() {
        if modulator.destination & 0x8000 != 0 {
            return fail(format!(
                "SoundFont modulator {index} has unsupported linked destination"
            ));
        }
        if modulator.amount_source != 0 {
            return fail(format!(
                "SoundFont modulator {index} has unsupported secondary controller"
            ));
        }
        if !matches!(modulator.transform, 0 | 2) {
            return fail(format!(
                "SoundFont modulator {index} has unknown transform {}",
                modulator.transform
            ));
        }
        if modulator.source & 0x0080 != 0 {
            return fail(format!("SoundFont modulator {index} depends on ambient MIDI CC state"));
        }
        let source = modulator.source & 0x007f;
        if !matches!(source, 2 | 3) {
            return fail(format!("SoundFont modulator {index} has unsupported source {source}"));
        }
        let curve = modulator.source >> 10;
        if curve > 3 {
            return fail(format!("SoundFont modulator {index} has unknown source curve {curve}"));
        }
        if !matches!(modulator.destination, 8 | 17 | 48 | 51 | 52) {
            return fail(format!(
                "SoundFont modulator {index} targets unsupported {}",
                generator_name(modulator.destination)
            ));
        }
    }
    Ok(())
}

fn write_region(
    source: &mut String,
    bank: &Bank<'_>,
    region: &RegionSpec,
    digest: &str,
    samples: &mut BTreeMap<String, Arc<[u8]>>,
) -> Result<usize, ProjectError> {
    let header = bank
        .samples
        .get(region.sample)
        .ok_or_else(|| ProjectError::Assets("SoundFont region sample header is absent".to_owned()))?;
    if !matches!(header.kind & 0x7fff, 1 | 2 | 4) {
        return fail(format!(
            "SoundFont sample {} has unsupported type {}",
            region.sample, header.kind
        ));
    }
    let sample_index = region.sample;
    {
        let sample = header;
        let logical = format!("embedded:sf2:sha256:{digest}:{sample_index}.wav");
        if !samples.contains_key(&logical) {
            samples.insert(logical.clone(), encode_sample(bank, sample)?);
        }
        let bounds = region_bounds(sample, &region.values)?;
        let root = match value(&region.values, 58) {
            -1 => sample.original_key,
            root => u8::try_from(root)
                .map_err(|_| ProjectError::Assets("SoundFont overridingRootKey is invalid".to_owned()))?,
        };
        let tune = value(&region.values, 51)
            .checked_mul(100)
            .and_then(|coarse| coarse.checked_add(value(&region.values, 52)))
            .and_then(|tune| tune.checked_add(i32::from(sample.correction)))
            .ok_or_else(|| ProjectError::Assets("SoundFont tuning overflows".to_owned()))?;
        let pan = value(&region.values, 17);
        let attenuation = value(&region.values, 48);
        let mode = match value(&region.values, 54) & 3 {
            0 | 2 => "NoLoop",
            1 => "ForwardContinuousLoop",
            3 => "ForwardSustainLoop",
            _ => return fail("SoundFont sample mode is outside its two-bit domain".to_owned()),
        };
        let exclusive = value(&region.values, 57);
        let tune = rational(tune, 1);
        let gain = rational(
            attenuation
                .checked_neg()
                .ok_or_else(|| ProjectError::Assets("SoundFont attenuation cannot be negated".to_owned()))?,
            10,
        );
        let pan = rational(pan, 500);
        let delay = rational(value(&region.values, 33), 1);
        let attack = rational(value(&region.values, 34), 1);
        let hold = rational(value(&region.values, 35), 1);
        let decay = rational(value(&region.values, 36), 1);
        let sustain = rational(value(&region.values, 37), 1);
        let release = rational(value(&region.values, 38), 1);
        let hold_key = rational(value(&region.values, 39), 1);
        let decay_key = rational(value(&region.values, 40), 1);
        let cutoff = rational(value(&region.values, 8), 1);
        let resonance = rational(value(&region.values, 9), 1);
        writeln!(
            source,
            "            SampleRegion {{ asset = \"{}\", key_low = {}, key_high = {}, root_key = {root}, expression_low = {}/127, expression_high = {}/127, technique = \"\", connection = AnyConnection, trigger = AttackTrigger, pedal = AnyPedal, sequence_group = 0, sequence_position = 0, sequence_length = 0, weight = 1, priority = 0, tune_cents = {tune}, gain = DecibelGain({gain}), expression_gain = ConstantExpressionGain, pan = {pan}, start_frame = {}, end_frame = {}, loop_start = {}, loop_end = {}, loop_mode = {mode}, envelope = SampleEnvelope {{ delay = SoundFontTimecents({delay}), attack = SoundFontTimecents({attack}), hold = SoundFontTimecents({hold}), decay = SoundFontTimecents({decay}), sustain = SoundFontAttenuationCentibels({sustain}), release = SoundFontTimecents({release}), hold_key_timecents = {hold_key}, decay_key_timecents = {decay_key}, curve = SoundFont2Envelope }}, filter = SampleFilter {{ cutoff_cents = {cutoff}, resonance_centibels = {resonance} }}, modulations = [{}], group = {exclusive}, off_by = {exclusive}, off_mode = FastOff }},",
            escape(&logical),
            region.key.low,
            region.key.high,
            region.velocity.low,
            region.velocity.high,
            bounds.0,
            bounds.1,
            bounds.2,
            bounds.3,
            write_modulations(&region.modulators)?,
        )
        .map_err(|_| ProjectError::Assets("cannot construct checked SoundFont region source".to_owned()))?;
    }
    Ok(1)
}

fn region_bounds(sample: &SampleHeader, values: &BTreeMap<u16, i32>) -> Result<(u64, u64, u64, u64), ProjectError> {
    let length = i64::from(sample.end.saturating_sub(sample.start));
    let start = address_offset(values, 0, 4)?;
    let end = length
        .checked_add(address_offset(values, 1, 12)?)
        .ok_or_else(|| ProjectError::Assets("SoundFont end offset overflows".to_owned()))?;
    let loop_start = i64::from(sample.loop_start.saturating_sub(sample.start))
        .checked_add(address_offset(values, 2, 45)?)
        .ok_or_else(|| ProjectError::Assets("SoundFont loop-start offset overflows".to_owned()))?;
    let loop_end = i64::from(sample.loop_end.saturating_sub(sample.start))
        .checked_add(address_offset(values, 3, 50)?)
        .ok_or_else(|| ProjectError::Assets("SoundFont loop-end offset overflows".to_owned()))?;
    if start < 0 || start >= end || end > length {
        return fail(format!(
            "SoundFont sample `{}` address offsets escape its header",
            sample.name
        ));
    }
    let loops = matches!(value(values, 54) & 3, 1 | 3);
    if loops && (loop_start < start || loop_start >= loop_end || loop_end > end) {
        return fail(format!(
            "SoundFont sample `{}` loop offsets escape its playback interval",
            sample.name
        ));
    }
    Ok((
        start as u64,
        end as u64,
        if loops { loop_start as u64 } else { 0 },
        if loops { loop_end as u64 } else { 0 },
    ))
}

fn address_offset(values: &BTreeMap<u16, i32>, fine: u16, coarse: u16) -> Result<i64, ProjectError> {
    i64::from(value(values, coarse))
        .checked_mul(32_768)
        .and_then(|coarse| coarse.checked_add(i64::from(value(values, fine))))
        .ok_or_else(|| ProjectError::Assets("SoundFont sample address offset overflows".to_owned()))
}

fn encode_sample(bank: &Bank<'_>, sample: &SampleHeader) -> Result<Arc<[u8]>, ProjectError> {
    let start = usize::try_from(sample.start).unwrap_or(usize::MAX);
    let end = usize::try_from(sample.end).unwrap_or(usize::MAX);
    let mut cursor = Cursor::new(Vec::new());
    let bits = if bank.sm24.is_some() { 24 } else { 16 };
    {
        let mut writer = hound::WavWriter::new(
            &mut cursor,
            hound::WavSpec {
                channels: 1,
                sample_rate: sample.sample_rate,
                bits_per_sample: bits,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .map_err(|error| ProjectError::Assets(format!("cannot frame embedded SoundFont sample: {error}")))?;
        for index in start..end {
            let high_offset = index.saturating_mul(2);
            let high = bank
                .smpl
                .get(high_offset..high_offset.saturating_add(2))
                .and_then(|bytes| bytes.try_into().ok())
                .map(i16::from_le_bytes)
                .ok_or_else(|| ProjectError::Assets("SoundFont sample data is truncated".to_owned()))?;
            if let Some(low) = bank.sm24 {
                let low = i32::from(
                    *low.get(index)
                        .ok_or_else(|| ProjectError::Assets("SoundFont 24-bit sample data is truncated".to_owned()))?,
                );
                writer
                    .write_sample((i32::from(high) << 8) | low)
                    .map_err(|error| ProjectError::Assets(format!("cannot encode embedded sample: {error}")))?;
            } else {
                writer
                    .write_sample(high)
                    .map_err(|error| ProjectError::Assets(format!("cannot encode embedded sample: {error}")))?;
            }
        }
        writer
            .finalize()
            .map_err(|error| ProjectError::Assets(format!("cannot finalize embedded sample: {error}")))?;
    }
    Ok(cursor.into_inner().into())
}

fn write_modulations(modulators: &[Modulator]) -> Result<String, ProjectError> {
    let written = modulators
        .iter()
        .map(|modulator| -> Result<String, ProjectError> {
            let source = match modulator.source & 0x007f {
                2 => "ExpressionModulationSource",
                3 => "KeyModulationSource",
                source => return fail(format!("SoundFont modulator has unsupported source {source}")),
            };
            let direction = if modulator.source & 0x0100 == 0 {
                "PositiveModulationDirection"
            } else {
                "NegativeModulationDirection"
            };
            let polarity = if modulator.source & 0x0200 == 0 {
                "UnipolarModulation"
            } else {
                "BipolarModulation"
            };
            let curve = match modulator.source >> 10 {
                0 => "LinearModulationCurve",
                1 => "ConcaveModulationCurve",
                2 => "ConvexModulationCurve",
                3 => "SwitchModulationCurve",
                curve => return fail(format!("SoundFont modulator has unknown source curve {curve}")),
            };
            let transform = if modulator.transform == 0 {
                "LinearModulationTransform"
            } else {
                "AbsoluteModulationTransform"
            };
            let (target, numerator, denominator) = match modulator.destination {
                8 => ("FilterCutoffCentsTarget", i32::from(modulator.amount), 1),
                17 => ("PanTarget", i32::from(modulator.amount), 500),
                48 => (
                    "GainDecibelsTarget",
                    i32::from(modulator.amount)
                        .checked_neg()
                        .ok_or_else(|| ProjectError::Assets("SoundFont modulation amount cannot be negated".to_owned()))?,
                    10,
                ),
                51 => (
                    "TuneCentsTarget",
                    i32::from(modulator.amount)
                        .checked_mul(100)
                        .ok_or_else(|| ProjectError::Assets("SoundFont coarse-tune modulation overflows".to_owned()))?,
                    1,
                ),
                52 => ("TuneCentsTarget", i32::from(modulator.amount), 1),
                destination => {
                    return fail(format!(
                        "SoundFont modulator targets unsupported {}",
                        generator_name(destination)
                    ));
                }
            };
            let amount = rational(numerator, denominator);
            Ok(format!(
                "SampleModulation {{ source = {source}, source_maximum = 127/128, direction = {direction}, polarity = {polarity}, curve = {curve}, transform = {transform}, target = {target}, amount = {amount} }}"
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(written.join(", "))
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn rational(numerator: i32, denominator: i32) -> String {
    if numerator < 0 {
        format!("(0/1) - {}/{denominator}", numerator.unsigned_abs())
    } else {
        format!("{numerator}/{denominator}")
    }
}

fn records<T>(
    chunk: &Chunk<'_>,
    width: usize,
    limit: usize,
    mut decode: impl FnMut(&[u8]) -> Result<T, ProjectError>,
) -> Result<Vec<T>, ProjectError> {
    if width == 0 || !chunk.data.len().is_multiple_of(width) || chunk.data.len() < width {
        return Err(error_at(
            chunk.offset,
            &format!("`{}` has an invalid record width", fourcc(chunk.id)),
        ));
    }
    let count = chunk
        .data
        .len()
        .checked_div(width)
        .ok_or_else(|| error_at(chunk.offset, "SoundFont record width is zero"))?;
    if count > limit {
        return fail(format!(
            "SoundFont `{}` record count {count} exceeds explicit limit {limit}",
            fourcc(chunk.id)
        ));
    }
    chunk.data.chunks_exact(width).map(&mut decode).collect()
}

fn bags(chunk: &Chunk<'_>, limit: usize) -> Result<Vec<Bag>, ProjectError> {
    records(chunk, 4, limit, |record| {
        Ok(Bag {
            generator: usize::from(read_u16(record, 0)?),
            modulator: usize::from(read_u16(record, 2)?),
        })
    })
}

fn generators(chunk: &Chunk<'_>, limit: usize) -> Result<Vec<Generator>, ProjectError> {
    records(chunk, 4, limit, |record| {
        Ok(Generator {
            operator: read_u16(record, 0)?,
            amount: read_u16(record, 2)?,
        })
    })
}

fn modulators(chunk: &Chunk<'_>, limit: usize) -> Result<Vec<Modulator>, ProjectError> {
    records(chunk, 10, limit, |record| {
        Ok(Modulator {
            source: read_u16(record, 0)?,
            destination: read_u16(record, 2)?,
            amount: read_u16(record, 4)? as i16,
            amount_source: read_u16(record, 6)?,
            transform: read_u16(record, 8)?,
        })
    })
}

fn name(record: &[u8], offset: usize) -> Result<String, ProjectError> {
    let field = record
        .get(offset..offset.saturating_add(20))
        .ok_or_else(|| error_at(offset, "SoundFont name field is truncated"))?;
    let end = field.iter().position(|byte| *byte == 0).unwrap_or(field.len());
    if !field.get(..end).unwrap_or_default().is_ascii() {
        return Err(error_at(offset, "SoundFont names must be ASCII"));
    }
    Ok(String::from_utf8_lossy(field.get(..end).unwrap_or_default()).into_owned())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ProjectError> {
    bytes
        .get(offset..offset.saturating_add(2))
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| error_at(offset, "SoundFont 16-bit field is truncated"))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ProjectError> {
    bytes
        .get(offset..offset.saturating_add(4))
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| error_at(offset, "SoundFont 32-bit field is truncated"))
}

fn fourcc(id: [u8; 4]) -> String {
    String::from_utf8_lossy(&id).into_owned()
}

fn error_at(offset: usize, message: &str) -> ProjectError {
    ProjectError::Assets(format!("SoundFont at byte {offset}: {message}"))
}

fn fail<T>(message: String) -> Result<T, ProjectError> {
    Err(ProjectError::Assets(message))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing, clippy::trivially_copy_pass_by_ref)]

    use std::path::Path;

    use musa_compiler::{CompileOptions, SourceDocument, compile, lower_gestures};
    use musa_dsp::{
        AudioFormat, ChannelLayout, CollapsePolicy, EventMessage, FrameRounding, MessageKind, ScheduleLimits,
        SchedulePolicy, TimeMap, schedule,
    };

    use super::{Sf2Limits, adapt};
    use crate::{ProjectSession, lock_assets};

    fn field(name: &str) -> [u8; 20] {
        let mut field = [0u8; 20];
        let bytes = name.as_bytes();
        field[..bytes.len()].copy_from_slice(bytes);
        field
    }

    fn chunk(id: &[u8; 4], data: Vec<u8>) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(id);
        bytes.extend(u32::try_from(data.len()).expect("fixture chunk length").to_le_bytes());
        bytes.extend(data);
        if bytes.len() % 2 != 0 {
            bytes.push(0);
        }
        bytes
    }

    fn list(kind: &[u8; 4], chunks: impl IntoIterator<Item = Vec<u8>>) -> Vec<u8> {
        let mut data = kind.to_vec();
        for chunk in chunks {
            data.extend(chunk);
        }
        chunk(b"LIST", data)
    }

    fn generator(operator: u16, amount: u16) -> [u8; 4] {
        let mut row = [0u8; 4];
        row[..2].copy_from_slice(&operator.to_le_bytes());
        row[2..].copy_from_slice(&amount.to_le_bytes());
        row
    }

    fn modulator(row: super::Modulator) -> [u8; 10] {
        let mut bytes = [0u8; 10];
        bytes[..2].copy_from_slice(&row.source.to_le_bytes());
        bytes[2..4].copy_from_slice(&row.destination.to_le_bytes());
        bytes[4..6].copy_from_slice(&row.amount.to_le_bytes());
        bytes[6..8].copy_from_slice(&row.amount_source.to_le_bytes());
        bytes[8..].copy_from_slice(&row.transform.to_le_bytes());
        bytes
    }

    fn preset(name: &str, program: u16, bank: u16, bag: u16) -> [u8; 38] {
        let mut row = [0u8; 38];
        row[..20].copy_from_slice(&field(name));
        row[20..22].copy_from_slice(&program.to_le_bytes());
        row[22..24].copy_from_slice(&bank.to_le_bytes());
        row[24..26].copy_from_slice(&bag.to_le_bytes());
        row
    }

    fn instrument(name: &str, bag: u16) -> [u8; 22] {
        let mut row = [0u8; 22];
        row[..20].copy_from_slice(&field(name));
        row[20..].copy_from_slice(&bag.to_le_bytes());
        row
    }

    #[allow(clippy::too_many_arguments)]
    fn sample(name: &str, start: u32, end: u32, loop_start: u32, loop_end: u32, link: u16, kind: u16) -> [u8; 46] {
        let mut row = [0u8; 46];
        row[..20].copy_from_slice(&field(name));
        row[20..24].copy_from_slice(&start.to_le_bytes());
        row[24..28].copy_from_slice(&end.to_le_bytes());
        row[28..32].copy_from_slice(&loop_start.to_le_bytes());
        row[32..36].copy_from_slice(&loop_end.to_le_bytes());
        row[36..40].copy_from_slice(&48_000u32.to_le_bytes());
        row[40] = 60;
        row[41] = (-4i8) as u8;
        row[42..44].copy_from_slice(&link.to_le_bytes());
        row[44..46].copy_from_slice(&kind.to_le_bytes());
        row
    }

    fn bank(extra_generator: Option<(u16, u16)>) -> Vec<u8> {
        bank_with(extra_generator, None, false)
    }

    fn bank_with(
        extra_generator: Option<(u16, u16)>,
        explicit_modulator: Option<super::Modulator>,
        with_sm24: bool,
    ) -> Vec<u8> {
        let info = list(
            b"INFO",
            [chunk(b"ifil", [2u16.to_le_bytes(), 4u16.to_le_bytes()].concat())],
        );
        let pcm: Vec<u8> = (0..64i16).flat_map(|sample| sample.to_le_bytes()).collect();
        let mut sample_chunks = vec![chunk(b"smpl", pcm)];
        if with_sm24 {
            sample_chunks.push(chunk(b"sm24", (0..64u8).collect()));
        }
        let sdta = list(b"sdta", sample_chunks);

        let phdr = chunk(
            b"phdr",
            [preset("Tone", 5, 2, 0).as_slice(), preset("EOP", 0, 0, 1).as_slice()].concat(),
        );
        let pbag = chunk(b"pbag", [[0u8, 0, 0, 0].as_slice(), [1u8, 0, 0, 0].as_slice()].concat());
        let pmod = chunk(b"pmod", vec![0; 10]);
        let pgen = chunk(
            b"pgen",
            [generator(41, 0).as_slice(), generator(0, 0).as_slice()].concat(),
        );
        let inst = chunk(
            b"inst",
            [
                instrument("Tone samples", 0).as_slice(),
                instrument("EOI", 1).as_slice(),
            ]
            .concat(),
        );
        let mut generators = vec![generator(43, 0x7f00), generator(44, 0x7f00)];
        if let Some((operator, amount)) = extra_generator {
            generators.push(generator(operator, amount));
        }
        generators.extend([
            generator(17, 0),
            generator(33, (-12_000i16) as u16),
            generator(34, (-12_000i16) as u16),
            generator(35, (-12_000i16) as u16),
            generator(36, (-12_000i16) as u16),
            generator(37, 120),
            generator(38, (-8_000i16) as u16),
            generator(54, 1),
            generator(57, 7),
            generator(53, 0),
        ]);
        let terminal = u16::try_from(generators.len()).expect("fixture generators");
        let ibag = chunk(
            b"ibag",
            [
                [0u8, 0, 0, 0].as_slice(),
                [
                    terminal as u8,
                    (terminal >> 8) as u8,
                    u8::from(explicit_modulator.is_some()),
                    0,
                ]
                .as_slice(),
            ]
            .concat(),
        );
        let mut modulator_rows = explicit_modulator.map_or_else(Vec::new, |row| modulator(row).to_vec());
        modulator_rows.extend([0; 10]);
        let imod = chunk(b"imod", modulator_rows);
        generators.push(generator(0, 0));
        let igen = chunk(b"igen", generators.into_iter().flatten().collect());
        let shdr = chunk(
            b"shdr",
            [
                sample("Tone", 0, 64, 8, 48, 0, 1).as_slice(),
                sample("EOS", 0, 0, 0, 0, 0, 0).as_slice(),
            ]
            .concat(),
        );
        let pdta = list(b"pdta", [phdr, pbag, pmod, pgen, inst, ibag, imod, igen, shdr]);
        let mut form = b"sfbk".to_vec();
        form.extend(info);
        form.extend(sdta);
        form.extend(pdta);
        let mut riff = b"RIFF".to_vec();
        riff.extend(u32::try_from(form.len()).expect("fixture RIFF length").to_le_bytes());
        riff.extend(form);
        riff
    }

    fn velocity_layer_bank() -> Vec<u8> {
        let info = list(
            b"INFO",
            [chunk(b"ifil", [2u16.to_le_bytes(), 4u16.to_le_bytes()].concat())],
        );
        let pcm: Vec<u8> = (0..128i16).flat_map(|sample| sample.to_le_bytes()).collect();
        let sdta = list(b"sdta", [chunk(b"smpl", pcm)]);
        let phdr = chunk(
            b"phdr",
            [preset("Layers", 5, 2, 0).as_slice(), preset("EOP", 0, 0, 1).as_slice()].concat(),
        );
        let pbag = chunk(b"pbag", [[0u8, 0, 0, 0].as_slice(), [1u8, 0, 0, 0].as_slice()].concat());
        let pmod = chunk(b"pmod", vec![0; 10]);
        let pgen = chunk(
            b"pgen",
            [generator(41, 0).as_slice(), generator(0, 0).as_slice()].concat(),
        );
        let inst = chunk(
            b"inst",
            [
                instrument("Layer samples", 0).as_slice(),
                instrument("EOI", 2).as_slice(),
            ]
            .concat(),
        );
        let ibag = chunk(
            b"ibag",
            [
                [0u8, 0, 0, 0].as_slice(),
                [2u8, 0, 0, 0].as_slice(),
                [4u8, 0, 0, 0].as_slice(),
            ]
            .concat(),
        );
        let imod = chunk(b"imod", vec![0; 10]);
        let igen = chunk(
            b"igen",
            [
                generator(44, 63 << 8).as_slice(),
                generator(53, 0).as_slice(),
                generator(44, (127 << 8) | 64).as_slice(),
                generator(53, 1).as_slice(),
                generator(0, 0).as_slice(),
            ]
            .concat(),
        );
        let shdr = chunk(
            b"shdr",
            [
                sample("Quiet", 0, 64, 8, 48, 0, 1).as_slice(),
                sample("Loud", 64, 128, 72, 112, 0, 1).as_slice(),
                sample("EOS", 0, 0, 0, 0, 0, 0).as_slice(),
            ]
            .concat(),
        );
        let pdta = list(b"pdta", [phdr, pbag, pmod, pgen, inst, ibag, imod, igen, shdr]);
        let mut form = b"sfbk".to_vec();
        form.extend(info);
        form.extend(sdta);
        form.extend(pdta);
        let mut riff = b"RIFF".to_vec();
        riff.extend(u32::try_from(form.len()).expect("fixture RIFF length").to_le_bytes());
        riff.extend(form);
        riff
    }

    fn sf2_limits() -> Sf2Limits {
        Sf2Limits {
            max_file_bytes: 64 * 1024,
            max_chunks: 32,
            max_presets: 4,
            max_instruments: 4,
            max_zones: 8,
            max_generators: 64,
            max_modulators: 16,
            max_samples: 4,
            max_pcm_bytes: 4096,
        }
    }

    fn sampler_limits() -> musa_dsp::SamplerLimits {
        musa_dsp::SamplerLimits {
            sample_rate: 48_000,
            max_voices: 64,
            max_regions: 8,
            max_decoded_bytes: 16 * 1024,
            max_selection_work: 8,
            max_step_work: 64 * 48,
        }
    }

    #[test]
    fn strict_bank_adapts_to_exact_checked_source_and_prepared_pcm() {
        let bytes = bank(None);
        let adapted = adapt("tone", "assets/tone.sf2#preset=2:5", &bytes, sf2_limits()).expect("valid strict bank");
        assert!(adapted.source.contains("SoundFontTimecents((0/1) - 12000/1)"));
        assert!(adapted.source.contains("SoundFontAttenuationCentibels(120/1)"));
        assert!(adapted.source.contains("source_maximum = 127/128"));
        assert_eq!(adapted.facts.preset_name, "Tone");
        let document = SourceDocument::new(adapted.source, "fixture:sf2");
        let checked = musa_compiler::checked_source_value(
            &document,
            &CompileOptions::default(),
            "imported_sf2_map",
            &musa_dsp::sample_map_schema(),
        )
        .expect("adapter source checks");
        let map = musa_dsp::decode_sample_map(&checked).expect("source projection");
        let prepared = musa_dsp::prepare_sample_map(
            map,
            |logical| adapted.samples.get(logical).cloned().ok_or_else(|| "absent".to_owned()),
            sampler_limits(),
        )
        .expect("embedded PCM preparation");
        assert_eq!(prepared.resources().regions, 1);
    }

    #[test]
    fn velocity_layers_flatten_to_disjoint_regions() {
        let adapted = adapt(
            "layers",
            "assets/layers.sf2#preset-name=Layers",
            &velocity_layer_bank(),
            sf2_limits(),
        )
        .expect("two strict velocity layers");
        assert_eq!(adapted.facts.regions, 2);
        assert!(
            adapted
                .source
                .contains("expression_low = 0/127, expression_high = 63/127")
        );
        assert!(
            adapted
                .source
                .contains("expression_low = 64/127, expression_high = 127/127")
        );
        assert_eq!(adapted.samples.len(), 2);
    }

    #[test]
    fn unsupported_sound_changing_generator_and_bounds_are_named_refusals() {
        let error = adapt(
            "tone",
            "assets/tone.sf2#preset-name=Tone",
            &bank(Some((15, 100))),
            sf2_limits(),
        )
        .err()
        .expect("chorus send is unsupported");
        assert!(error.to_string().contains("chorusEffectsSend"));

        let error = adapt(
            "tone",
            "assets/tone.sf2#preset=2:5",
            &bank(None),
            Sf2Limits {
                max_chunks: 2,
                ..sf2_limits()
            },
        )
        .err()
        .expect("chunk bound");
        assert!(error.to_string().contains("chunk count"));
    }

    #[test]
    fn explicit_modulators_and_twenty_four_bit_samples_cross_the_checked_boundary() {
        let explicit = super::Modulator {
            source: 3 | 0x0200 | (2 << 10),
            destination: 17,
            amount_source: 0,
            transform: 2,
            amount: 250,
        };
        let adapted = adapt(
            "tone",
            "assets/tone.sf2#preset-name=Tone",
            &bank_with(None, Some(explicit), true),
            sf2_limits(),
        )
        .expect("admitted explicit modulator and sm24");
        assert!(adapted.source.contains("source = KeyModulationSource"));
        assert!(adapted.source.contains("curve = ConvexModulationCurve"));
        assert!(adapted.source.contains("transform = AbsoluteModulationTransform"));
        assert!(adapted.source.contains("target = PanTarget, amount = 250/500"));
        let wav = adapted.samples.values().next().expect("one virtual sample");
        let reader = hound::WavReader::new(std::io::Cursor::new(wav.as_ref())).expect("virtual WAV");
        assert_eq!(reader.spec().bits_per_sample, 24);

        let disabled_default = super::Modulator {
            source: 0x0502,
            destination: 48,
            amount_source: 0,
            transform: 0,
            amount: 0,
        };
        let adapted = adapt(
            "tone",
            "assets/tone.sf2#preset=2:5",
            &bank_with(None, Some(disabled_default), false),
            sf2_limits(),
        )
        .expect("an identical zero modulator replaces the implicit default");
        assert!(!adapted.source.contains("target = GainDecibelsTarget"));
    }

    #[test]
    fn stereo_layers_require_one_reciprocal_pair_and_use_the_right_pitch_generators() {
        let samples = vec![
            super::SampleHeader {
                name: "Left".to_owned(),
                start: 0,
                end: 64,
                loop_start: 8,
                loop_end: 48,
                sample_rate: 48_000,
                original_key: 60,
                correction: 0,
                link: 1,
                kind: 4,
            },
            super::SampleHeader {
                name: "Right".to_owned(),
                start: 64,
                end: 128,
                loop_start: 72,
                loop_end: 112,
                sample_rate: 48_000,
                original_key: 60,
                correction: 0,
                link: 0,
                kind: 2,
            },
        ];
        let bank = super::Bank {
            presets: Vec::new(),
            preset_bags: Vec::new(),
            preset_modulators: Vec::new(),
            preset_generators: Vec::new(),
            instruments: Vec::new(),
            instrument_bags: Vec::new(),
            instrument_modulators: Vec::new(),
            instrument_generators: Vec::new(),
            samples,
            smpl: &[],
            sm24: None,
        };
        let region = |sample, tune| {
            let mut values = super::defaults();
            values.insert(52, tune);
            super::RegionSpec {
                sample,
                key: super::Range { low: 24, high: 96 },
                velocity: super::Range { low: 1, high: 127 },
                values,
                modulators: Vec::new(),
            }
        };
        let mut regions = vec![region(0, -12), region(1, 7)];
        super::normalize_stereo_regions(&bank, &mut regions).expect("reciprocal synchronized pair");
        assert_eq!(regions[0].values.get(&52), Some(&7));

        let error = super::normalize_stereo_regions(&bank, &mut [region(0, -12)]).expect_err("unpaired stereo sample");
        assert!(error.to_string().contains("synchronized partner"));
    }

    #[test]
    fn local_and_package_soundfont_fragments_resolve_to_the_locked_base_asset() {
        assert_eq!(
            crate::assets::soundfont_asset_base("assets/tone.sf2#preset=2:5").as_deref(),
            Some("assets/tone.sf2")
        );
        assert_eq!(
            crate::assets::soundfont_asset_base("pkg:orchestra/bank.sf2#preset-name=Solo").as_deref(),
            Some("pkg:orchestra/bank.sf2")
        );
        assert!(crate::assets::soundfont_asset_base("assets/tone.sf3#preset=2:5").is_none());
    }

    fn write(path: &Path, bytes: impl AsRef<[u8]>) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("fixture directory");
        }
        std::fs::write(path, bytes).expect("fixture write");
    }

    #[test]
    fn session_resolves_the_fragment_only_after_the_verified_bank_identity() {
        let directory = tempfile::tempdir().expect("temporary project");
        write(
            &directory.path().join("musa.toml"),
            br#"[project]
name = "SoundFont fixture"

[assets."assets/tone.sf2"]
kind = "sound-font"
adapter = "sf2@1"
max_bytes = 65536
"#,
        );
        write(
            &directory.path().join("piece.musa"),
            br#"instrument tone from "assets/tone.sf2#preset=2:5" conforms note_instrument;
piece "SoundFont" { meter 4/4; key c major; score { part p { voice v { c4/4 } } } }
"#,
        );
        write(&directory.path().join("assets/tone.sf2"), bank(None));
        lock_assets(directory.path()).expect("verified base-bank lock");
        let session = ProjectSession::open(directory.path().join("piece.musa")).expect("project session");
        let (prepared, facts) = session
            .prepare_sf2_instrument("tone", sf2_limits(), sampler_limits())
            .expect("session SoundFont preparation");
        assert_eq!(facts.asset, "assets/tone.sf2");
        assert_eq!(facts.adapter, "sf2@1");
        assert_eq!(prepared.resources().regions, 1);

        let compilation = compile(
            &SourceDocument::new(
                "piece \"gesture\" { meter 4/4; key c major; score { part p { voice v { c4/4 } } } }",
                "sf2-gesture.musa",
            ),
            &CompileOptions::default(),
        );
        let gestures = lower_gestures(compilation.snapshot().expect("gesture score")).expect("gesture lowering");
        let lane = gestures.lanes().first().expect("one gesture lane");
        let gesture = lane
            .track()
            .occurrences()
            .first()
            .map(|occurrence| occurrence.payload())
            .expect("one gesture");
        let token = prepared
            .selector(7, 11)
            .select(gesture, "")
            .expect("SoundFont selection token");
        let assignments = lane
            .track()
            .occurrences()
            .iter()
            .flat_map(|occurrence| [occurrence.span().start(), occurrence.span().end()])
            .chain([lane.track().duration().reach()])
            .map(|position| (position, lane.physical(position)));
        let policy = SchedulePolicy::new(
            1,
            FrameRounding::NearestTiesLater,
            CollapsePolicy::Ordered,
            [MessageKind::End, MessageKind::Point, MessageKind::Begin],
            ScheduleLimits {
                max_frame: 48_000,
                max_time_map_entries: 8,
                max_occurrences: 8,
                max_messages: 16,
                max_batches: 16,
            },
        )
        .expect("schedule policy");
        let format = AudioFormat::new(
            std::num::NonZeroU32::new(48_000).expect("sample rate"),
            ChannelLayout::Stereo,
        );
        let scheduled =
            schedule(format, policy, &TimeMap::new(1, assignments), lane.track()).expect("gesture schedule");
        let handle = scheduled
            .batches()
            .flat_map(|(_, batch)| batch.messages())
            .find_map(|message| match message {
                EventMessage::Begin(handle, _) => Some(handle.clone()),
                EventMessage::End(_) | EventMessage::Point(_, _) => None,
            })
            .expect("begin handle");
        let render = || {
            let mut runtime = prepared.runtime();
            runtime.note_on(&handle, &token);
            let mut output = vec![0.0; 128];
            runtime.render(&mut output);
            output
        };
        let first = render();
        assert_eq!(first, render(), "one SoundFont preset renders deterministically");
        assert!(first.iter().any(|sample| sample.abs() > f32::EPSILON));
    }
}
