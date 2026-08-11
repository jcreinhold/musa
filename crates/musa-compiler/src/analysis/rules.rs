//! The rules the three profiles check, as data.
//!
//! Every rule has four things and cannot exist without them: a stable id a
//! report and an assertion can both name, the strength its own tradition gives
//! it, the sentence it states, and the page that states it. The last two are
//! the reason this is a table rather than a set of function names — a rule
//! whose citation is "everyone knows" is the thing prompt 119 exists to
//! refuse.
//!
//! # Strength is about a pedagogy, not about the music
//!
//! [`Strength`] says what a *style* makes of a departure. It never says the
//! music is wrong. Parallel fifths are forbidden in an eighteenth-century
//! part-writing exercise, are the substance of organum, and are how a great
//! deal of rock music is voiced; the notes are the same notes in all three.
//! So a finding here reports the motion — which is a fact about the score —
//! and carries the strength alongside it, and a reader who is not writing that
//! exercise can see immediately which of the two they are looking at.

use super::{AnalysisFinding, Evidence, Ground, NoteRef, Observation, RuleName, Standing, Strength};
use crate::origin::Interval;
use crate::scope::Scope;
use crate::time::MusicalTime;

/// One rule of one profile.
///
/// The public view is [`RuleName`], which is this without the profile: a
/// reader of a finding wants the id, the sentence, the strength, and the page,
/// and the profile is already on the request they made.
pub(super) struct Rule {
    /// The stable id, in `profile_rule` form.
    pub(super) id: &'static str,
    /// What the rule states, as one sentence in the voice of the style.
    pub(super) states: &'static str,
    /// What the style makes of a departure.
    pub(super) strength: Strength,
    /// Where it is written down.
    pub(super) cites: &'static str,
}

impl Rule {
    /// The public name of this rule.
    pub(super) const fn name(&'static self) -> RuleName {
        RuleName {
            id: self.id,
            states: self.states,
            strength: self.strength,
            cites: self.cites,
        }
    }
}

// ---------------------------------------------------------------- SATB (022)

/// SATB writing is four voices, one note each. Definitional: a texture that is
/// not that is not being read wrongly by this profile — it is not what the
/// profile is about.
pub(super) const SATB_FOUR_VOICES: Rule = Rule {
    id: "satb_four_voices",
    states: "four voices sound, each sounding one note at a time",
    strength: Strength::Definitional,
    cites: "OMT 022 §Voices",
};

/// The ranges OMT 022 gives for the four voices.
pub(super) const SATB_RANGES: Rule = Rule {
    id: "satb_ranges",
    states: "each voice stays within the range its part is written for",
    strength: Strength::Exercise,
    cites: "OMT 022 §Ranges",
};

pub(super) const SATB_SPACING: Rule = Rule {
    id: "satb_spacing",
    states: "adjacent upper voices lie within an octave of each other",
    strength: Strength::Exercise,
    cites: "OMT 022 §Spacing",
};

pub(super) const SATB_CROSSING: Rule = Rule {
    id: "satb_crossing",
    states: "a voice does not sound below the voice beneath it",
    strength: Strength::Exercise,
    cites: "OMT 022 §Crossing",
};

pub(super) const SATB_OVERLAP: Rule = Rule {
    id: "satb_overlap",
    states: "a voice does not move past the note its neighbour has just left",
    strength: Strength::Exercise,
    cites: "OMT 022 §Overlap",
};

pub(super) const SATB_PARALLEL_PERFECTS: Rule = Rule {
    id: "satb_parallel_perfects",
    states: "no two voices move in parallel fifths, octaves, or unisons",
    strength: Strength::Exercise,
    cites: "OMT 022 §Parallels",
};

/// Direct (hidden) perfects are a guideline in the source and stay one here.
pub(super) const SATB_DIRECT_PERFECTS: Rule = Rule {
    id: "satb_direct_perfects",
    states: "the outer voices do not arrive at a perfect fifth or octave in similar motion with a leap on top",
    strength: Strength::Guideline,
    cites: "OMT 022 §Direct fifths and octaves",
};

pub(super) const SATB_DOUBLING: Rule = Rule {
    id: "satb_doubling",
    states: "the leading tone of the key in force is not doubled",
    strength: Strength::Guideline,
    cites: "OMT 022 §Doubling",
};

