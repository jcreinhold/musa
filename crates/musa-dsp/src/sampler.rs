//! Deterministic, preloaded sample-instrument preparation and execution.
//!
//! Selection is a control-side operation producing an opaque token. The
//! real-time runtime consumes that token using fixed voice and PCM storage;
//! it performs no allocation, I/O, locking, logging, or random draw.
#![allow(clippy::arithmetic_side_effects)]

use std::io::Cursor;
use std::sync::Arc;

use musa_events::Canonical as _;
use musa_score::{Gesture, Letter};
use num_rational::Ratio;
use thiserror::Error;

use crate::sample_source::{
    ConnectionCondition, EnvelopeCurve, Gain, LoopMode, OffMode, PedalCondition, Region, SelectionPolicy, Trigger,
};
use crate::{EventHandle, SampleMap};

/// Explicit native preparation bounds. There is deliberately no default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SamplerLimits {
    /// Output frames per second.
    pub sample_rate: u32,
    /// Greatest accepted source voice count.
    pub max_voices: u16,
    /// Greatest accepted number of source regions.
    pub max_regions: usize,
    /// Greatest total retained decoded PCM byte count.
    pub max_decoded_bytes: usize,
    /// Greatest regions inspected by one control-side selection.
    pub max_selection_work: usize,
    /// Greatest conservatively priced work in one output-frame step.
    pub max_step_work: u64,
}

/// Conservatively priced immutable and mutable native resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SamplerResources {
    /// Fixed voice slots.
    pub voices: u16,
    /// Immutable normalized regions.
    pub regions: usize,
    /// Canonical decoded PCM retained across voices.
    pub decoded_pcm_bytes: usize,
    /// Fixed mutable voice state, excluding allocator bookkeeping.
    pub voice_state_bytes: usize,
    /// Greatest priced work for one reference-frame step.
    pub max_step_work: u64,
}

/// Stable failure before a sample runtime reaches the audio thread.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SamplerPrepareError {
    /// A checked source-map field is not usable by native preparation.
    #[error("sample map is invalid: {0}")]
    Map(String),
    /// A required verified asset was unavailable.
    #[error("sample asset `{path}` is unavailable: {reason}")]
    Asset { path: String, reason: String },
    /// WAV framing or samples are unsupported or malformed.
    #[error("sample asset `{path}` is not supported deterministic PCM: {reason}")]
    Decode { path: String, reason: String },
    /// A declared finite resource exceeds its explicit bound.
    #[error("sampler {resource} requirement {actual} exceeds explicit limit {limit}")]
    Resource {
        /// Resource class with stable spelling.
        resource: &'static str,
        /// Conservatively priced requirement.
        actual: u64,
        /// Product-supplied limit.
        limit: u64,
    },
}

#[derive(Clone)]
struct Pcm {
    channels: u16,
    sample_rate: u32,
    samples: Arc<[f32]>,
}

impl Pcm {
    fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }

    fn at(&self, frame: usize, channel: usize) -> f32 {
        let channel = channel.min(usize::from(self.channels).saturating_sub(1));
        self.samples
            .get(frame.saturating_mul(usize::from(self.channels)).saturating_add(channel))
            .copied()
            .unwrap_or(0.0)
    }
}

#[derive(Clone)]
struct PreparedRegion {
    source: Region,
    asset: usize,
    start: usize,
    end: usize,
    loop_start: usize,
    loop_end: usize,
    attack_frames: u32,
    decay_frames: u32,
    release_frames: u32,
    sustain: f32,
    gain_left: f32,
    gain_right: f32,
}

struct PreparedData {
    map: SampleMap,
    assets: Vec<Pcm>,
    regions: Vec<PreparedRegion>,
    decoded_bytes: usize,
    sample_rate: u32,
    resources: SamplerResources,
}

/// A fully decoded immutable sample map. Mutable selection and voice state are
/// created separately so control and audio ownership cannot be confused.
#[derive(Clone)]
pub struct PreparedSampleMap(Arc<PreparedData>);

impl PreparedSampleMap {
    /// Complete source map identity retained beside this projection.
    pub fn exact_source_bytes(&self) -> &[u8] {
        self.0.map.exact_source_bytes()
    }

    /// Bytes retained by canonical decoded PCM.
    pub fn decoded_bytes(&self) -> usize {
        self.0.decoded_bytes
    }

