//! The expanded score (roadmap §6.3): a finite, sorted, immutable snapshot
//! that keeps written spelling and full provenance, independent of any
//! notation backend.

// Span computation uses the total rational-time operators defined in
// `time.rs`; see that module for the arithmetic-lint justification.
#![allow(clippy::arithmetic_side_effects)]
use indexmap::IndexMap;
use num_rational::Ratio;
use serde::{Deserialize, Serialize};

use crate::context::ContextTrack;
use crate::marks::Mark;
use crate::origin::Origin;
use crate::pitch::{PitchClass, WrittenPitch};
use crate::scope::{ContextKind, Scope};
use crate::time::{MusicalDuration, MusicalTime};

/// A stable event identity within one snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EventId(pub u64);

/// What a score event sounds like.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreEventKind {
    /// A single written pitch.
    Note {
        /// The spelled pitch.
        pitch: WrittenPitch,
    },
    /// Silence.
    Rest,
    /// Explicit simultaneous pitches (roadmap §7.2: chords are explicit).
    Chord {
        /// The chord tones, in source order.
        pitches: Vec<WrittenPitch>,
    },
}

/// One event in a voice lane.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreEvent {
    /// Snapshot-local identity (assigned in source order; deterministic).
    pub id: EventId,
    /// Why this event exists.
    pub origin: Origin,
    /// Onset in whole notes from the piece start.
    pub onset: MusicalTime,
    /// The written duration: exact value plus notational spelling.
    pub notated_duration: NotatedDuration,
    /// What the event is.
    pub kind: ScoreEventKind,
    /// The freedom written on this note, when the performer was given one
    /// (`g4/4 to 2/1`).
    ///
    /// `notated_duration` holds what the note *sounds* — the realization's
    /// answer — exactly as it does inside a tuplet, and this is what recovers
    /// the symbol: the engraver draws `least` and brackets it up to `most`.
    /// Roadmap §2's row, notated duration ≠ performed duration, with both
    /// values present instead of one standing in for the other.
    pub free: Option<FreeDuration>,
}

/// How long a note may be held: the written value, and the longest the
/// performer may take it to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeDuration {
    /// The written value — what the notehead is.
    pub least: MusicalDuration,
    /// The longest it may be held.
    pub most: MusicalDuration,
}

/// How the language writes a duration value: `1`, `1/4`, `3/8`.
fn spell_value(value: Ratio<i64>) -> String {
    if *value.denom() == 1 {
        value.numer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}

/// A written duration: exact temporal value plus its notational spelling
/// (roadmap §6.3: a dotted quarter and a tied quarter+eighth share a span
/// but not a notation).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotatedDuration {
    /// Exact temporal value in whole notes.
    pub value: MusicalDuration,
    /// The written form, as a fraction of a whole note (`1/2`, `3/8`,
    /// `1/4 ~ 1/8`).
    ///
    /// One duration has one spelling here even where the language has two:
    /// `c4/4.` records `3/8`, because this string reaches diagnostics, the
    /// desktop inspector and every kernel golden, and a duration that arrived
    /// there under two names would be two facts. The record of what the
    /// composer typed is the CST, which is lossless and is right there.
    pub spelling: String,
    /// The written pieces this duration is spelled with, in order, joined by
    /// ties. Non-empty, and they sum to `value` exactly.
    ///
    /// A single written value is the common case. An explicit tie
    /// (`c4/4 ~` then `c4/8`) makes one event with two pieces: the
    /// composer asked for two noteheads, and a renderer that re-derived the
    /// spelling from `value` would print the dotted quarter they did not
    /// write.
    ///
    /// These are *sounding* values. Inside a tuplet a written eighth sounds
    /// `1/12` and is stored as `1/12`; the tuplet annotation carries the
    /// ratio that turns it back into the eighth-note symbol.
    pub pieces: Vec<MusicalDuration>,
}

impl NotatedDuration {
    /// A duration spelled with one written value.
    pub fn single(value: MusicalDuration, spelling: impl Into<String>) -> Self {
        Self {
            value,
            spelling: spelling.into(),
            pieces: vec![value],
        }
    }

    /// A duration a computed value asked for, spelled the way a composer
    /// writes one: `1/4`, `3/8`, `2`.
    ///
    /// A ratio that reached the score through a function has no written form
    /// of its own, so one is derived here rather than at each caller — three
    /// callers deriving it three ways is three spellings of one duration.
    pub(crate) fn spelled(value: Ratio<i64>) -> Self {
        let spelling = if *value.denom() == 1 {
            value.numer().to_string()
        } else {
            format!("{}/{}", value.numer(), value.denom())
        };
        Self::single(MusicalDuration::new(value), spelling)
    }

    /// The same duration sounding `factor` times as long — how a tuplet
    /// scales the values written inside it.
    pub(crate) fn scaled(&self, factor: Ratio<i64>) -> Self {
        Self {
            value: MusicalDuration::new(self.value.as_ratio() * factor),
            spelling: self.spelling.clone(),
            pieces: self
                .pieces
                .iter()
                .map(|piece| MusicalDuration::new(piece.as_ratio() * factor))
                .collect(),
        }
    }

