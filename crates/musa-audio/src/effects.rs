//! Time effects and the master limiter (roadmap §13.6).
//!
//! Hand-rolled for the reason [`crate::filter`] gives: these are small, and
//! what matters about them — a delay that is frame-exact, a reverb whose tail
//! decays to a stated time, a limiter whose ceiling is a guarantee — is easier
//! to state and test directly than to obtain from a graph library and then
//! verify anyway. Nothing here allocates outside its constructor: every line
//! is sized in `compile_graph` and never resized (§13.2).
//!
//! FP arithmetic is the subject matter; the workspace lint is allowed at
//! module scope as it is in the other DSP modules.
#![allow(clippy::arithmetic_side_effects)]

/// A fixed-capacity delay line with fractional reads.
///
/// The capacity is what bounds a written `time`: a patch cannot ask for more
/// delay than the line the plan preallocated for it, and the parameter's
/// range says so rather than the line failing at render time.
#[derive(Clone, Debug)]
pub(crate) struct DelayLine {
    buffer: Box<[f32]>,
    write: usize,
}

impl DelayLine {
    /// A silent line holding `frames` samples (at least one).
    pub(crate) fn new(frames: usize) -> Self {
        Self {
            buffer: vec![0.0; frames.max(1)].into_boxed_slice(),
            write: 0,
        }
    }

    /// How many frames it can hold.
    pub(crate) fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Write one sample at the head and advance.
    pub(crate) fn push(&mut self, sample: f32) {
        if let Some(slot) = self.buffer.get_mut(self.write) {
            *slot = sample;
        }
        self.write = (self.write + 1) % self.buffer.len();
    }

    /// The sample `delay` frames behind the head, interpolated linearly
    /// between its two neighbours so a moving delay glides rather than steps.
    ///
    /// A delay of zero reads the sample most recently written.
    pub(crate) fn read(&self, delay: f32) -> f32 {
        let length = self.buffer.len();
        let clamped = delay.clamp(0.0, (length - 1) as f32);
        let whole = clamped.floor();
        let fraction = clamped - whole;
        // `write` points one past the newest sample, so the newest is at
        // `write - 1`; the extra `length` keeps the subtraction in range.
        let base = self.write + length - 1 - whole as usize;
        let near = self.buffer.get(base % length).copied().unwrap_or(0.0);
        let far = self.buffer.get((base + length - 1) % length).copied().unwrap_or(0.0);
        near + fraction * (far - near)
    }
}

/// A feedback delay: the graph's one legal cycle, in one node (§13.3).
///
/// Feedback inside the node rather than around it is what makes the cycle
/// *causal by construction*: the signal that comes back has been delayed, and
/// the amount is written in the patch. A cycle drawn through the graph is
/// legal for the same reason and only when it passes through one of these.
#[derive(Clone, Debug)]
pub(crate) struct Delay {
    line: DelayLine,
    /// Delay in frames, walked toward the written time so a moved knob glides.
    frames: f32,
}

impl Delay {
    /// A delay line long enough for `capacity` seconds.
    pub(crate) fn new(capacity: f32, sample_rate: f32) -> Self {
        Self {
            line: DelayLine::new((capacity * sample_rate) as usize),
            frames: 0.0,
        }
    }

    /// Process one sample. `time` is in seconds, `feedback` a linear ratio.
    ///
    /// The line is read before it is written, so a read position of zero is
    /// already one frame of delay; a written time of zero is that one frame
    /// rather than a wire, because a delay of no frames with feedback would
    /// be a summation with itself.
    pub(crate) fn process(&mut self, input: f32, time: f32, feedback: f32, sample_rate: f32) -> f32 {
        // The read position glides toward the written time, slowly enough to
        // be a pitch bend rather than a click and fast enough that a moved
        // slider feels connected to the sound.
        const GLIDE: f32 = 0.0005;
        let target = time
            .mul_add(sample_rate, -1.0)
            .clamp(0.0, (self.line.capacity() - 1) as f32);
        self.frames = GLIDE.mul_add(target - self.frames, self.frames);
        let delayed = self.line.read(self.frames);
        self.line.push(feedback.clamp(0.0, 0.95).mul_add(delayed, input));
        delayed
    }
}

/// A chorus: a short delay whose length wobbles under an LFO.
///
/// One voice per channel, the right channel a quarter period behind the left,
/// which is what makes a chorus wide rather than merely detuned.
#[derive(Clone, Debug)]
pub(crate) struct Chorus {
    line: DelayLine,
    phase: f64,
}

impl Chorus {
    /// The centre of the modulated delay: short enough to be a chorus rather
    /// than a slapback, long enough to be more than a comb filter.
    const CENTRE: f32 = 0.011;
    /// The deepest wobble a chorus can ask for, in seconds.
    pub(crate) const MAX_DEPTH: f32 = 0.01;

