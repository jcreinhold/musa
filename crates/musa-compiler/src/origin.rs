//! Provenance (roadmap §9): why an expanded event exists. Every
//! `ScoreEvent` traces to a source span and a declaration, plus the path of
//! expansion steps that produced it (empty for directly authored notes until
//! prompt 06).

use num_rational::Ratio;
use serde::{Deserialize, Serialize};

/// A byte span in the source document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    /// First byte of the span.
    pub start: u32,
    /// One past the last byte of the span.
    pub end: u32,
}

impl SourceSpan {
    /// Create a span from start and end byte offsets.
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

/// The declaration an event originates from: an ordinal over the piece's
/// declarations in source order. Stable within one compilation; not a
/// permanent project identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeclarationId(pub u32);

/// Why an event exists (roadmap §9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Origin {
    /// The source span that (transitively) produced the event.
    pub source_span: SourceSpan,
    /// The span of the statement that literally spells this event: the note
    /// inside the `motif` body for a generated event, and the same as
    /// `source_span` for an authored one. Editing the definition edits here;
    /// `source_span` is where the event *came from*, this is what wrote it.
    pub definition_span: SourceSpan,
    /// The enclosing declaration.
    pub declaration: DeclarationId,
    /// The expansion steps from declaration to event; empty for directly
    /// authored notes.
    pub expansion_path: Vec<ExpansionStep>,
}

/// One step in an expansion path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpansionStep {
    /// A motif was applied at this call site.
    MotifApplication {
        /// The span of the `use` statement.
        call_site: SourceSpan,
    },
    /// A `repeat` iteration (0-based).
    RepeatIteration(u32),
    /// A transposition by an interval.
    Transposition(Interval),
    /// A time stretch by an exact factor.
    Stretch(Ratio<i64>),
    /// A retrograde.
    Retrograde,
}

/// A signed musical interval (roadmap §5.4): diatonic steps plus semitones,
/// with direction baked into the sign. `P5` up is `(4, 7)`; down is
/// `(-4, -7)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interval {
    /// Signed diatonic steps (a fifth is 4).
    pub diatonic_steps: i8,
    /// Signed semitones (a perfect fifth is 7).
    pub semitones: i8,
}

impl Interval {
    /// The zero interval.
    pub const ZERO: Self = Self {
        diatonic_steps: 0,
        semitones: 0,
    };

    /// Parse an interval literal (`P5`, `M3`, `m3`) with a direction.
    pub fn parse(text: &str, down: bool) -> Option<Self> {
        let (quality, size_text) = text.split_at(1);
        let size: i8 = size_text.parse().ok()?;
        let (steps, semitones): (i8, i8) = match (quality, size) {
            ("P", 1) => (0, 0),
            ("P", 4) => (3, 5),
            ("P", 5) => (4, 7),
            ("P", 8) => (7, 12),
            ("M", 2) => (1, 2),
            ("M", 3) => (2, 4),
            ("M", 6) => (5, 9),
            ("M", 7) => (6, 11),
            ("m", 2) => (1, 1),
            ("m", 3) => (2, 3),
            ("m", 6) => (5, 8),
            ("m", 7) => (6, 10),
            _ => return None,
        };
        let sign: i8 = if down { -1 } else { 1 };
        Some(Self {
            diatonic_steps: steps.saturating_mul(sign),
            semitones: semitones.saturating_mul(sign),
        })
    }
}
