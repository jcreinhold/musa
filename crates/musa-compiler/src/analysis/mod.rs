//! Musical analysis: observing a compiled score without changing it.
//!
//! `docs/language/05-verification.md` divides theory work three ways. A
//! **constructor invariant** is what a value must satisfy to exist. An
//! **assertion** proves a decidable claim a composer wrote down
//! ([`crate::assert`]). An **analysis** is the third thing: it reads a
//! finished score and reports what it saw, it may be ambiguous, and its
//! findings do not become facts merely because an algorithm selected one.
//!
//! # An analysis is an abstract interpretation
//!
//! Every kind admitted here states three things, in the shape Peyton Jones
//! (1987) §22.1 gives a program analysis — an abstract domain, an abstract
//! reading of the concrete object, and a stated relationship between the two:
//!
//! 1. **The abstract domain** its findings live in.
//! 2. **The abstraction map** from the concrete score projection — that is,
//!    from [`ScoreSnapshot`] — into that domain.
//! 3. **The soundness claim**: what a finding licenses a reader to conclude,
//!    and what it does not.
//!
//! This is not decoration. It is what makes [`Standing`] mean something: a
//! [`Standing::Candidate`] is precisely a concrete reading the abstraction
//! cannot separate from another, and a [`Standing::Conflict`] is precisely two
//! readings the abstraction says cannot both hold. Without the map those words
//! are severity labels chosen by feel, and a finding with no stated
//! relationship to the score is exactly the false claim
//! `docs/language/03-musical-domains.md` §5 forbids. An analysis whose
//! soundness claim cannot be written must not ship.
//!
//! # What this module may not do
//!
//! It never constructs or rewrites music, never emits a compiler diagnostic,
//! and never touches the source. [`analyze`] borrows a snapshot and returns a
//! value; there is no path from here back into the resolver. That is the
//! reason the boundary is one function rather than a compiler pass: an
//! analysis that could report *into* the compilation would be a lint with
//! extra steps, and prompt 83 already drew that line.
//!
//! # Why one function and not a crate
//!
//! The alternative was a public `musa-analysis` crate owning the algorithms.
//! It is rejected for the reason a public elaboration API is rejected: its
//! only caller would be this crate — every consumer reaches analysis through
//! `musa-project` — while its public surface would be segmenters, candidate
//! graphs, and theory indexes, which are pass details that prompt 118 and 119
//! will change. The boundary here is one request, one report, and one
//! function; everything that computes a finding stays private.

use crate::chord::ChordClass;
use crate::harmony::ChordSymbol;
use crate::origin::{Interval, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::scope::Scope;
use crate::score::{EventId, Key, Meter, PartId, ScoreSnapshot, VoiceId};
use crate::time::{MusicalDuration, MusicalTime};

/// Which analysis to run.
///
/// A closed list, for the reason [`crate::assert`]'s claim family is one: each
/// kind owes an abstract domain, an abstraction map, and a soundness claim,
/// and a kind supplied by a caller could not owe anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnalysisKind {
    /// The score's own statements, and nothing read into them.
    Facts,
    /// Simultaneities, and which chords their pitch content fits.
    Chords,
    /// Roman numerals, key regions, tonicization, and modulation.
    Tonal,
    /// Cadences, with the evidence each one does and does not have.
    Cadences,
    /// How the voices of a chordal texture move, against a named style profile.
    VoiceLeading,
    /// Species counterpoint against a designated cantus firmus.
    Counterpoint,
}

impl AnalysisKind {
    /// Every kind, in the order a listing prints them.
    pub const ALL: [Self; 6] = [
        Self::Facts,
        Self::Chords,
        Self::Tonal,
        Self::Cadences,
        Self::VoiceLeading,
        Self::Counterpoint,
    ];

    /// How the kind is written on a command line.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Facts => "facts",
            Self::Chords => "chords",
            Self::Tonal => "tonal",
            Self::Cadences => "cadences",
            Self::VoiceLeading => "voice-leading",
            Self::Counterpoint => "counterpoint",
        }
    }

    /// The kind written as `name`, or `None` if none is.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == name)
    }

    /// One line saying what the analysis does, carried by every report so a
    /// reader never has to guess which algorithm produced it.
    pub fn method(self) -> &'static str {
        match self {
            Self::Facts => {
                "reads the score's own statements: what sounds, what rests, what is written above the staff, and what is in force"
            }
            Self::Chords => {
                "segments the music into simultaneities and reports every chord class whose members stand in a stated relation to each one (OMT 017-019)"
            }
            Self::Tonal => {
                "reads each simultaneity as a Roman numeral against every key the passage supports, and reports tonicization and modulation candidates with the OMT 050-051 criteria each satisfies"
            }
            Self::Cadences => {
                "looks at each potential cadence point for the harmonic, melodic, and formal evidence OMT 036 requires, and says which of it is missing"
            }
            Self::VoiceLeading => {
                "reads the motion between adjacent sonorities against the rules of the requested style profile, and reports each departure with the rule's own strength"
            }
            Self::Counterpoint => {
                "reads a counterpoint against its designated cantus firmus under the rules of the requested species (OMT 023-028)"
            }
        }
    }

    /// What the analysis takes for granted, one sentence each.
    ///
    /// No kind may leave this empty: an empty list is itself a claim — that
    /// the reading depends on nothing — so every kind states at least the
    /// boundary of what it read.
    pub fn assumptions(self) -> &'static [&'static str] {
        match self {
            Self::Facts => &[
                "the score compiled: an analysis reads a finished snapshot, never half-resolved source",
                "written spelling is kept: `d#4` is reported as D-sharp and never as E-flat",
                "nothing outside the requested scope and window was read, and nothing about it is claimed",
            ],
            Self::Chords => &[
                "the requested segmentation is the right one: which notes sound together is a reading, and the request chose it",
                "a chord is its set of pitch classes; voicing, doubling, and octave are not part of the fit (OMT 019)",
                "nothing here decides which notes are embellishing tones — a fit that needs one is reported as needing one (OMT 039)",
            ],
            Self::Tonal => &[
                "common-practice Western tonality: these numerals describe that repertoire and nothing beyond it",
                "a key written in the source is strong evidence about the key and not a proof of it (OMT 051)",
                "tonicization and modulation lie on a continuum; where the criteria underdetermine it, both readings are reported (OMT 051)",
            ],
            Self::Cadences => &[
                "a cadence needs harmony, melody, and a phrase ending to agree; each is reported separately (OMT 036)",
                "phrase endings come from what the source marks — a `phrase`, a rest, or the end of the piece — never from a guess about form",
                "a subverted cadence looks exactly like a cadence to a reader of pitch alone, so a candidate here is not a decision",
            ],
            Self::VoiceLeading | Self::Counterpoint => &[
                "the requested profile is a historical pedagogy and not a law of music: a departure is a departure from that style and nothing more",
                "each rule carries its own strength — definitional, hard within the exercise, or a guideline — and the report never flattens the three",
                "voices, spans, and intervals are the notated ones, with ties already joined; performed time is never read for a notational rule",
            ],
        }
    }
}

