//! Continuous shape as a payload value (docs/rules/kernel/03 `Progress`, §32 Q4).
//!
//! Rational arithmetic on musa's magnitudes is total, so the workspace
//! arithmetic lint is allowed module-wide (the sanctioned pattern, see
//! musa-kernel/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use std::fmt::Write as _;

use num_rational::Ratio;

use crate::occurrence::Canonical;

/// A monotone reparameterization of an occurrence's own span: a
/// piecewise-linear map from normalized local time `u ∈ [0, 1]` to an exact
/// fraction `v ∈ [0, 1]`.
///
/// `Progress` says *how far along* — never *how loud*, *how fast*, or *how
/// high*. What the fraction means is the consuming layer's business (§12): the
/// performance profile maps a hairpin's endpoints to amplitudes, the tempo map
/// maps them to seconds. A dynamic marking is still not a decibel.
///
/// # Why normalized local time
///
/// Because `u` is relative to the occurrence's own span, **every kernel
/// operation acts on the span and leaves these bytes identical**: `scale`
/// multiplies the span, `sequence` translates it, `overlay` does not touch it,
/// and `map_payload` never inspects a payload at all. An absolute-time curve
/// would have to be rewritten by `scale` and `sequence`, which would mean the
/// kernel looking inside payloads to transform them — the §12 violation this
/// design exists to avoid. The invariance is stated as L24 and tested.
///
/// # What this deliberately cannot express
///
/// - **Steps.** Piecewise-*linear* only: no step segments, no jump
///   discontinuities. A sudden change is a fact at a point, the timeline
///   already has one, and `Timeline::prevailing` (D11) already answers what is
///   in force there. Expressing one change two ways is the complecting this
///   type exists to remove.
/// - **Units.** Values are unit-free fractions in `[0, 1]`. A curve "in
///   decibels" is a consumer's mapping of the endpoints applied to this
///   fraction.
/// - **Shapes that are not monotone in `u`.** Vibrato and an LFO are periodic,
///   not progress. They are a different construct; if one is ever wanted it
///   arrives with its own name and its own evidence.
/// - **An easing catalogue.** No `ease_in`, no exponential, no Bézier. Any of
///   those is approximated by breakpoints when something needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    /// Breakpoints `(u, v)`, strictly increasing in `u`, with `u₀ = 0` and
    /// `uₙ = 1`. Non-empty by construction and at least two long.
    points: Vec<(Ratio<i64>, Ratio<i64>)>,
}

impl Progress {
    /// The straight line from 0 to 1: `at(u) = u`.
    pub fn linear() -> Self {
        Self {
            points: vec![
                (Ratio::new_raw(0, 1), Ratio::new_raw(0, 1)),
                (Ratio::new_raw(1, 1), Ratio::new_raw(1, 1)),
            ],
        }
    }

    /// A curve through `points`, or `None` if they are not a curve.
    ///
    /// Construction is the only place this can fail (*A Philosophy of Software Design* ch. 6): a
    /// `Progress` that exists is well-formed, so no consumer diagnoses one.
    /// The rejected shapes are those that would make `at` ill-defined or
    /// non-monotone:
    ///
    /// - fewer than two points, or `u` not strictly increasing;
    /// - `u₀ ≠ 0` or `uₙ ≠ 1` — the curve must span its occurrence;
    /// - any `v` outside `[0, 1]`, or `v` decreasing — progress does not run
    ///   backwards.
    pub fn piecewise(points: impl IntoIterator<Item = (Ratio<i64>, Ratio<i64>)>) -> Option<Self> {
        let points: Vec<(Ratio<i64>, Ratio<i64>)> = points.into_iter().collect();
        let (first, last) = (points.first()?, points.last()?);
        if points.len() < 2 || first.0 != Ratio::ZERO || last.0 != Ratio::ONE {
            return None;
        }
        let well_formed = points.windows(2).all(|pair| match pair {
            [(u0, v0), (u1, v1)] => u0 < u1 && v0 <= v1,
            _ => false,
        });
        let bounded = points.iter().all(|(_, v)| *v >= Ratio::ZERO && *v <= Ratio::ONE);
        (well_formed && bounded).then_some(Self { points })
    }

