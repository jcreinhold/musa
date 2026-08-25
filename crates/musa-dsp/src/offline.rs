//! Deterministic offline rendering (roadmap §13.8: offline == live — this
//! executes the same repeated one-frame operation as the live callback).

use crate::PreparedAudio;

/// Interleaved stereo f32 samples plus the sample rate.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedAudio {
    samples: Vec<f32>,
    sample_rate: u32,
}

impl RenderedAudio {
    /// Interleaved stereo samples.
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Samples per second.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

/// Render the prepared machine's remaining finite extent.
/// Deterministic: equal preparation and starting state yield equal samples.
pub fn render_offline(audio: &mut PreparedAudio) -> RenderedAudio {
    const CHUNK: u64 = 4096;
    let mut samples = Vec::new();
    let frames = audio.total_frames().saturating_sub(audio.position());
    let sample_rate = audio.sample_rate();
    let mut rendered = 0_u64;
    let mut buffer = vec![0.0f32; (2 * CHUNK) as usize];
    while rendered < frames {
        let count = frames.saturating_sub(rendered).min(CHUNK);
        let len = count.saturating_mul(2) as usize;
        let Some(chunk) = buffer.get_mut(..len) else {
            break;
        };
        audio.render(chunk);
        samples.extend_from_slice(chunk);
        rendered = rendered.saturating_add(count);
    }
    RenderedAudio { samples, sample_rate }
}
