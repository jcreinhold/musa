//! Explicit musical assertions (prompt 116): claims a composer writes down
//! and the compiler proves.
//!
//! Three things are kept apart here, and `docs/language/05-verification.md` is
//! why. A **constructor invariant** is what a value must satisfy to exist at
//! all, and it is checked where the value is made. An **analysis** is an
//! interpretation, it is named, and it never blocks a compilation. Between
//! them sits this module: a decidable claim about a particular passage that
//! the composer asked for, in writing, and that the compiler either proves or
//! refuses with the counterexample in hand.
//!
//! Three rules follow from that middle position, and every design decision
//! below is one of them:
//!
//! - **Nothing here is a rule about the piece.** A claim binds the braces it
//!   is written on and no other music. There is no global out-of-key warning,
//!   no chord-fit check, no SATB pass; a composer who wants one writes it,
//!   passage by passage, and a composer who does not never meets it.
//! - **A claim is drawn from a registry, not from the composer's own
//!   functions.** [`CLAIMS`] is the whole family. That is not a limitation
//!   waiting to be lifted: an arbitrary predicate over the music would need
//!   the music handed to it, and handing over a `ScoreFact` would make every
//!   payload field a public interface. What a claim may read is [`Passage`],
//!   which is sounded written pitches with their exact spans and nothing else.
//! - **Nothing is repaired.** A passage that leaves its scale is reported with
//!   the note that left it; it is never respelled, never transposed, never
//!   rewritten. `05-verification.md` §2: "assertions never repair or respell
//!   music."
//!
//! The one claim that predates this module is the bar's. `bar { … }` has
//! asserted "this is one measure" since prompt 57, and [`Claim::FillsMeter`]
//! is that same check with the same sentences — the noun changes, because a
//! passage is not a bar, and nothing else does. A bar and an assertion are two
//! spellings of one obligation, and they are checked by one function so they
//! cannot drift into two answers.

// The arithmetic here is on `Ratio<i64>` through the `MusicalTime` operators,
// which are exact mathematical arithmetic and total for musa's magnitudes
// (see `time.rs`); the workspace lint is allowed at module scope for that.
#![allow(clippy::arithmetic_side_effects)]

use crate::chord::ChordClass;
use crate::diagnose::{Code, Diagnostic};
use crate::origin::SourceSpan;
use crate::pitch::{PitchClass, WrittenPitch};
use crate::scale::{Degree, Scale};
use crate::time::{MusicalDuration, MusicalTime};

/// One note an assertion is allowed to see.
///
/// Written pitch, not sounding frequency; exact rational bounds, not seconds.
/// The `at` span is where the note is written, which is what a diagnostic
/// points at — a claim that fails names the note the composer can go and look
/// at, and never the whole passage.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sounded {
    /// The written pitch, spelled as the composer spelled it.
    pub(crate) pitch: WrittenPitch,
    /// When it starts, relative to the passage.
    pub(crate) start: MusicalTime,
    /// When it stops.
    pub(crate) end: MusicalTime,
    /// Where it is written.
    pub(crate) at: SourceSpan,
}

/// The passage a claim is about: everything a claim may read, and nothing
/// else.
///
/// This is the "coherent private view" — the reason no claim takes a callback.
/// A predicate that could see a `ScoreFact` would see dynamics, articulations,
/// clefs, provenance, and the tie flag that elaboration has already resolved,
/// and every one of those would become an interface the compiler could not
/// change. What is here is what the four claims below need and what a musician
/// would say they are about: which notes sound, when, and where they are
/// written.
pub(crate) struct Passage {
    /// Where the claim is written — the assertion statement, or the bar.
    pub(crate) span: SourceSpan,
    /// Where the passage begins in the piece. Absolute, because a bar's
    /// length is measured against the meter in force *there*.
    pub(crate) at: MusicalTime,
    /// How long the passage is.
    pub(crate) extent: MusicalDuration,
    /// Where an inserted rest would go: after the last thing written inside
    /// the braces. Absent when there is nothing written inside them.
    pub(crate) content_end: Option<u32>,
    /// Every note sounded in the passage, in occurrence order.
    pub(crate) notes: Vec<Sounded>,
    /// What the composer wrote, for the sentence that names it: a `bar`, or
    /// the passage an `assert` was written on.
    pub(crate) noun: &'static str,
}