    /// Complete bounded-memory and per-frame-work report.
    pub fn resources(&self) -> SamplerResources {
        self.0.resources
    }

    /// Create one control-side selector for a prepared instrument instance.
    pub fn selector(&self, realization_seed: u64, instrument_instance: u64) -> SampleSelector {
        SampleSelector {
            prepared: Arc::clone(&self.0),
            realization_seed,
            instrument_instance,
            sequence: Vec::new(),
        }
    }

    /// Allocate the fixed real-time voice pool off-thread.
    pub fn runtime(&self) -> SampleRuntime {
        let region_capacity = self.0.regions.len();
        SampleRuntime {
            prepared: Arc::clone(&self.0),
            voices: vec![Voice::idle(); usize::from(self.0.map.voices)],
            notes: (0..self.0.map.voices)
                .map(|_| ActiveNote::idle(region_capacity))
                .collect(),
            pedal_down: false,
        }
    }
}

/// Decode every referenced WAV and normalize region bounds off-thread.
///
/// The resolver must return already verified raw bytes. It is invoked at most
/// once per distinct logical path and is never retained.
///
/// # Errors
/// Refuses unavailable/undecodable assets, invalid bounds, non-finite samples,
/// and any explicit preparation-budget excess.
pub fn prepare_sample_map(
    map: SampleMap,
    mut resolve: impl FnMut(&str) -> Result<Arc<[u8]>, String>,
    limits: SamplerLimits,
) -> Result<PreparedSampleMap, SamplerPrepareError> {
    if limits.sample_rate == 0 {
        return Err(SamplerPrepareError::Map("the output sample rate is zero".to_owned()));
    }
    check_limit("voice count", usize::from(map.voices), usize::from(limits.max_voices))?;
    check_limit("region count", map.regions.len(), limits.max_regions)?;
    check_limit("selection work", map.regions.len(), limits.max_selection_work)?;
    let step_work = u64::from(map.voices).saturating_mul(24);
    if step_work > limits.max_step_work {
        return Err(SamplerPrepareError::Resource {
            resource: "one-frame work",
            actual: step_work,
            limit: limits.max_step_work,
        });
    }

    let mut paths = Vec::<String>::new();
    let mut assets = Vec::new();
    let mut decoded_bytes = 0usize;
    for region in &map.regions {
        if paths.iter().any(|path| path == &region.asset) {
            continue;
        }
        let bytes = resolve(&region.asset).map_err(|reason| SamplerPrepareError::Asset {
            path: region.asset.clone(),
            reason,
        })?;
        let remaining = limits.max_decoded_bytes.saturating_sub(decoded_bytes);
        let pcm = decode_wav(&region.asset, &bytes, remaining)?;
        decoded_bytes = decoded_bytes
            .checked_add(pcm.samples.len().saturating_mul(std::mem::size_of::<f32>()))
            .ok_or(SamplerPrepareError::Resource {
                resource: "decoded PCM bytes",
                actual: u64::MAX,
                limit: limits.max_decoded_bytes as u64,
            })?;
        check_limit("decoded PCM bytes", decoded_bytes, limits.max_decoded_bytes)?;
        paths.push(region.asset.clone());
        assets.push(pcm);
    }

    let mut regions = Vec::with_capacity(map.regions.len());
    for source in &map.regions {
        let asset = paths
            .iter()
            .position(|path| path == &source.asset)
            .ok_or_else(|| SamplerPrepareError::Map(format!("asset `{}` was not prepared", source.asset)))?;
        let pcm = assets
            .get(asset)
            .ok_or_else(|| SamplerPrepareError::Map("prepared asset index is absent".to_owned()))?;
        let available = pcm.frames();
        let start = usize::try_from(source.start_frame)
            .ok()
            .filter(|start| *start < available)
            .ok_or_else(|| SamplerPrepareError::Map(format!("region `{}` starts outside its asset", source.asset)))?;
        let end = if source.end_frame == 0 {
            available
        } else {
            usize::try_from(source.end_frame).unwrap_or(usize::MAX)
        };
        if end <= start || end > available {
            return Err(SamplerPrepareError::Map(format!(
                "region `{}` has playback bounds outside its asset",
                source.asset
            )));
        }
        let (loop_start, loop_end) = if source.loop_mode.loops() {
            let loop_start = usize::try_from(source.loop_start).unwrap_or(usize::MAX);
            let loop_end = usize::try_from(source.loop_end).unwrap_or(usize::MAX);
            if loop_start < start || loop_start >= loop_end || loop_end > end {
                return Err(SamplerPrepareError::Map(format!(
                    "region `{}` has loop bounds outside its playback interval",
                    source.asset
                )));
            }
            (loop_start, loop_end)
        } else {
            (0, 0)
        };
        let pan = ratio_f32(source.pan, "pan")?;
        let gain = match &source.gain {
            Gain::Linear(value) => ratio_f32(*value, "linear gain")?,
            Gain::Decibels(value) => {
                let decibels = ratio_f32(*value, "decibel gain")?;
                10.0f32.powf(decibels / 20.0)
            }
        };
        if !gain.is_finite() {
            return Err(SamplerPrepareError::Map("gain is not a finite native value".to_owned()));
        }
        let left = (-pan).midpoint(1.0).sqrt() * gain;
        let right = 1.0f32.midpoint(pan).sqrt() * gain;
        regions.push(PreparedRegion {
            source: source.clone(),
            asset,
            start,
            end,
            loop_start,
            loop_end,
            attack_frames: seconds_frames(source.envelope.attack, limits.sample_rate)?,
            decay_frames: seconds_frames(source.envelope.decay, limits.sample_rate)?,
            release_frames: seconds_frames(source.envelope.release, limits.sample_rate)?,
            sustain: ratio_f32(source.envelope.sustain, "envelope sustain")?,
            gain_left: left,
            gain_right: right,
        });
    }
    let resources = SamplerResources {
        voices: map.voices,
        regions: map.regions.len(),
        decoded_pcm_bytes: decoded_bytes,
        voice_state_bytes: usize::from(map.voices)
            .saturating_mul(std::mem::size_of::<Voice>())
            .saturating_add(
                usize::from(map.voices)
                    .saturating_mul(map.regions.len())
                    .saturating_mul(3)
                    .saturating_mul(std::mem::size_of::<usize>()),
            ),
        max_step_work: step_work,
    };
    Ok(PreparedSampleMap(Arc::new(PreparedData {
        map,
        assets,
        regions,
        decoded_bytes,
        sample_rate: limits.sample_rate,
        resources,
    })))
}

