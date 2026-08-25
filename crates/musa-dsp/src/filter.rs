//! Filters: a one-pole low-pass and an RBJ biquad (roadmap §13.6, §17.5).
//!
//! **The fundsp decision (§13.6).** Hand-rolled, like the envelope. The
//! coefficient formulas below are the published RBJ cookbook ones and the
//! state is four floats; what a framework would supply is exactly what is
//! written here, minus the ability to say what happens at the boundaries.
//!
//! **Stability is by construction, not by inspection.** Cutoff is clamped to
//! `[10 Hz, 0.45·sr]` and `q` to a sane band *before* the coefficients are
//! computed, and a non-finite request is refused rather than propagated, so
//! no setting a modulation can reach produces a NaN in the output (§17.5).
//! Coefficients are recomputed after each frame's smoothed parameters arrive
//! (see `plan.rs`), which is what keeps a swept cutoff from zippering.
#![allow(clippy::arithmetic_side_effects)]

use crate::spec::FilterKind;

/// A one-pole low-pass: `y += a·(x − y)`.
///
/// 6 dB/octave, no resonance, one multiply-add per sample. Used both as an
/// audio filter and as the parameter smoother.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OnePole {
    state: f32,
}

impl OnePole {
    /// The coefficient for a `cutoff`-Hz corner at `sample_rate`.
    ///
    /// Returns 1 (no filtering) when the request is not a usable frequency,
    /// which is the honest answer for "smooth over no time".
    pub(crate) fn coefficient(cutoff: f32, sample_rate: f32) -> f32 {
        if !cutoff.is_finite() || cutoff <= 0.0 || sample_rate <= 0.0 {
            return 1.0;
        }
        let a = 1.0 - (-std::f32::consts::TAU * cutoff / sample_rate).exp();
        a.clamp(0.0, 1.0)
    }

    /// The coefficient for a smoothing *time* rather than a frequency: the
    /// value covers ~63% of the distance in `seconds`.
    pub(crate) fn time_coefficient(seconds: f32, sample_rate: f32) -> f32 {
        if !seconds.is_finite() || seconds <= 0.0 {
            return 1.0;
        }
        Self::coefficient(1.0 / (std::f32::consts::TAU * seconds), sample_rate)
    }

    /// Filter one sample with a precomputed coefficient.
    pub(crate) fn process(&mut self, input: f32, coefficient: f32) -> f32 {
        self.state = coefficient.mul_add(input - self.state, self.state);
        self.state
    }
}

/// A biquad's normalized coefficients.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Coefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Default for Coefficients {
    /// Identity: the filter passes its input untouched until it is told a
    /// cutoff.
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

impl Coefficients {
    /// The RBJ cookbook coefficients for `kind` at `cutoff`/`q`.
    ///
    /// The clamping is the stability guarantee: a cutoff at or above Nyquist
    /// makes `cos(w0)` meaningless and a `q` at zero divides by zero, and
    /// both are reachable from a modulation the user is allowed to write.
    pub(crate) fn new(kind: FilterKind, cutoff: f32, q: f32, sample_rate: f32) -> Self {
        if !cutoff.is_finite() || !q.is_finite() || sample_rate <= 0.0 {
            return Self::default();
        }
        let cutoff = cutoff.clamp(10.0, 0.45 * sample_rate);
        let q = q.clamp(0.05, 20.0);
        let w0 = std::f32::consts::TAU * cutoff / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha;
        let (b0, b1, b2) = match kind {
            FilterKind::LowPass => ((1.0 - cos) * 0.5, 1.0 - cos, (1.0 - cos) * 0.5),
            FilterKind::HighPass => (f32::midpoint(1.0, cos), -(1.0 + cos), f32::midpoint(1.0, cos)),
        };
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha) / a0,
        }
    }
}

impl Coefficients {
    /// The per-sample step that walks these coefficients to `target` over
    /// `count` samples.
    ///
    /// Interpolating the coefficients rather than jumping to them is what
    /// makes a swept cutoff a sweep when the target changes between frames.
    pub(crate) fn step_to(&self, target: &Self, count: usize) -> Self {
        let steps = count.max(1) as f32;
        Self {
            b0: (target.b0 - self.b0) / steps,
            b1: (target.b1 - self.b1) / steps,
            b2: (target.b2 - self.b2) / steps,
            a1: (target.a1 - self.a1) / steps,
            a2: (target.a2 - self.a2) / steps,
        }
    }

    /// Take one interpolation step.
    pub(crate) fn advance(&mut self, step: &Self) {
        self.b0 += step.b0;
        self.b1 += step.b1;
        self.b2 += step.b2;
        self.a1 += step.a1;
        self.a2 += step.a2;
    }
}