/// What the piece turned out to be, which no passage can know on its own.
///
/// Where the barlines fall depends on every `meter` in every voice, so it is
/// settled once, after all of them have been read — which is why an obligation
/// is recorded during elaboration and discharged after it. A claim is proved
/// against what was written and against what the piece decided, and these are
/// the second of those two.
pub(crate) struct Settled<'a> {
    /// Where the barlines fall.
    pub(crate) bars: &'a crate::BarLines,
    /// Whether the piece states a `meter` at all. Only the measure claim
    /// reads it, and only for its closing note.
    pub(crate) meter_written: bool,
}

/// How exactly a passage must match the chord it claims to realize.
///
/// Three modes because they are three different claims, and a single
/// `realizes` that quietly meant the loosest of them would prove almost
/// nothing. Each is stated over *pitch classes*: a voicing that doubles the
/// root sounds four notes and three classes, and doubling is invisible to
/// every one of these by construction — which is right, because OMT `019`
/// makes doubling a matter of voicing and not of chord content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Realization {
    /// The classes sounded are exactly the chord's members: nothing omitted,
    /// nothing added. Strict pitch-set equality.
    Exactly,
    /// Every class sounded is a member, and a member may be missing — the
    /// three-voice seventh chord that leaves out the fifth.
    MayOmit,
    /// Every member sounds, and other notes may sound too — the chord tone
    /// with passing notes and suspensions around it.
    MayAdd,
}

impl Realization {
    /// The three spellings, in the order a diagnostic lists them.
    pub(crate) const ALL: [Self; 3] = [Self::Exactly, Self::MayOmit, Self::MayAdd];

    /// How the source spells it.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Exactly => "exactly",
            Self::MayOmit => "may_omit",
            Self::MayAdd => "may_add",
        }
    }

    /// Read one of the three words, or `None` for anything else.
    pub(crate) fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|policy| policy.as_str() == word)
    }
}

/// What kind of thing one argument of a claim is.
///
/// The registry declares these, and they are the only shapes an argument can
/// have. [`ParamType::Policy`] is the odd one and deliberately so: its three
/// inhabitants are words rather than values in the elaboration language,
/// because a type no function can take or return would be language surface
/// with no caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParamType {
    /// A `Scale`, written `scale c major` or named.
    Scale,
    /// A `ChordClass`, written `chord c major7` or computed.
    Chord,
    /// A `Nat`.
    Count,
    /// A `List<(Pitch, Pitch)>`, low and high for each voice.
    Ranges,
    /// One of [`Realization`]'s three words.
    Policy,
}

impl ParamType {
    /// How the type is written, for the sentence that says what was expected.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Scale => "Scale",
            Self::Chord => "ChordClass",
            Self::Count => "Nat",
            Self::Ranges => "List<(Pitch, Pitch)>",
            // The three words themselves, because a policy has no type in
            // the elaboration language to name here — it is a word the
            // registry reads, and the honest way to say what is expected is
            // to list what there is.
            Self::Policy => "exactly|may_omit|may_add",
        }
    }
}

/// One evaluated argument, handed across from the elaboration language.
///
/// The value language's own `Value` stays private to `core`: this is the five
/// shapes a claim can be built from, and the boundary is here so that adding a
/// claim cannot widen what the checker can see.
pub(crate) enum Argument {
    /// A scale.
    Scale(Scale),
    /// A chord class.
    Chord(ChordClass),
    /// A count.
    Count(u64),
    /// Low and high bounds, one pair per voice, lowest voice first.
    Ranges(Vec<(WrittenPitch, WrittenPitch)>),
    /// A realization policy.
    Policy(Realization),
}

