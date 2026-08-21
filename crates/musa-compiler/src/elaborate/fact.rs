//! What a score fact is: the kinds a piece states, the payload that carries
//! one, and the maps a transform performs over it.
//!
//! One concern of the `elaborate` module; see its docs for the semantic path.

use crate::origin::Origin;
use crate::pitch::{PitchClass, WrittenPitch};
use crate::scope::Scope;
use crate::score::{DynamicMark, Mode, NotatedDuration};
use musa_kernel::{EventTrack, WrittenTime};
use num_rational::Ratio;

/// What a fact *states*. Where it is in time is the span; where it is in the
/// score is the scope; why it exists is the origin.
///
/// Articulations are a field of `Note`/`Rest` rather than facts of their own,
/// by the rule that decides the question: does it have an extent and an
/// identity? A staccato dot has neither — no span but its note's, unmovable
/// without moving the note — so making it an occurrence would only force the
/// projection to re-join it by span, which is the information loss this
/// design exists to delete, inverted. A slur has both.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FactKind {
    /// A sounding note. `duration` is notation intent: how many noteheads
    /// spell the span (roadmap §2 — notated ≠ performed).
    Note {
        pitch: WrittenPitch,
        duration: NotatedDuration,
        articulations: Vec<crate::Mark>,
        /// The freedom written on this note, when it was given one.
        free: Option<crate::score::FreeDuration>,
    },
    /// A written rest — notation intent, not a silence object (§2).
    Rest {
        duration: NotatedDuration,
        articulations: Vec<crate::Mark>,
        /// As [`FactKind::Note`]'s: a rest can be held too.
        free: Option<crate::score::FreeDuration>,
    },
    /// A notation mark that is not written on a note: a point at the instant
    /// it is written, or a span over the music its block covers. Which of the
    /// two this occurrence is, is its span — a point's is empty.
    ///
    /// Note-anchored marks are *not* here: a staccato dot has no extent and no
    /// identity of its own, so it stays a field of the note (see this enum's
    /// own doc). A pedal has both.
    Mark {
        mark: crate::Mark,
        argument: Option<crate::marks::MarkArgument>,
    },
    /// One grace note, as a **point** occurrence at the principal note's
    /// onset: a written pitch with no written duration.
    ///
    /// Not a [`FactKind::Mark`], by this enum's own rule. A grace note has its
    /// own pitch, its own accidental, its own beam and its own slur to the
    /// note it leans on, and four of them stand in an order that matters —
    /// that is an identity. A mark whose payload was a list of pitches would
    /// be a note under another name.
    ///
    /// What time it steals is *not* here: that is a reading, and readings
    /// belong to the profile (§2 — notated duration ≠ performed duration).
    Grace {
        pitch: WrittenPitch,
        articulations: Vec<crate::Mark>,
        /// Where this grace note stands among the ones written with it.
        ///
        /// Load-bearing rather than decorative. N2 orders occurrences by
        /// `(start, end, payload key)`, and every grace note in one group
        /// shares a start and an end — so without the index in the payload,
        /// `grace { c5 d5 }` and `grace { d5 c5 }` normalize to the same
        /// timeline and the kernel calls them equal music. They are not. The
        /// alternative, giving them nonzero written durations so they sort,
        /// would put performed time into the notation.
        index: u8,
    },
    /// A slur over the region it spans.
    Slur,
    /// A named phrase over the region it spans.
    Phrase { name: String },
    /// A tuplet bracket, unreduced as the backends need it.
    Tuplet { num: u32, den: u32 },
    /// A dynamic marking: a point at the onset it applies from.
    Dynamic { mark: DynamicMark },
    /// A hairpin over the region it spans, the mark it arrives at, and the
    /// shape of the growth. The shape is a kernel value (`Progress`), so it
    /// survives serialization and every consumer reads the same curve; how
    /// often to sample it is the consumer's policy (docs/rules/kernel/07).
    Hairpin {
        grows: bool,
        target: DynamicMark,
        shape: musa_kernel::Progress,
    },
    /// The key signature, over the region it governs — the whole piece
    /// while the grammar has no `modulate`.
    Key { tonic: PitchClass, mode: Mode },
    /// The meter, over the region it governs — likewise the whole piece.
    Meter { numerator: u32, denominator: u32 },
    /// The clef a staff is read in, over the region it governs.
    Clef { clef: crate::Clef },
    /// The tempo *marking*, over the region it governs.
    ///
    /// The marking, not the map. `♩ = 92` is notation written at a place —
    /// the engraver prints it, the exporters carry it — and the written-time
    /// → second function performance integrates is *derived* from the markings
    /// (docs/rules/kernel/06-surface-elaboration.md). Keeping the two apart is why this is a fact:
    /// a fact has a place in the piece, and a function does not.
    ///
    /// Both halves are optional and neither implies the other. `tempo
    /// "Andante";` prints a word and changes no clock — which is what most
    /// tempo markings in most scores do — and a metronome mark with no word
    /// is the common modern case.
    Tempo {
        /// The metronome mark, when the marking states one.
        metronome: Option<crate::score::Metronome>,
        /// The word printed with it, when the marking states one.
        text: Option<String>,
        /// How it gets somewhere else, when the change is gradual.
        ramp: Option<crate::score::Ramp>,
    },
    /// A form marker at the place it names.
    Section { name: String },
    /// A chord symbol at the place it is written; a region once a chord's
    /// duration can be written.
    Harmony { symbol: crate::harmony::ChordSymbol },
    /// A repeat over every pass it plays. The page prints the body once
    /// between repeat barlines; the timeline holds all `times` of it, which is
    /// the layer table's own example (roadmap §2).
    Repeat {
        times: u32,
        /// The passes the *source* asked for, when it left the count open:
        /// `repeat 4 to 16` is `Some((4, 16))` and `times` is the reading this
        /// performance took. The page prints the range and plays the count.
        range: Option<(u32, u32)>,
    },
    /// A mobile over the region it plays: the fragments as written, and the
    /// order this performance chose. The realized music is the fragments'
    /// own occurrences; this is the instruction the page prints over them
    /// (the rule the mobile and open-form facts share).
    Mobile { fragments: Vec<String>, order: Vec<u32> },
    /// An improvised frame over the region it occupies. It sounds as silence,
    /// because musa does not improvise.
    Improvise { over: Option<String> },
    /// One ending, over the region one pass of it plays.
    ///
    /// `bracket` is which volta is printed — the page has one per distinct
    /// ending — and `pass` is which time through it sounds. They differ
    /// whenever there are fewer endings than passes, which is what a bracket
    /// labelled `2.–4.` means.
    Ending { bracket: u32, pass: u32 },
}

