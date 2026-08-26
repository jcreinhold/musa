//! Bounded off-thread preparation of source-owned recorded media.
//!
//! The score projection supplies finite intent; the project boundary supplies
//! verified immutable bytes. This module owns decoded PCM, frame conversion,
//! and the stateless absolute-frame reader used by both live and offline
//! rendering. No decoder or asset handle reaches the callback.
#![allow(clippy::arithmetic_side_effects)]

use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::Arc;

use musa_score::score::{MediaFit, MediaKind};
use musa_score::{IntegratedTempoMap, ScoreSnapshot};

use crate::schedule::AudioFormat;

/// Explicit bounds for decoded media retained by one prepared performance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaLimits {
    /// Greatest number of distinct verified assets decoded.
    pub max_assets: usize,
    /// Greatest total source frames retained after channel conversion.
    pub max_decoded_frames: u64,
    /// Greatest total bytes retained as stereo `f32` PCM.
    pub max_decoded_bytes: u64,
}

/// A stable preparation failure, always produced off the audio thread.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MediaPrepareError {
    /// The verified project boundary could not supply an asset.
    #[error("recorded-media asset `{asset}` is unavailable: {detail}")]
    Asset { asset: String, detail: String },
    /// The edition-one decoder accepts canonical WAV recordings only.
    #[error("recorded-media asset `{asset}` is not a supported WAV recording: {detail}")]
    Decode { asset: String, detail: String },
    /// A source value cannot become a finite DSP coefficient.
    #[error("recorded-media gain for `{name}` is not a finite DSP value")]
    Gain { name: String },
    /// Preparation exceeded an explicit retained-resource bound.
    #[error("recorded-media {resource} requirement {actual} exceeds explicit limit {limit}")]
    Resource {
        resource: &'static str,
        actual: u64,
        limit: u64,
    },
}

#[derive(Clone)]
struct DecodedMedia {
    samples: Arc<[f32]>,
    frames: u64,
    sample_rate: u32,
}

#[derive(Clone, Copy)]
enum PreparedFit {
    Crop,
    Loop,
    Rate,
    Fixed,
}

struct PreparedOccurrence {
    source: Arc<DecodedMedia>,
    start: u64,
    end: u64,
    fit: PreparedFit,
    gain: f32,
}

/// Immutable decoded sources plus absolute-frame occurrence readers.
///
/// Fields and constructors stay private: this is a preparation result, not a
/// second constructible media language.
pub struct PreparedMedia {
    occurrences: Vec<PreparedOccurrence>,
    names: Vec<String>,
    groups: Vec<Vec<usize>>,
    finish_frame: u64,
    output_rate: u32,
}

impl PreparedMedia {
    /// Last frame occupied by a prepared recording.
    pub const fn finish_frame(&self) -> u64 {
        self.finish_frame
    }

    /// Distinct source machine names in stable source order.
    pub(crate) fn names(&self) -> &[String] {
        &self.names
    }

    /// Mix all media active at one absolute output frame.
    #[cfg(test)]
    pub(crate) fn frame(&self, frame: u64) -> [f32; 2] {
        self.frame_for(self.occurrences.iter(), frame)
    }

    /// Mix occurrences belonging to one compact prepared source slot.
    pub(crate) fn slot_frame(&self, slot: usize, frame: u64) -> [f32; 2] {
        let Some(indices) = self.groups.get(slot) else {
            return [0.0; 2];
        };
        self.frame_for(indices.iter().filter_map(|index| self.occurrences.get(*index)), frame)
    }