/// Reading an argument back out, one shape at a time.
///
/// Five one-line methods rather than five `match` arms with a fallthrough:
/// [`Claim::build`] asks for exactly the shape its own signature declares, and
/// a wrong shape is a bug in the checker that filled the vector rather than a
/// case to handle. `None` is the belt to that braces.
impl Argument {
    fn scale(self) -> Option<Scale> {
        if let Self::Scale(scale) = self {
            Some(scale)
        } else {
            None
        }
    }

    fn chord(self) -> Option<ChordClass> {
        if let Self::Chord(chord) = self {
            Some(chord)
        } else {
            None
        }
    }

    fn count(self) -> Option<u64> {
        if let Self::Count(count) = self {
            Some(count)
        } else {
            None
        }
    }

    fn ranges(self) -> Option<Vec<(WrittenPitch, WrittenPitch)>> {
        if let Self::Ranges(ranges) = self {
            Some(ranges)
        } else {
            None
        }
    }

    fn policy(self) -> Option<Realization> {
        if let Self::Policy(policy) = self {
            Some(policy)
        } else {
            None
        }
    }
}

/// One claim in the family: its name, its arguments, and what it checks.
///
/// A claim that says "these notes are in this scale" has to mean one definite
/// thing, and the honest way to fix which one is to name the page it comes
/// from. Those citations are not a field here, because a citation nobody reads
/// back is a string the compiler carries around: each one is written on the
/// function that implements the claim, where the next person to change the
/// rule will be standing.
pub(crate) struct Predicate {
    /// How the claim is written.
    pub(crate) name: &'static str,
    /// Its arguments, in order.
    pub(crate) parameters: &'static [ParamType],
    /// What it checks, as one sentence.
    pub(crate) checks: &'static str,
}

/// Every claim a composer may write. This list *is* the family — there is no
/// registration, no extension point, and no user predicate.
pub(crate) const CLAIMS: [Predicate; 5] = [
    Predicate {
        name: "fills_meter",
        parameters: &[],
        checks: "the passage is exactly one measure of the meter in force where it is written",
    },
    Predicate {
        name: "pitches_in",
        parameters: &[ParamType::Scale],
        checks: "every note sounded in the passage is spelled as a member of the scale",
    },
    Predicate {
        name: "realizes",
        parameters: &[ParamType::Chord, ParamType::Policy],
        checks: "the pitch classes sounded in the passage stand in the named relation to the chord's members",
    },
    Predicate {
        name: "voices",
        parameters: &[ParamType::Count],
        checks: "wherever anything sounds in the passage, exactly that many notes sound at once",
    },
    Predicate {
        name: "within_ranges",
        parameters: &[ParamType::Ranges],
        checks: "each voice of each sonority, counted from the bottom, lies within the range given for it",
    },
];

/// The claim written under `name`, or `None` if nothing is.
pub(crate) fn predicate(name: &str) -> Option<&'static Predicate> {
    CLAIMS.iter().find(|claim| claim.name == name)
}

/// The claim names, for the "did you mean" of a name nobody wrote down.
pub(crate) fn names() -> impl Iterator<Item = &'static str> {
    CLAIMS.iter().map(|claim| claim.name)
}

/// A claim with its arguments supplied: what an obligation carries and what
/// [`check`] proves.
#[derive(Clone)]
pub(crate) enum Claim {
    /// One measure of the meter in force.
    FillsMeter,
    /// Every sounded pitch spelled in the collection.
    PitchesIn(Scale),
    /// The sounded classes realize the chord, under a policy.
    Realizes {
        /// The chord claimed.
        chord: ChordClass,
        /// How exact the match must be.
        policy: Realization,
    },
    /// How many notes sound at once.
    Voices(u64),
    /// Where each voice may lie, lowest first.
    WithinRanges(Vec<(WrittenPitch, WrittenPitch)>),
}

