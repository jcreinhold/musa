//! Groove: where the beat *is*, as against where the notes are written.
//!
//! Most of the world's recorded music is not performed the way it is written.
//! A written pair of eighths sounds long-short; a house bass lands a hair
//! ahead of the offbeat; a shuffle is straight eighths with *swing* at the
//! top of the page. Until this module musa could express none of it, because
//! the notated duration *was* the performed duration, exactly, forever.
//!
//! That is roadmap §2's notated-duration ≠ performed-duration row, and a
//! groove is its first implementation. The one idea:
//!
//! > A groove does not change *when the notes are*. It changes *where the
//! > beat is*.
//!
//! So the type is a warp on beat time, composed strictly **before** tempo:
//!
//! ```text
//! written beat ──groove──▶ performed beat ──tempo──▶ seconds ──▶ frames
//! ```
//!
//! Composing the other way would put the groove in seconds, so a shuffle
//! would straighten out when the band sped up. Musicians do the opposite.
//!
//! Two invariants make the warp safe to apply anywhere, and both are property
//! tests rather than comments:
//!
//! - **Monotone.** `s ≤ t` implies `warp(s) ≤ warp(t)`, so notes never
//!   reorder and no duration can come out negative.
//! - **Boundary-preserving.** Every boundary of the grid a groove operates on
//!   is a fixed point, so the downbeat never moves and a groove cannot drift
//!   a piece off its own barlines.
//!
//! Everything here is `Ratio<i64>`. "Feel" is not an excuse to leave exact
//! time (§4): a swing is `2/3`, and floats still appear no earlier than the
//! existing `Beat → Second` edge.
//!
//! **A groove must never reach notation.** There is no swung notation — that
//! is the whole reason one writes straight eighths and *swing* above them —
//! so an engraver that read this module would be printing an interpretation.
//! `plan.rs` does not, and the test is that every notation golden stays
//! byte-identical.

// Rational arithmetic on `Ratio<i64>` is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use num_rational::Ratio;
use serde::{Deserialize, Serialize};

use crate::score::Meter;
use crate::time::MusicalTime;

/// One entry in the groove vocabulary: what it is called and what it takes.
///
/// The same shape as `marks.rs`, for the same reason — a vocabulary is a
/// table, and a row is added by the change that has a piece needing it.
#[derive(Clone, Copy, Debug)]
pub struct GrooveDef {
    /// The name the composer writes.
    pub name: &'static str,
    /// The settings this groove reads, in the order a diagnostic lists them.
    pub params: &'static [&'static str],
}

/// Every groove musa reads.
pub(crate) const VOCABULARY: &[GrooveDef] = &[
    GrooveDef {
        name: "straight",
        params: &[],
    },
    GrooveDef {
        name: "swing",
        params: &["ratio"],
    },
    GrooveDef {
        name: "push",
        params: &["grid", "by"],
    },
];

/// Every groove's name, for the diagnostic that lists them.
pub fn names() -> Vec<&'static str> {
    VOCABULARY.iter().map(|def| def.name).collect()
}

/// Look a groove up by the name the composer wrote.
pub fn lookup(name: &str) -> Option<&'static GrooveDef> {
    VOCABULARY.iter().find(|def| def.name == name)
}

/// How a part's beat is displaced from the page's.
///
/// Constructed only through [`Groove::swing`] and [`Groove::push`], which
/// reject the arguments that would break monotonicity. That is why [`warp`]
/// takes no error and needs no clamp: an unrepresentable groove never exists.
///
/// [`warp`]: Groove::warp
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Groove(Kind);

/// The three warps. Private: a groove is asked what it does to a time, never
/// which of these it is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
enum Kind {
    /// The identity, and the default — so a part can override an ensemble
    /// groove by naming it.
    #[default]
    Straight,
    /// Written pairs sound long-short: the first of each pair takes `first`
    /// of the pair's length.
    Swing { first: Ratio<i64> },
    /// The offbeats of the `grid` move by `by`; its downbeats do not.
    Push { grid: Ratio<i64>, by: Ratio<i64> },
}

impl Groove {
    /// No displacement at all.
    pub const STRAIGHT: Self = Self(Kind::Straight);

    /// Swing, where `first` is the fraction of each written pair the first
    /// note takes: `2/3` is the ordinary long-short of jazz and blues.
    ///
    /// Returns `None` outside `0 < first < 1`, which is exactly the range
    /// where the warp stays strictly increasing. `1/2` is admitted and is the
    /// identity, because a piece is allowed to say "even eighths" out loud.
    pub fn swing(first: Ratio<i64>) -> Option<Self> {
        (first > Ratio::ZERO && first < Ratio::ONE).then_some(Self(Kind::Swing { first }))
    }

