//! One notated item: what it is, what it carries, and the edges that join
//! it to its neighbours.
//!
//! One concern of the `plan` module; see its docs for what a plan is.

use musa_compiler::{DynamicMark, EventId, Mark, MusicalDuration, NotatedDuration, WrittenPitch};

use super::marks::{BeamGroup, HairpinMark, PhraseMark, SpanMark, TupletMark};

/// Where one item sits inside a spanning mark: the span may open here, close
/// here, both (a one-item span), or neither.
#[derive(Clone, Copy, Default)]
pub(super) struct Edges {
    pub(super) start: bool,
    pub(super) stop: bool,
}

/// One word per item in the plan snapshots, which are the review surface for
/// this module: two nested `bool` fields per span would bury the change under
/// the shape that carries it.
impl std::fmt::Debug for Edges {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match (self.start, self.stop) {
            (false, false) => "none",
            (true, false) => "start",
            (false, true) => "stop",
            (true, true) => "start+stop",
        })
    }
}

/// One notated symbol: a note, rest, or chord with one spelled duration.
/// Cross-measure events appear once per piece; all pieces of one event share
/// its `EventId` and are chained by tie flags.
#[derive(Clone, Debug)]
pub struct NotatedItem {
    pub(super) event: EventId,
    pub(super) kind: NotatedKind,
    pub(super) onset_in_measure: MusicalDuration,
    pub(super) duration: NotatedDuration,
    pub(super) tie: Edges,
    pub(super) beam: Option<BeamGroup>,
    pub(super) tuplet: Option<TupletMark>,
    pub(super) slur: Edges,
    pub(super) phrase: Option<PhraseMark>,
    pub(super) hairpin: Option<HairpinMark>,
    pub(super) dynamic: Option<DynamicMark>,
    pub(super) articulations: Vec<Mark>,
    pub(super) graces: Vec<PlannedGrace>,
    pub(super) spans: Vec<SpanMark>,
    pub(super) free: Option<musa_compiler::FreeDuration>,
}

/// One grace note leaning on a notated item, ready to print.
///
/// It is *inside* the item rather than beside it because that is what every
/// backend wants: MEI nests a `<graceGrp>` before the note, `MusicXML` writes
/// `<grace/>` notes ahead of it, and `LilyPond` writes `\acciaccatura` as a
/// prefix. A grace has no place of its own in the measure — it has no written
/// duration to occupy one — so a plan that laid graces out among the items
/// would have to invent onsets no backend would use.
///
/// No duration field, and that is the point: how long a grace lasts is the
/// profile's answer (`musa_compiler::GracePolicy`), and the page is silent on
/// it. What the page *does* say — which pitches, in what order, with what
/// marks — is exactly what is here.
#[derive(Clone, Debug)]
pub struct PlannedGrace {
    /// The spelled pitch, verbatim from the score.
    pub pitch: WrittenPitch,
    /// Articulations printed on the grace note itself.
    pub articulations: Vec<Mark>,
}

impl NotatedItem {
    /// The score event this symbol (or tie piece) came from.
    pub fn event(&self) -> EventId {
        self.event
    }

    /// Note, rest, or chord content.
    pub fn kind(&self) -> &NotatedKind {
        &self.kind
    }

    /// The onset within the measure, in whole notes.
    pub fn onset_in_measure(&self) -> MusicalDuration {
        self.onset_in_measure
    }

    /// This piece's spelled duration.
    pub fn duration(&self) -> &NotatedDuration {
        &self.duration
    }

    /// A tie starts here (the event continues in the next piece).
    pub fn tie_start(&self) -> bool {
        self.tie.start
    }

    /// A tie ends here (the event began in an earlier piece).
    pub fn tie_stop(&self) -> bool {
        self.tie.stop
    }

    /// The beam group, when this item beams with its beat neighbors.
    pub fn beam(&self) -> Option<BeamGroup> {
        self.beam
    }

    /// The tuplet bracket this item belongs to, if any.
    pub fn tuplet(&self) -> Option<TupletMark> {
        self.tuplet
    }

    /// A slur begins at this item.
    pub fn slur_start(&self) -> bool {
        self.slur.start
    }

    /// A slur ends at this item.
    pub fn slur_stop(&self) -> bool {
        self.slur.stop
    }

    /// The phrase this item belongs to, if any.
    pub fn phrase(&self) -> Option<&PhraseMark> {
        self.phrase.as_ref()
    }

    /// The hairpin this item is under, if any.
    pub fn hairpin(&self) -> Option<HairpinMark> {
        self.hairpin
    }

    /// The dynamic marking printed at this item, if any.
    pub fn dynamic(&self) -> Option<DynamicMark> {
        self.dynamic
    }

    /// The bounds of a freely-held note: what is drawn, and how far the
    /// bracket reaches. `None` on a note that sounds what it says.
    pub fn free(&self) -> Option<&musa_compiler::FreeDuration> {
        self.free.as_ref()
    }

    /// Articulations printed on this item, in written order.
    pub fn articulations(&self) -> &[Mark] {
        &self.articulations
    }

    /// The grace notes leaning on this item, in written order.
    pub fn graces(&self) -> &[PlannedGrace] {
        &self.graces
    }

    /// The notation marks spanning this item, with the ends they open or
    /// close here.
    pub fn spans(&self) -> &[SpanMark] {
        &self.spans
    }
}

/// The content of a notated item.
#[derive(Clone, Debug)]
pub enum NotatedKind {
    /// A single written pitch.
    Note {
        /// The spelled pitch, verbatim from the score.
        pitch: WrittenPitch,
    },
    /// A rest.
    Rest,
    /// A chord.
    Chord {
        /// The spelled pitches, in source order.
        pitches: Vec<WrittenPitch>,
    },
}