    fn frame_for<'a>(&self, occurrences: impl Iterator<Item = &'a PreparedOccurrence>, frame: u64) -> [f32; 2] {
        let mut mixed = [0.0_f32; 2];
        for occurrence in occurrences {
            if frame < occurrence.start || frame >= occurrence.end {
                continue;
            }
            let local = frame.saturating_sub(occurrence.start);
            let span = occurrence.end.saturating_sub(occurrence.start);
            let source_position = match occurrence.fit {
                PreparedFit::Rate => {
                    if span == 0 {
                        continue;
                    }
                    local as f64 * occurrence.source.frames as f64 / span as f64
                }
                PreparedFit::Crop | PreparedFit::Fixed => {
                    local as f64 * f64::from(occurrence.source.sample_rate) / f64::from(self.output_rate)
                }
                PreparedFit::Loop => {
                    if occurrence.source.frames == 0 {
                        continue;
                    }
                    (local as f64 * f64::from(occurrence.source.sample_rate) / f64::from(self.output_rate))
                        % occurrence.source.frames as f64
                }
            };
            if !source_position.is_finite()
                || source_position < 0.0
                || source_position >= occurrence.source.frames as f64
            {
                continue;
            }
            let base_frame = source_position.floor() as u64;
            let next_frame = match occurrence.fit {
                PreparedFit::Loop => base_frame.saturating_add(1) % occurrence.source.frames,
                PreparedFit::Crop | PreparedFit::Rate | PreparedFit::Fixed => base_frame
                    .saturating_add(1)
                    .min(occurrence.source.frames.saturating_sub(1)),
            };
            let Ok(base) = usize::try_from(base_frame.saturating_mul(2)) else {
                continue;
            };
            let Ok(next) = usize::try_from(next_frame.saturating_mul(2)) else {
                continue;
            };
            let along = (source_position - base_frame as f64) as f32;
            let interpolate = |channel: usize| {
                let first = occurrence
                    .source
                    .samples
                    .get(base.saturating_add(channel))
                    .copied()
                    .unwrap_or(0.0);
                let later = occurrence
                    .source
                    .samples
                    .get(next.saturating_add(channel))
                    .copied()
                    .unwrap_or(first);
                (later - first).mul_add(along, first)
            };
            mixed[0] = interpolate(0).mul_add(occurrence.gain, mixed[0]);
            mixed[1] = interpolate(1).mul_add(occurrence.gain, mixed[1]);
        }
        [finite(mixed[0]), finite(mixed[1])]
    }
}

/// Decode every distinct checked media asset and prepare absolute frame
/// readers. `load` is called during preparation only.
///
/// # Errors
/// Refuses unavailable/undecodable bytes, non-finite gain, or an explicit
/// retained-resource bound.
pub fn prepare_media(
    score: &ScoreSnapshot,
    tempo: &IntegratedTempoMap,
    format: AudioFormat,
    mut load: impl FnMut(&str) -> Result<Arc<[u8]>, String>,
    limits: MediaLimits,
) -> Result<PreparedMedia, MediaPrepareError> {
    let output_rate = format.sample_rate().get();
    let mut decoded: BTreeMap<String, Arc<DecodedMedia>> = BTreeMap::new();
    let mut decoded_frames = 0_u64;
    let mut decoded_bytes = 0_u64;
    let mut occurrences = Vec::with_capacity(score.annotations().media().len());
    let mut names = Vec::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut finish_frame = 0_u64;

    for media in score.annotations().media() {
        let source = if let Some(source) = decoded.get(&media.asset) {
            Arc::clone(source)
        } else {
            let next_assets = decoded.len().saturating_add(1) as u64;
            check("asset count", next_assets, limits.max_assets as u64)?;
            let bytes = load(&media.asset).map_err(|detail| MediaPrepareError::Asset {
                asset: media.asset.clone(),
                detail,
            })?;
            let source = Arc::new(decode_wav(&media.asset, &bytes)?);
            decoded_frames = decoded_frames.saturating_add(source.frames);
            decoded_bytes = decoded_bytes.saturating_add(source.frames.saturating_mul(8));
            check("decoded frame count", decoded_frames, limits.max_decoded_frames)?;
            check("decoded byte count", decoded_bytes, limits.max_decoded_bytes)?;
            decoded.insert(media.asset.clone(), Arc::clone(&source));
            source
        };
        let start = tempo.frames(media.start, output_rate);
        let (fit, end) = match media.kind {
            MediaKind::MusicalClip(MediaFit::Crop) => (PreparedFit::Crop, tempo.frames(media.end, output_rate)),
            MediaKind::MusicalClip(MediaFit::Loop) => (PreparedFit::Loop, tempo.frames(media.end, output_rate)),
            MediaKind::MusicalClip(MediaFit::Rate) => (PreparedFit::Rate, tempo.frames(media.end, output_rate)),
            MediaKind::FixedMediaCue => {
                let duration = source
                    .frames
                    .saturating_mul(u64::from(output_rate))
                    .div_ceil(u64::from(source.sample_rate));
                (PreparedFit::Fixed, start.saturating_add(duration))
            }
        };
        let db = *media.gain_db.numer() as f64 / *media.gain_db.denom() as f64;
        let gain = 10.0_f64.powf(db / 20.0);
        if !gain.is_finite() || gain > f64::from(f32::MAX) {
            return Err(MediaPrepareError::Gain {
                name: media.name.clone(),
            });
        }
        finish_frame = finish_frame.max(end);
        let group = if let Some(group) = names.iter().position(|name| name == &media.name) {
            group
        } else {
            names.push(media.name.clone());
            groups.push(Vec::new());
            names.len().saturating_sub(1)
        };
        let occurrence = occurrences.len();
        occurrences.push(PreparedOccurrence {
            source,
            start,
            end,
            fit,
            gain: gain as f32,
        });
        if let Some(indices) = groups.get_mut(group) {
            indices.push(occurrence);
        }
    }
    Ok(PreparedMedia {
        occurrences,
        names,
        groups,
        finish_frame,
        output_rate,
    })
}