impl FactKind {
    /// Whether this fact is a note or a rest — the facts that become events
    /// and are given identities.
    pub(crate) fn is_event(&self) -> bool {
        matches!(self, Self::Note { .. } | Self::Rest { .. })
    }

    /// The articulations written on this fact, if any.
    pub(crate) fn articulations_of(&self) -> &[crate::Mark] {
        match self {
            Self::Note { articulations, .. } | Self::Rest { articulations, .. } => articulations,
            Self::Mark { .. }
            | Self::Grace { .. }
            | Self::Slur
            | Self::Phrase { .. }
            | Self::Tuplet { .. }
            | Self::Dynamic { .. }
            | Self::Hairpin { .. }
            | Self::Key { .. }
            | Self::Meter { .. }
            | Self::Clef { .. }
            | Self::Tempo { .. }
            | Self::Section { .. }
            | Self::Harmony { .. }
            | Self::Repeat { .. }
            | Self::Mobile { .. }
            | Self::Improvise { .. }
            | Self::Ending { .. } => &[],
        }
    }

    /// The written duration, for the facts that have one.
    pub(crate) fn duration_of(&self) -> Option<&NotatedDuration> {
        match self {
            Self::Note { duration, .. } | Self::Rest { duration, .. } => Some(duration),
            Self::Mark { .. }
            | Self::Grace { .. }
            | Self::Slur
            | Self::Phrase { .. }
            | Self::Tuplet { .. }
            | Self::Dynamic { .. }
            | Self::Hairpin { .. }
            | Self::Key { .. }
            | Self::Meter { .. }
            | Self::Clef { .. }
            | Self::Tempo { .. }
            | Self::Section { .. }
            | Self::Harmony { .. }
            | Self::Repeat { .. }
            | Self::Mobile { .. }
            | Self::Improvise { .. }
            | Self::Ending { .. } => None,
        }
    }

