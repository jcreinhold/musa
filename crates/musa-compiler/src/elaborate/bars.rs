//! Barlines: the meters that decide them, and the claims measured against
//! them.
//!
//! One concern of the `elaborate` module; see its docs for the semantic path.
#![allow(clippy::arithmetic_side_effects)]

use crate::resolve::Resolver;
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;
use musa_score::scope::Scope;
use musa_score::score::{Meter, ScoreSnapshot};
use musa_score::time::MusicalTime;
use num_rational::Ratio;

/// The score facts one voicing sounds, as one simultaneous segment.
///
/// Shared by the `stack` sugar and by `play`, so the two cannot disagree
/// about what a voicing sounds. The enclosing transposition applies to each
/// pitch exactly as it does to a written chord: a voicing is notes, and notes
/// move.
pub(super) fn reported_an_error(resolver: &Resolver) -> bool {
    resolver
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == musa_score::diagnose::Severity::Error)
}

/// The barlines a scope counts against, when they are not the piece's.
///
/// `None` is not "no barlines" — it is "the piece's", which every scope in a
/// piece that is not polymetric answers.
pub(super) fn part_bars(resolver: &Resolver, scope: Scope) -> Option<musa_score::BarLines> {
    if resolver.part_meters.is_empty() {
        return None;
    }
    let part = match scope {
        Scope::Piece => return None,
        Scope::Part { part } | Scope::Voice { part, .. } => part,
    };
    // Uniform, and that is the whole of it: `Meter` inherits by `Override`
    // (`scope.rs`), so a part that states its own meter does not hear the
    // piece's changes at all, and the grammar gives a part exactly one.
    resolver
        .part_meters
        .get(&part)
        .copied()
        .map(musa_score::BarLines::uniform)
}

/// Fold the meters the piece states into barlines, refusing any change that
/// does not land on one.
///
/// Ascending, because the question "is this a barline" is answered by the
/// changes before it and by nothing else. This is why the meters are a pass
/// of their own rather than a lookup: a measure coordinate is a position in
/// bars, and where the bars fall is what the meters decide.
pub(super) fn resolve_meters(
    resolver: &mut Resolver,
    changes: Vec<(MusicalTime, Meter, SourceSpan)>,
) -> musa_score::BarLines {
    let mut bars = musa_score::BarLines::uniform(resolver.meter);
    let mut stated: Vec<(MusicalTime, Meter, SourceSpan)> = Vec::new();
    for (at, meter, span) in changes {
        if let Some((_, already, first)) = stated.iter().find(|(other, _, _)| *other == at) {
            // Two voices naming the same change is how a composer writes it,
            // and repeating a fact is not a mistake. Naming two different
            // meters for one place is.
            if *already != meter {
                resolver.report(two_at_once("meters", span, *first));
            }
            continue;
        }
        if !bars.change(at, meter) {
            resolver.report(off_barline("meter", &bars, at, span));
            continue;
        }
        stated.push((at, meter, span));
    }
    bars
}

/// Refuse any modulation that does not land on a barline.
///
/// A key signature is printed at a barline, so a modulation a third of the
/// way through a measure is a page nobody can engrave. Same refusal as the
/// meter's, and deliberately *not* the clef's: run after the meters, because
/// the barlines it is measured against are what the meters decided.
pub(super) fn check_keys(
    resolver: &mut Resolver,
    bars: &musa_score::BarLines,
    changes: Vec<(MusicalTime, musa_score::Key, SourceSpan)>,
) {
    let mut stated: Vec<(MusicalTime, musa_score::Key, SourceSpan)> = Vec::new();
    for (at, key, span) in changes {
        if let Some((_, already, first)) = stated.iter().find(|(other, _, _)| *other == at) {
            if *already != key {
                resolver.report(two_at_once("keys", span, *first));
            }
            continue;
        }
        if bars.meter_at(at).is_measured() && bars.at(at).into != musa_score::MusicalDuration::ZERO {
            resolver.report(off_barline("key", bars, at, span));
            continue;
        }
        stated.push((at, key, span));
    }
}

/// "You wrote it here, and here is not a barline" — the same sentence for a
/// meter and for a key, because it is the same mistake and the composer's fix
/// is the same either way.
fn off_barline(what: &str, bars: &musa_score::BarLines, at: MusicalTime, span: SourceSpan) -> Diagnostic {
    let here = bars.at(at);
    Diagnostic::error(Code::DoesNotAddUp, format!("a {what} change must land on a barline"))
        .at(
            span,
            format!(
                "this is {} into measure {}",
                fraction(here.into.as_ratio()),
                here.measure
            ),
        )
        .help(format!(
            "add {} before it, or move it past the next barline",
            fraction(bars.measure_at(at).end.as_ratio() - at.as_ratio())
        ))
}

/// Two voices may both name a change — repeating a fact is not a mistake —
/// but they may not disagree about it.
fn two_at_once(what: &str, span: SourceSpan, first: SourceSpan) -> Diagnostic {
    Diagnostic::error(Code::Misplaced, format!("two {what} at the same place"))
        .at(span, "the second of two")
        .also(first, "the first is here")
}

/// A musical amount, spelled the way the language spells it.
fn fraction(value: Ratio<i64>) -> String {
    if *value.denom() == 1 {
        value.numer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}

/// A tuplet has to be spellable, and a group split across a barline is not:
/// the notes on either side would need their own bracket and their own
/// ratio, which is a different piece of music from the one that was written.
pub(super) fn check_tuplets(resolver: &mut Resolver, snapshot: &ScoreSnapshot) {
    // Which part each event belongs to, so a tuplet in a 7/8 part is measured
    // against the 7/8 barlines. Built once: a tuplet names its first event,
    // and there is no other way from an event back to its staff.
    let mut owner: std::collections::BTreeMap<musa_score::EventId, u32> = std::collections::BTreeMap::new();
    for (id, part) in snapshot.parts().iter() {
        for (_, voice) in part.voices() {
            for event in voice.events() {
                owner.insert(event.id, id.0);
            }
        }
    }
    let mut offenders = Vec::new();
    for tuplet in snapshot.annotations().tuplets() {
        let bars = owner.get(&tuplet.from).map_or_else(
            || snapshot.bars(Scope::Piece),
            |part| snapshot.bars(Scope::Part { part: *part }),
        );
        let mut start = None;
        let mut end = None;
        for event in snapshot.events_in(tuplet.from, tuplet.to) {
            let event_end = event.onset + event.notated_duration.value;
            start = Some(start.map_or(event.onset, |current: MusicalTime| current.min(event.onset)));
            end = Some(end.map_or(event_end, |current: MusicalTime| current.max(event_end)));
        }
        let (Some(start), Some(end)) = (start, end) else {
            continue;
        };
        // `closing` is the reason the epsilon this replaced is gone: a group
        // ending exactly on a barline closes the measure before it, which is
        // the question being asked, said exactly rather than nearly.
        if bars.at(start).measure != bars.closing(end) {
            offenders.push(tuplet.origin.definition_span);
        }
    }
    for span in offenders {
        resolver.error(
            Code::DoesNotAddUp,
            "this tuplet is longer than a measure",
            span,
            "spills past the barline",
        );
    }
}