/// One channel of biquad state (direct form I).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Biquad {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    /// Filter one sample.
    pub(crate) fn process(&mut self, input: f32, c: &Coefficients) -> f32 {
        // Fused throughout: a biquad is the one place in the graph where
        // the accumulated rounding of five products is worth avoiding.
        let feed_forward = c.b2.mul_add(self.x2, c.b1.mul_add(self.x1, c.b0 * input));
        let output = c.a2.mul_add(-self.y2, c.a1.mul_add(-self.y1, feed_forward));
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    /// An amplitude ratio in decibels.
    fn decibels(ratio: f32) -> f32 {
        20.0 * ratio.log10()
    }

    /// The steady-state amplitude a filter shows a sine at `frequency`.
    fn response(kind: FilterKind, cutoff: f32, q: f32, frequency: f32) -> f32 {
        let coefficients = Coefficients::new(kind, cutoff, q, RATE);
        let mut filter = Biquad::default();
        let mut peak = 0.0f32;
        let frames = (RATE / frequency * 40.0) as usize;
        for frame in 0..frames {
            let phase = std::f32::consts::TAU * frequency * frame as f32 / RATE;
            let output = filter.process(phase.sin(), &coefficients);
            // The first ten periods are the transient; the rest is the answer.
            if frame > frames / 4 {
                peak = peak.max(output.abs());
            }
        }
        peak
    }

    /// §17.5: a Butterworth-`q` biquad is 3 dB down at its cutoff, and the
    /// stopband keeps falling. The tolerance is measurement, not slack: the
    /// peak of a finite window of a sine lands just under the true peak.
    #[test]
    fn a_biquad_is_three_decibels_down_at_its_cutoff() {
        let q = std::f32::consts::FRAC_1_SQRT_2;
        let at_cutoff = response(FilterKind::LowPass, 1_000.0, q, 1_000.0);
        assert!(
            (decibels(at_cutoff) + 3.0).abs() < 0.3,
            "expected −3 dB at the cutoff, got {} dB",
            decibels(at_cutoff)
        );
        assert!(
            response(FilterKind::LowPass, 1_000.0, q, 100.0) > 0.98,
            "the passband passes"
        );
        assert!(
            response(FilterKind::LowPass, 1_000.0, q, 8_000.0) < 0.02,
            "three octaves up is 36 dB down"
        );
        let high = response(FilterKind::HighPass, 1_000.0, q, 1_000.0);
        assert!(
            (decibels(high) + 3.0).abs() < 0.3,
            "the high-pass is the same filter reflected: {} dB",
            decibels(high)
        );
        assert!(response(FilterKind::HighPass, 1_000.0, q, 100.0) < 0.02);
    }

    /// Resonance is the point of `q`: above Butterworth the cutoff lifts.
    #[test]
    fn resonance_lifts_the_cutoff() {
        let flat = response(FilterKind::LowPass, 1_000.0, std::f32::consts::FRAC_1_SQRT_2, 1_000.0);
        let resonant = response(FilterKind::LowPass, 1_000.0, 8.0, 1_000.0);
        assert!(resonant > flat * 4.0, "{resonant} should tower over {flat}");
    }

    /// §17.5: the one-pole's impulse response is a decaying exponential that
    /// sums to unity — the filter neither loses nor invents energy.
    #[test]
    fn a_one_pole_decays_to_unity() {
        let coefficient = OnePole::coefficient(1_000.0, RATE);
        let mut filter = OnePole::default();
        let mut sum = filter.process(1.0, coefficient);
        let mut previous = sum;
        for _ in 0..10_000 {
            let sample = filter.process(0.0, coefficient);
            // `<=` rather than `<`: the tail eventually reaches zero and
            // stays there, which is decay finishing, not decay stopping.
            assert!(sample <= previous, "the tail must decay monotonically");
            previous = sample;
            sum += sample;
        }
        assert!((sum - 1.0).abs() < 1e-3, "impulse response sums to {sum}");
    }

    /// §17.5: no setting a modulation can reach produces a non-finite sample.
    /// Cutoffs beyond Nyquist and a zero `q` are both reachable from a legal
    /// `modulate`, so both must be survivable rather than merely unlikely.
    #[test]
    fn no_reachable_setting_produces_a_non_finite_sample() {
        let adversarial = [
            (f32::NAN, 1.0),
            (f32::INFINITY, 1.0),
            (-1.0, 1.0),
            (0.0, 0.0),
            (RATE, 0.0),
            (RATE * 10.0, f32::NAN),
            (1_000.0, f32::INFINITY),
        ];
        for (cutoff, q) in adversarial {
            for kind in [FilterKind::LowPass, FilterKind::HighPass] {
                let coefficients = Coefficients::new(kind, cutoff, q, RATE);
                let mut filter = Biquad::default();
                for frame in 0..10_000 {
                    let sample = filter.process(if frame % 2 == 0 { 1.0 } else { -1.0 }, &coefficients);
                    assert!(sample.is_finite(), "cutoff {cutoff}, q {q} produced {sample}");
                }
            }
        }
    }
}