/// The style a voice-leading or counterpoint reading is against.
///
/// Named data rather than a hidden compiler mode: a profile is chosen in the
/// request, printed in the report, and its rule list is part of what the
/// report says. Two profiles reading one passage will disagree, and that is
/// the honest situation — the disagreement is between two traditions, and the
/// request names which one is speaking.
///
/// Its parameters live on [`AnalysisRequest`] rather than inside the variants.
/// A species profile needs a designated cantus firmus, but a request also
/// needs a scope, a window, and a key, and a profile whose variants carried
/// half of those would be a second request type with the same job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnalysisProfile {
    /// Four-voice chorale-style part writing (OMT 022).
    Satb,
    /// First species: note against note.
    Species1,
    /// Second species: two notes against one.
    Species2,
    /// Third species: four notes against one.
    Species3,
    /// Fourth species: syncopated notes against one, tied across the bar.
    Species4,
    /// Fifth species: the previous four, mixed measure by measure.
    Species5,
    /// Jazz voicing motion (OMT 076), whose rules are guidelines by its own
    /// account.
    JazzVoiceLeading,
}

impl AnalysisProfile {
    /// Every profile, in the order a listing prints them.
    pub const ALL: [Self; 7] = [
        Self::Satb,
        Self::Species1,
        Self::Species2,
        Self::Species3,
        Self::Species4,
        Self::Species5,
        Self::JazzVoiceLeading,
    ];

    /// How the profile is written on a command line.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Satb => "satb_common_practice",
            Self::Species1 => "species_1",
            Self::Species2 => "species_2",
            Self::Species3 => "species_3",
            Self::Species4 => "species_4",
            Self::Species5 => "species_5",
            Self::JazzVoiceLeading => "jazz_voice_leading",
        }
    }

    /// The profile written as `name`, or `None` if none is.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|profile| profile.as_str() == name)
    }

    /// Which analysis this profile is a profile of.
    ///
    /// A profile belongs to exactly one kind, so a request that pairs them
    /// wrongly is refused rather than quietly reinterpreted.
    pub fn kind(self) -> AnalysisKind {
        match self {
            Self::Satb | Self::JazzVoiceLeading => AnalysisKind::VoiceLeading,
            Self::Species1 | Self::Species2 | Self::Species3 | Self::Species4 | Self::Species5 => {
                AnalysisKind::Counterpoint
            }
        }
    }

    /// Whether this profile reads a designated cantus firmus.
    pub fn needs_cantus(self) -> bool {
        matches!(self.kind(), AnalysisKind::Counterpoint)
    }

    /// What the profile takes for granted about the music it is given, beyond
    /// what its kind already states.
    pub fn assumptions(self) -> &'static [&'static str] {
        match self {
            Self::Satb => &[
                "four voices, written as four voices: a chord written inside one voice is reported as a departure from the profile's own definition, not silently split",
                "voices are ordered by where they lie, lowest first, whatever order the source declares them in",
                "the doubling and tendency-tone rules read the key in force; where no key is written, they are not checked and say so",
            ],
            Self::Species1 | Self::Species2 | Self::Species3 | Self::Species4 | Self::Species5 => &[
                "exactly two voices, one of which the request designates as the cantus firmus",
                "the species is the rhythmic relation between them; a counterpoint that does not stand in it is reported against the species that was asked for",
                "the perfect fourth is a dissonance against the bass and a consonance above it (OMT 023), and which one it is depends on the voice below",
            ],
            Self::JazzVoiceLeading => &[
                "voices are the positions of a voicing counted from the bottom, because a jazz voicing is a chord and not four independent lines",
                "the written harmony lane supplies the chord: nothing here derives a symbol from the notes",
                "every rule is a guideline by OMT 076's own account, and none of them is reported as anything else",
            ],
        }
    }
}

/// What a style makes of a departure from one of its rules.
///
/// Three strengths and not a severity scale. The distinction is about where
/// the rule comes from, not about how much it matters: a definitional rule
/// says what the profile is *about*, an exercise rule is binding inside the
/// exercise the profile models, and a guideline is advice its own source
/// gives as advice. Nothing here is a claim that music which departs is bad
/// music.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Strength {
    /// The profile is defined by this. Music that departs is not being read
    /// wrongly — it is not what the profile describes.
    Definitional,
    /// Binding within the exercise the profile models, and stated as binding
    /// by the source cited.
    Exercise,
    /// Advice, given as advice by the source cited.
    Guideline,
}

