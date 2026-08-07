//! The compile → lower → graph → render chain behind `render --to wav`.
//!
//! TODO(prompt-19): move this orchestration to `musa-project` when the
//! session facade exists; it is deliberately tiny so the migration is
//! mechanical.

use musa_compiler::{PerformanceEvent, PerformanceOptions, ScoreSnapshot, lower_performance};

/// A WAV render of a compiled score (bytes + description for errors).
pub(crate) struct RenderedWav {
    pub(crate) bytes: Vec<u8>,
}

/// Compile the chain: performance lowering → default instrument graph →
/// deterministic offline render → 32-bit float stereo WAV bytes.
///
/// # Errors
/// A string description (the CLI reports it verbatim) when performance
/// lowering or graph compilation fails.
pub(crate) fn render_to_wav(score: &ScoreSnapshot) -> Result<RenderedWav, String> {
    let (mut plan, events, frames) = build_playback(score)?;
    let audio = musa_audio::render_offline(&mut plan, &events, frames);
    Ok(RenderedWav {
        bytes: wav_bytes(&audio)?,
    })
}

/// The same chain prepared for live playback (§13.8: one preparation,
/// offline or live).
///
/// # Errors
/// As [`render_to_wav`].
pub(crate) fn prepare_playback(score: &ScoreSnapshot) -> Result<musa_engine::PreparedPlaybackPlan, String> {
    let (plan, events, frames) = build_playback(score)?;
    Ok(musa_engine::PreparedPlaybackPlan::new(plan, events, frames))
}

/// Performance lowering → default instrument graph → scheduled events.
fn build_playback(score: &ScoreSnapshot) -> Result<(musa_audio::RenderPlan, Vec<PerformanceEvent>, u64), String> {
    let performance = lower_performance(score, &PerformanceOptions::default()).map_err(|e| e.to_string())?;
    let sample_rate = performance_options_rate();
    let events = collect_events(&performance);
    let spec = musa_audio::poly_sine_spec(16);
    let options = musa_audio::GraphOptions {
        sample_rate,
        block_size: 128,
    };
    let plan = musa_audio::compile_graph(&spec, &options).map_err(|e| e.to_string())?;
    let tail = u64::from(sample_rate); // 1 s release tail until envelopes exist
    let frames = events
        .iter()
        .map(PerformanceEvent::frame)
        .max()
        .unwrap_or(0)
        .saturating_add(tail);
    Ok((plan, events, frames))
}

fn performance_options_rate() -> u32 {
    PerformanceOptions::default().sample_rate
}

/// All lanes' events merged into one frame-sorted slice.
fn collect_events(performance: &musa_compiler::PerformancePlan) -> Vec<PerformanceEvent> {
    let mut events: Vec<PerformanceEvent> = performance
        .lanes()
        .iter()
        .flat_map(|lane| lane.events().iter().cloned())
        .collect();
    events.sort_by_key(PerformanceEvent::frame);
    events
}

/// Encode rendered audio as a 32-bit float stereo WAV (§13.8).
///
/// # Errors
/// Propagates encoder failures (impossible for an in-memory cursor, but the
/// writer API is fallible).
fn wav_bytes(audio: &musa_audio::RenderedAudio) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: audio.sample_rate(),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).map_err(|e| e.to_string())?;
        for sample in audio.samples() {
            writer.write_sample(*sample).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
    }
    Ok(cursor.into_inner())
}
