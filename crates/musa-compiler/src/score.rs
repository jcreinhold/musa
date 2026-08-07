//! The expanded score (roadmap §6.3): a finite, sorted, immutable snapshot
//! that keeps written spelling and full provenance, independent of any
//! notation backend.

// Span computation uses the total rational-time operators defined in
// `time.rs`; see that module for the arithmetic-lint justification.
#![allow(clippy::arithmetic_side_effects)]
use indexmap::IndexMap;
use num_rational::Ratio;
use serde::{Deserialize, Serialize};

use crate::origin::Origin;
use crate::pitch::{PitchClass, WrittenPitch};
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
}

/// A written duration: exact temporal value plus its notational spelling
/// (roadmap §6.3: a dotted quarter and a tied quarter+eighth share a span
/// but not a notation).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotatedDuration {
    /// Exact temporal value in whole notes.
    pub value: MusicalDuration,
    /// The written form as the composer typed it (`1/2`, `3/8`, `1/4 ~ 1/8`).
    pub spelling: String,
    /// The written pieces this duration is spelled with, in order, joined by
    /// ties. Non-empty, and they sum to `value` exactly.
    ///
    /// A single written value is the common case. An explicit tie
    /// (`c4 1/4 ~;` then `c4 1/8;`) makes one event with two pieces: the
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
    /// Events sorted by onset.
    pub events: Vec<ScoreEvent>,
}

impl Voice {
    /// The voice's span: the end of its last event.
    pub fn span(&self) -> MusicalDuration {
        self.events.last().map_or(MusicalDuration::ZERO, |event| {
            (event.onset - MusicalTime::ZERO) + event.notated_duration.value
        })
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
}

/// A part identity within the score.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PartId(pub u32);

/// A named part with its voices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    /// The part identity.
    pub id: PartId,
    /// The part name from the source.
    pub name: String,
    /// The written clef, if declared.
    pub clef: Option<Clef>,
    /// Voice lanes by identity, in source order.
    pub voices: IndexMap<VoiceId, Voice>,
    /// Voice names by identity.
    pub voice_names: IndexMap<VoiceId, String>,
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

/// The initial tempo (roadmap §6.3's tempo map; prompt 26 makes it a
/// piecewise curve).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TempoMap {
    /// The beat unit as a fraction of a whole note (`1/4` for a quarter).
    pub beat: Ratio<i64>,
    /// Beats per minute.
    pub bpm: u32,
}

impl Default for TempoMap {
    fn default() -> Self {
        Self {
            beat: Ratio::new(1, 4),
            bpm: 120,
        }
    }
}

/// The initial meter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeterMap {
    /// Beats per measure.
    pub numerator: u32,
    /// The beat unit denominator (`4` for quarters).
    pub denominator: u32,
}

impl Default for MeterMap {
    fn default() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
        }
    }
}

impl MeterMap {
    /// The length of one measure in whole notes.
    pub fn measure_len(&self) -> MusicalDuration {
        MusicalDuration::new(Ratio::new(i64::from(self.numerator), i64::from(self.denominator)))
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
pub struct KeyMap {
    /// The tonic pitch class.
    pub tonic: PitchClass,
    /// The mode.
    pub mode: Mode,
}

/// A dynamic marking as written (roadmap §2: a marking is not a decibel
/// value — what it does to a note is prompt 28's business, not this one's).
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

/// An articulation as written (roadmap §2: not a gate multiplier).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ArticulationMark {
    /// `staccato`
    Staccato,
    /// `staccatissimo`
    Staccatissimo,
    /// `tenuto`
    Tenuto,
    /// `accent`
    Accent,
    /// `marcato`
    Marcato,
}

impl ArticulationMark {
    /// Read an articulation as the language spells it.
    pub fn parse(text: &str) -> Option<Self> {
        let mark = match text {
            "staccato" => Self::Staccato,
            "staccatissimo" => Self::Staccatissimo,
            "tenuto" => Self::Tenuto,
            "accent" => Self::Accent,
            "marcato" => Self::Marcato,
            _ => return None,
        };
        Some(mark)
    }

    /// The articulation's name, spelled as it is written.
    pub fn name(self) -> &'static str {
        match self {
            Self::Staccato => "staccato",
            Self::Staccatissimo => "staccatissimo",
            Self::Tenuto => "tenuto",
            Self::Accent => "accent",
            Self::Marcato => "marcato",
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
    pub mark: ArticulationMark,
    /// Why this articulation exists.
    pub origin: Origin,
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
}

impl AnnotationStore {
    /// Slurs, in source order.
    pub fn slurs(&self) -> &[SlurSpan] {
        &self.slurs
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
}

/// A motif declaration, kept so a consumer can point at where a motif is
/// written rather than only at where it was used.
///
/// Expansion records the call site; the declaration is the other half of the
/// answer to "where did this note come from", and only the compiler knows it
/// (`docs/interface/04-provenance.md` §3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotifDeclaration {
    /// The motif's name, as declared.
    pub name: String,
    /// The span of the whole `motif` declaration.
    pub span: crate::origin::SourceSpan,
}

/// The expanded score: finite, sorted, immutable, still musically spelled
/// (roadmap §6.3).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreSnapshot {
    /// The piece's title, as written in its `piece` declaration.
    pub title: String,
    /// Parts in source order.
    pub parts: PartMap,
    /// Tempo information.
    pub tempo_map: TempoMap,
    /// Meter information.
    pub meter_map: MeterMap,
    /// Key information.
    pub key_map: Option<KeyMap>,
    /// Score annotations.
    pub annotations: AnnotationStore,
    /// Every motif declared in the piece, in source order.
    pub motifs: Vec<MotifDeclaration>,
}