    /// The same duration written out `factor` times as long — what an
    /// augmentation does to the page.
    ///
    /// Unlike [`Self::scaled`], which is a tuplet and keeps the written
    /// symbol while changing what it sounds for, this respells: a quarter
    /// stretched by two *is* a half note, and the inspector must say so.
    pub(crate) fn stretched(&self, factor: Ratio<i64>) -> Self {
        let pieces: Vec<MusicalDuration> = self
            .pieces
            .iter()
            .map(|piece| MusicalDuration::new(piece.as_ratio() * factor))
            .collect();
        let spelling = pieces
            .iter()
            .map(|piece| spell_value(piece.as_ratio()))
            .collect::<Vec<_>>()
            .join(" ~ ");
        Self {
            value: MusicalDuration::new(self.value.as_ratio() * factor),
            spelling,
            pieces,
        }
    }

    /// This duration tied to `next`: one sounding event, two written pieces.
    pub(crate) fn tied_to(&self, next: &Self) -> Self {
        let mut pieces = self.pieces.clone();
        pieces.extend(next.pieces.iter().copied());
        Self {
            value: self.value + next.value,
            spelling: format!("{} ~ {}", self.spelling, next.spelling),
            pieces,
        }
    }
}

/// A voice lane: identified, sequential, sorted by onset (roadmap §5.3:
/// lanes are identified, not anonymous event lists).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voice {
    events: Vec<ScoreEvent>,
}

impl Voice {
    /// The voice's events, in onset order.
    ///
    /// Sorted and contiguous in id: a voice's events carry consecutive
    /// [`EventId`]s in this order, which is what makes
    /// [`ScoreSnapshot::events_in`] a slice rather than a search.
    pub fn events(&self) -> &[ScoreEvent] {
        &self.events
    }

    /// The voice's span: the end of its last event.
    pub fn span(&self) -> MusicalDuration {
        self.events.last().map_or(MusicalDuration::ZERO, |event| {
            (event.onset - MusicalTime::ZERO) + event.notated_duration.value
        })
    }

    pub(crate) fn new(events: Vec<ScoreEvent>) -> Self {
        Self { events }
    }
}

/// A voice identity within a part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VoiceId(pub u32);

/// The clef a part is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Clef {
    /// Treble (G) clef.
    Treble,
    /// Bass (F) clef.
    Bass,
    /// Alto (C) clef.
    Alto,
    /// Tenor (C) clef.
    Tenor,
}

impl Clef {
    /// Every clef musa reads, for diagnostics that have to say so.
    pub const NAMES: &'static [&'static str] = &["treble", "bass", "alto", "tenor"];

    /// Parse a clef name.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "treble" => Some(Self::Treble),
            "bass" => Some(Self::Bass),
            "alto" => Some(Self::Alto),
            "tenor" => Some(Self::Tenor),
            _ => None,
        }
    }

    /// The name it is written under. The inverse of [`Self::parse`], which is
    /// what lets a clef survive the interchange file as the word a composer
    /// typed rather than as an ordinal.
    pub fn name(self) -> &'static str {
        match self {
            Self::Treble => "treble",
            Self::Bass => "bass",
            Self::Alto => "alto",
            Self::Tenor => "tenor",
        }
    }
}

/// A part identity within the score.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PartId(pub u32);

/// A named part with its voices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    id: PartId,
    name: String,
    voices: IndexMap<VoiceId, Voice>,
    voice_names: IndexMap<VoiceId, String>,
}

impl Part {
    /// The part identity.
    pub fn id(&self) -> PartId {
        self.id
    }

    /// The part name from the source.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The voice lanes, in source order.
    pub fn voices(&self) -> impl Iterator<Item = (VoiceId, &Voice)> {
        self.voices.iter().map(|(id, voice)| (*id, voice))
    }

    /// One voice lane by identity.
    pub fn voice(&self, id: VoiceId) -> Option<&Voice> {
        self.voices.get(&id)
    }

    /// The name a voice was written under; `None` when it was unnamed.
    pub fn voice_name(&self, id: VoiceId) -> Option<&str> {
        self.voice_names.get(&id).map(String::as_str)
    }

    /// The part's span: the end of its longest voice.
    pub fn span(&self) -> MusicalDuration {
        self.voices.values().map(Voice::span).max().unwrap_or_default()
    }

    pub(crate) fn new(
        id: PartId,
        name: String,
        voices: IndexMap<VoiceId, Voice>,
        voice_names: IndexMap<VoiceId, String>,
    ) -> Self {
        Self {
            id,
            name,
            voices,
            voice_names,
        }
    }

    /// Add one named voice lane, for a caller that discovers its voices one
    /// at a time rather than reading them off a `part` block.
    ///
    /// The two maps are written together because they are one fact: a lane
    /// with no name would be a lane nothing can address, and the assembly
    /// order is the order the voices appear in the score.
    pub(crate) fn add_voice(&mut self, id: VoiceId, name: String, voice: Voice) {
        self.voices.insert(id, voice);
        self.voice_names.insert(id, name);
    }
}

/// The score's parts, in source order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartMap {
    parts: IndexMap<PartId, Part>,
}

impl PartMap {
    /// Iterate over `(PartId, Part)` in source order.
    pub fn iter(&self) -> impl Iterator<Item = (PartId, &Part)> {
        self.parts.iter().map(|(id, part)| (*id, part))
    }

    /// Look up a part by identity.
    pub fn get(&self, id: PartId) -> Option<&Part> {
        self.parts.get(&id)
    }

    /// The number of parts.
    pub fn len(&self) -> usize {
        self.parts.len()
    }

    /// Whether the score has no parts.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    pub(crate) fn insert(&mut self, id: PartId, part: Part) {
        self.parts.insert(id, part);
    }
}