impl Strength {
    /// The word a report prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Definitional => "definitional",
            Self::Exercise => "hard in this exercise",
            Self::Guideline => "guideline",
        }
    }
}

/// One rule of a profile, as a reader of a finding sees it.
///
/// Four fields, all of which a finding needs and none of which it can invent:
/// the id an assertion can name, the sentence the rule states, the strength
/// its tradition gives it, and the page it comes from. A rule with no citation
/// cannot be constructed, because this type is only ever built from the
/// registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RuleName {
    id: &'static str,
    states: &'static str,
    strength: Strength,
    cites: &'static str,
}

impl RuleName {
    /// The stable id: `satb_parallel_perfects`. What a filter matches and what
    /// an assertion names.
    pub fn id(self) -> &'static str {
        self.id
    }

    /// What the rule states, in the voice of the style that states it.
    pub fn states(self) -> &'static str {
        self.states
    }

    /// What that style makes of a departure.
    pub fn strength(self) -> Strength {
        self.strength
    }

    /// Where it is written down: `OMT 022 §Spacing`.
    pub fn cites(self) -> &'static str {
        self.cites
    }
}

impl std::fmt::Display for RuleName {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.id)
    }
}

/// Every rule any profile checks, in registry order.
///
/// Public because a caller that renders a rule table — the documentation, an
/// interface panel listing what a profile will look at — must not have to run
/// an analysis to discover the rules exist.
pub fn rule_names() -> impl Iterator<Item = RuleName> {
    rules::RULES.iter().map(|rule| rule.name())
}

/// The rules a composer may name in an `assert follows(…)`, in registry order.
///
/// Crate-visible rather than public: it is the vocabulary of one statement in
/// the elaboration language, and the language's own checker is its only
/// caller. What is public is [`rule_names`], because a reader of a report
/// needs every rule and not only the assertable ones.
pub(crate) fn assertable() -> impl Iterator<Item = RuleName> {
    rules::ASSERTABLE.iter().map(|rule| rule.name())
}

/// Which music to read.
///
/// Named rather than numbered, because whoever is asking is a person at a
/// command line or an interface panel and `PartId(1)` is not something anyone
/// wrote down. A name nothing answers to is [`AnalysisError::NoSuchPart`],
/// which is the point of naming: a typo is refused rather than silently
/// analyzing the whole piece.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalysisScope {
    /// Every part and every voice.
    Score,
    /// One part, by the name its declaration gives it.
    Part(String),
    /// One voice of one part, both by name.
    Voice {
        /// The part's name.
        part: String,
        /// The voice's name within that part.
        voice: String,
    },
}

/// What to observe, where, and how far.
///
/// Immutable once built. The window is half-open — `from` is read and `to` is
/// not — because that is the only convention under which two adjacent windows
/// tile a piece without reporting the note on the seam twice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisRequest {
    kind: AnalysisKind,
    scope: AnalysisScope,
    window: Option<(MusicalTime, MusicalTime)>,
    segmentation: Segmentation,
    key: Option<Key>,
    profile: Option<AnalysisProfile>,
    cantus: Option<String>,
}

/// How to decide which notes count as sounding together.
///
/// A policy and not a discovery. Which notes form a chord is the first
/// interpretive choice in any harmonic analysis, and the three answers below
/// genuinely disagree on real music — a passing tone is inside the sonority
/// under [`Self::Attacks`] and outside it under [`Self::Beats`]. So the
/// caller states it, the report carries it in its assumptions, and no
/// algorithm here pretends the question was settled for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Segmentation {
    /// A new simultaneity begins at every attack, and holds until the next
    /// one. The plainest reading, and the one that keeps every note.
    #[default]
    Attacks,
    /// One simultaneity per notated beat, holding what sounds when the beat
    /// arrives. The metric reading: it hides offbeat passing motion, and it
    /// loses a genuine chord change that arrives between beats.
    Beats,
    /// One window per written chord symbol, running to the next one. The
    /// source's own segmentation, available only where the source wrote a
    /// harmony lane.
    HarmonyLane,
}

impl Segmentation {
    /// How the policy is written on a command line.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Attacks => "attacks",
            Self::Beats => "beats",
            Self::HarmonyLane => "harmony-lane",
        }
    }

    /// The policy written as `name`, or `None` if none is.
    pub fn named(name: &str) -> Option<Self> {
        [Self::Attacks, Self::Beats, Self::HarmonyLane]
            .into_iter()
            .find(|policy| policy.as_str() == name)
    }
}

impl AnalysisRequest {
    /// The whole score, from beginning to end.
    ///
    /// The defaults live here because every caller that does not narrow its
    /// request wants exactly them, and a three-argument constructor two of
    /// whose arguments are almost always "all of it" is a constructor that
    /// makes its callers say nothing three times.
    pub fn new(kind: AnalysisKind) -> Self {
        Self {
            kind,
            scope: AnalysisScope::Score,
            window: None,
            segmentation: Segmentation::Attacks,
            key: None,
            profile: None,
            cantus: None,
        }
    }

    /// Read only this part or voice.
    #[must_use]
    pub fn scoped(mut self, scope: AnalysisScope) -> Self {
        self.scope = scope;
        self
    }

    /// Read only `[from, to)`, in whole notes from the start of the piece.
    #[must_use]
    pub fn within(mut self, from: MusicalTime, to: MusicalTime) -> Self {
        self.window = Some((from, to));
        self
    }

    /// Decide which notes sound together this way.
    ///
    /// Ignored by kinds that read no simultaneities, which is why it is a
    /// builder rather than an argument: a `facts` request has no segmentation
    /// to state and should not have to say so.
    #[must_use]
    pub fn segmenting(mut self, how: Segmentation) -> Self {
        self.segmentation = how;
        self
    }