    /// The bounds of a freely-held note, for the facts that have them.
    pub(crate) fn free_of(&self) -> Option<&crate::score::FreeDuration> {
        match self {
            Self::Note { free, .. } | Self::Rest { free, .. } => free.as_ref(),
            Self::Mark { .. }
            | Self::Grace { .. }
            | Self::Slur
            | Self::Phrase { .. }
            | Self::Tuplet { .. }
            | Self::Dynamic { .. }
            | Self::Hairpin { .. }
            | Self::Key { .. }
            | Self::Meter { .. }
            | Self::Clef { .. }
            | Self::Tempo { .. }
            | Self::Section { .. }
            | Self::Harmony { .. }
            | Self::Repeat { .. }
            | Self::Mobile { .. }
            | Self::Improvise { .. }
            | Self::Ending { .. } => None,
        }
    }
}

/// One elaborated fact of a score: what is stated, where in the score's
/// structure it belongs, and why it exists (docs/rules/kernel/06).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ScoreFact {
    pub(crate) scope: Scope,
    pub(crate) kind: FactKind,
    pub(crate) origin: Origin,
    /// Elaboration-only: this written notehead is tied to the next one.
    ///
    /// A tie is not a fact — it says two noteheads spell **one** occurrence —
    /// so it is resolved by merging during elaboration and is `false` on
    /// every fact that leaves [`elaborate_items`]. Nothing downstream reads
    /// it; the projection asserts as much in debug builds.
    pub(crate) tied: bool,
}

impl ScoreFact {
    pub(super) fn new(scope: Scope, kind: FactKind, origin: Origin) -> Self {
        Self {
            scope,
            kind,
            origin,
            tied: false,
        }
    }

