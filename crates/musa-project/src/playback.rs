//! The compile → lower → graph chain, prepared once and used both ways.
//!
//! Roadmap §13.8: an offline render and a live performance must come from
//! the same preparation, or "export what I hear" is a lie. [`build`] is that
//! single preparation; [`prepare`] hands it to the engine and [`to_wav`]
//! runs it offline.

use musa_compiler::{PerformanceEvent, PerformanceOptions, PerformancePlan, ScoreSnapshot, lower_performance};
use musa_engine::PreparedPlaybackPlan;

use crate::error::ProjectError;

/// Voices the default instrument graph can sound at once.
const POLYPHONY: u8 = 16;

/// Audio block size for the studio graph.
const BLOCK_SIZE: usize = 128;

/// The sample rate the whole chain runs at: the performance lowering's rate,
/// so frame numbers from the compiler are frame numbers in the render.
pub(crate) fn sample_rate() -> u32 {
    PerformanceOptions::default().sample_rate
}

/// Performance lowering → default instrument graph → frame-sorted events.
fn build(score: &ScoreSnapshot) -> Result<(musa_audio::RenderPlan, Vec<PerformanceEvent>, u64), ProjectError> {
    let performance = lower_performance(score, &PerformanceOptions::default())
        .map_err(|e| ProjectError::Performance(e.to_string()))?;
    let sample_rate = sample_rate();
    let events = collect_events(&performance);
    let spec = musa_audio::poly_sine_spec(POLYPHONY);
    let options = musa_audio::GraphOptions {
        sample_rate,
        block_size: BLOCK_SIZE,
    };
    let plan = musa_audio::compile_graph(&spec, &options).map_err(|e| ProjectError::Performance(e.to_string()))?;
    let tail = u64::from(sample_rate); // 1 s release tail until envelopes exist
    let frames = events
        .iter()
        .map(PerformanceEvent::frame)
        .max()
        .unwrap_or(0)
        .saturating_add(tail);
    Ok((plan, events, frames))
}

/// The chain prepared for the engine.
///
/// # Errors
/// [`ProjectError::Performance`] if lowering or graph compilation fails.
pub(crate) fn prepare(score: &ScoreSnapshot) -> Result<PreparedPlaybackPlan, ProjectError> {
    let (plan, events, frames) = build(score)?;
    Ok(PreparedPlaybackPlan::new(plan, events, frames))
}

/// The same chain rendered offline to 32-bit float stereo WAV bytes.
///
/// # Errors
/// [`ProjectError::Performance`] if lowering, graph compilation, or WAV
/// encoding fails.
pub(crate) fn to_wav(score: &ScoreSnapshot) -> Result<Vec<u8>, ProjectError> {
    let (mut plan, events, frames) = build(score)?;
    let audio = musa_audio::render_offline(&mut plan, &events, frames);
    wav_bytes(&audio)
}

/// The same score as a Standard MIDI File.
///
/// # Errors
/// [`ProjectError::Performance`] if lowering fails, or
/// [`ProjectError::Notation`] if a written pitch is outside MIDI's range.
pub(crate) fn to_midi(score: &ScoreSnapshot, mode: musa_render::MidiMode) -> Result<Vec<u8>, ProjectError> {
    let performance = musa_compiler::lower_performance(score, &PerformanceOptions::default())
        .map_err(|error| ProjectError::Performance(error.to_string()))?;
    let options = musa_render::MidiOptions {
        mode,
        ..musa_render::MidiOptions::default()
    };
    musa_render::render_midi(&performance, &options).map_err(|error| ProjectError::Notation(error.to_string()))
}

/// All lanes' events merged into one frame-sorted slice.
fn collect_events(performance: &PerformancePlan) -> Vec<PerformanceEvent> {
    let mut events: Vec<PerformanceEvent> = performance
        .lanes()
        .iter()
        .flat_map(|lane| lane.events().iter().cloned())
        .collect();
    events.sort_by_key(PerformanceEvent::frame);
    events
}

/// Encode rendered audio as a 32-bit float stereo WAV (§13.8).
fn wav_bytes(audio: &musa_audio::RenderedAudio) -> Result<Vec<u8>, ProjectError> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: audio.sample_rate(),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer =
            hound::WavWriter::new(&mut cursor, spec).map_err(|e| ProjectError::Performance(e.to_string()))?;
        for sample in audio.samples() {
            writer
                .write_sample(*sample)
                .map_err(|e| ProjectError::Performance(e.to_string()))?;
        }
        writer
            .finalize()
            .map_err(|e| ProjectError::Performance(e.to_string()))?;
    }
    Ok(cursor.into_inner())
}

/// The performance lowering, formatted as the `render --to performance`
/// debug dump. Lives here because it is the one place that already knows the
/// performance vocabulary; nothing above this crate sees a
/// [`PerformanceEvent`].
///
/// # Errors
/// [`ProjectError::Performance`] if lowering fails.
pub(crate) fn performance_dump(score: &ScoreSnapshot) -> Result<String, ProjectError> {
    use std::fmt::Write as _;

    let performance = lower_performance(score, &PerformanceOptions::default())
        .map_err(|e| ProjectError::Performance(e.to_string()))?;
    let mut out = String::new();
    for lane in performance.lanes() {
        let _ = writeln!(out, "lane {}:", lane.name());
        for event in lane.events() {
            let _ = match event {
                PerformanceEvent::NoteOn { frame, note, instance } => writeln!(
                    out,
                    "  on  {frame} {} {:.2}Hz event-{:x} i{}",
                    note.pitch, note.frequency, note.event.0, instance.0
                ),
                PerformanceEvent::NoteOff { frame, instance } => writeln!(out, "  off {frame} i{}", instance.0),
                PerformanceEvent::Parameter { frame, target, value } => {
                    writeln!(out, "  par {frame} p{} {value}", target.0)
                }
            };
        }
    }
    Ok(out)
}