    /// Analyze against this key rather than against the keys the passage
    /// supports.
    ///
    /// The honest use is "read this the way I hear it". It narrows the
    /// reading; it does not make it true, and a numeral produced under an
    /// assumed key is still a candidate whenever the fit underneath it is.
    #[must_use]
    pub fn in_key(mut self, key: Key) -> Self {
        self.key = Some(key);
        self
    }

    /// Read against this style profile.
    ///
    /// Required by [`AnalysisKind::VoiceLeading`] and
    /// [`AnalysisKind::Counterpoint`] and refused by every other kind: there
    /// is no default style, because a default style would be this library
    /// choosing a pedagogy on the caller's behalf and then not saying so.
    #[must_use]
    pub fn under(mut self, profile: AnalysisProfile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Name the voice that is the cantus firmus.
    ///
    /// A species exercise has a given line and a written one, and which is
    /// which is not recoverable from the notes — the cantus is a whole note
    /// per bar in the first species and so is the counterpoint. So the request
    /// says, and a species request that does not is refused.
    #[must_use]
    pub fn designating(mut self, cantus: impl Into<String>) -> Self {
        self.cantus = Some(cantus.into());
        self
    }
}

/// A request no score could answer.
///
/// Every case is something the caller can fix by asking differently, which is
/// the test for belonging here at all: an empty result is not an error, and a
/// window containing no notes reports nothing rather than failing.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AnalysisError {
    /// The score has no part of that name.
    #[error("no part named `{name}`; this score has {}", spell(.available))]
    NoSuchPart {
        /// The name that was asked for.
        name: String,
        /// The names there are, in score order.
        available: Vec<String>,
    },
    /// The part exists and has no voice of that name.
    #[error("part `{part}` has no voice named `{name}`; it has {}", spell(.available))]
    NoSuchVoice {
        /// The part that was found.
        part: String,
        /// The voice name that was asked for.
        name: String,
        /// The voice names that part has, in score order.
        available: Vec<String>,
    },
    /// The window ends before it starts, or is a single instant.
    #[error("a window from {from} to {to} contains no music")]
    EmptyWindow {
        /// Where it was asked to start.
        from: MusicalTime,
        /// Where it was asked to stop.
        to: MusicalTime,
    },
    /// The kind reads against a style and the request named none.
    #[error("`{}` reads against a style profile; name one with a profile", .kind.as_str())]
    ProfileRequired {
        /// The kind that was asked for.
        kind: AnalysisKind,
    },
    /// The profile belongs to a different kind than the one requested.
    #[error("`{}` is a profile of `{}`, not of `{}`", .profile.as_str(), .profile.kind().as_str(), .kind.as_str())]
    WrongProfile {
        /// The profile that was named.
        profile: AnalysisProfile,
        /// The kind it was asked for under.
        kind: AnalysisKind,
    },
    /// A species profile with no cantus firmus designated.
    #[error("`{}` reads a counterpoint against a cantus firmus; name the voice that is the cantus", .profile.as_str())]
    NoCantus {
        /// The profile that needs one.
        profile: AnalysisProfile,
    },
    /// The designated cantus names no voice the request selected.
    #[error("no voice named `{name}` in the music read; it has {}", spell(.available))]
    NoSuchCantus {
        /// The name that was designated.
        name: String,
        /// The voice names the reading found, in score order.
        available: Vec<String>,
    },
    /// A species profile given something other than two voices.
    #[error("species counterpoint reads two voices; this request selects {found}")]
    NotTwoVoices {
        /// How many voices the request selected.
        found: usize,
    },
}

/// `` `a` ``, `` `b` ``, and `` `c` `` — or `none`, which is a thing a score's
/// part list can be.
fn spell(names: &[String]) -> String {
    match names {
        [] => "none".to_owned(),
        [only] => format!("`{only}`"),
        [rest @ .., last] => {
            let mut out = rest
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(" and `");
            out.push_str(last);
            out.push('`');
            out
        }
    }
}

/// How firmly a finding is held.
///
/// These are not severities. Each is a position of the finding relative to the
/// analysis's own abstraction, and each is meaningful only because that
/// abstraction was written down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Standing {
    /// The abstraction determines this: every concrete score the report is
    /// consistent with agrees about it.
    Fact,
    /// Several concrete readings survive the abstraction and this is one of
    /// them. A candidate is never reported alone — the readings it competes
    /// with are in the same report — and the order they appear in is the
    /// analysis's stated deterministic ranking, never a probability.
    Candidate,
    /// Two readings the abstraction says cannot both hold, reported together
    /// because choosing between them is the reader's business.
    Conflict,
}

impl Standing {
    /// The word a report prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Candidate => "candidate",
            Self::Conflict => "conflict",
        }
    }
}