/// A metronome mark: a note value and a number of them per minute.
///
/// Not a tempo *map* — this is the pair a reader sees printed over the staff.
/// What it means in seconds is the performance layer's integration of every
/// such mark in the piece (docs/rules/kernel/06-surface-elaboration.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metronome {
    /// The beat unit as a fraction of a whole note (`1/4` for a quarter).
    pub beat: Ratio<i64>,
    /// Beats per minute.
    pub bpm: u32,
}

impl Default for Metronome {
    /// A quarter at 120: what an unmarked piece performs at, and the one
    /// value no caller should have to pass.
    fn default() -> Self {
        Self {
            beat: Ratio::new_raw(1, 4),
            bpm: 120,
        }
    }
}

/// A tempo marking written at a place in the piece.
///
/// Both halves are optional, and that is the whole design. `Allegro` with no
/// number is what most scores actually say; a metronome mark with no word is
/// what most modern ones say; and a marking with neither is not a marking.
/// A `TempoMarking` with no [`Metronome`] prints and changes no clock, which
/// is the fact that keeps the notation and the `Beat → Second` function from
/// collapsing back into one struct.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TempoMarking {
    /// The metronome mark, when the marking states one.
    pub metronome: Option<Metronome>,
    /// The word printed with it, when the marking states one.
    pub text: Option<String>,
    /// How the marking gets somewhere else, when it is gradual.
    pub ramp: Option<Ramp>,
}

/// A gradual tempo change: a *rit.*, an *accel.*, an eight-bar riser.
///
/// The reach is in the payload rather than in the occurrence's span, because
/// every context change is a point on the timeline and the track projected
/// from those points is what turns them into stretches (`context.rs`). A
/// ramp is a context change like any other; what is new is that it says
/// where it is going and how long it takes to get there.
///
/// The interpolation is linear in **seconds per beat**, not in beats per
/// minute — which is both the musically right answer and the exactly
/// representable one. Interpolating bpm makes each beat's duration a
/// reciprocal, so the integral leaves the rationals; interpolating duration
/// keeps every intermediate value exact, and orchestral practice hears an
/// even *rit.* as even in duration rather than even in bpm.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ramp {
    /// Beats per minute at the end of the change, in the marking's own beat
    /// unit — or `None` for a worded ramp (`tempo "rit." over 2/1;`), which
    /// prints and leaves the speed to the performer.
    pub to: Option<u32>,
    /// How far the change reaches, in whole notes.
    pub over: crate::time::MusicalDuration,
    /// How the change is spread across that reach, in normalized local time
    /// (docs/rules/kernel/03 `Progress`). The shape is normative; how finely to
    /// sample it is each consumer's choice.
    #[serde(with = "progress_serde")]
    pub shape: musa_kernel::Progress,
}

/// The initial meter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meter {
    numerator: u32,
    denominator: u32,
}

impl Default for Meter {
    fn default() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
        }
    }
}

impl Meter {
    /// No meter: music with real durations and no barlines — a cadenza, a
    /// chant, an unmeasured prelude.
    ///
    /// It is a *value* of the meter rather than a mechanism beside it, which
    /// is what keeps "where do the barlines fall" one question with one
    /// answer. A meter with no beats has no measure length, and every
    /// consumer that asks about barlines already asks the meter.
    pub const NONE: Self = Self {
        numerator: 0,
        denominator: 4,
    };

    /// Beats per measure. Zero when the music is unmeasured.
    pub fn numerator(self) -> u32 {
        self.numerator
    }

    /// The beat unit denominator (`4` for quarters).
    pub fn denominator(self) -> u32 {
        self.denominator
    }

    /// Whether barlines fall under this meter at all.
    ///
    /// The one question `meter none` adds, asked wherever a barline, a time
    /// signature or a bar-length check would otherwise be assumed.
    pub fn is_measured(self) -> bool {
        self.numerator > 0 && self.denominator > 0
    }

    /// The length of one measure in whole notes. Zero when unmeasured.
    pub fn measure_len(&self) -> MusicalDuration {
        if !self.is_measured() {
            return MusicalDuration::ZERO;
        }
        MusicalDuration::new(Ratio::new(i64::from(self.numerator), i64::from(self.denominator)))
    }

    pub(crate) fn new(numerator: u32, denominator: u32) -> Self {
        Self { numerator, denominator }
    }
}

/// The key mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// Major.
    Major,
    /// Minor.
    Minor,
}

/// The initial key signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    tonic: PitchClass,
    mode: Mode,
}

impl Key {
    /// The tonic pitch class.
    pub fn tonic(self) -> PitchClass {
        self.tonic
    }

    /// The mode.
    pub fn mode(self) -> Mode {
        self.mode
    }

    /// The key a tonic and a mode name.
    ///
    /// Public because naming a key is not a snapshot-building privilege: a
    /// caller that spells MIDI input, or a test that states what `bf major`
    /// means, is entitled to say one without holding a score.
    #[must_use]
    pub fn new(tonic: PitchClass, mode: Mode) -> Self {
        Self { tonic, mode }
    }

    /// The key written the way the language writes one: a pitch class, a
    /// space, and `major` or `minor`.
    ///
    /// The same spelling `key c major` uses in source, so a reader who wants
    /// an analysis read against a key they hear types what they would write.
    /// Absent for anything outside that grammar.
    pub fn parse(text: &str) -> Option<Self> {
        let (tonic, mode) = text.split_once(char::is_whitespace)?;
        let mode = match mode.trim() {
            "major" => Mode::Major,
            "minor" => Mode::Minor,
            _ => return None,
        };
        Some(Self::new(PitchClass::parse(tonic.trim())?, mode))
    }