impl Claim {
    /// Build the claim `name` names from its evaluated arguments.
    ///
    /// Returns `None` only when the arguments do not have the shapes the
    /// registry declares — which the caller has already checked, so this is
    /// the belt to that braces rather than a diagnostic path.
    pub(crate) fn build(name: &str, arguments: Vec<Argument>) -> Option<Self> {
        let mut arguments = arguments.into_iter();
        let claim = match name {
            "fills_meter" => Self::FillsMeter,
            "pitches_in" => Self::PitchesIn(arguments.next()?.scale()?),
            "realizes" => Self::Realizes {
                chord: arguments.next()?.chord()?,
                policy: arguments.next()?.policy()?,
            },
            "voices" => Self::Voices(arguments.next()?.count()?),
            "within_ranges" => Self::WithinRanges(arguments.next()?.ranges()?),
            _ => return None,
        };
        arguments.next().is_none().then_some(claim)
    }

    /// How the claim reads back: the name with its arguments, spelled the way
    /// a composer would write them.
    ///
    /// This is what the [`crate::ExpansionStep::Assertion`] on every fact
    /// underneath the assertion records. The arguments belong in it because
    /// "this note exists under `pitches_in(scale c major)`" is a provenance
    /// line someone can act on, and "under `pitches_in`" is not.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::FillsMeter => "fills_meter()".to_owned(),
            Self::PitchesIn(scale) => format!("pitches_in({scale})"),
            Self::Realizes { chord, policy } => format!("realizes({chord}, {})", policy.as_str()),
            Self::Voices(count) => format!("voices({count})"),
            Self::WithinRanges(ranges) => format!("within_ranges({} ranges)", ranges.len()),
        }
    }
}

/// Prove `claim` about `passage`, or say why it does not hold.
///
/// One entry point for every obligation the compiler has taken on, whether a
/// composer wrote `assert` or wrote a bar. The order inside is the order the
/// claims are declared, and each arm reports at most one thing: the *smallest*
/// witness, because a passage with four notes outside its scale is one mistake
/// with four instances and a composer fixes the first one first.
pub(crate) fn check(claim: &Claim, passage: &Passage, settled: &Settled<'_>) -> Option<Diagnostic> {
    match claim {
        Claim::FillsMeter => fills_meter(passage, settled),
        Claim::PitchesIn(scale) => pitches_in(passage, *scale),
        Claim::Realizes { chord, policy } => realizes(passage, *chord, *policy),
        Claim::Voices(count) => voices(passage, *count),
        Claim::WithinRanges(ranges) => within_ranges(passage, ranges),
    }
}