    /// Move the offbeats of the `grid` by `by` — negative early, positive
    /// late. `grid = 1/8, by = -1/64` is the pushed offbeat of house and
    /// garage; a positive `by` is the same operation laid back.
    ///
    /// Returns `None` on either of two counts, both found by the property
    /// tests below rather than reasoned out in advance:
    ///
    /// - `|by| ≥ grid`. The displaced point reaches its neighbour and the
    ///   warp stops being injective, which is what would let a groove reorder
    ///   notes.
    /// - The pattern does not tile the whole note. A push repeats every
    ///   `2 × grid`, so unless a whole number of those fits in a whole note
    ///   the pattern lands differently in each bar and the downbeat it claims
    ///   to fix drifts. `1/8` and `1/12` tile; `1/3` does not.
    pub fn push(grid: Ratio<i64>, by: Ratio<i64>) -> Option<Self> {
        let positive = grid > Ratio::ZERO;
        let injective = by < grid && -by < grid;
        let tiles = positive && (Ratio::ONE / (grid * 2)).is_integer();
        (positive && injective && tiles).then_some(Self(Kind::Push { grid, by }))
    }

    /// Whether this groove is the identity, so callers that would rather not
    /// walk a whole piece can skip it.
    pub fn is_straight(self) -> bool {
        matches!(self.0, Kind::Straight)
    }

    /// Where the written instant `at` is actually played.
    ///
    /// `meter` is the meter in force there, which is what tells a swing how
    /// long a "pair" is: the beat unit, so two eighths in 4/4 and two
    /// sixteenths in 6/8. Reading the meter per instant rather than once is
    /// what makes a groove follow a meter change without knowing there was
    /// one.
    pub fn warp(self, meter: Meter, at: MusicalTime) -> MusicalTime {
        let t = at.as_ratio();
        if t < Ratio::ZERO {
            return at;
        }
        let warped = match self.0 {
            Kind::Straight => return at,
            Kind::Swing { first } => {
                // The pair is the beat unit: `1/4` in 4/4, `1/8` in 6/8.
                let pair = Ratio::new(1, i64::from(meter.denominator().max(1)));
                let half = pair / 2;
                let (whole, offset) = split(t, pair);
                // Two linear pieces meeting where the first note ends, so
                // both ends of the pair are fixed and the middle moves.
                let moved = if offset <= half {
                    offset * (first * 2)
                } else {
                    first * pair + (offset - half) * ((Ratio::ONE - first) * 2)
                };
                whole + moved
            }
            Kind::Push { grid, by } => {
                // Fixed points are the even multiples of `grid`; the odd one
                // between them is displaced, and the two segments either side
                // of it stretch to meet it.
                let span = grid * 2;
                let (whole, offset) = split(t, span);
                let displaced = grid + by;
                let moved = if offset <= grid {
                    offset * (displaced / grid)
                } else {
                    displaced + (offset - grid) * ((span - displaced) / grid)
                };
                whole + moved
            }
        };
        MusicalTime::new(warped)
    }
}

/// Split `t` into the last grid boundary at or below it and the offset past
/// it, so a warp can be written once for one cell and repeated.
fn split(t: Ratio<i64>, grid: Ratio<i64>) -> (Ratio<i64>, Ratio<i64>) {
    let cells = (t / grid).floor();
    let start = cells * grid;
    (start, t - start)
}

#[cfg(test)]
mod tests {
    // A failure of these is a bug in the fixture, not in a caller's input.
    #![allow(clippy::expect_used)]

    use proptest::prelude::*;

    use super::*;

    fn at(whole_notes: Ratio<i64>) -> MusicalTime {
        MusicalTime::new(whole_notes)
    }

    /// The example the design is written around: written `0, 1/8, 1/4, 3/8`
    /// under a 2:1 swing sound at `0, 1/6, 1/4, 5/12`.
    #[test]
    fn a_two_to_one_swing_sounds_where_the_design_says() {
        let groove = Groove::swing(Ratio::new(2, 3)).expect("2/3 is a swing");
        let meter = Meter::default();
        let sounded = |written: (i64, i64)| groove.warp(meter, at(Ratio::new(written.0, written.1))).as_ratio();
        assert_eq!(sounded((0, 1)), Ratio::new(0, 1));
        assert_eq!(sounded((1, 8)), Ratio::new(1, 6));
        assert_eq!(sounded((1, 4)), Ratio::new(1, 4));
        assert_eq!(sounded((3, 8)), Ratio::new(5, 12));
    }

    /// A groove reads the meter, so `6/8` swings sixteenths without being
    /// told the meter changed.
    #[test]
    fn the_pair_is_the_meter_s_beat_unit() {
        let groove = Groove::swing(Ratio::new(2, 3)).expect("2/3 is a swing");
        let compound = Meter::new(6, 8);
        // In 6/8 the pair is two sixteenths, so the first sixteenth stretches
        // and the eighth-note boundary stays put.
        assert_eq!(
            groove.warp(compound, at(Ratio::new(1, 16))).as_ratio(),
            Ratio::new(1, 12)
        );
        assert_eq!(groove.warp(compound, at(Ratio::new(1, 8))).as_ratio(), Ratio::new(1, 8));
    }