    /// Where the key sits on the circle of fifths: positive counts sharps in
    /// the signature, negative counts flats.
    ///
    /// This is a fact about the key, not about any one backend's spelling of
    /// it, so it lives with the type — the MEI/`LilyPond`/`MusicXML` writers
    /// and MIDI note entry all need the same number and must not each derive
    /// their own.
    pub fn fifths(self) -> i8 {
        let letter: i8 = match self.tonic.letter {
            crate::Letter::C => 0,
            crate::Letter::G => 1,
            crate::Letter::D => 2,
            crate::Letter::A => 3,
            crate::Letter::E => 4,
            crate::Letter::B => 5,
            crate::Letter::F => -1,
        };
        // Each accidental on the tonic moves the key seven fifths.
        let alteration = i8::try_from(self.tonic.accidental.0).unwrap_or_else(|_| {
            if self.tonic.accidental.0.is_negative() {
                i8::MIN
            } else {
                i8::MAX
            }
        });
        let major = letter.saturating_add(alteration.saturating_mul(7));
        match self.mode {
            Mode::Major => major,
            // A minor key's signature is its relative major's: three fifths down.
            Mode::Minor => major.saturating_sub(3),
        }
    }
}

/// A dynamic marking as written (roadmap §2: a marking is not a decibel
/// value — what it does to a note is the performance layer's business).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DynamicMark {
    /// `ppp`
    Ppp,
    /// `pp`
    Pp,
    /// `p`
    P,
    /// `mp`
    Mp,
    /// `mf`
    Mf,
    /// `f`
    F,
    /// `ff`
    Ff,
    /// `fff`
    Fff,
    /// `sf`
    Sf,
    /// `sfz`
    Sfz,
    /// `fp`
    Fp,
}

impl DynamicMark {
    /// Read a marking as the language spells it.
    /// Every marking musa reads. Eleven is past the point where printing the
    /// list helps, which is why `suggest` sends the reader to `musa explain`
    /// instead when nothing is close.
    pub const NAMES: &'static [&'static str] = &["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff", "sf", "sfz", "fp"];

    pub fn parse(text: &str) -> Option<Self> {
        let mark = match text {
            "ppp" => Self::Ppp,
            "pp" => Self::Pp,
            "p" => Self::P,
            "mp" => Self::Mp,
            "mf" => Self::Mf,
            "f" => Self::F,
            "ff" => Self::Ff,
            "fff" => Self::Fff,
            "sf" => Self::Sf,
            "sfz" => Self::Sfz,
            "fp" => Self::Fp,
            _ => return None,
        };
        Some(mark)
    }

    /// The marking's name, spelled as it is written and printed.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ppp => "ppp",
            Self::Pp => "pp",
            Self::P => "p",
            Self::Mp => "mp",
            Self::Mf => "mf",
            Self::F => "f",
            Self::Ff => "ff",
            Self::Fff => "fff",
            Self::Sf => "sf",
            Self::Sfz => "sfz",
            Self::Fp => "fp",
        }
    }
}

/// A slur over a run of events in one voice, inclusive of both ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlurSpan {
    /// The first slurred event.
    pub from: EventId,
    /// The last slurred event.
    pub to: EventId,
    /// Why this slur exists.
    pub origin: Origin,
}

/// A tuplet over a run of events in one voice, inclusive of both ends:
/// `num` written values sounding in the time of `den` of them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TupletSpan {
    /// The first event in the group.
    pub from: EventId,
    /// The last event in the group.
    pub to: EventId,
    /// How many written values the group holds.
    pub num: u32,
    /// How many of those values the group actually lasts.
    pub den: u32,
    /// Why this tuplet exists.
    pub origin: Origin,
}

/// A dynamic marking anchored to the event it applies from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicMarking {
    /// The event the marking is written at.
    pub at: EventId,
    /// The marking.
    pub mark: DynamicMark,
    /// Why this marking exists.
    pub origin: Origin,
}

/// An articulation on one event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticulationMarking {
    /// The event the articulation belongs to.
    pub at: EventId,
    /// The articulation.
    pub mark: Mark,
    /// Why this articulation exists.
    pub origin: Origin,
}

/// One grace note, leaning on the event it precedes.
///
/// Attached to the principal rather than standing among the events, because a
/// grace note has no place in the measure's rhythm: it fills none of the bar,
/// and an event with zero notated duration would have to be excluded by hand
/// from every bar check, every measure fill and every plan decomposition. In
/// the kernel it *is* an occurrence with its own identity — see
/// `FactKind::Grace` — and this is how that occurrence reaches the page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraceNote {
    /// The note this one leans on.
    pub at: EventId,
    /// The written pitch, spelled as written (§6.3).
    pub pitch: WrittenPitch,
    /// Its place in the group it was written in. The order is the music.
    pub index: u8,
    /// The articulations written on it.
    pub articulations: Vec<Mark>,
    /// Why this grace note exists.
    pub origin: Origin,
}