/// What one finding says, in the domain its analysis reasons about.
///
/// Typed rather than a sentence: a consumer that wants to draw the pitch on a
/// staff, or hand it to the next analysis, must not have to parse English back
/// out of a string. Composing the sentence is the renderer's job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observation {
    /// A written pitch sounding over an exact span.
    Sounding {
        /// The spelled pitch. Spelling is kept: `d#4` is not `eb4`.
        pitch: WrittenPitch,
        /// Where it starts, in whole notes from the piece start.
        onset: MusicalTime,
        /// How long it sounds.
        extent: MusicalDuration,
    },
    /// A rest: nothing sounding, over an exact span.
    Silence {
        /// Where the silence starts.
        onset: MusicalTime,
        /// How long it lasts.
        extent: MusicalDuration,
    },
    /// A chord symbol written above the staff.
    ///
    /// Recorded, never interpreted — `crate::harmony`'s standing rule. The
    /// finding says the symbol is written there. It says nothing about whether
    /// the notes underneath agree with it.
    Written {
        /// The symbol as the source spells it.
        symbol: ChordSymbol,
        /// Where it is written, in whole notes from the piece start.
        at: MusicalTime,
    },
    /// A key in force from an instant.
    KeyInForce {
        /// The key.
        key: Key,
        /// Where it takes force.
        from: MusicalTime,
    },
    /// A meter in force from an instant.
    MeterInForce {
        /// The meter.
        meter: Meter,
        /// Where it takes force.
        from: MusicalTime,
    },
    /// A simultaneity: what sounds together over an exact span, under the
    /// request's segmentation policy.
    Sonority {
        /// The sounding pitches, lowest first.
        pitches: Vec<WrittenPitch>,
        /// Where the slice begins.
        onset: MusicalTime,
        /// How long it lasts.
        extent: MusicalDuration,
    },
    /// A chord class the simultaneity's pitch content fits, and how.
    ChordFit {
        /// The chord, spelled.
        chord: ChordName,
        /// The relation between the sounding classes and the chord's members.
        fit: Fit,
        /// Where the simultaneity begins.
        onset: MusicalTime,
        /// How long it lasts.
        extent: MusicalDuration,
    },
    /// What a written chord symbol and the notes under it say about each
    /// other. Reported only when a symbol is written: the compiler still
    /// derives nothing from one, and this is a reader comparing two things the
    /// source already said.
    SymbolReading {
        /// The symbol, as the source spells it.
        symbol: ChordSymbol,
        /// The chord the notes fit, when they fit one.
        sounding: Option<ChordName>,
        /// Where the symbol is written.
        at: MusicalTime,
    },
    /// A Roman numeral for a simultaneity, against a key.
    Numeral {
        /// The numeral as it is written: `V7`, `viio6`, `V7/V`.
        numeral: String,
        /// The key it is a numeral in.
        key: Key,
        /// How well the notes fit the chord the numeral names.
        fit: Fit,
        /// Where the simultaneity begins.
        onset: MusicalTime,
        /// How long it lasts.
        extent: MusicalDuration,
    },
    /// A stretch of music a key would account for.
    KeyRegion {
        /// The key.
        key: Key,
        /// Where the stretch begins.
        from: MusicalTime,
        /// Where it ends.
        to: MusicalTime,
    },
    /// A cadence reading at a potential cadence point.
    Cadence {
        /// Which cadence, at the strength its evidence supports.
        cadence: Cadence,
        /// The key it cadences in.
        key: Key,
        /// Where the final chord of the cadence begins.
        at: MusicalTime,
    },
    /// A non-tonic chord made to sound like a temporary tonic (OMT 050).
    Tonicization {
        /// The numeral being tonicized, in the home key: `V`, `ii`.
        target: String,
        /// The home key.
        key: Key,
        /// Where the tonicizing chord begins.
        from: MusicalTime,
        /// Where the tonicized chord ends.
        to: MusicalTime,
    },
    /// A rule the requested profile checked over a stretch of music.
    ///
    /// Reported whether or not anything departs from it, because a report that
    /// listed only departures would leave a reader unable to tell a passage
    /// that satisfies a rule from one the reading never looked at.
    RuleInForce {
        /// The rule, with its strength and its citation.
        rule: RuleName,
        /// Where the profile began reading.
        from: MusicalTime,
    },
    /// A place where the music departs from one of the profile's rules.
    ///
    /// The observation is the motion, which is a fact about the notes. What
    /// the style makes of it is `rule.strength()`, and the two are separate on
    /// purpose: parallel fifths are a mistake in one exercise, a technique in
    /// another, and the same two notes in both.
    Departure {
        /// The rule departed from.
        rule: RuleName,
        /// The voices involved, named the way the profile names them.
        voices: Vec<String>,
        /// The interval at issue, for a rule that is about one.
        interval: Option<Interval>,
        /// Where the departure begins.
        from: MusicalTime,
        /// Where it ends.
        to: MusicalTime,
    },
    /// A longer-term change of tonic (OMT 051).
    Modulation {
        /// The key being left.
        from_key: Key,
        /// The key being reached.
        to_key: Key,
        /// How the new key is introduced.
        how: Approach,
        /// Where the change is heard.
        at: MusicalTime,
    },
}

/// A chord an analysis names: a spelled root, the vocabulary word for what is
/// stacked on it, and the class in the bass when the bass is not the root.
///
/// This is the analysis's own name for a chord and not the compiler's chord
/// algebra. Publishing `crate::chord::ChordClass` was the alternative, and it
/// would have exported a construction API — inversion, re-rooting, slash
/// basses, the spelled member table — so that a reader could print four words.
/// A caller of [`analyze`] never builds a chord; it reads one. So the boundary
/// owns a value that answers exactly the three questions a reader asks, and
/// the algebra stays where the chords are built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChordName {
    root: PitchClass,
    quality: &'static str,
    bass: Option<PitchClass>,
}

impl ChordName {
    /// The spelled root class. Spelling is kept: an F-sharp chord is not a
    /// G-flat chord.
    pub fn root(self) -> PitchClass {
        self.root
    }

    /// The vocabulary word for what is stacked on the root: `major`,
    /// `dominant7`, `half_diminished7`. The same words `chord` takes in
    /// source, so a reading can be written back as a chord if an author wants
    /// it in the score.
    pub fn quality(self) -> &'static str {
        self.quality
    }

    /// The class in the bass, when the sonority does not put the root there.
    pub fn bass(self) -> Option<PitchClass> {
        self.bass
    }

    /// The name of a chord the theory layer built.
    fn of(class: ChordClass) -> Self {
        Self {
            root: class.root(),
            quality: class.kind().name(),
            bass: class.bass().filter(|bass| *bass != class.root()),
        }
    }
}

