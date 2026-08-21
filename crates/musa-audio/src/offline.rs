//! Deterministic offline rendering (roadmap §13.8: offline == live — this
//! executes the same `RenderPlan::render` a live stream will).

use musa_score::PerformanceEvent;

use crate::plan::{EventSlice, RenderPlan};

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

/// Render `frames` of the plan with `events` (sorted by frame) scheduled.
/// Deterministic: same plan, events, and frame count → identical samples.
pub fn render_offline(plan: &mut RenderPlan, events: &[PerformanceEvent], frames: u64) -> RenderedAudio {
    const CHUNK: u64 = 4096;
    let mut samples = Vec::new();
    let mut rendered = 0u64;
    let mut buffer = vec![0.0f32; (2 * CHUNK) as usize];
    while rendered < frames {
        let count = frames.saturating_sub(rendered).min(CHUNK);
        let len = count.saturating_mul(2) as usize;
        let Some(chunk) = buffer.get_mut(..len) else {
            break;
        };
        plan.render(&EventSlice::new(events), chunk, count as usize);
        samples.extend_from_slice(chunk);
        rendered = rendered.saturating_add(count);
    }
    RenderedAudio {
        samples,
        sample_rate: plan.sample_rate(),
    }
}