/// Mutable control-side deterministic variation state.
pub struct SampleSelector {
    prepared: Arc<PreparedData>,
    realization_seed: u64,
    instrument_instance: u64,
    sequence: Vec<(u64, u64)>,
}

/// Opaque instruction selected off-thread and borrowed by the audio callback.
#[derive(Clone, Debug, PartialEq)]
pub struct SampleSelectionToken {
    attack: Box<[usize]>,
    release_up: Box<[usize]>,
    release_down: Box<[usize]>,
    release_key: Box<[usize]>,
    key: u8,
    expression: f32,
}

impl SampleSelector {
    /// Resolve attack and both possible release-pedal states before callback
    /// execution. Empty `technique` means the ordinary technique.
    ///
    /// # Errors
    /// Refuses a pitch outside the native key lattice or non-finite exact
    /// expression conversion.
    pub fn select(&mut self, gesture: &Gesture, technique: &str) -> Result<SampleSelectionToken, SamplerPrepareError> {
        let key = pitch_key(gesture)?;
        let expression = ratio_f32(gesture.amplitude(), "gesture expression")?;
        let connection = gesture
            .controls()
            .iter()
            .find(|control| control.namespace() == "std.performance" && control.name() == "phrase_relation")
            .and_then(|control| control.symbol())
            .unwrap_or("Ordinary");
        let exact = gesture.canonical_key();
        let attack = self.choose(
            key,
            gesture.amplitude(),
            technique,
            connection,
            Trigger::Attack,
            false,
            exact.as_bytes(),
        );
        let release_up = self.choose(
            key,
            gesture.amplitude(),
            technique,
            connection,
            Trigger::Release,
            false,
            exact.as_bytes(),
        );
        let release_down = self.choose(
            key,
            gesture.amplitude(),
            technique,
            connection,
            Trigger::Release,
            true,
            exact.as_bytes(),
        );
        let release_key = self.choose(
            key,
            gesture.amplitude(),
            technique,
            connection,
            Trigger::ReleaseKey,
            false,
            exact.as_bytes(),
        );
        Ok(SampleSelectionToken {
            attack: attack.into_boxed_slice(),
            release_up: release_up.into_boxed_slice(),
            release_down: release_down.into_boxed_slice(),
            release_key: release_key.into_boxed_slice(),
            key,
            expression,
        })
    }