impl std::fmt::Display for ChordName {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "{} {}", self.root, self.quality)?;
        match self.bass {
            Some(bass) => write!(out, "/{bass}"),
            None => Ok(()),
        }
    }
}

/// How the sounding pitch classes stand to a chord's members.
///
/// Four relations rather than a score, because they are what a reader has to
/// argue about. "This is a C major triad" and "this is a C major triad with a
/// passing D in it" are different claims about the same notes, and flattening
/// them into one number with a threshold would hide exactly the disagreement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fit {
    /// The sounding classes are exactly the chord's members.
    Exact,
    /// Every sounding class is a member, and at least one member is absent —
    /// most often the fifth, which OMT 020 notes is routinely omitted.
    Incomplete,
    /// Every member sounds, and something else sounds too: the extra notes are
    /// candidates for embellishing tones (OMT 039), which this does not decide.
    WithExtraTones,
    /// Some members are absent *and* something else sounds. The weakest
    /// relation that is still worth reporting, and never a `Fact`.
    Partial,
}

impl Fit {
    /// The word a report prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Incomplete => "incomplete",
            Self::WithExtraTones => "with extra tones",
            Self::Partial => "partial",
        }
    }
}

/// Which cadence a potential cadence point is, at the strength its evidence
/// supports (OMT 036).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cadence {
    /// V–I with *do* in the top voice over a root-position tonic and a
    /// root-position dominant. The strongest cadence available.
    PerfectAuthentic,
    /// V–I with either of those two conditions unmet.
    ImperfectAuthentic,
    /// A phrase ending on V, whatever precedes it.
    Half,
    /// V–vi (or V–VI): the dominant resolves somewhere other than tonic.
    Deceptive,
}

impl Cadence {
    /// The abbreviation a reader writes on a score.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PerfectAuthentic => "PAC",
            Self::ImperfectAuthentic => "IAC",
            Self::Half => "HC",
            Self::Deceptive => "DC",
        }
    }

    /// The name spelled out.
    pub fn name(self) -> &'static str {
        match self {
            Self::PerfectAuthentic => "perfect authentic cadence",
            Self::ImperfectAuthentic => "imperfect authentic cadence",
            Self::Half => "half cadence",
            Self::Deceptive => "deceptive cadence",
        }
    }
}

/// How a new key arrives (OMT 051).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Approach {
    /// Straight into the new key, with nothing preparing it.
    Direct,
    /// Through a chord diatonic in both keys.
    Pivot,
}

impl Approach {
    /// The word a report prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Pivot => "pivot",
        }
    }
}

impl Observation {
    /// Where in the piece the observation is anchored — the first key every
    /// report is ordered by.
    pub fn at(&self) -> MusicalTime {
        match *self {
            Self::Sounding { onset, .. }
            | Self::Silence { onset, .. }
            | Self::Sonority { onset, .. }
            | Self::ChordFit { onset, .. }
            | Self::Numeral { onset, .. } => onset,
            Self::Written { at, .. } | Self::SymbolReading { at, .. } => at,
            Self::Cadence { at, .. } | Self::Modulation { at, .. } => at,
            Self::KeyInForce { from, .. }
            | Self::MeterInForce { from, .. }
            | Self::KeyRegion { from, .. }
            | Self::Tonicization { from, .. }
            | Self::RuleInForce { from, .. } => from,
            Self::Departure { from, .. } => from,
        }
    }

    /// The tie-break within one instant, so a key and the note under it print
    /// in a fixed order rather than in whatever order the reading produced.
    fn rank(&self) -> u8 {
        match *self {
            Self::MeterInForce { .. } => 0,
            Self::KeyInForce { .. } => 1,
            Self::KeyRegion { .. } => 2,
            Self::Modulation { .. } => 3,
            Self::Tonicization { .. } => 4,
            Self::Written { .. } => 5,
            Self::SymbolReading { .. } => 6,
            Self::Sonority { .. } => 7,
            Self::ChordFit { .. } => 8,
            Self::Numeral { .. } => 9,
            Self::Cadence { .. } => 10,
            Self::Sounding { .. } => 11,
            Self::Silence { .. } => 12,
            Self::RuleInForce { .. } => 13,
            Self::Departure { .. } => 14,
        }
    }
}

/// Where in the score a finding can be seen.
///
/// Three shapes rather than one optional span, because the three answer
/// genuinely different questions. An event has an identity a caller can select
/// by; an annotation has source text and no identity of its own; and a context
/// has neither, because a context track records where a value *takes force*,
/// which is a place in the piece rather than a place in a file. Collapsing
/// them into `Option<SourceSpan>` would make every consumer rediscover which
/// case it was looking at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// One score event, and the source that spells it.
    Event(NoteRef),
    /// A stretch of music and the notes in it — a simultaneity, a cadence's
    /// two chords, a key region. Every note is named rather than a hull span
    /// being invented, because two notes of one chord can come from two
    /// different motifs and no single source range covers them.
    Passage {
        /// Where the stretch begins.
        from: MusicalTime,
        /// Where it ends.
        to: MusicalTime,
        /// The notes in it, in the report's own order.
        notes: Vec<NoteRef>,
    },
    /// Something written above the staff, and its source.
    Annotation {
        /// Where the annotation is written.
        span: SourceSpan,
    },
    /// A value in force from an instant, with no statement to point at.
    InForce {
        /// The scope reading it, since a key in force in one voice may have
        /// been written for the piece.
        scope: Scope,
        /// Where the value takes force.
        from: MusicalTime,
    },
}