/// A notation mark standing at one place: a breath, a text direction, a
/// rehearsal letter, a sample name.
///
/// Anchored by time rather than by event, unlike the annotations below it,
/// because a point mark is written *between* notes as often as on one — a
/// breath is exactly the mark that belongs to no note — and there is no event
/// whose identity it would survive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointMark {
    /// The mark, as written.
    pub mark: Mark,
    /// What was written after its name, when the mark takes an argument.
    pub argument: Option<crate::marks::MarkArgument>,
    /// The part it was written in.
    pub part: PartId,
    /// The voice it was written in — a sample name belongs over the drum
    /// staff, not over whichever staff the engraver reaches first.
    pub voice: VoiceId,
    /// Where it stands.
    pub at: MusicalTime,
    /// Why this mark exists.
    pub origin: Origin,
}

/// A notation mark over a run of events in one voice, inclusive of both ends:
/// a pedal, an ottava.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkSpan {
    /// The mark, as written.
    pub mark: Mark,
    /// What was written after its name, when the mark takes an argument.
    pub argument: Option<crate::marks::MarkArgument>,
    /// The first event under it.
    pub from: EventId,
    /// The last event under it.
    pub to: EventId,
    /// Why this mark exists.
    pub origin: Origin,
}

/// A named span over a run of events in one voice, inclusive of both ends.
///
/// Anchored to events, like a slur: a phrase is written *on* music, and it
/// must survive that music being re-barred or re-spelled.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhraseSpan {
    /// The phrase's name, as written.
    pub name: String,
    /// The first event of the phrase.
    pub from: EventId,
    /// The last event of the phrase.
    pub to: EventId,
    /// Why this phrase exists.
    pub origin: Origin,
}

/// `Progress` as exact breakpoint quadruples, so the kernel needs no `serde`.
///
/// The kernel's dependency list is `num-rational` and `thiserror`; a payload
/// value type is not a reason to widen it. `Progress::points` and
/// `Progress::piecewise` are the two halves of this conversion and already
/// exist for their own reasons, so the adapter is arithmetic-free and cannot
/// admit a curve the constructor would reject.
mod progress_serde {
    use musa_kernel::Progress;
    use num_rational::Ratio;
    use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};

    type Breakpoint = (i64, i64, i64, i64);

    pub(super) fn serialize<S: Serializer>(shape: &Progress, out: S) -> Result<S::Ok, S::Error> {
        let points: Vec<Breakpoint> = shape
            .points()
            .iter()
            .map(|(u, v)| (*u.numer(), *u.denom(), *v.numer(), *v.denom()))
            .collect();
        points.serialize(out)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<Progress, D::Error> {
        let points = Vec::<Breakpoint>::deserialize(input)?;
        Progress::piecewise(
            points
                .into_iter()
                .map(|(un, ud, vn, vd)| (Ratio::new(un, ud), Ratio::new(vn, vd))),
        )
        .ok_or_else(|| serde::de::Error::custom("breakpoints do not describe a progress curve"))
    }
}

/// A hairpin: a growth or fade over the notes it covers.
///
/// The mark it arrives at is written; the mark it leaves from is whatever
/// dynamic is in force at its first note, which is what a hairpin means on a
/// page and what the performance layer interpolates between.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HairpinSpan {
    /// The first event under it.
    pub from: EventId,
    /// The last event under it.
    pub to: EventId,
    /// True for a crescendo, false for a diminuendo.
    pub grows: bool,
    /// The dynamic it arrives at.
    pub target: DynamicMark,
    /// How the growth is shaped across the region, in normalized local time.
    /// The shape is normative; the sampling policy is the consumer's
    /// (docs/rules/kernel/07).
    #[serde(with = "progress_serde")]
    pub shape: musa_kernel::Progress,
    /// Why this hairpin exists.
    pub origin: Origin,
}

/// A form marker at a position in the piece.
///
/// Anchored to *time*, unlike a phrase: a section begins where the composer
/// says it begins, and a bar line is a place even when no note starts there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionMark {
    /// The section's name, as written.
    pub name: String,
    /// Where it begins, in whole notes from the piece start.
    pub at: MusicalTime,
    /// Why this marker exists.
    pub origin: Origin,
}

/// A chord symbol at a position in the piece (roadmap §8.2).
///
/// Recorded, never interpreted: nothing derives notes from it and nothing
/// checks the notes against it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarmonyMark {
    /// The symbol, parsed.
    pub symbol: crate::harmony::ChordSymbol,
    /// Where it is written, in whole notes from the piece start.
    pub at: MusicalTime,
    /// Why this symbol exists.
    pub origin: Origin,
}

/// A repeat, as the page has to print it (roadmap §2 — one statement, two
/// projections).
///
/// The timeline holds every pass; this says which stretch of it is the one
/// worth printing, and how many times the printed stretch is played. Anchored
/// to *time* rather than to events, because repeat barlines are barlines: they
/// fall between measures and they apply to the whole system, so an answer given
/// in one voice's event ids would be an answer to a different question.
///
/// Only repeats every voice agrees about reach here. A repeat one voice writes
/// and another does not is not a repeat the page can draw, and the compiler
/// says so rather than printing something the source does not mean.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepeatRegion {
    /// Where the repeated body begins.
    pub start: MusicalTime,
    /// Where the body ends: the closing repeat barline, and the start of the
    /// first ending when there is one.
    pub body_end: MusicalTime,
    /// Where the last pass stops sounding.
    pub end: MusicalTime,
    /// How many times the body is played.
    pub times: u32,
    /// The passes the source asked for, when it left the count open.
    ///
    /// `repeat 4 to 16` is `Some((4, 16))`, and [`Self::times`] is the reading
    /// this performance took. Both, because they answer different questions:
    /// the page prints `4–16×` because that is the instruction, and the
    /// timeline holds the passes because that is what sounds.
    pub range: Option<(u32, u32)>,
    /// The endings, in the order they are printed. Empty for a plain repeat.
    pub endings: Vec<EndingRegion>,
    /// Why this repeat exists.
    pub origin: Origin,
}