    fn choose(
        &mut self,
        key: u8,
        expression: Ratio<i64>,
        technique: &str,
        connection: &str,
        trigger: Trigger,
        pedal_down: bool,
        identity: &[u8],
    ) -> Vec<usize> {
        let Some(highest) = self
            .prepared
            .regions
            .iter()
            .filter(|region| applies(region, key, expression, technique, connection, trigger, pedal_down))
            .map(|region| region.source.priority)
            .max()
        else {
            return Vec::new();
        };
        let candidates = self
            .prepared
            .regions
            .iter()
            .enumerate()
            .filter(|(_, region)| {
                region.source.priority == highest
                    && applies(region, key, expression, technique, connection, trigger, pedal_down)
            })
            .collect::<Vec<_>>();
        let mut selected = candidates
            .iter()
            .filter(|(_, region)| region.source.sequence_group == 0)
            .map(|(index, _)| *index)
            .collect::<Vec<_>>();
        let mut groups = candidates
            .iter()
            .map(|(_, region)| region.source.sequence_group)
            .filter(|group| *group != 0)
            .collect::<Vec<_>>();
        groups.sort_unstable();
        groups.dedup();
        match self.prepared.map.selection {
            SelectionPolicy::First => {
                for group in groups {
                    if let Some((index, _)) = candidates
                        .iter()
                        .find(|(_, region)| region.source.sequence_group == group)
                    {
                        selected.push(*index);
                    }
                }
            }
            SelectionPolicy::RoundRobin => {
                for group in groups {
                    let next = match self.sequence.iter_mut().find(|(candidate, _)| *candidate == group) {
                        Some((_, next)) => {
                            let current = *next;
                            *next = next.saturating_add(1);
                            current
                        }
                        None => {
                            self.sequence.push((group, 1));
                            0
                        }
                    };
                    selected.extend(candidates.iter().filter_map(|(index, region)| {
                        (region.source.sequence_group == group
                            && (next % region.source.sequence_length).saturating_add(1)
                                == region.source.sequence_position)
                            .then_some(*index)
                    }));
                }
            }
            SelectionPolicy::StableWeighted => {
                for group in groups {
                    let total = candidates
                        .iter()
                        .filter(|(_, region)| region.source.sequence_group == group)
                        .fold(0u64, |sum, (_, region)| sum.saturating_add(region.source.weight));
                    let mut choice = stable_hash(
                        self.realization_seed ^ group.rotate_left(13),
                        self.instrument_instance,
                        identity,
                    ) % total;
                    for (index, region) in &candidates {
                        if region.source.sequence_group != group {
                            continue;
                        }
                        if choice < region.source.weight {
                            selected.push(*index);
                            break;
                        }
                        choice -= region.source.weight;
                    }
                }
            }
        }
        selected.sort_unstable();
        selected
    }
}

fn applies(
    region: &PreparedRegion,
    key: u8,
    expression: Ratio<i64>,
    technique: &str,
    connection: &str,
    trigger: Trigger,
    pedal_down: bool,
) -> bool {
    region.source.trigger == trigger
        && (region.source.key_low..=region.source.key_high).contains(&key)
        && expression >= region.source.expression_low
        && expression <= region.source.expression_high
        && (region.source.technique.is_empty() || region.source.technique == technique)
        && match region.source.connection {
            ConnectionCondition::Any => true,
            ConnectionCondition::First => connection == "Ordinary",
            ConnectionCondition::Detached => connection == "Detached",
            ConnectionCondition::Ordinary => connection == "Ordinary",
            ConnectionCondition::Legato => connection == "Legato",
        }
        && match region.source.pedal {
            PedalCondition::Any => true,
            PedalCondition::Up => !pedal_down,
            PedalCondition::Down => pedal_down,
        }
}

#[derive(Clone, Copy)]
enum EnvelopeStage {
    Attack,
    Decay,
    Sustain,
    Release,
    Idle,
}