/// The measure claim, which `bar { … }` has been making since prompt 57.
///
/// OMT `010`, on simple meter and time signatures: a measure holds what the
/// signature says it holds, no more and no less.
///
/// Its sentences are prompt 57's, unchanged, because a composer who has read
/// one of these has read all of them. The single difference is the noun: a
/// `bar` says "this bar", and an `assert fills_meter()` says "this passage",
/// because it is not a bar and calling it one would be the diagnostic lying
/// about what is on the page.
fn fills_meter(passage: &Passage, settled: &Settled<'_>) -> Option<Diagnostic> {
    let here = settled.bars.measure_at(passage.at);
    // Inside an unmeasured stretch there is no measure for the passage to be
    // one of. Refused rather than ignored: the claim cannot be checked, and a
    // claim nobody checks is exactly what this module exists to prevent.
    if !here.meter.is_measured() {
        return Some(
            Diagnostic::error(
                Code::DoesNotAddUp,
                format!("a `{}` here has no measure to be one of", passage.noun),
            )
            .at(passage.span, "this is inside unmeasured music")
            .help(format!(
                "delete the `{}`, or close the unmeasured stretch before it",
                passage.noun
            ))
            .note("`senza { ... }` and `meter none;` say the barlines stop; a `bar` says where one falls"),
        );
    }
    let measure = here.length().as_ratio();
    let written = passage.extent.as_ratio();
    if written == measure {
        return None;
    }
    let long = written > measure;
    let difference = if long { written - measure } else { measure - written };
    // The rule goes in the note rather than in a second label on the `meter`
    // statement: the meter is usually pages away, and miette draws a distant
    // span as its own framed snippet — which doubles the size of every one of
    // these to restate a fact the note states in six words.
    let diagnostic = Diagnostic::error(
        Code::DoesNotAddUp,
        format!(
            "this {} is {} {}",
            passage.noun,
            fraction(difference),
            if long { "too long" } else { "short" }
        ),
    )
    .at(passage.span, format!("these add up to {}", fraction(written)))
    .note(if settled.meter_written {
        format!(
            "`meter {}/{}` makes a bar {}",
            here.meter.numerator(),
            here.meter.denominator(),
            fraction(measure)
        )
    } else {
        format!(
            "a piece that writes no `meter` is in 4/4, so a bar is {}",
            fraction(measure)
        )
    });
    Some(if long {
        // Which note to remove is the composer's decision, and a fix that
        // guesses is worse than a help line that does not (prompt 56).
        diagnostic.help("shorten a duration, or move the last of these into the next bar")
    } else {
        let rest = format!("rest{}", musa_language::spell_duration(&fraction(difference)));
        let filled = diagnostic.help(format!("add `{rest}`, or lengthen one of the durations"));
        match passage.content_end {
            Some(at) => filled.fix(format!("add `{rest}`"), SourceSpan::new(at, at), format!(" {rest}")),
            None => filled,
        }
    })
}

/// Every sounded pitch is spelled as a member of the collection.
///
/// *Spelled* is the whole content of the claim. `fs5` in C major fails, and it
/// fails as F-sharp: the diagnostic does not read it as G-flat, does not read
/// it as a raised fourth degree, and does not offer to make it an F. OMT `013`
/// gives a scale as a definite collection of spelled classes, and prompt 116's
/// rule is that a chromatic alteration is reported rather than respelled —
/// which is the same rule as `AGENTS.md`'s "written pitch is not a MIDI
/// number", said about a diagnostic.
fn pitches_in(passage: &Passage, scale: Scale) -> Option<Diagnostic> {
    let members = members_of(scale);
    let stray = passage
        .notes
        .iter()
        .find(|note| !members.contains(&note.pitch.pitch_class()))?;
    let spelled: Vec<String> = members.iter().map(ToString::to_string).collect();
    Some(
        Diagnostic::error(Code::UnmetClaim, format!("`{}` is not in {scale}", stray.pitch))
            .at(stray.at, "this note leaves the collection")
            .also(passage.span, "the passage claims every note is in it")
            .help(format!("{scale} is spelled {}", spelled.join(" ")))
            .note("the note is reported, never respelled: an enharmonic neighbour is a different written pitch"),
    )
}