    /// The fraction reached at normalized local time `u`.
    ///
    /// `u` is clamped to `[0, 1]`: asking outside the occurrence's own span is
    /// a question about its endpoints, not an error.
    pub fn at(&self, u: Ratio<i64>) -> Ratio<i64> {
        let u = u.clamp(Ratio::ZERO, Ratio::ONE);
        for pair in self.points.windows(2) {
            let [(u0, v0), (u1, v1)] = pair else { continue };
            if u > *u1 {
                continue;
            }
            // `u1 > u0` by construction, so the division is defined.
            return *v0 + (*v1 - *v0) * ((u - *u0) / (*u1 - *u0));
        }
        // Unreachable while `uₙ = 1` and `u ≤ 1`, but a total function is
        // cheaper than a panic the invariant already rules out.
        self.points.last().map_or(Ratio::ONE, |(_, v)| *v)
    }

    /// The breakpoints, in order.
    pub fn points(&self) -> &[(Ratio<i64>, Ratio<i64>)] {
        &self.points
    }

    /// Whether this is the straight line.
    pub fn is_linear(&self) -> bool {
        *self == Self::linear()
    }
}

impl Canonical for Progress {
    const OWNER_TYPE_ID: &'static str = "musa.kernel.Progress";
    const QUOTIENT_VERSION: u32 = 1;

    /// `u:v` pairs joined by `,`, each rational as `num/den` (docs/rules/kernel/05
    /// N3). Exact, float-free, and complete for `Progress` equality because
    /// the breakpoints are strictly increasing.
    fn canonical_key(&self) -> String {
        let mut key = String::with_capacity(self.points.len() * 12);
        for (index, (u, v)) in self.points.iter().enumerate() {
            if index > 0 {
                key.push(',');
            }
            // The write cannot fail: the target is a `String`.
            let _ = write!(key, "{}/{}:{}/{}", u.numer(), u.denom(), v.numer(), v.denom());
        }
        key
    }
}

#[cfg(test)]
mod tests {
    use super::Progress;
    use num_rational::Ratio;

    fn r(numer: i64, denom: i64) -> Ratio<i64> {
        Ratio::new(numer, denom)
    }

    #[test]
    fn linear_progress_is_the_identity_on_normalized_time() {
        let curve = Progress::linear();
        assert_eq!(curve.at(r(0, 1)), r(0, 1));
        assert_eq!(curve.at(r(1, 3)), r(1, 3));
        assert_eq!(curve.at(r(1, 1)), r(1, 1));
    }

    #[test]
    fn a_bent_curve_interpolates_within_each_segment() {
        let curve = Progress::piecewise([(r(0, 1), r(0, 1)), (r(1, 2), r(1, 4)), (r(1, 1), r(1, 1))]);
        let Some(curve) = curve else {
            assert!(curve.is_some(), "these breakpoints are well formed");
            return;
        };
        assert_eq!(curve.at(r(1, 4)), r(1, 8), "half way through the first segment");
        assert_eq!(curve.at(r(1, 2)), r(1, 4), "the breakpoint itself");
        assert_eq!(curve.at(r(3, 4)), r(5, 8), "half way through the second");
    }

    #[test]
    fn progress_outside_its_own_span_clamps_to_the_endpoints() {
        let curve = Progress::linear();
        assert_eq!(curve.at(r(-1, 2)), r(0, 1));
        assert_eq!(curve.at(r(3, 2)), r(1, 1));
    }

    #[test]
    fn ill_formed_breakpoints_are_rejected_at_construction() {
        assert!(Progress::piecewise([]).is_none(), "empty");
        assert!(Progress::piecewise([(r(0, 1), r(0, 1))]).is_none(), "one point");
        assert!(
            Progress::piecewise([(r(1, 4), r(0, 1)), (r(1, 1), r(1, 1))]).is_none(),
            "does not start at 0"
        );
        assert!(
            Progress::piecewise([(r(0, 1), r(0, 1)), (r(1, 2), r(1, 2))]).is_none(),
            "does not reach 1"
        );
        assert!(
            Progress::piecewise([
                (r(0, 1), r(0, 1)),
                (r(1, 2), r(1, 1)),
                (r(1, 2), r(1, 2)),
                (r(1, 1), r(1, 1))
            ])
            .is_none(),
            "u repeats"
        );
        assert!(
            Progress::piecewise([(r(0, 1), r(1, 2)), (r(1, 1), r(1, 4))]).is_none(),
            "v runs backwards"
        );
        assert!(
            Progress::piecewise([(r(0, 1), r(0, 1)), (r(1, 1), r(2, 1))]).is_none(),
            "v leaves [0, 1]"
        );
    }
}
