//! Interpretation profiles (roadmap §6.4): how a part turns written marks
//! into realization parameters.
//!
//! The separation table (§2) is the whole point of this module. A staccato
//! mark is not "×0.55"; a `p` is not a velocity. Those are *choices*, they
//! differ per instrument and per performer, and the language makes them
//! explicit and nameable instead of burying one house style in the renderer:
//!
//! ```text
//! performance {
//!     profile violin {
//!         articulation staccato { gate = 0.55; attack = 8 ms; }
//!         dynamic p { amplitude = 0.35; }
//!     }
//! }
//! ```
//!
//! Declarations only. Nothing here touches a `ScoreEvent`: the profiles ride
//! along in the snapshot the way motif declarations do, and interpretation
//! happens once, in `performance.rs`, on the way to a `PerformancePlan`.
//!
//! Values stay **exact rationals**, like musical time: `0.55` is a written
//! decimal, and rounding it to a float here would put an approximation in the
//! snapshot that every later comparison has to live with. Floats appear at
//! the DSP edge, where they always have.

// Rational arithmetic over declared ratios: gates in `0..=1` multiply and
// decimals scale by powers of ten, both total for the magnitudes the parser
// admits (see `time.rs` for the same argument about musical time).
#![allow(clippy::arithmetic_side_effects)]

use indexmap::IndexMap;
use num_rational::Ratio;
use serde::{Deserialize, Serialize};

use crate::groove::Groove;
use crate::marks::Mark;
use crate::score::DynamicMark;

/// What an articulation does to a note, per profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticulationRealization {
    /// Fraction of the notated duration the note actually sounds. `1` is
    /// the full written value; a staccato profile might say `0.55`.
    pub gate: Ratio<i64>,
    /// Requested attack time in seconds.
    pub attack: Ratio<i64>,
    /// How much longer than written the note is held. `1` is the written
    /// value; `2` is a fermata a profile reads as twice as long.
    ///
    /// It lengthens the *note*, not the bar: the music that follows still
    /// starts where the page says it does. A fermata that stops the clock is a
    /// tempo fact, and musa has no way to state one until prompt 72 — so this
    /// is the honest half of a fermata rather than a whole one, and the
    /// difference is written down here rather than discovered in the sound.
    pub hold: Ratio<i64>,
}

impl ArticulationRealization {
    /// Full gate, instant attack, written length: what an unprofiled note
    /// gets, and what prompts 15–17 produced before profiles existed.
    pub const NEUTRAL: Self = Self {
        gate: Ratio::new_raw(1, 1),
        attack: Ratio::new_raw(0, 1),
        hold: Ratio::new_raw(1, 1),
    };
}

/// One named profile: an instrument's reading of the written marks.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerformanceProfile {
    name: String,
    articulations: IndexMap<Mark, ArticulationRealization>,
    dynamics: IndexMap<DynamicMark, Ratio<i64>>,
    groove: Groove,
}

impl PerformanceProfile {
    /// The profile name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How this profile reads a note's articulations, in written order.
    ///
    /// Gates **multiply** — two shortening marks shorten twice — and holds
    /// multiply for the same reason, while the attack is the last written
    /// rule's, because two attack times cannot be combined into a third that
    /// either performer would recognize.
    pub fn realize(&self, marks: &[Mark]) -> ArticulationRealization {
        let mut realization = ArticulationRealization::NEUTRAL;
        for mark in marks {
            let Some(rule) = self.articulations.get(mark) else {
                continue;
            };
            realization.gate *= rule.gate;
            realization.hold *= rule.hold;
            realization.attack = rule.attack;
        }
        realization
    }

    /// The abstract amplitude this profile gives a dynamic marking, if it
    /// declares one. Still not a decibel value (§2): the mapping to gain
    /// happens at the instrument boundary.
    pub fn amplitude(&self, mark: DynamicMark) -> Option<Ratio<i64>> {
        self.dynamics.get(&mark).copied()
    }

    pub(crate) fn set_mark(&mut self, mark: Mark, rule: ArticulationRealization) {
        self.articulations.insert(mark, rule);
    }

    pub(crate) fn set_dynamic(&mut self, mark: DynamicMark, amplitude: Ratio<i64>) {
        self.dynamics.insert(mark, amplitude);
    }

    /// Where this profile's beat sits against the page's.
    ///
    /// A profile without a `groove` rule is straight, which is the identity —
    /// so the profiles written before this existed perform exactly as they
    /// did, and no caller needs to ask whether a groove was declared.
    pub fn groove(&self) -> Groove {
        self.groove
    }

    pub(crate) fn set_groove(&mut self, groove: Groove) {
        self.groove = groove;
    }

    pub(crate) fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
}

/// Every profile a piece declares, plus which part each one realizes.
///
/// Parts are keyed by **name** rather than `PartId`: the binding is a source
/// fact written inside the part, and a name survives edits that renumber
/// parts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSet {
    profiles: IndexMap<String, PerformanceProfile>,
    parts: IndexMap<String, String>,
}

impl ProfileSet {
    /// A declared profile by name.
    pub fn get(&self, name: &str) -> Option<&PerformanceProfile> {
        self.profiles.get(name)
    }

    /// The profile realizing the named part, if it declared one.
    pub fn for_part(&self, part: &str) -> Option<&PerformanceProfile> {
        self.profiles.get(self.parts.get(part)?)
    }

    /// Whether the piece declares no profiles at all — the case that must
    /// reproduce the pre-profile behavior exactly.
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Declared profile names, in source order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.profiles.keys().map(String::as_str)
    }

    pub(crate) fn declares(&self, name: &str) -> bool {
        self.profiles.contains_key(name)
    }

    pub(crate) fn insert(&mut self, profile: PerformanceProfile) {
        self.profiles.insert(profile.name.clone(), profile);
    }

    pub(crate) fn assign(&mut self, part: impl Into<String>, profile: impl Into<String>) {
        self.parts.insert(part.into(), profile.into());
    }
}

/// Parse a written decimal or integer as an exact rational: `0.55` is
/// `11/20`, not `0.55000000000000004`.
pub(crate) fn parse_decimal(text: &str) -> Option<Ratio<i64>> {
    let (sign, text) = text.strip_prefix('-').map_or((1i64, text), |rest| (-1, rest));
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let whole: i64 = if whole.is_empty() { 0 } else { whole.parse().ok()? };
    let magnitude = if fraction.is_empty() {
        Ratio::from_integer(whole)
    } else {
        let digits: i64 = fraction.parse().ok()?;
        let scale = 10i64.checked_pow(u32::try_from(fraction.len()).ok()?)?;
        Ratio::from_integer(whole) + Ratio::new(digits, scale)
    };
    Some(magnitude * sign)
}
