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

/// The master mix and one aligned stem per declared tap.
///
/// Every buffer here starts at the same frame and has the same length, so
/// the stems line up with the master and with each other without any
/// alignment step of their own.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedMultitrack {
    master: RenderedAudio,
    stems: Vec<RenderedStem>,
}

impl RenderedMultitrack {
    /// The mix, byte-for-byte what [`render_offline`] produces from the same
    /// preparation and starting state.
    pub fn master(&self) -> &RenderedAudio {
        &self.master
    }

    /// One stem per tap, in [`PreparedAudio::taps`] order.
    pub fn stems(&self) -> &[RenderedStem] {
        &self.stems
    }

    /// Frames in the master and in every stem.
    pub fn frames(&self) -> u64 {
        self.master.samples.len() as u64 / 2
    }
}

/// One tap's rendered signal, named by the route it was read from.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedStem {
    tap: crate::AudioTap,
    audio: RenderedAudio,
}

impl RenderedStem {
    /// Which declared route this is.
    pub fn tap(&self) -> &crate::AudioTap {
        &self.tap
    }

    /// The signal that route carried.
    pub fn audio(&self) -> &RenderedAudio {
        &self.audio
    }
}

/// Render the master and every declared tap in one traversal.
///
/// One traversal rather than one render per stem is what makes the outputs
/// aligned by construction: a tap is read from the buffer the same frame
/// wrote, so no stem can drift from the master or from another stem, and no
/// decoded asset is stored twice.
///
/// Memory is bounded by the render itself: `2 · frames · (1 + taps)` samples
/// of output, plus the one stereo scratch frame per tap that this function
/// allocates once and reuses.
pub fn render_offline_multitrack(audio: &mut PreparedAudio) -> RenderedMultitrack {
    let frames = audio.total_frames().saturating_sub(audio.position());
    let sample_rate = audio.sample_rate();
    let taps = audio.taps().to_vec();
    let capacity = usize::try_from(frames.saturating_mul(2)).unwrap_or(usize::MAX);
    let mut master = Vec::with_capacity(capacity);
    // Not `vec![Vec::with_capacity(..); n]`: cloning a vector copies its
    // length, not its capacity, so every stem but the first would have grown
    // its way to the same size and peaked at roughly twice the bound below.
    let mut stems = (0..taps.len())
        .map(|_| Vec::with_capacity(capacity))
        .collect::<Vec<Vec<f32>>>();
    let mut frame = vec![[0.0f32; 2]; taps.len()];
    for _ in 0..frames {
        let [left, right] = audio.step_with_taps(&mut frame);
        master.push(left);
        master.push(right);
        for (samples, [left, right]) in stems.iter_mut().zip(frame.iter().copied()) {
            samples.push(left);
            samples.push(right);
        }
    }
    RenderedMultitrack {
        master: RenderedAudio {
            samples: master,
            sample_rate,
        },
        stems: taps
            .into_iter()
            .zip(stems)
            .map(|(tap, samples)| RenderedStem {
                tap,
                audio: RenderedAudio { samples, sample_rate },
            })
            .collect(),
    }
}