/// A stretch whose realization the performance chose, and the instruction
/// the page prints over it.
///
/// Anchored to *time* rather than to events for the reason [`RepeatRegion`] is:
/// a box drawn round a passage is drawn between barlines, and an improvised
/// frame holds no events at all — there are no event ids to name it by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRegion {
    /// Where the region begins.
    pub start: MusicalTime,
    /// Where it ends.
    pub end: MusicalTime,
    /// What freedom was written here.
    pub kind: OpenKind,
    /// Why this region exists.
    pub origin: Origin,
}

/// The kinds of written freedom a page has to print.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenKind {
    /// `mobile { a; b; c; }` — the fragments as written, and the order this
    /// performance plays them in, as indices into `fragments`.
    ///
    /// Both, because they answer different questions: the page prints the
    /// boxes in the order they are *written*, and the Origin view has to be
    /// able to say which reading produced the music underneath.
    Mobile {
        /// The fragment names, in written order.
        fragments: Vec<String>,
        /// The played order, as indices into `fragments`.
        order: Vec<u32>,
    },
    /// `improvise 8/1 over "Dm7 | G7";` — a frame with unnotated contents.
    Improvise {
        /// The changes to play over, if any were written.
        over: Option<String>,
    },
}

/// One volta bracket: the passes it is labelled with, and the stretch of the
/// timeline it prints from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndingRegion {
    /// The passes this bracket covers, ascending. `[1]` prints `1.`; `[2, 3]`
    /// prints `2.–3.`.
    pub passes: Vec<u32>,
    /// Where the printed pass of this ending begins.
    pub start: MusicalTime,
    /// Where it stops.
    pub end: MusicalTime,
}

/// Score-level annotations (roadmap §6.3): symbols that are *about* events
/// rather than events themselves, each carrying its own provenance.
///
/// Anchored by [`EventId`] rather than by time: an annotation belongs to the
/// notes it was written on, and it must survive those notes being re-spelled
/// or re-barred.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationStore {
    slurs: Vec<SlurSpan>,
    tuplets: Vec<TupletSpan>,
    dynamics: Vec<DynamicMarking>,
    articulations: Vec<ArticulationMarking>,
    graces: Vec<GraceNote>,
    phrases: Vec<PhraseSpan>,
    hairpins: Vec<HairpinSpan>,
    sections: Vec<SectionMark>,
    harmony: Vec<HarmonyMark>,
    repeats: Vec<RepeatRegion>,
    open: Vec<OpenRegion>,
    points: Vec<PointMark>,
    marks: Vec<MarkSpan>,
}

impl AnnotationStore {
    /// Slurs, in source order.
    pub fn slurs(&self) -> &[SlurSpan] {
        &self.slurs
    }

    /// The regions whose realization the performance chose, in time order.
    pub fn open(&self) -> &[OpenRegion] {
        &self.open
    }

    /// Tuplet groups, in source order.
    pub fn tuplets(&self) -> &[TupletSpan] {
        &self.tuplets
    }

    /// Dynamic markings, in source order.
    pub fn dynamics(&self) -> &[DynamicMarking] {
        &self.dynamics
    }

    /// Articulations, in source order.
    pub fn articulations(&self) -> &[ArticulationMarking] {
        &self.articulations
    }

    /// Grace notes, in the order they were written.
    pub fn graces(&self) -> &[GraceNote] {
        &self.graces
    }

    /// Phrases, in source order.
    pub fn phrases(&self) -> &[PhraseSpan] {
        &self.phrases
    }

    /// Hairpins, in source order.
    pub fn hairpins(&self) -> &[HairpinSpan] {
        &self.hairpins
    }

    /// Form markers, in the order they occur in the piece.
    pub fn sections(&self) -> &[SectionMark] {
        &self.sections
    }

    /// Chord symbols, in the order they occur in the piece.
    pub fn harmony(&self) -> &[HarmonyMark] {
        &self.harmony
    }

    /// Repeats the whole system agrees about, in the order they are reached.
    pub fn repeats(&self) -> &[RepeatRegion] {
        &self.repeats
    }

    /// Marks standing at one place, in the order they are reached.
    pub fn points(&self) -> &[PointMark] {
        &self.points
    }

    /// Marks covering a run of events, in source order.
    pub fn marks(&self) -> &[MarkSpan] {
        &self.marks
    }

    /// Record a point mark, keeping the lane sorted by position: like a form
    /// marker, it is read where it is reached rather than where it was typed.
    pub(crate) fn push_point(&mut self, point: PointMark) {
        let at = self.points.partition_point(|existing| existing.at <= point.at);
        self.points.insert(at, point);
    }

    pub(crate) fn push_mark(&mut self, mark: MarkSpan) {
        self.marks.push(mark);
    }

    pub(crate) fn push_phrase(&mut self, phrase: PhraseSpan) {
        self.phrases.push(phrase);
    }

    pub(crate) fn push_hairpin(&mut self, hairpin: HairpinSpan) {
        self.hairpins.push(hairpin);
    }

    /// Record a form marker, keeping the lane sorted by position: markers are
    /// read in the order they are reached, not the order they were typed.
    pub(crate) fn push_section(&mut self, section: SectionMark) {
        let at = self.sections.partition_point(|existing| existing.at <= section.at);
        self.sections.insert(at, section);
    }