    /// A chorus voice starting at `phase` of its cycle.
    pub(crate) fn new(phase: f64, sample_rate: f32) -> Self {
        Self {
            line: DelayLine::new(((Self::CENTRE + Self::MAX_DEPTH) * sample_rate) as usize + 2),
            phase,
        }
    }

    /// Process one sample: `rate` in Hz, `depth` in seconds.
    pub(crate) fn process(&mut self, input: f32, rate: f32, depth: f32, sample_rate: f32) -> f32 {
        let sweep = (std::f64::consts::TAU * self.phase).sin() as f32;
        self.phase += f64::from(rate) / f64::from(sample_rate);
        self.phase -= self.phase.floor();
        let delay = depth.clamp(0.0, Self::MAX_DEPTH).mul_add(sweep, Self::CENTRE) * sample_rate;
        self.line.push(input);
        self.line.read(delay)
    }
}

/// One comb filter of the reverb network, with its damping one-pole.
#[derive(Clone, Debug)]
struct Comb {
    line: DelayLine,
    store: f32,
}

impl Comb {
    fn process(&mut self, input: f32, feedback: f32, damping: f32) -> f32 {
        let output = self.line.read((self.line.capacity() - 1) as f32);
        self.store = damping.mul_add(self.store - output, output);
        self.line.push(feedback.mul_add(self.store, input));
        output
    }
}

/// One allpass of the reverb network: it diffuses without colouring.
#[derive(Clone, Debug)]
struct Allpass {
    line: DelayLine,
}

impl Allpass {
    const FEEDBACK: f32 = 0.5;

    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.line.read((self.line.capacity() - 1) as f32);
        self.line.push(Self::FEEDBACK.mul_add(delayed, input));
        delayed - input
    }
}

/// An algorithmic reverb: parallel combs into serial allpasses, the
/// Schroeder/Freeverb arrangement §13.6 names.
///
/// `room` sets the comb feedback and so the decay time; `damping` is how much
/// of each pass the one-poles take off the top, which is what makes a long
/// tail sound like a room rather than a ring.
#[derive(Clone, Debug)]
pub(crate) struct Reverb {
    combs: Vec<Comb>,
    allpasses: Vec<Allpass>,
}

impl Reverb {
    /// The network's delays, in frames at 44.1 kHz — the tuning these
    /// numbers are famous for, scaled to whatever rate the plan runs at.
    const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
    const ALLPASSES: [usize; 4] = [556, 441, 341, 225];
    /// How far the right channel's delays are offset from the left's.
    const SPREAD: usize = 23;
    /// Input scaling: eight combs in parallel would otherwise arrive at the
    /// allpasses eight times too loud.
    const INPUT: f32 = 0.015;

    /// Build one channel's network. `offset` spreads it against the other.
    pub(crate) fn new(offset: usize, sample_rate: f32) -> Self {
        let scale = |frames: usize| ((frames as f32 * sample_rate / 44_100.0) as usize).max(1) + offset;
        Self {
            combs: Self::COMBS
                .iter()
                .map(|frames| Comb {
                    line: DelayLine::new(scale(*frames)),
                    store: 0.0,
                })
                .collect(),
            allpasses: Self::ALLPASSES
                .iter()
                .map(|frames| Allpass {
                    line: DelayLine::new(scale(*frames)),
                })
                .collect(),
        }
    }

    /// Process one sample. `room` and `damping` are both `0..=1`.
    pub(crate) fn process(&mut self, input: f32, room: f32, damping: f32) -> f32 {
        // Feedback below 0.7 is a box rather than a room, and at 1 it never
        // decays; the written `room` spans what is between.
        let feedback = room.clamp(0.0, 1.0).mul_add(0.28, 0.7);
        let damping = damping.clamp(0.0, 1.0) * 0.4;
        let scaled = input * Self::INPUT;
        let mut wet = 0.0;
        for comb in &mut self.combs {
            wet += comb.process(scaled, feedback, damping);
        }
        for allpass in &mut self.allpasses {
            wet = allpass.process(wet);
        }
        wet
    }

    /// The offset the right channel of a stereo pair is built with.
    pub(crate) const fn spread() -> usize {
        Self::SPREAD
    }
}

/// A peak limiter: the last thing on master, and a guarantee rather than a
/// preference (§13.6).
///
/// Attack is instantaneous and there is no lookahead, so the ceiling holds
/// exactly, sample for sample, and the master bus adds no latency. The price
/// is a little distortion when it engages hard — which is the right trade for
/// a bus that must be sample-aligned between the offline render and the live
/// stream (§13.8). Recovery is a one-pole so a loud passage does not leave
/// the following bars ducked.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Limiter {
    /// The gain in force, never above one.
    gain: f32,
    /// How far the gain closes on unity per sample once the peak has passed.
    release: f32,
}