pub(super) const SATB_TENDENCY_RESOLUTION: Rule = Rule {
    id: "satb_tendency_resolution",
    states: "the leading tone of the key in force rises to the tonic when it moves",
    strength: Strength::Guideline,
    cites: "OMT 022 §Tendency tones",
};

// ------------------------------------------------------- Species (023 – 028)

/// The rhythmic relation *is* the species. A first-species exercise whose
/// counterpoint has two notes to the cantus's one is a second-species exercise
/// being read by the wrong rules, which is why this is definitional.
pub(super) const SPECIES_RHYTHM: Rule = Rule {
    id: "species_rhythm",
    states: "the counterpoint stands in this species' rhythmic relation to the cantus firmus",
    strength: Strength::Definitional,
    cites: "OMT 023 §The species",
};

pub(super) const SPECIES_BEGIN: Rule = Rule {
    id: "species_begin",
    states: "the exercise begins on a perfect consonance",
    strength: Strength::Exercise,
    cites: "OMT 024 §Beginning",
};

pub(super) const SPECIES_END: Rule = Rule {
    id: "species_end",
    states: "the exercise ends on a unison or octave, approached by step",
    strength: Strength::Exercise,
    cites: "OMT 024 §Ending",
};

pub(super) const SPECIES_CONSONANCE: Rule = Rule {
    id: "species_consonance",
    states: "the interval on each strong beat is a consonance",
    strength: Strength::Exercise,
    cites: "OMT 023 §Consonance and dissonance",
};

pub(super) const SPECIES_DISSONANCE_PASSING: Rule = Rule {
    id: "species_dissonance_passing",
    states: "a dissonance on a weak beat is approached and left by step in one direction",
    strength: Strength::Exercise,
    cites: "OMT 025 §Dissonance",
};

pub(super) const SPECIES_SUSPENSION: Rule = Rule {
    id: "species_suspension",
    states: "a dissonance on a strong beat is prepared as a consonance, held over, and resolved down by step",
    strength: Strength::Exercise,
    cites: "OMT 027 §Suspensions",
};

pub(super) const SPECIES_PARALLEL_PERFECTS: Rule = Rule {
    id: "species_parallel_perfects",
    states: "consecutive strong beats do not repeat a perfect consonance of the same size",
    strength: Strength::Exercise,
    cites: "OMT 023 §Motion",
};

pub(super) const SPECIES_DIRECT_PERFECTS: Rule = Rule {
    id: "species_direct_perfects",
    states: "a perfect consonance is not approached by similar motion",
    strength: Strength::Guideline,
    cites: "OMT 023 §Motion",
};

pub(super) const SPECIES_LEAP_RECOVERY: Rule = Rule {
    id: "species_leap_recovery",
    states: "a leap of a fourth or more is followed by motion in the other direction",
    strength: Strength::Guideline,
    cites: "OMT 023 §Melodic shape",
};

// ------------------------------------------------------------- Jazz (076)

/// OMT `076` §"Guidelines versus Rules" is explicit that these are guidelines,
/// and every jazz rule here carries that strength for that reason.
pub(super) const JAZZ_GUIDE_TONES: Rule = Rule {
    id: "jazz_guide_tones",
    states: "the third and the seventh of the written symbol sound in the voicing",
    strength: Strength::Guideline,
    cites: "OMT 076 §Guide tones",
};

pub(super) const JAZZ_GUIDE_TONE_MOTION: Rule = Rule {
    id: "jazz_guide_tone_motion",
    states: "a guide tone is held or moves by step into the next voicing",
    strength: Strength::Guideline,
    cites: "OMT 076 §Guide tones",
};

pub(super) const JAZZ_COMMON_TONE: Rule = Rule {
    id: "jazz_common_tone",
    states: "a pitch class both voicings contain is kept in one voice",
    strength: Strength::Guideline,
    cites: "OMT 076 §Voice leading",
};

pub(super) const JAZZ_SMALL_MOTION: Rule = Rule {
    id: "jazz_small_motion",
    states: "no voice moves by more than a third between voicings",
    strength: Strength::Guideline,
    cites: "OMT 076 §Voice leading",
};

pub(super) const JAZZ_SPACING: Rule = Rule {
    id: "jazz_spacing",
    states: "no interval within the voicing exceeds an octave, and no second sounds below the tenor register",
    strength: Strength::Guideline,
    cites: "OMT 076 §Spacing",
};

pub(super) const JAZZ_OMISSION: Rule = Rule {
    id: "jazz_omission",
    states: "the voicing sounds the root or the fifth of the written symbol",
    strength: Strength::Guideline,
    cites: "OMT 076 §Rootless voicings",
};