/// One note, by every coordinate a caller can act on.
///
/// Public fields: this is a coordinate record, and an accessor per field would
/// be four functions that hide nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteRef {
    /// Which part it is in.
    pub part: PartId,
    /// Which voice of that part.
    pub voice: VoiceId,
    /// The event's snapshot-local identity.
    pub id: EventId,
    /// The statement that spells it — the note inside the motif when the event
    /// came from one, so the reference points at editable text.
    pub span: SourceSpan,
}

/// One criterion a finding was judged against, and whether it held.
///
/// This is the part of a finding that makes it arguable. "Perfect authentic
/// cadence" is an assertion; "authentic cadence, root position yes, *do* in
/// the top voice no, phrase ending unknown" is a reading a musician can
/// disagree with in a specific place. Every kind that classifies states its
/// criteria this way, with the source that defines each one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ground {
    /// What was checked, in a few words.
    pub criterion: &'static str,
    /// Whether the music satisfies it.
    pub satisfied: bool,
    /// Where the criterion comes from: `OMT 036`.
    pub cites: &'static str,
}

/// One thing an analysis saw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisFinding {
    code: &'static str,
    standing: Standing,
    observation: Observation,
    evidence: Evidence,
    grounds: Vec<Ground>,
}

impl AnalysisFinding {
    /// The stable code, for a consumer that filters or groups without matching
    /// on prose: `sounding-pitch`, `key-in-force`.
    pub fn code(&self) -> &'static str {
        self.code
    }

    /// How firmly the finding is held.
    pub fn standing(&self) -> Standing {
        self.standing
    }

    /// What was seen.
    pub fn observation(&self) -> &Observation {
        &self.observation
    }

    /// Where it can be seen in the score.
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }

    /// A finding with no criteria behind it: an observation, not a judgement.
    fn stated(code: &'static str, observation: Observation, evidence: Evidence) -> Self {
        Self {
            code,
            standing: Standing::Fact,
            observation,
            evidence,
            grounds: Vec::new(),
        }
    }

    /// A finding a kind reached by checking criteria, with those criteria and
    /// their verdicts attached.
    fn judged(
        code: &'static str,
        standing: Standing,
        observation: Observation,
        evidence: Evidence,
        grounds: Vec<Ground>,
    ) -> Self {
        Self {
            code,
            standing,
            observation,
            evidence,
            grounds,
        }
    }

    /// What it was judged against, and what held.
    ///
    /// Empty for a kind that classifies nothing — a `facts` finding is not a
    /// judgement and has no criteria to show.
    pub fn grounds(&self) -> &[Ground] {
        &self.grounds
    }
}

/// What an analysis saw, with the method and assumptions that produced it.
///
/// Immutable: a report records one reading, and a consumer able to add a
/// finding to it would be publishing its own conclusions under the analysis's
/// name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisReport {
    kind: AnalysisKind,
    profile: Option<AnalysisProfile>,
    findings: Vec<AnalysisFinding>,
}

impl AnalysisReport {
    /// Which analysis produced this.
    pub fn kind(&self) -> AnalysisKind {
        self.kind
    }

    /// The style profile it read against, for the kinds that read against one.
    pub fn profile(&self) -> Option<AnalysisProfile> {
        self.profile
    }

    /// One line saying what it does.
    pub fn method(&self) -> &'static str {
        self.kind.method()
    }

    /// What it took for granted: the kind's assumptions, then the profile's.
    ///
    /// Owned rather than borrowed because a profiled reading assumes two sets
    /// of things and a reader needs both — the alternative was a second
    /// accessor whose caller would have to remember to print it.
    pub fn assumptions(&self) -> Vec<&'static str> {
        let mut assumptions = self.kind.assumptions().to_vec();
        if let Some(profile) = self.profile {
            assumptions.extend_from_slice(profile.assumptions());
        }
        assumptions
    }

    /// Everything it saw, in a deterministic order: by instant, then by the
    /// fixed rank of the observation kind, then by part, voice, and event.
    ///
    /// The order belongs to the report and not to a renderer, because "two
    /// runs of one analysis on one score produce the same bytes" has to be a
    /// property of [`analyze`] for any caller to diff two reports at all.
    pub fn findings(&self) -> &[AnalysisFinding] {
        &self.findings
    }

    /// Put the findings in that order and drop exact repeats.
    ///
    /// A repeat is possible because context tracks are read per voice and two
    /// voices of one part see the same piece-wide key. Two voices *sounding*
    /// the same pitch are not repeats — their evidence differs — so
    /// deduplication is on the whole finding and never on the observation
    /// alone. Applied at the one construction site, so no kind can forget it.
    fn canonical(mut self) -> Self {
        self.findings.sort_by_key(order);
        self.findings.dedup();
        self
    }
}

/// Read `snapshot` the way `request` asks, and report what is there.
///
/// This is the whole boundary. Segmenters, indexes, candidate graphs, and
/// theory values stay behind it: a caller supplies a request and receives a
/// report, and nothing about how the reading was computed is observable.
///
/// The snapshot is borrowed and never mutated — an analysis cannot change a
/// score, and that is a property of this signature rather than a promise in
/// prose.
///
/// # Errors
/// [`AnalysisError`] when the request names a part or a voice this score does
/// not have, or a window that contains no music. An empty *answer* is not an
/// error: a window with nothing in it is a report with no findings.
pub fn analyze(snapshot: &ScoreSnapshot, request: &AnalysisRequest) -> Result<AnalysisReport, AnalysisError> {
    let lanes = select(snapshot, &request.scope)?;
    let window = window_of(request)?;
    let profile = profile_of(request)?;
    let findings = match request.kind {
        AnalysisKind::Facts => facts::observe(snapshot, &lanes, &request.scope, window),
        AnalysisKind::Chords => {
            let slices = segment::slices(snapshot, &lanes, request.segmentation, window);
            chords::observe(snapshot, &slices, &request.scope, window)
        }
        AnalysisKind::Tonal => {
            let slices = segment::slices(snapshot, &lanes, request.segmentation, window);
            tonal::observe(snapshot, &lanes, &slices, request.key)
        }
        AnalysisKind::Cadences => {
            let slices = segment::slices(snapshot, &lanes, request.segmentation, window);
            cadence::observe(snapshot, &lanes, &slices, request.key)
        }
        AnalysisKind::VoiceLeading => voice_leading::observe(snapshot, &lanes, profile, request.key, window)?,
        AnalysisKind::Counterpoint => {
            counterpoint::observe(snapshot, &lanes, profile, request.cantus.as_deref(), window)?
        }
    };
    Ok(AnalysisReport {
        kind: request.kind,
        profile,
        findings,
    }
    .canonical())
}