#[derive(Clone)]
struct Voice {
    handle: Option<EventHandle>,
    region: usize,
    position: f64,
    increment: f64,
    direction: f64,
    level: f32,
    stage: EnvelopeStage,
    stage_frame: u32,
    release_start: f32,
    held: bool,
    deferred_release: bool,
    one_shot: bool,
    age: u64,
    expression: f32,
}

impl Voice {
    fn idle() -> Self {
        Self {
            handle: None,
            region: 0,
            position: 0.0,
            increment: 0.0,
            direction: 1.0,
            level: 0.0,
            stage: EnvelopeStage::Idle,
            stage_frame: 0,
            release_start: 0.0,
            held: false,
            deferred_release: false,
            one_shot: false,
            age: 0,
            expression: 1.0,
        }
    }

    fn is_idle(&self) -> bool {
        matches!(self.stage, EnvelopeStage::Idle)
    }
}

struct ActiveNote {
    handle: Option<EventHandle>,
    release_up: Vec<usize>,
    release_down: Vec<usize>,
    release_key: Vec<usize>,
    key: u8,
    expression: f32,
    deferred: bool,
    age: u64,
}

impl ActiveNote {
    fn idle(region_capacity: usize) -> Self {
        Self {
            handle: None,
            release_up: Vec::with_capacity(region_capacity),
            release_down: Vec::with_capacity(region_capacity),
            release_key: Vec::with_capacity(region_capacity),
            key: 0,
            expression: 1.0,
            deferred: false,
            age: 0,
        }
    }

    fn clear(&mut self) {
        self.handle = None;
        self.release_up.clear();
        self.release_down.clear();
        self.release_key.clear();
        self.deferred = false;
    }
}

/// Fixed-memory native sampler state owned by one prepared instrument instance.
pub struct SampleRuntime {
    prepared: Arc<PreparedData>,
    voices: Vec<Voice>,
    notes: Vec<ActiveNote>,
    pedal_down: bool,
}

impl SampleRuntime {
    /// Begin one scheduled note using a token computed off-thread.
    pub fn note_on(&mut self, handle: &EventHandle, token: &SampleSelectionToken) {
        let note_slot = self.notes.iter().position(|note| note.handle.is_none()).or_else(|| {
            self.notes
                .iter()
                .enumerate()
                .max_by_key(|(_, note)| note.age)
                .map(|(index, _)| index)
        });
        if let Some(note) = note_slot.and_then(|index| self.notes.get_mut(index)) {
            note.handle = Some(handle.clone());
            note.release_up.clear();
            note.release_up.extend_from_slice(&token.release_up);
            note.release_down.clear();
            note.release_down.extend_from_slice(&token.release_down);
            note.release_key.clear();
            note.release_key.extend_from_slice(&token.release_key);
            note.key = token.key;
            note.expression = token.expression;
            note.deferred = false;
            note.age = 0;
        }
        for &region in &token.attack {
            self.start_voice(Some(handle), region, token.key, token.expression);
        }
    }

    /// Release the scheduled occurrence. Pedal deferral and release samples
    /// use choices already carried by the note's token.
    pub fn note_off(&mut self, handle: &EventHandle) {
        for voice in &mut self.voices {
            if voice.handle.as_ref() == Some(handle) && voice.held {
                voice.held = false;
                if self.pedal_down {
                    voice.deferred_release = true;
                } else if !voice.one_shot {
                    begin_release(voice);
                }
            }
        }
        let Some(note_index) = self.notes.iter().position(|note| note.handle.as_ref() == Some(handle)) else {
            return;
        };
        let (key, expression, release_key) = {
            let Some(note) = self.notes.get_mut(note_index) else {
                return;
            };
            (note.key, note.expression, std::mem::take(&mut note.release_key))
        };
        for &region in &release_key {
            self.start_voice(None, region, key, expression);
        }
        let Some(note) = self.notes.get_mut(note_index) else {
            return;
        };
        note.release_key = release_key;
        if self.pedal_down {
            note.deferred = true;
        } else {
            let release_up = std::mem::take(&mut note.release_up);
            for &region in &release_up {
                self.start_voice(None, region, key, expression);
            }
            let Some(note) = self.notes.get_mut(note_index) else {
                return;
            };
            note.release_up = release_up;
            note.clear();
        }
    }