/// Every rule, in the order a listing prints them.
///
/// The array *is* the registry: an assertion resolves a rule word here, the
/// documentation table is generated from the same rows a report prints, and a
/// rule that is not in it cannot be checked by anything.
pub(super) const RULES: [&Rule; 24] = [
    &SATB_FOUR_VOICES,
    &SATB_RANGES,
    &SATB_SPACING,
    &SATB_CROSSING,
    &SATB_OVERLAP,
    &SATB_PARALLEL_PERFECTS,
    &SATB_DIRECT_PERFECTS,
    &SATB_DOUBLING,
    &SATB_TENDENCY_RESOLUTION,
    &SPECIES_RHYTHM,
    &SPECIES_BEGIN,
    &SPECIES_END,
    &SPECIES_CONSONANCE,
    &SPECIES_DISSONANCE_PASSING,
    &SPECIES_SUSPENSION,
    &SPECIES_PARALLEL_PERFECTS,
    &SPECIES_DIRECT_PERFECTS,
    &SPECIES_LEAP_RECOVERY,
    &JAZZ_GUIDE_TONES,
    &JAZZ_GUIDE_TONE_MOTION,
    &JAZZ_COMMON_TONE,
    &JAZZ_SMALL_MOTION,
    &JAZZ_SPACING,
    &JAZZ_OMISSION,
];

/// The rules a composer may name in an `assert follows(…)`.
///
/// A short list, and short for a reason. An assertion sees a passage — sounded
/// pitches with exact spans, and no voices, no key, and no cantus firmus,
/// because [`crate::assert::Passage`] is deliberately that small. So the rules
/// an assertion can name are the ones that are decidable from the notes and
/// their vertical order alone. A rule that needs the key in force, or which
/// line is the given one, is a question for `musa analyze`, where the request
/// can supply what the rule needs.
pub(super) const ASSERTABLE: [&Rule; 5] = [
    &SATB_PARALLEL_PERFECTS,
    &SATB_SPACING,
    &SATB_OVERLAP,
    &JAZZ_SMALL_MOTION,
    &JAZZ_SPACING,
];

/// The finding that says a rule was looked at.
pub(super) fn in_force(rule: &'static Rule, from: MusicalTime) -> AnalysisFinding {
    AnalysisFinding::stated(
        "rule-in-force",
        Observation::RuleInForce {
            rule: rule.name(),
            from,
        },
        Evidence::InForce {
            scope: Scope::Piece,
            from,
        },
    )
}

/// One departure from one rule.
///
/// The standing is the caller's because it is a claim about the *abstraction*
/// and not about the rule: a departure decidable from the notes is a fact
/// whatever the rule's strength, and one that depends on a key the request
/// assumed is a candidate for the same reason a numeral under an assumed key
/// is. The rule's strength travels separately, inside the observation.
pub(super) struct Departure {
    /// The rule departed from.
    pub(super) rule: &'static Rule,
    /// How firmly the reading holds the departure.
    pub(super) standing: Standing,
    /// The voices involved, in the profile's own names.
    pub(super) voices: Vec<String>,
    /// The interval at issue, for a rule about one.
    pub(super) interval: Option<Interval>,
    /// Where it begins.
    pub(super) from: MusicalTime,
    /// Where it ends.
    pub(super) to: MusicalTime,
    /// The notes a reader should look at.
    pub(super) notes: Vec<NoteRef>,
    /// The rule's sub-criteria and their verdicts, for a rule that has them.
    ///
    /// A suspension is three claims at once — prepared, held, resolved — and a
    /// reader who is told only "this is not a suspension" has been told the
    /// conclusion and not the evidence.
    pub(super) also: Vec<Ground>,
}

impl Departure {
    /// The finding this departure is.
    pub(super) fn finding(self) -> AnalysisFinding {
        AnalysisFinding::judged(
            "departure",
            self.standing,
            Observation::Departure {
                rule: self.rule.name(),
                voices: self.voices,
                interval: self.interval,
                from: self.from,
                to: self.to,
            },
            Evidence::Passage {
                from: self.from,
                to: self.to,
                notes: self.notes,
            },
            std::iter::once(Ground {
                criterion: self.rule.states,
                satisfied: false,
                cites: self.rule.cites,
            })
            .chain(self.also)
            .collect(),
        )
    }
}
