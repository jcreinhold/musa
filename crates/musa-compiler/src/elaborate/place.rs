//! Placing a fact in time, and the shared coordinates a template's instances
//! are written at.
//!
//! One concern of the `elaborate` module; see its docs for the semantic path.
use musa_syntax::ast::AstNode as _;

use super::fact::{FactKind, ScoreFact, VoiceTrack};
use crate::resolve::{self, Resolver};
use musa_events::{Duration, Occurrence, Position, Span, WrittenTime, empty, track};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::{Origin, SourceSpan};
use musa_score::scope::Scope;
use musa_score::time::MusicalTime;
use num_rational::Ratio;

/// The markers a score places by coordinate rather than by where a fold
/// reached: its form sections and its chord symbols (roadmap §8.2).
///
/// Here rather than in the reading, because `at bar 9` is a position in *bars*
/// and where the bars fall is what the meters decide — which is not settled
/// until the piece has been evaluated. Both are points, because a position is
/// only meaningful once the piece has a length to be inside of, and nothing
/// interprets either: a chord symbol is parsed so that a later library can read
/// it, and that is the end of the core's involvement.
pub(super) fn placed(
    resolver: &mut Resolver,
    score: &musa_syntax::ast::ScoreDecl,
    bars: &musa_score::BarLines,
    extent: Duration<WrittenTime>,
) -> VoiceTrack {
    let declaration = musa_score::origin::DeclarationId::default();
    let at_span = |span: SourceSpan| Origin {
        source_span: span,
        definition_span: span,
        declaration,
        expansion_path: Vec::new(),
    };
    let extent_time = MusicalTime::new(extent.as_ratio());
    let mut occurrences = Vec::new();
    for section in score.sections() {
        let span = resolve::trimmed_span(section.syntax());
        let Some(at) = resolve_position(resolver, section.position().as_ref(), span, bars, extent_time) else {
            continue;
        };
        occurrences.push(point_at(
            at,
            ScoreFact::new(
                Scope::Piece,
                FactKind::Section {
                    name: section.name().unwrap_or_default(),
                },
                at_span(span),
            ),
        ));
    }
    let lanes = score.harmonies();
    for extra in lanes.iter().skip(1) {
        resolver.error(
            Code::Misplaced,
            "this score already has a harmony lane",
            resolve::trimmed_span(extra.syntax()),
            "write every chord in the first one",
        );
    }
    for chord in lanes.iter().flat_map(musa_syntax::ast::HarmonyDecl::chords) {
        let span = resolve::trimmed_span(chord.syntax());
        let Some(at) = resolve_position(resolver, chord.position().as_ref(), span, bars, extent_time) else {
            continue;
        };
        let Some(written) = chord.symbol() else {
            continue;
        };
        if !written.is_one_word() {
            resolver.error(
                Code::NotAValue,
                "a chord symbol is one word",
                span,
                "expected something like `am` or `fmaj7`",
            );
            continue;
        }
        let text = written.text();
        let Some(symbol) = musa_score::harmony::ChordSymbol::parse(&text) else {
            resolver.report(
                Diagnostic::error(Code::NotAValue, format!("`{text}` is not a chord symbol musa reads"))
                    .at(span, "unknown chord")
                    .note("a root, an optional quality, and an optional seventh: `am`, `fmaj7`, `g7`, `bdim`"),
            );
            continue;
        };
        occurrences.push(point_at(
            at,
            ScoreFact::new(Scope::Piece, FactKind::Harmony { symbol }, at_span(span)),
        ));
    }
    track_or_empty(extent, occurrences)
}

/// A point occurrence at an absolute time, for the facts that are placed by
/// coordinate rather than by where the cursor reached.
fn point_at(at: MusicalTime, fact: ScoreFact) -> Occurrence<WrittenTime, ScoreFact> {
    let instant = Position::new(at.as_ratio());
    let span = Span::new(instant, instant).unwrap_or(Span::ZERO);
    Occurrence::new(span, fact)
}

/// Turn a `measure:beat` coordinate into time, or report why it is not one.
///
/// Measures and beats count from one, the way a composer reads them off the
/// page, and a position past the end of the piece is an error: a chord symbol
/// nobody will ever reach is a mistake, not a comment.
///
/// `bars` are folded from the meter *occurrences* and `extent` is the
/// timeline's own extent — neither is recomputed from the snapshot. The
/// extent is exact rather than a maximum over event ends,
/// and a piece that ends in a rest still ends where the rest ends, because a
/// rest is an occurrence.
fn resolve_position(
    resolver: &mut Resolver,
    position: Option<&musa_syntax::ast::Position>,
    span: SourceSpan,
    bars: &musa_score::BarLines,
    extent: MusicalTime,
) -> Option<MusicalTime> {
    let position = position?;
    let measure: i64 = position.measure()?.parse().ok()?;
    let beat_text = position.beat()?;
    let beat = resolve::parse_ratio(&beat_text).or_else(|| beat_text.parse::<i64>().ok().map(Ratio::from_integer))?;
    if measure < 1 || beat < Ratio::ONE {
        resolver.error(
            Code::OutOfRange,
            "measures and beats count from `1:1`",
            span,
            "before the piece starts",
        );
        return None;
    }
    let measure = u32::try_from(measure).unwrap_or(u32::MAX);
    let at = bars.time_of(measure, beat)?;
    if at >= extent && extent > MusicalTime::default() {
        resolver.error(
            Code::OutOfRange,
            format!("the piece ends before `{measure}:{beat_text}`"),
            span,
            "past the last note",
        );
        return None;
    }
    Some(at)
}

/// The placeholder a shared body carries where the call site would be.
///
/// A motif body's occurrences take their `source_span` from the *call*, which
/// is exactly what cannot be baked into a shared body. They carry this
/// instead, and each reference's mark says what to put there. No document is
/// four gigabytes, so it cannot collide with a real span.
pub(crate) const SHARED_ORIGIN: SourceSpan = SourceSpan::new(u32::MAX, u32::MAX);

/// The placeholder a shared body carries where the voice would be, for the
/// same reason: the same motif called from two voices is one body.
pub(crate) const SHARED_SCOPE: Scope = Scope::Voice {
    part: u32::MAX,
    voice: u32::MAX,
};

/// Apply a mark to a freshly instantiated body (E-Mark).
///
/// Payloads only, which is the whole of T6's contract: the spans, the extent,
/// the count and the order are the instantiated timeline's own and are not
/// touched here.
pub(crate) fn instantiate(mark: &str, instance: &mut VoiceTrack) {
    let Some(crate::factext::ReferenceMark {
        depth,
        steps,
        origin,
        scope,
    }) = crate::factext::read_reference_mark(mark)
    else {
        return;
    };
    for payload in instance.payloads_mut() {
        let path = &mut payload.origin.expansion_path;
        let at = depth.min(path.len());
        path.splice(at..at, steps.iter().cloned());
        if let Some(span) = origin
            && payload.origin.source_span == SHARED_ORIGIN
        {
            payload.origin.source_span = span;
        }
        if let Some(scope) = scope
            && payload.scope == SHARED_SCOPE
        {
            payload.scope = scope;
        }
    }
}

pub(super) fn track_or_empty(
    extent: Duration<WrittenTime>,
    occurrences: Vec<Occurrence<WrittenTime, ScoreFact>>,
) -> VoiceTrack {
    track(extent, occurrences).unwrap_or_else(|_| empty_segment())
}

/// The empty segment `(0, ∅)` — contributes nothing to the sequence.
fn empty_segment() -> VoiceTrack {
    empty(Duration::ZERO)
}