    /// Apply the source sustain-control interpretation without allocation.
    pub fn set_pedal(&mut self, down: bool) {
        if self.pedal_down && !down {
            for voice in &mut self.voices {
                if voice.deferred_release {
                    voice.deferred_release = false;
                    if !voice.one_shot {
                        begin_release(voice);
                    }
                }
            }
            let note_count = self.notes.len();
            for index in 0..note_count {
                let Some(note) = self.notes.get_mut(index) else {
                    continue;
                };
                if !note.deferred {
                    continue;
                }
                let key = note.key;
                let expression = note.expression;
                let release_down = std::mem::take(&mut note.release_down);
                for &region in &release_down {
                    self.start_voice(None, region, key, expression);
                }
                let Some(note) = self.notes.get_mut(index) else {
                    continue;
                };
                note.release_down = release_down;
                note.clear();
            }
        }
        self.pedal_down = down;
    }

    /// Produce one stereo frame. Work is bounded by the prepared voice count.
    pub fn step(&mut self) -> [f32; 2] {
        let mut output = [0.0f32; 2];
        for note in &mut self.notes {
            if note.handle.is_some() {
                note.age = note.age.saturating_add(1);
            }
        }
        for voice in &mut self.voices {
            if voice.is_idle() {
                continue;
            }
            voice.age = voice.age.saturating_add(1);
            let Some(region) = self.prepared.regions.get(voice.region) else {
                voice.stage = EnvelopeStage::Idle;
                continue;
            };
            let Some(pcm) = self.prepared.assets.get(region.asset) else {
                voice.stage = EnvelopeStage::Idle;
                continue;
            };
            let position = voice
                .position
                .clamp(region.start as f64, (region.end.saturating_sub(1)) as f64);
            let base = position.floor() as usize;
            let next = base.saturating_add(1).min(region.end.saturating_sub(1));
            let along = (position - base as f64) as f32;
            let interpolate = |channel| {
                let base = pcm.at(base, channel);
                (pcm.at(next, channel) - base).mul_add(along, base)
            };
            let left = interpolate(0);
            let right = if pcm.channels == 1 { left } else { interpolate(1) };
            tick_envelope(voice, region);
            output[0] = (left * region.gain_left * voice.level * voice.expression).mul_add(1.0, output[0]);
            output[1] = (right * region.gain_right * voice.level * voice.expression).mul_add(1.0, output[1]);
            advance(voice, region);
        }
        for sample in &mut output {
            if !sample.is_finite() {
                *sample = 0.0;
            }
        }
        output
    }

    /// Render interleaved stereo by repeating the reference step.
    pub fn render(&mut self, output: &mut [f32]) {
        let (frames, remainder) = output.as_chunks_mut::<2>();
        for frame in frames {
            *frame = self.step();
        }
        remainder.fill(0.0);
    }

    fn start_voice(&mut self, handle: Option<&EventHandle>, region_index: usize, key: u8, expression: f32) {
        let Some(region) = self.prepared.regions.get(region_index) else {
            return;
        };
        if region.source.group != 0 {
            for voice in &mut self.voices {
                let off_mode = self.prepared.regions.get(voice.region).and_then(|playing| {
                    (playing.source.off_by == region.source.group).then_some(playing.source.off_mode)
                });
                if !voice.is_idle()
                    && let Some(off_mode) = off_mode
                {
                    match off_mode {
                        OffMode::Fast => {
                            voice.stage = EnvelopeStage::Idle;
                            voice.handle = None;
                        }
                        OffMode::Normal => begin_release(voice),
                    }
                }
            }
        }
        let slot = self.voices.iter().position(Voice::is_idle).or_else(|| {
            self.voices
                .iter()
                .enumerate()
                .max_by_key(|(_, voice)| (u8::from(!voice.held), voice.age))
                .map(|(index, _)| index)
        });
        let Some(voice) = slot.and_then(|index| self.voices.get_mut(index)) else {
            return;
        };
        let cents = ratio_f64(region.source.tune_cents);
        let semitones = f64::from(i16::from(key) - i16::from(region.source.root_key)) + cents / 100.0;
        let Some(asset) = self.prepared.assets.get(region.asset) else {
            return;
        };
        let source_rate = f64::from(asset.sample_rate);
        voice.handle = handle.cloned();
        voice.region = region_index;
        voice.position = region.start as f64;
        voice.increment = (semitones / 12.0).exp2() * source_rate / f64::from(self.prepared.sample_rate);
        voice.direction = 1.0;
        voice.level = 0.0;
        voice.stage = if region.attack_frames == 0 {
            EnvelopeStage::Decay
        } else {
            EnvelopeStage::Attack
        };
        if region.attack_frames == 0 {
            voice.level = 1.0;
        }
        voice.stage_frame = 0;
        voice.release_start = 0.0;
        voice.held = handle.is_some();
        voice.deferred_release = false;
        voice.one_shot = region.source.loop_mode == LoopMode::OneShot || region.source.trigger != Trigger::Attack;
        voice.age = 0;
        voice.expression = expression.clamp(0.0, 1.0);
    }
}