/// The profile the request selects, checked against the kind that will use it.
///
/// Both directions are errors and both are the caller's to fix: a kind that
/// reads against a style with no style named cannot pick one, and a style
/// named for the wrong kind is a request that means something the caller did
/// not write.
fn profile_of(request: &AnalysisRequest) -> Result<Option<AnalysisProfile>, AnalysisError> {
    let profiled = matches!(request.kind, AnalysisKind::VoiceLeading | AnalysisKind::Counterpoint);
    match (request.profile, profiled) {
        (Some(profile), true) if profile.kind() != request.kind => Err(AnalysisError::WrongProfile {
            profile,
            kind: request.kind,
        }),
        (Some(profile), true) => Ok(Some(profile)),
        (None, true) => Err(AnalysisError::ProfileRequired { kind: request.kind }),
        // A profile on a kind that reads no style is ignored rather than
        // refused: the field is a builder, and a caller reusing one request
        // for several kinds should not have to unset it.
        (_, false) => Ok(None),
    }
}

/// The half-open window a request selects, or the error saying it selects
/// nothing.
fn window_of(request: &AnalysisRequest) -> Result<Option<(MusicalTime, MusicalTime)>, AnalysisError> {
    match request.window {
        Some((from, to)) if to <= from => Err(AnalysisError::EmptyWindow { from, to }),
        other => Ok(other),
    }
}

/// One voice an analysis reads, resolved from the names the request used.
struct Lane {
    part: PartId,
    voice: VoiceId,
}

impl Lane {
    /// The scope this lane reads context tracks through, so a key written for
    /// the piece and one written for the part both answer correctly.
    fn scope(&self) -> Scope {
        Scope::Voice {
            part: self.part.0,
            voice: self.voice.0,
        }
    }
}

/// Resolve the request's names into the voices it selects, in score order.
///
/// The lookup lives here rather than in each kind because every kind asks the
/// same question about scope, and because a misspelled part is a mistake about
/// the request rather than about the music.
fn select(snapshot: &ScoreSnapshot, scope: &AnalysisScope) -> Result<Vec<Lane>, AnalysisError> {
    let lanes = |id: PartId, part: &crate::score::Part| -> Vec<Lane> {
        part.voices().map(|(voice, _)| Lane { part: id, voice }).collect()
    };
    let named = |wanted: &str| {
        snapshot
            .parts()
            .iter()
            .find(|(_, part)| part.name() == wanted)
            .ok_or_else(|| AnalysisError::NoSuchPart {
                name: wanted.to_owned(),
                available: snapshot
                    .parts()
                    .iter()
                    .map(|(_, part)| part.name().to_owned())
                    .collect(),
            })
    };
    match *scope {
        AnalysisScope::Score => Ok(snapshot.parts().iter().flat_map(|(id, part)| lanes(id, part)).collect()),
        AnalysisScope::Part(ref name) => {
            let (id, part) = named(name)?;
            Ok(lanes(id, part))
        }
        AnalysisScope::Voice { ref part, ref voice } => {
            let (id, found) = named(part)?;
            let wanted = found
                .voices()
                .map(|(candidate, _)| candidate)
                .find(|candidate| found.voice_name(*candidate) == Some(voice.as_str()))
                .ok_or_else(|| AnalysisError::NoSuchVoice {
                    part: part.clone(),
                    name: voice.clone(),
                    available: found
                        .voices()
                        .filter_map(|(candidate, _)| found.voice_name(candidate))
                        .map(str::to_owned)
                        .collect(),
                })?;
            Ok(vec![Lane {
                part: id,
                voice: wanted,
            }])
        }
    }
}

/// Whether `at` falls in the half-open window, with `None` meaning the whole
/// piece.
fn inside(window: Option<(MusicalTime, MusicalTime)>, at: MusicalTime) -> bool {
    match window {
        None => true,
        Some((from, to)) => at >= from && at < to,
    }
}

/// The total order every report is in.
fn order(finding: &AnalysisFinding) -> (MusicalTime, u8, u32, u32, u64, u32) {
    let (part, voice, id, span) = match finding.evidence {
        Evidence::Event(note) => (note.part.0, note.voice.0, note.id.0, note.span.start),
        Evidence::Passage { ref notes, .. } => notes.first().map_or((u32::MAX, u32::MAX, u64::MAX, u32::MAX), |note| {
            (note.part.0, note.voice.0, note.id.0, note.span.start)
        }),
        Evidence::Annotation { span } => (u32::MAX, u32::MAX, u64::MAX, span.start),
        Evidence::InForce { .. } => (u32::MAX, u32::MAX, u64::MAX, u32::MAX),
    };
    (
        finding.observation.at(),
        finding.observation.rank(),
        part,
        voice,
        id,
        span,
    )
}

mod cadence;
mod chords;
mod counterpoint;
mod facts;
pub(crate) mod motion;
mod rules;
mod segment;
mod tonal;
mod voice_leading;