    /// Record a chord symbol, keeping the lane sorted by position.
    pub(crate) fn push_harmony(&mut self, harmony: HarmonyMark) {
        let at = self.harmony.partition_point(|existing| existing.at <= harmony.at);
        self.harmony.insert(at, harmony);
    }

    pub(crate) fn push_slur(&mut self, slur: SlurSpan) {
        self.slurs.push(slur);
    }

    pub(crate) fn push_tuplet(&mut self, tuplet: TupletSpan) {
        self.tuplets.push(tuplet);
    }

    pub(crate) fn push_dynamic(&mut self, dynamic: DynamicMarking) {
        self.dynamics.push(dynamic);
    }

    pub(crate) fn push_articulation(&mut self, articulation: ArticulationMarking) {
        self.articulations.push(articulation);
    }

    pub(crate) fn push_grace(&mut self, grace: GraceNote) {
        self.graces.push(grace);
    }

    pub(crate) fn set_repeats(&mut self, repeats: Vec<RepeatRegion>) {
        self.repeats = repeats;
    }

    pub(crate) fn push_open(&mut self, region: OpenRegion) {
        self.open.push(region);
    }
}

/// A motif declaration, kept so a consumer can point at where a motif is
/// written rather than only at where it was used.
///
/// Expansion records the call site; the declaration is the other half of the
/// answer to "where did this note come from", and only the compiler knows it
/// (`docs/rules/desktop/04-provenance.md` §3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotifDeclaration {
    /// The motif's name, as declared.
    pub name: String,
    /// The span of the whole `motif` declaration.
    pub span: crate::origin::SourceSpan,
}

/// The four lines a printed edition carries besides the title.
///
/// Each is absent unless the piece writes it, and absent prints nothing: a
/// page with an empty composer line is worse than a page without one. The
/// title is not here because a piece already has one — its `piece` name — and
/// a second way to spell it is a way for the two to disagree.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrontMatter {
    /// A second line under the title.
    pub subtitle: Option<String>,
    /// Who wrote it. A piece that says nothing inherits its project's
    /// composer, which `musa-project` supplies.
    pub composer: Option<String>,
    /// Who arranged it.
    pub arranger: Option<String>,
    /// The notice at the foot of the first page.
    pub copyright: Option<String>,
}

/// The expanded score: finite, sorted, immutable, still musically spelled
/// (roadmap §6.3).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreSnapshot {
    title: String,
    front_matter: FrontMatter,
    parts: PartMap,
    contexts: Contexts,
    annotations: AnnotationStore,
    motifs: Vec<MotifDeclaration>,
    profiles: crate::profile::ProfileSet,
    performance: Option<u64>,
}

/// The four questions that are one question: what is in force here.
///
/// One track per kind, each carrying its own inheritance rule
/// ([`crate::ContextKind`]). Grouped rather than three fields on the snapshot
/// because they are built together, in one pass over the timeline, and a
/// consumer that reads one usually reads the others.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Contexts {
    pub(crate) keys: ContextTrack<Key>,
    pub(crate) meters: ContextTrack<Meter>,
    pub(crate) clefs: ContextTrack<Clef>,
    pub(crate) tempos: ContextTrack<TempoMarking>,
}

impl Default for Contexts {
    fn default() -> Self {
        Self {
            keys: ContextTrack::new(ContextKind::Key),
            meters: ContextTrack::new(ContextKind::Meter),
            clefs: ContextTrack::new(ContextKind::Clef),
            tempos: ContextTrack::new(ContextKind::Tempo),
        }
    }
}

impl ScoreSnapshot {
    /// The piece's title, as written in its `piece` declaration.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Which performance this is, when the piece left anything to one.
    ///
    /// `None` is the determinate case and it is the common one: a piece that
    /// asks no questions was not realized, it was simply compiled, and every
    /// consumer that would otherwise show a seed shows nothing.
    /// `Some(seed)` says this page is *one reading* — which is exactly the
    /// guarantee `docs/rules/kernel/11-realization.md` weakened, said out loud where
    /// a reader of the score can see it.
    pub fn performance(&self) -> Option<u64> {
        self.performance
    }

    pub(crate) fn set_performance(&mut self, seed: u64) {
        self.performance = Some(seed);
    }

    /// What a printed edition puts around the music: who wrote it, who
    /// arranged it, what the second line of the title says, and the notice at
    /// the foot of the first page. Facts about the piece — where any of them
    /// sits on paper is the engraver's business (roadmap §2).
    pub fn front_matter(&self) -> &FrontMatter {
        &self.front_matter
    }

    /// Take a composer from outside the piece, if the piece named none.
    ///
    /// A directory project's `musa.toml` says who wrote the pieces in it
    /// (roadmap §16), and a piece that names its own composer is not
    /// overruled by it. Applied once, where the project is known, so that
    /// every backend and the interface see one answer rather than each
    /// deciding the precedence for itself.
    pub fn inherit_composer(&mut self, composer: &str) {
        self.front_matter.composer.get_or_insert_with(|| composer.to_owned());
    }

    /// The parts, in source order.
    pub fn parts(&self) -> &PartMap {
        &self.parts
    }