/// The sounded pitch classes stand in the policy's relation to the chord.
///
/// Two things are checked, and both come from OMT. The *content* is the set
/// relation the policy names (`017`, `018`). The *bass* is checked only when
/// the chord class designates one, and then it is checked as `019` defines it:
/// the lowest note sounding is what decides inversion, so a class inverted to
/// put the third in the bass is a claim about which note is lowest and is
/// proved by looking at the lowest note.
fn realizes(passage: &Passage, chord: ChordClass, policy: Realization) -> Option<Diagnostic> {
    let members: Vec<PitchClass> = (0..chord.size()).filter_map(|at| chord.member_class(at)).collect();
    let spelled: Vec<String> = members.iter().map(ToString::to_string).collect();
    let claimed = || format!("`{chord}` is {}", spelled.join(" "));
    if matches!(policy, Realization::Exactly | Realization::MayOmit)
        && let Some(stray) = passage
            .notes
            .iter()
            .find(|note| !members.contains(&note.pitch.pitch_class()))
    {
        return Some(
            Diagnostic::error(
                Code::UnmetClaim,
                format!("`{}` is not a member of `{chord}`", stray.pitch),
            )
            .at(stray.at, "this note is not in the chord")
            .also(passage.span, "the passage claims to realize it")
            .help(claimed())
            .note(format!(
                "`{}` permits no added tones; `{}` is the policy that does",
                policy.as_str(),
                Realization::MayAdd.as_str()
            )),
        );
    }
    if matches!(policy, Realization::Exactly | Realization::MayAdd)
        && let Some(missing) = members
            .iter()
            .find(|member| !passage.notes.iter().any(|note| note.pitch.pitch_class() == **member))
    {
        return Some(
            Diagnostic::error(Code::UnmetClaim, format!("`{chord}` is missing its `{missing}`"))
                .at(passage.span, "nothing here sounds that member")
                .help(claimed())
                .note(format!(
                    "`{}` requires every member to sound; `{}` is the policy that lets one go",
                    policy.as_str(),
                    Realization::MayOmit.as_str()
                )),
        );
    }
    let bass = chord.bass()?;
    let lowest = passage
        .notes
        .iter()
        .min_by_key(|note| (note.pitch.chromatic_height(), note.pitch.diatonic_height()))?;
    (lowest.pitch.pitch_class() != bass).then(|| {
        Diagnostic::error(
            Code::UnmetClaim,
            format!("`{chord}` puts `{bass}` in the bass, and `{}` is lowest", lowest.pitch),
        )
        .at(lowest.at, "this is the lowest note sounding")
        .also(passage.span, "the passage claims the chord's own bass")
        .note("OMT 019: the lowest note is what decides inversion, whatever the chord's root is")
    })
}

/// Exactly `count` notes sound wherever anything sounds.
///
/// OMT `022`, chords in SATB style: four voices means four, at every chord.
///
/// Silence is not a violation — a rest in every voice is four voices resting —
/// so the claim is checked over the stretches where something is sounding.
/// Those stretches are the maximal intervals between consecutive note
/// boundaries, which is the finest division the passage can distinguish and so
/// the only one that cannot miss a moment.
fn voices(passage: &Passage, count: u64) -> Option<Diagnostic> {
    let claimed = usize::try_from(count).unwrap_or(usize::MAX);
    let (at, sounding) = sonorities(passage)
        .into_iter()
        .find(|(_, sounding)| sounding.len() != claimed)?;
    let lowest = sounding.first().copied()?;
    Some(
        Diagnostic::error(
            Code::UnmetClaim,
            format!("{} here, and the passage claims {count}", spell_notes(sounding.len())),
        )
        .at(lowest.at, "sounding from here")
        .also(passage.span, format!("`voices({count})` is the claim"))
        .help(format!("this is {} into the passage", fraction(at.as_ratio())))
        .note("silence is not counted: a moment where nothing sounds is no voices, not a wrong number of them"),
    )
}

