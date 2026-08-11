//! The per-voice ADSR envelope (roadmap §13.5, §17.5).
//!
//! **The fundsp decision (§13.6).** Hand-rolled. An ADSR is four linear
//! segments and a state variable; adopting a DSP framework to avoid writing
//! sixty total lines would add a dependency whose types must then be kept
//! private for no gain in correctness or speed. The decision is per-processor
//! and is revisited where the arithmetic stops being trivial — the reverb
//! is the first place that is plausibly true.
//!
//! **Segment timing is frame-exact by construction.** Each segment's
//! per-frame step is `distance / frames`, so a segment declared as `n` frames
//! long is traversed in exactly `n` frames — the property §17.5 asks for is a
//! consequence of how the steps are computed, not something checked after.
//!
//! **The default is the old ramp, made exact.** With no envelope written the
//! settings are a 5 ms attack, no decay, full sustain, and a 50 ms release —
//! the shape of the ramp that preceded it. It is not quite the
//! same samples: the ramp compared an accumulated float against 1.0 and so
//! took 241 frames to finish a 240-frame attack, and this one takes 240.
#![allow(clippy::arithmetic_side_effects)]

/// Which segment an envelope is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Silent and finished.
    Idle,
    /// Rising to full.
    Attack,
    /// Falling to the sustain level.
    Decay,
    /// Holding while the note is held.
    Sustain,
    /// Falling to silence.
    Release,
}

/// An envelope's written shape: times in seconds, sustain as a level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AdsrSettings {
    pub(crate) attack: f32,
    pub(crate) decay: f32,
    pub(crate) sustain: f32,
    pub(crate) release: f32,
}

impl Default for AdsrSettings {
    fn default() -> Self {
        Self {
            attack: 0.005,
            decay: 0.0,
            sustain: 1.0,
            release: 0.05,
        }
    }
}

/// The per-frame steps and segment lengths a shape becomes at a sample rate.
/// Computed once, when the plan is compiled — the render loop only adds,
/// counts down, and compares.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AdsrSteps {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    attack_frames: u32,
    decay_frames: u32,
    release_frames: u32,
}

impl AdsrSteps {
    /// The steps for `settings` at `sample_rate`.
    pub(crate) fn new(settings: AdsrSettings, sample_rate: u32) -> Self {
        let sustain = settings.sustain.clamp(0.0, 1.0);
        // A zero-length decay is legal and means "arrive at sustain now";
        // attack and release are held to at least one frame, because an
        // instantaneous gate is a click.
        let frames = |seconds: f32| (seconds * sample_rate as f32).round().clamp(0.0, u32::MAX as f32) as u32;
        let attack_frames = frames(settings.attack).max(1);
        let release_frames = frames(settings.release).max(1);
        let decay_frames = frames(settings.decay);
        Self {
            attack: 1.0 / attack_frames as f32,
            decay: if decay_frames > 0 {
                (1.0 - sustain) / decay_frames as f32
            } else {
                0.0
            },
            sustain,
            release: sustain / release_frames as f32,
            attack_frames,
            decay_frames,
            release_frames,
        }
    }
}

impl AdsrSteps {
    /// The default shape at the default rate, for a voice that has not been
    /// gated yet and so has no shape of its own.
    pub(crate) const DEFAULT: Self = Self {
        attack: 1.0 / 240.0,
        decay: 0.0,
        sustain: 1.0,
        release: 1.0 / 2400.0,
        attack_frames: 240,
        decay_frames: 0,
        release_frames: 2400,
    };
}

/// One voice's envelope: where it is, how loud it is there, and how many
/// frames of the current segment are left.
///
/// The countdown is what makes the timing exact. Comparing an accumulated
/// `f32` against the segment's end value would end the segment a frame or
/// two late, because a step of `1/n` added `n` times is not exactly 1 — the
/// error is inaudible but it is not what the patch asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Adsr {
    stage: Stage,
    level: f32,
    remaining: u32,
}

impl Adsr {
    /// A finished, silent envelope.
    pub(crate) const IDLE: Self = Self {
        stage: Stage::Idle,
        level: 0.0,
        remaining: 0,
    };

    /// Gate the envelope. The level is kept rather than reset, so a stolen
    /// voice slides into its new note instead of clicking into it.
    pub(crate) fn gate(&mut self, steps: &AdsrSteps) {
        self.stage = Stage::Attack;
        self.remaining = steps.attack_frames;
    }