    /// The tempo marking in force at a moment, as a reader in `scope` sees
    /// it. A piece that writes none has an unmarked page — which is not the
    /// same as a piece at 120, and the difference is why this returns an
    /// option and [`crate::IntegratedTempoMap`] supplies the default.
    pub fn tempo_at(&self, scope: Scope, at: MusicalTime) -> Option<&TempoMarking> {
        self.contexts.tempos.at(scope, at)
    }

    /// Every tempo marking the piece states, and where each begins.
    pub fn tempos(&self) -> &ContextTrack<TempoMarking> {
        &self.contexts.tempos
    }

    /// The meter in force at a moment, as a reader in `scope` sees it. A
    /// piece that names none is in 4/4.
    pub fn meter_at(&self, scope: Scope, at: MusicalTime) -> Meter {
        self.contexts.meters.at(scope, at).copied().unwrap_or_default()
    }

    /// Every meter the piece states, and where each begins.
    pub fn meters(&self) -> &ContextTrack<Meter> {
        &self.contexts.meters
    }

    /// Where the barlines fall, as a reader in `scope` counts them.
    ///
    /// The argument is polymeter, and it is *only* an argument: `Meter`
    /// inherits by `Override` (`scope.rs`), so a part with its own meter
    /// reads its own stretches and every other scope reads the piece's. The
    /// algorithm below did not change when polymeter arrived, which is the
    /// property `at`/`time_of` were built on an arbitrary meter sequence
    /// to get.
    ///
    /// Built over *unfolded* time: measures as they are played. Notation
    /// numbers a folded repeat differently and builds its own (see
    /// [`crate::BarLines`]'s module documentation).
    pub fn bars(&self, scope: Scope) -> crate::BarLines {
        let mut changes = self.contexts.meters.changes(scope);
        let opening = changes.next().map_or_else(Meter::default, |(_, meter)| *meter);
        let mut bars = crate::BarLines::uniform(opening);
        for (at, meter) in changes {
            // A change that does not land on a barline is an error the
            // compiler already reported, and dropping it keeps a piece that
            // does not compile printable — which is what the desktop shows
            // while the composer is still typing.
            let _ = bars.change(at, *meter);
        }
        bars
    }

    /// The key signature in force at a moment, as a reader in `scope` sees
    /// it, or `None` when nothing has said one — a piece that names no key
    /// has none, and saying so is more use than inventing C major.
    pub fn key_at(&self, scope: Scope, at: MusicalTime) -> Option<Key> {
        self.contexts.keys.at(scope, at).copied()
    }

    /// Every key the piece states, and where each begins.
    pub fn keys(&self) -> &ContextTrack<Key> {
        &self.contexts.keys
    }

    /// The clef a part is read in at a moment, or `None` when neither the
    /// part nor anything containing it names one.
    pub fn clef_at(&self, part: PartId, at: MusicalTime) -> Option<Clef> {
        self.contexts.clefs.at(Scope::Part { part: part.0 }, at).copied()
    }

    /// Every clef the piece states, and where each begins.
    pub fn clefs(&self) -> &ContextTrack<Clef> {
        &self.contexts.clefs
    }

    /// The annotations, by kind.
    ///
    /// Kind-major because that is how they are read: an outline pane wants
    /// every section, an export backend wants every slur. Per-event questions
    /// are [`Self::events_in`] and the marking lanes' own `at` fields.
    pub fn annotations(&self) -> &AnnotationStore {
        &self.annotations
    }

    /// Every motif declared in the piece, in source order.
    pub fn motifs(&self) -> &[MotifDeclaration] {
        &self.motifs
    }

    /// The piece's interpretation profiles and their part assignments.
    /// Declarations only: no [`ScoreEvent`] is touched by them (§6.4).
    pub fn profiles(&self) -> &crate::profile::ProfileSet {
        &self.profiles
    }

    /// The events a region annotation covers, from its first to its last
    /// inclusive.
    ///
    /// A slur, phrase, tuplet or hairpin names the events at its ends; the
    /// events between them are the ones it is *about*, and every consumer
    /// needs them. Answering it here means the rule — a region lies inside one
    /// voice, and a voice's events are contiguous in id — is stated once,
    /// where the projection that guarantees it lives, instead of being
    /// re-derived by id arithmetic in one caller and by a pair of linear
    /// searches in another.
    ///
    /// Empty when the ends name no voice, or name different ones, or are the
    /// wrong way round.
    pub fn events_in(&self, from: EventId, to: EventId) -> &[ScoreEvent] {
        for (_, part) in self.parts.iter() {
            for (_, voice) in part.voices() {
                let events = voice.events();
                let Some(start) = events.iter().position(|event| event.id == from) else {
                    continue;
                };
                let Some(end) = events.iter().position(|event| event.id == to) else {
                    return &[];
                };
                return events.get(start..=end).unwrap_or_default();
            }
        }
        &[]
    }

    pub(crate) fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub(crate) fn front_matter_mut(&mut self) -> &mut FrontMatter {
        &mut self.front_matter
    }

    pub(crate) fn parts_mut(&mut self) -> &mut PartMap {
        &mut self.parts
    }

    pub(crate) fn set_contexts(&mut self, contexts: Contexts) {
        self.contexts = contexts;
    }

    pub(crate) fn set_annotations(&mut self, annotations: AnnotationStore) {
        self.annotations = annotations;
    }

    pub(crate) fn push_motif(&mut self, motif: MotifDeclaration) {
        self.motifs.push(motif);
    }

    pub(crate) fn profiles_mut(&mut self) -> &mut crate::profile::ProfileSet {
        &mut self.profiles
    }
}