/// Each voice of each sonority lies in the range given for it.
///
/// The pairing is by height and nothing else: the lowest note sounding is
/// checked against the first range, the next against the second, and so on
/// upward. That is what makes a list of four ranges an SATB claim without the
/// compiler having any notion of "the alto" — OMT `022` gives four ranges in
/// exactly that order, and the passage's own notes say which is which.
///
/// A sonority with a different number of notes than there are ranges is
/// reported as that, and not silently paired off: the positional reading has
/// nothing to say when the positions do not line up, and `voices(n)` is the
/// claim that pins the count.
fn within_ranges(passage: &Passage, ranges: &[(WrittenPitch, WrittenPitch)]) -> Option<Diagnostic> {
    if let Some((low, high)) = ranges
        .iter()
        .find(|(low, high)| low.chromatic_height() > high.chromatic_height())
    {
        return Some(
            Diagnostic::error(Code::UnmetClaim, format!("the range `{low}` to `{high}` is empty"))
                .at(passage.span, "this range is written from high to low")
                .help("write each range as its lowest pitch and then its highest: `(c3, g4)`"),
        );
    }
    for (at, sounding) in sonorities(passage) {
        if sounding.len() != ranges.len() {
            let lowest = sounding.first().copied()?;
            return Some(
                Diagnostic::error(
                    Code::UnmetClaim,
                    format!(
                        "{} here, and {} given",
                        spell_notes(sounding.len()),
                        spell_ranges(ranges.len())
                    ),
                )
                .at(lowest.at, "sounding from here")
                .also(passage.span, "each range belongs to one voice, counted from the bottom")
                .help(format!("this is {} into the passage", fraction(at.as_ratio())))
                .note("`voices(n)` is the claim that fixes how many voices there are"),
            );
        }
        for (position, (note, (low, high))) in sounding.iter().zip(ranges.iter()).enumerate() {
            let height = note.pitch.chromatic_height();
            if height >= low.chromatic_height() && height <= high.chromatic_height() {
                continue;
            }
            let which = if height < low.chromatic_height() {
                "below"
            } else {
                "above"
            };
            return Some(
                Diagnostic::error(
                    Code::UnmetClaim,
                    format!("`{}` is {which} the range for the {} voice", note.pitch, nth(position)),
                )
                .at(note.at, format!("this voice may lie from `{low}` to `{high}`"))
                .also(passage.span, "the ranges are given lowest voice first")
                .note("OMT 022 gives the SATB ranges as bass f2-d4, tenor c3-g4, alto g3-d5, soprano c4-g5"),
            );
        }
    }
    None
}

/// The passage's sonorities: each stretch over which the same notes sound,
/// with those notes ordered from the bottom up.
///
/// Boundaries are every note's start and end, which is exactly the set of
/// moments at which the sounding notes can change. Stretches where nothing
/// sounds are dropped, and a zero-length stretch cannot arise because
/// consecutive distinct boundaries bound a positive interval.
fn sonorities(passage: &Passage) -> Vec<(MusicalTime, Vec<&Sounded>)> {
    let mut boundaries: Vec<MusicalTime> = passage.notes.iter().flat_map(|note| [note.start, note.end]).collect();
    // By value, not by (numerator, denominator): 1/2 is before 1, and a sort
    // on the pair puts it after.
    boundaries.sort_unstable_by_key(|at| at.as_ratio());
    boundaries.dedup();
    let mut found = Vec::new();
    for pair in boundaries.windows(2) {
        let (Some(start), Some(end)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        let mut sounding: Vec<&Sounded> = passage
            .notes
            .iter()
            .filter(|note| note.start <= *start && note.end >= *end)
            .collect();
        if sounding.is_empty() {
            continue;
        }
        sounding.sort_by_key(|note| (note.pitch.chromatic_height(), note.pitch.diatonic_height()));
        found.push((*start, sounding));
    }
    found
}

/// The spelled members of a collection, one per degree of its period.
fn members_of(scale: Scale) -> Vec<PitchClass> {
    (1..=scale.size())
        .filter_map(|ordinal| scale.class(Degree::new(i64::try_from(ordinal).ok()?)))
        .collect()
}

/// A musical amount, spelled the way the language spells it.
fn fraction(value: num_rational::Ratio<i64>) -> String {
    if *value.denom() == 1 {
        value.numer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}

/// `one note sounds`, `three notes sound` — a count with the verb that agrees
/// with it, because the sentence around it reads badly either way otherwise.
fn spell_notes(count: usize) -> String {
    match count {
        1 => "one note sounds".to_owned(),
        other => format!("{other} notes sound"),
    }
}

/// `one range was`, `four ranges were`.
fn spell_ranges(count: usize) -> String {
    match count {
        1 => "one range was".to_owned(),
        other => format!("{other} ranges were"),
    }
}

/// `lowest`, `second`, `third` … — how a diagnostic names a voice it knows
/// only by its height.
fn nth(position: usize) -> &'static str {
    const WORDS: [&str; 8] = [
        "lowest", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth",
    ];
    WORDS.get(position).copied().unwrap_or("highest")
}