impl Limiter {
    /// How long the gain takes to recover, in seconds.
    const RELEASE: f32 = 0.100;

    /// A limiter that is not yet limiting.
    pub(crate) fn new(sample_rate: f32) -> Self {
        Self {
            gain: 1.0,
            release: crate::filter::OnePole::time_coefficient(Self::RELEASE, sample_rate),
        }
    }

    /// The gain to apply to this frame, given the loudest of its channels.
    ///
    /// Stereo is limited as one signal: two independent limiters would move
    /// the image whenever one channel was louder than the other.
    pub(crate) fn gain_for(&mut self, peak: f32, ceiling: f32) -> f32 {
        let ceiling = ceiling.clamp(0.0, 1.0);
        let recovered = self.release.mul_add(1.0 - self.gain, self.gain);
        // Whatever the recovery would have allowed, the ceiling wins: this
        // `min` is the guarantee, and it is why the bound holds for every
        // sample rather than on average.
        let allowed = if peak > ceiling && peak > 0.0 {
            ceiling / peak
        } else {
            1.0
        };
        self.gain = recovered.min(allowed);
        self.gain
    }
}

#[cfg(test)]
mod tests {
    use super::{Delay, Limiter, Reverb};

    const RATE: f32 = 48_000.0;

    /// A delay is frame-exact: an impulse comes back where it was sent to,
    /// not a sample either side of it.
    #[test]
    fn an_impulse_returns_after_exactly_the_frames_it_asked_for() {
        for frames in [1usize, 64, 4801] {
            let time = frames as f32 / RATE;
            let mut delay = Delay::new(2.0, RATE);
            // Settle the glide first: the read position is a parameter, and
            // what is exact is where it settles, not where it starts.
            for _ in 0..200_000 {
                delay.process(0.0, time, 0.0, RATE);
            }
            let mut heard = None;
            for i in 0..frames + 8 {
                let input = if i == 0 { 1.0 } else { 0.0 };
                if delay.process(input, time, 0.0, RATE) > 0.5 {
                    heard = Some(i);
                    break;
                }
            }
            assert_eq!(heard, Some(frames), "a {frames}-frame delay");
        }
    }

    /// Feedback is bounded: even asked for more than one, the line decays.
    #[test]
    fn feedback_stays_bounded_at_any_written_amount() {
        let mut delay = Delay::new(2.0, RATE);
        let mut peak: f32 = 0.0;
        for i in 0..RATE as usize * 4 {
            let input = if i < 64 { 1.0 } else { 0.0 };
            peak = peak.max(delay.process(input, 0.01, 4.0, RATE).abs());
        }
        assert!(peak.is_finite() && peak < 32.0, "peak {peak}");
        let tail = (0..RATE as usize)
            .map(|_| delay.process(0.0, 0.01, 4.0, RATE).abs())
            .fold(0.0f32, f32::max);
        assert!(tail < peak, "a bounded delay decays: {tail} vs {peak}");
    }

    /// A reverb tail decays, and a bigger room decays more slowly.
    #[test]
    fn a_bigger_room_rings_longer() {
        let energy = |room: f32| {
            let mut reverb = Reverb::new(0, RATE);
            for i in 0..RATE as usize {
                reverb.process(if i == 0 { 1.0 } else { 0.0 }, room, 0.2);
            }
            (0..RATE as usize / 2)
                .map(|_| reverb.process(0.0, room, 0.2).abs())
                .fold(0.0f32, f32::max)
        };
        let (small, large) = (energy(0.1), energy(0.95));
        assert!(large > small * 2.0, "small {small}, large {large}");
    }

    /// The ceiling is a guarantee: no input reaches the output above it.
    #[test]
    fn no_signal_passes_the_ceiling() {
        let mut limiter = Limiter::new(RATE);
        let mut worst: f32 = 0.0;
        for i in 0..RATE as usize {
            // A sine that grows to eight times full scale, plus a step that
            // arrives with no warning at all.
            let phase = i as f32 / 128.0;
            let sample = phase
                .sin()
                .mul_add(i as f32 / RATE * 8.0, if i == 12_000 { 40.0 } else { 0.0 });
            worst = worst.max((sample * limiter.gain_for(sample.abs(), 1.0)).abs());
        }
        assert!(worst <= 1.0, "peak {worst}");
    }

    /// A signal below the ceiling is not touched at all.
    #[test]
    fn a_quiet_signal_is_left_exactly_alone() {
        let mut limiter = Limiter::new(RATE);
        for i in 0..1_000 {
            let sample = (i as f32 / 64.0).sin() * 0.5;
            let passed = sample * limiter.gain_for(sample.abs(), 1.0);
            assert!((passed - sample).abs() < f32::EPSILON, "{passed} is not {sample}");
        }
    }
}