fn decode_wav(asset: &str, bytes: &[u8]) -> Result<DecodedMedia, MediaPrepareError> {
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).map_err(|error| MediaPrepareError::Decode {
        asset: asset.to_owned(),
        detail: error.to_string(),
    })?;
    let spec = reader.spec();
    if spec.sample_rate == 0 || !(spec.channels == 1 || spec.channels == 2) {
        return Err(MediaPrepareError::Decode {
            asset: asset.to_owned(),
            detail: "edition one accepts mono or stereo recordings at a nonzero sample rate".to_owned(),
        });
    }
    let samples = match spec.sample_format {
        hound::SampleFormat::Float if spec.bits_per_sample == 32 => reader
            .samples::<f32>()
            .map(|sample| sample.map_err(|error| error.to_string()))
            .collect::<Result<Vec<_>, _>>(),
        hound::SampleFormat::Int if spec.bits_per_sample <= 32 => {
            let scale = 2.0_f32.powi(i32::from(spec.bits_per_sample).saturating_sub(1));
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| value as f32 / scale)
                        .map_err(|error| error.to_string())
                })
                .collect::<Result<Vec<_>, _>>()
        }
        hound::SampleFormat::Float | hound::SampleFormat::Int => {
            Err(format!("unsupported {}-bit WAV sample format", spec.bits_per_sample))
        }
    }
    .map_err(|detail| MediaPrepareError::Decode {
        asset: asset.to_owned(),
        detail,
    })?;
    let channels = usize::from(spec.channels);
    if samples.len() % channels != 0 {
        return Err(MediaPrepareError::Decode {
            asset: asset.to_owned(),
            detail: "partial final sample frame".to_owned(),
        });
    }
    let frames = samples.len() / channels;
    let mut stereo = Vec::with_capacity(frames.saturating_mul(2));
    for frame in samples.chunks_exact(channels) {
        let Some(first) = frame.first() else { continue };
        let left = finite(*first);
        let right = finite(*frame.get(1).unwrap_or(first));
        stereo.extend([left, right]);
    }
    Ok(DecodedMedia {
        samples: stereo.into(),
        frames: frames as u64,
        sample_rate: spec.sample_rate,
    })
}

fn check(resource: &'static str, actual: u64, limit: u64) -> Result<(), MediaPrepareError> {
    if actual > limit {
        Err(MediaPrepareError::Resource {
            resource,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