    /// Release the envelope from wherever it is.
    pub(crate) fn release(&mut self, steps: &AdsrSteps) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
            self.remaining = steps.release_frames;
        }
    }

    /// Whether the envelope has finished and its voice can be reused.
    pub(crate) fn is_idle(&self) -> bool {
        self.stage == Stage::Idle
    }

    /// Advance one frame and return the level to apply to that frame.
    pub(crate) fn tick(&mut self, steps: &AdsrSteps) -> f32 {
        self.remaining = self.remaining.saturating_sub(1);
        match self.stage {
            Stage::Idle => self.level = 0.0,
            Stage::Attack => {
                self.level = (self.level + steps.attack).min(1.0);
                if self.remaining == 0 {
                    self.level = 1.0;
                    self.enter_decay(steps);
                }
            }
            Stage::Decay => {
                self.level = (self.level - steps.decay).max(steps.sustain);
                if self.remaining == 0 {
                    self.level = steps.sustain;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = steps.sustain,
            Stage::Release => {
                self.level -= steps.release;
                if self.remaining == 0 || self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }

    /// Begin the decay, which for a zero-length decay is the sustain.
    fn enter_decay(&mut self, steps: &AdsrSteps) {
        if steps.decay_frames == 0 {
            self.level = steps.sustain;
            self.stage = Stage::Sustain;
        } else {
            self.stage = Stage::Decay;
            self.remaining = steps.decay_frames;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    /// The frame each stage was entered on, gating at frame 0 and releasing
    /// at `release_at`. Stage entry is the boundary §17.5 asks about.
    fn boundaries(settings: AdsrSettings, release_at: Option<usize>) -> Vec<(Stage, usize)> {
        let steps = AdsrSteps::new(settings, RATE);
        let mut envelope = Adsr::IDLE;
        envelope.gate(&steps);
        let mut stage = envelope.stage;
        let mut seen = vec![(stage, 0usize)];
        for frame in 0..RATE as usize * 30 {
            if release_at == Some(frame) {
                envelope.release(&steps);
            }
            envelope.tick(&steps);
            if envelope.stage != stage {
                stage = envelope.stage;
                seen.push((stage, frame));
            }
        }
        seen
    }

    /// §17.5: a segment declared `n` frames long is traversed in `n` frames.
    /// The step is `distance / frames`, so this is arithmetic, not tuning.
    #[test]
    fn each_segment_lasts_exactly_the_frames_it_declares() {
        let settings = AdsrSettings {
            attack: 0.01,
            decay: 0.02,
            sustain: 0.5,
            release: 0.04,
        };
        let attack = (0.01 * f64::from(RATE)) as usize;
        let decay = (0.02 * f64::from(RATE)) as usize;
        let release = (0.04 * f64::from(RATE)) as usize;
        let held = 40_000;
        // Frames are counted from zero, so a segment entered on frame `f`
        // and `n` frames long ends on `f + n - 1`.
        assert_eq!(
            boundaries(settings, Some(held)),
            vec![
                (Stage::Attack, 0),
                (Stage::Decay, attack - 1),
                (Stage::Sustain, attack + decay - 1),
                (Stage::Release, held),
                (Stage::Idle, held + release - 1),
            ]
        );
        // And the same shape at another rate, so nothing above is a
        // coincidence of 48 kHz.
        assert_eq!(AdsrSteps::new(settings, 44_100).attack_frames, 441);
    }

    /// The default matches the ramp it replaced, keeping an unprofiled
    /// render byte-identical to the established golden.
    #[test]
    fn the_default_shape_is_the_ramp_it_replaced() {
        let steps = AdsrSteps::new(AdsrSettings::default(), RATE);
        assert_eq!(
            steps,
            AdsrSteps {
                attack: 1.0 / (RATE / 200) as f32,
                decay: 0.0,
                sustain: 1.0,
                release: 1.0 / (RATE / 20) as f32,
                attack_frames: RATE / 200,
                decay_frames: 0,
                release_frames: RATE / 20,
            }
        );
    }

    /// A zero-length decay is not a division by zero: it arrives at sustain
    /// on the frame after the attack ends.
    #[test]
    fn a_zero_length_decay_arrives_immediately() {
        let settings = AdsrSettings {
            attack: 0.001,
            decay: 0.0,
            sustain: 0.25,
            release: 0.001,
        };
        let attack = (0.001 * f64::from(RATE)) as usize;
        assert_eq!(
            boundaries(settings, None),
            vec![(Stage::Attack, 0), (Stage::Sustain, attack - 1)]
        );
    }

    /// A released envelope frees its voice rather than ringing forever.
    #[test]
    fn a_released_envelope_becomes_idle() {
        let steps = AdsrSteps::new(AdsrSettings::default(), RATE);
        let mut envelope = Adsr::IDLE;
        envelope.gate(&steps);
        for _ in 0..RATE {
            envelope.tick(&steps);
        }
        assert!(
            (envelope.tick(&steps) - 1.0).abs() < f32::EPSILON,
            "a held note sustains"
        );
        envelope.release(&steps);
        for _ in 0..RATE {
            envelope.tick(&steps);
        }
        assert!(envelope.is_idle());
    }
}