fn tick_envelope(voice: &mut Voice, region: &PreparedRegion) {
    match voice.stage {
        EnvelopeStage::Attack => {
            voice.stage_frame = voice.stage_frame.saturating_add(1);
            voice.level = (voice.stage_frame as f32 / region.attack_frames.max(1) as f32).min(1.0);
            if voice.stage_frame >= region.attack_frames {
                voice.stage = EnvelopeStage::Decay;
                voice.stage_frame = 0;
            }
        }
        EnvelopeStage::Decay => {
            voice.stage_frame = voice.stage_frame.saturating_add(1);
            let along = voice.stage_frame as f32 / region.decay_frames.max(1) as f32;
            voice.level = match region.source.envelope.curve {
                EnvelopeCurve::Linear => (region.sustain - 1.0).mul_add(along.min(1.0), 1.0),
                EnvelopeCurve::Sfz1 => {
                    let exponential = (-8.0 * along).exp();
                    (1.0 - region.sustain).mul_add(exponential, region.sustain)
                }
            };
            if region.decay_frames == 0 || voice.stage_frame >= region.decay_frames {
                voice.stage = EnvelopeStage::Sustain;
                voice.level = region.sustain;
            }
        }
        EnvelopeStage::Sustain => voice.level = region.sustain,
        EnvelopeStage::Release => {
            voice.stage_frame = voice.stage_frame.saturating_add(1);
            let along = voice.stage_frame as f32 / region.release_frames.max(1) as f32;
            voice.level = match region.source.envelope.curve {
                EnvelopeCurve::Linear => voice.release_start * (1.0 - along.min(1.0)),
                EnvelopeCurve::Sfz1 => voice.release_start * (-8.0 * along).exp(),
            };
            if region.release_frames == 0 || voice.stage_frame >= region.release_frames {
                voice.stage = EnvelopeStage::Idle;
                voice.handle = None;
            }
        }
        EnvelopeStage::Idle => {}
    }
}

fn begin_release(voice: &mut Voice) {
    voice.stage = EnvelopeStage::Release;
    voice.stage_frame = 0;
    voice.release_start = voice.level;
}

fn advance(voice: &mut Voice, region: &PreparedRegion) {
    if voice.is_idle() {
        return;
    }
    voice.position = voice.increment.mul_add(voice.direction, voice.position);
    let looping = region.source.loop_mode.loops()
        && (region.source.loop_mode.continuous() || voice.held || voice.deferred_release);
    if looping && voice.direction > 0.0 && voice.position >= region.loop_end as f64 {
        if region.source.loop_mode.alternating() {
            voice.position = region.loop_end as f64 - (voice.position - region.loop_end as f64);
            voice.direction = -1.0;
        } else {
            let width = (region.loop_end - region.loop_start) as f64;
            voice.position = region.loop_start as f64 + (voice.position - region.loop_end as f64) % width;
        }
    } else if looping && voice.direction < 0.0 && voice.position < region.loop_start as f64 {
        voice.position = region.loop_start as f64 + (region.loop_start as f64 - voice.position);
        voice.direction = 1.0;
    } else if voice.position >= region.end as f64 || voice.position < region.start as f64 {
        voice.stage = EnvelopeStage::Idle;
        voice.handle = None;
    }
}