    /// A pushed offbeat moves and its neighbours do not.
    #[test]
    fn a_push_moves_the_offbeat_and_leaves_the_downbeat() {
        let groove = Groove::push(Ratio::new(1, 8), Ratio::new(-1, 64)).expect("an eighth of a push");
        let meter = Meter::default();
        let sounded = |written: (i64, i64)| groove.warp(meter, at(Ratio::new(written.0, written.1))).as_ratio();
        assert_eq!(sounded((0, 1)), Ratio::new(0, 1));
        assert_eq!(sounded((1, 8)), Ratio::new(1, 8) - Ratio::new(1, 64));
        assert_eq!(sounded((1, 4)), Ratio::new(1, 4));
        assert_eq!(sounded((3, 8)), Ratio::new(3, 8) - Ratio::new(1, 64));
    }

    /// The two constructors refuse exactly the arguments that would let a
    /// warp fold time over itself.
    #[test]
    fn an_unusable_groove_cannot_be_built() {
        assert!(Groove::swing(Ratio::ZERO).is_none());
        assert!(Groove::swing(Ratio::ONE).is_none());
        assert!(Groove::swing(Ratio::new(3, 2)).is_none());
        assert!(Groove::push(Ratio::ZERO, Ratio::ZERO).is_none());
        // `by` reaching `at` puts the offbeat on top of its neighbour.
        assert!(Groove::push(Ratio::new(1, 8), Ratio::new(1, 8)).is_none());
        assert!(Groove::push(Ratio::new(1, 8), Ratio::new(-1, 8)).is_none());
        assert!(Groove::push(Ratio::new(1, 8), Ratio::new(1, 16)).is_some());
    }

    /// Every groove, over every meter, on a fine grid.
    fn grooves() -> impl Strategy<Value = Groove> {
        prop_oneof![
            Just(Groove::STRAIGHT),
            (1i64..96, 1i64..96).prop_filter_map("a swing is strictly inside the pair", |(n, d)| {
                Groove::swing(Ratio::new(n, n.checked_add(d)?))
            }),
            (1i64..17, -95i64..96).prop_filter_map("a push may not reach its grid", |(d, by)| {
                Groove::push(Ratio::new(1, d), Ratio::new(by, 96 * d))
            }),
        ]
    }

    fn meters() -> impl Strategy<Value = Meter> {
        (1u32..13, prop::sample::select(vec![1u32, 2, 4, 8, 16])).prop_map(|(n, d)| Meter::new(n, d))
    }

    proptest! {
        /// Notes never reorder, so no duration a groove touches can come out
        /// negative. This is the property that lets `warp` be total.
        #[test]
        fn a_groove_is_monotone(groove in grooves(), meter in meters(), a in 0i64..512, b in 0i64..512) {
            let (low, high) = if a <= b { (a, b) } else { (b, a) };
            let unit = |n: i64| MusicalTime::new(Ratio::new(n, 96));
            prop_assert!(groove.warp(meter, unit(low)) <= groove.warp(meter, unit(high)));
        }

        /// A groove displaces the beat inside a cell; it never stretches or
        /// shrinks the music, so the whole note is a fixed point of every
        /// groove under every meter, however long the piece runs.
        ///
        /// This is what rejects a `push` grid that does not tile — the
        /// property found `1/3` before a reader would have.
        #[test]
        fn a_groove_keeps_the_whole_note(groove in grooves(), meter in meters(), whole in 0i64..64) {
            let at = MusicalTime::new(Ratio::from_integer(whole));
            prop_assert_eq!(groove.warp(meter, at), at);
        }

        /// A swing's cell *is* the beat unit, so every barline is a cell
        /// boundary and the downbeat cannot move under any meter.
        #[test]
        fn a_swing_fixes_the_measure(first in (1i64..96, 1i64..96), meter in meters(), bar in 0i64..64) {
            let Some(groove) = Groove::swing(Ratio::new(first.0, first.0 + first.1)) else {
                return Ok(());
            };
            let at = MusicalTime::new(meter.measure_len().as_ratio() * Ratio::from_integer(bar));
            prop_assert_eq!(groove.warp(meter, at), at);
        }

        /// A push fixes the boundaries of its own grid — the downbeats it
        /// says it leaves alone — whatever meter is written over it.
        #[test]
        fn a_push_fixes_its_own_grid(
            grid in prop::sample::select(vec![2i64, 4, 6, 8, 12, 16, 24, 32]),
            by in -95i64..96,
            meter in meters(),
            cell in 0i64..64,
        ) {
            let grid = Ratio::new(1, grid);
            let Some(groove) = Groove::push(grid, Ratio::new(by, 96) * grid) else {
                return Ok(());
            };
            let at = MusicalTime::new(grid * 2 * Ratio::from_integer(cell));
            prop_assert_eq!(groove.warp(meter, at), at);
        }
    }

    /// Straight is the identity, which is what makes it usable as an
    /// override rather than as a special case everywhere else.
    #[test]
    fn straight_moves_nothing() {
        for eighth in 0..16i64 {
            let written = at(Ratio::new(eighth, 8));
            assert_eq!(Groove::STRAIGHT.warp(Meter::default(), written), written);
        }
    }
}