    /// The same fact sounding and notated `factor` times as long.
    pub(crate) fn stretched(&self, factor: Ratio<i64>) -> Self {
        let mut stretched = self.clone();
        match &mut stretched.kind {
            FactKind::Note { duration, .. } | FactKind::Rest { duration, .. } => {
                *duration = duration.stretched(factor);
            }
            FactKind::Mark { .. }
            | FactKind::Grace { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Clef { .. }
            | FactKind::Tempo { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. }
            | FactKind::Repeat { .. }
            | FactKind::Mobile { .. }
            | FactKind::Improvise { .. }
            | FactKind::Ending { .. } => {}
        }
        stretched
    }

    /// The same fact with its pitch raised by `interval`, or `None` only when a
    /// fixed-width storage coordinate would overflow.
    ///
    /// Beside [`Self::inverted`] rather than inside the fold, because the
    /// contextual path transposes by rebasing an [`ExpandCx`] and the track
    /// builtin registered in [`crate::registry::track`] has no context to
    /// rebase: it holds facts that already have written pitches. A non-note
    /// fact transposes to itself, which is the one thing both readings agree on
    /// and is why this is a method rather than a match at each caller.
    pub(crate) fn transposed(&self, interval: crate::Interval) -> Option<Self> {
        let FactKind::Note { pitch, .. } = &self.kind else {
            return Some(self.clone());
        };
        let raised = pitch.transpose(interval)?;
        let mut transposed = self.clone();
        if let FactKind::Note { pitch, .. } = &mut transposed.kind {
            *pitch = raised;
        }
        Some(transposed)
    }

    /// The same fact with its pitch mirrored about `axis`, or `None` only
    /// when a fixed-width storage coordinate would overflow.
    pub(crate) fn inverted(&self, axis: WrittenPitch) -> Option<Self> {
        let FactKind::Note { pitch, .. } = &self.kind else {
            return Some(self.clone());
        };
        let mirrored = pitch.invert(axis)?;
        let mut inverted = self.clone();
        if let FactKind::Note { pitch, .. } = &mut inverted.kind {
            *pitch = mirrored;
        }
        Some(inverted)
    }

    /// The pitch, for the facts that have one.
    pub(crate) fn pitch_of(&self) -> Option<WrittenPitch> {
        match &self.kind {
            FactKind::Note { pitch, .. } => Some(*pitch),
            FactKind::Rest { .. }
            | FactKind::Mark { .. }
            | FactKind::Grace { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Clef { .. }
            | FactKind::Tempo { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. }
            | FactKind::Repeat { .. }
            | FactKind::Ending { .. }
            | FactKind::Mobile { .. }
            | FactKind::Improvise { .. } => None,
        }
    }

    /// The pitch this fact *sounds*, and where it is stored.
    ///
    /// Wider than [`Self::pitch_of`] by exactly one case, and separate from it
    /// on purpose. `pitch_of` answers "is this a notehead a reader can point
    /// at", which is what numbers positions, groups a chord, and finds a tie —
    /// and a grace note is none of those, because `04-provenance.md`'s inspector
    /// numbers what a reader points at. This one answers "does this fact carry a
    /// pitch a mapper must see", and a grace note plainly does: a
    /// `map_note_pitches(pedal, …)` that left the ornaments alone would give
    /// back a passage the composer did not write.
    ///
    /// A get/set pair over one table, so the half of the controlled traversal
    /// that *collects* pitches and the half that *puts them back* cannot
    /// disagree about which facts carry one — and a newly-added [`FactKind`]
    /// fails to compile in both until its policy is chosen deliberately.
    pub(crate) const fn sounding_pitch(&self) -> Option<WrittenPitch> {
        match &self.kind {
            FactKind::Note { pitch, .. } | FactKind::Grace { pitch, .. } => Some(*pitch),
            FactKind::Rest { .. }
            | FactKind::Mark { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Clef { .. }
            | FactKind::Tempo { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. }
            | FactKind::Repeat { .. }
            | FactKind::Ending { .. }
            | FactKind::Mobile { .. }
            | FactKind::Improvise { .. } => None,
        }
    }

    /// The same place, to write through. See [`Self::sounding_pitch`].
    pub(crate) const fn sounding_pitch_mut(&mut self) -> Option<&mut WrittenPitch> {
        match &mut self.kind {
            FactKind::Note { pitch, .. } | FactKind::Grace { pitch, .. } => Some(pitch),
            FactKind::Rest { .. }
            | FactKind::Mark { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Clef { .. }
            | FactKind::Tempo { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. }
            | FactKind::Repeat { .. }
            | FactKind::Ending { .. }
            | FactKind::Mobile { .. }
            | FactKind::Improvise { .. } => None,
        }
    }
}
/// One voice's elaborated timeline, before the snapshot adapter sees it.
pub(crate) type VoiceTrack = EventTrack<WrittenTime, ScoreFact>;

/// The one exhaustive coverage table for the controlled traversal. Keeping
/// it at the fact boundary makes a newly-added `FactKind` a compile error
/// until its sounding-pitch policy is chosen deliberately.
#[cfg(test)]
pub(crate) fn map_note_pitch_fact(
    payload: &ScoreFact,
    mut mapper: impl FnMut(WrittenPitch) -> Option<WrittenPitch>,
) -> Option<ScoreFact> {
    let mut mapped = payload.clone();
    match &mut mapped.kind {
        FactKind::Note { pitch, .. } | FactKind::Grace { pitch, .. } => *pitch = mapper(*pitch)?,
        FactKind::Rest { .. }
        | FactKind::Mark { .. }
        | FactKind::Slur
        | FactKind::Phrase { .. }
        | FactKind::Tuplet { .. }
        | FactKind::Dynamic { .. }
        | FactKind::Hairpin { .. }
        | FactKind::Key { .. }
        | FactKind::Meter { .. }
        | FactKind::Clef { .. }
        | FactKind::Tempo { .. }
        | FactKind::Section { .. }
        | FactKind::Harmony { .. }
        | FactKind::Repeat { .. }
        | FactKind::Mobile { .. }
        | FactKind::Improvise { .. }
        | FactKind::Ending { .. } => {}
    }
    Some(mapped)
}