fn decode_wav(path: &str, bytes: &[u8], remaining_bytes: usize) -> Result<Pcm, SamplerPrepareError> {
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).map_err(|error| SamplerPrepareError::Decode {
        path: path.to_owned(),
        reason: error.to_string(),
    })?;
    let spec = reader.spec();
    if !matches!(spec.channels, 1 | 2) || spec.sample_rate == 0 {
        return Err(SamplerPrepareError::Decode {
            path: path.to_owned(),
            reason: "only mono or stereo WAV with a nonzero sample rate is supported".to_owned(),
        });
    }
    let estimated = usize::try_from(reader.duration())
        .unwrap_or(usize::MAX)
        .saturating_mul(usize::from(spec.channels))
        .saturating_mul(std::mem::size_of::<f32>());
    check_limit("decoded PCM bytes", estimated, remaining_bytes)?;
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => {
            if spec.bits_per_sample == 32 {
                reader
                    .samples::<f32>()
                    .map(|sample| sample.map_err(|error| error.to_string()))
                    .collect::<Result<Vec<_>, _>>()
            } else {
                Err(format!("{}-bit float samples are unsupported", spec.bits_per_sample))
            }
        }
        hound::SampleFormat::Int => {
            if (1..=32).contains(&spec.bits_per_sample) {
                let scale = 2.0f32.powi(i32::from(spec.bits_per_sample.saturating_sub(1)));
                reader
                    .samples::<i32>()
                    .map(|sample| {
                        sample
                            .map(|value| value as f32 / scale)
                            .map_err(|error| error.to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()
            } else {
                Err(format!("{}-bit integer samples are unsupported", spec.bits_per_sample))
            }
        }
    }
    .map_err(|reason| SamplerPrepareError::Decode {
        path: path.to_owned(),
        reason,
    })?;
    if samples.is_empty() || samples.len() % usize::from(spec.channels) != 0 || samples.iter().any(|x| !x.is_finite()) {
        return Err(SamplerPrepareError::Decode {
            path: path.to_owned(),
            reason: "samples must be nonempty, frame-aligned, and finite".to_owned(),
        });
    }
    Ok(Pcm {
        channels: spec.channels,
        sample_rate: spec.sample_rate,
        samples: samples.into(),
    })
}

fn pitch_key(gesture: &Gesture) -> Result<u8, SamplerPrepareError> {
    let pitch = gesture.pitch();
    let natural = match pitch.letter {
        Letter::C => 0,
        Letter::D => 2,
        Letter::E => 4,
        Letter::F => 5,
        Letter::G => 7,
        Letter::A => 9,
        Letter::B => 11,
    };
    let key = pitch
        .octave
        .checked_add(1)
        .and_then(|octave| octave.checked_mul(12))
        .and_then(|base| base.checked_add(natural))
        .and_then(|base| base.checked_add(pitch.accidental.0))
        .and_then(|key| u8::try_from(key).ok())
        .filter(|key| *key <= 127)
        .ok_or_else(|| SamplerPrepareError::Map(format!("pitch {} is outside keys 0 through 127", gesture.pitch())))?;
    Ok(key)
}

fn seconds_frames(value: Ratio<i64>, sample_rate: u32) -> Result<u32, SamplerPrepareError> {
    let frames = ratio_f64(value) * f64::from(sample_rate);
    if !frames.is_finite() || frames < 0.0 || frames > f64::from(u32::MAX) {
        return Err(SamplerPrepareError::Map(
            "an envelope time exceeds native frame bounds".to_owned(),
        ));
    }
    Ok(frames.round() as u32)
}

fn ratio_f32(value: Ratio<i64>, name: &str) -> Result<f32, SamplerPrepareError> {
    let value = ratio_f64(value);
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return Err(SamplerPrepareError::Map(format!("{name} is not a finite native value")));
    }
    Ok(value as f32)
}

fn ratio_f64(value: Ratio<i64>) -> f64 {
    *value.numer() as f64 / *value.denom() as f64
}

fn check_limit(resource: &'static str, actual: usize, limit: usize) -> Result<(), SamplerPrepareError> {
    if actual > limit {
        return Err(SamplerPrepareError::Resource {
            resource,
            actual: actual as u64,
            limit: limit as u64,
        });
    }
    Ok(())
}

fn stable_hash(seed: u64, instance: u64, bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64 ^ seed.rotate_left(17) ^ instance.rotate_left(41);
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
