//! The timeline → `ScoreSnapshot` projection (docs/kernel/06).
//!
//! Elaboration produces one `Timeline<ScoreFact>` for the whole piece. A
//! score, though, is read part by part and voice by voice, with notes that
//! have identities and annotations that name them. That reading is what this
//! module computes — and it is a *reading*, not a second representation: the
//! timeline stays the only place a temporal fact lives.
//!
//! It is a module rather than a method on the timeline because the kernel
//! must not learn what a score is (course correction §12).
//!
//! Rational arithmetic on musa's magnitudes is total; the workspace
//! arithmetic lint is allowed module-wide (see musa-compiler/src/time.rs).
#![allow(clippy::arithmetic_side_effects)]

use indexmap::IndexMap;
use musa_kernel::{Canonical as _, Occurrence, Timeline};

use crate::elaborate::{FactKind, ScoreFact};
use crate::resolve::Resolver;
use crate::score::{
    ArticulationMarking, DynamicMarking, EventId, HairpinSpan, HarmonyMark, KeyMap, MeterMap, PhraseSpan, ScoreEvent,
    ScoreEventKind, SectionMark, SlurSpan, TupletSpan, Voice,
};
use crate::time::MusicalTime;

/// The projected voices, keyed the way the snapshot's parts are.
pub(crate) type Voices = IndexMap<(u32, u32), Voice>;

/// Everything the snapshot reads off one piece timeline.
///
/// The context maps are *here*, not in the header resolver, because after
/// prompt 40 the key and the meter are occurrences: the timeline states them
/// and this is the reading of it. A piece with no key written has none, which
/// is why `key` is an `Option` and `meter` is not — 4/4 governs a piece that
/// never says so.
pub(crate) struct Projection {
    /// The voices, keyed by (part, voice).
    pub(crate) voices: Voices,
    /// The key signature, when the piece names one.
    pub(crate) key: Option<KeyMap>,
    /// The meter.
    pub(crate) meter: MeterMap,
}

/// Project the piece's timeline into voices and annotations.
///
/// One visit over the occurrences, bucketing by scope — never one filtering
/// pass per part or per annotation kind, which would be O(facts × parts) and
/// is what benchmark P3 watches for.
///
/// Ids are assigned to note and rest facts in the order they are visited,
/// which is the order the parts and voices were elaborated in.
///
/// **Invariant:** a region fact's boundaries coincide with event boundaries
/// in its own scope, because the region is built from the extent of the items
/// it encloses ([`crate::elaborate`]'s `over`). The projection relies on it to
/// name the events at a region's ends; if it is ever violated, the
/// elaboration that violated it is the bug.
pub(crate) fn project(resolver: &mut Resolver, timeline: &Timeline<ScoreFact>) -> Projection {
    let mut buckets: IndexMap<(u32, u32), Vec<&Occurrence<ScoreFact>>> = IndexMap::new();
    let mut piece: Vec<&Occurrence<ScoreFact>> = Vec::new();
    for occurrence in timeline.occurrences() {
        debug_assert!(
            !occurrence.payload().tied,
            "ties are merged during elaboration; the projection must never see one"
        );
        match occurrence.payload().scope.voice() {
            Some(key) => buckets.entry(key).or_default().push(occurrence),
            None => piece.push(occurrence),
        }
    }
    let mut voices = Voices::with_capacity(buckets.len());
    for (key, occurrences) in buckets {
        voices.insert(key, project_voice(resolver, &occurrences));
    }
    let (key, meter) = project_piece(resolver, &piece);
    Projection { voices, key, meter }
}

/// The piece-scoped facts: the context maps, and the annotations written at a
/// position rather than on a note.
///
/// The order is **canonical order** (N2: start, end, payload key), which is
/// time order with source position as its tie-break — the payload key ends in
/// the origin's source span. That matters twice. For the context maps it is
/// the prevailing rule of `docs/kernel/03` D11 applied in bulk: the last fact
/// starting at or before the piece's start is the one in force, and when
/// `modulate` arrives this sweep already answers correctly for a key that
/// changes at bar 40. Sorting on the fact's *source* position — which this did until
/// prompt 44 — was only ever right while every context fact spanned the whole
/// piece. For the markers it preserves the order the outline has always had:
/// a form marker's place is where the composer wrote it, and two markers at
/// one instant cannot swap on a re-elaboration.
///
/// This is D11's *definition* applied by one ordered pass, not a `prevailing`
/// call per fact. The kernel says what the answer is; bulk derivation sweeps
/// (docs/kernel/03 D11, "the performance rule").
fn project_piece(resolver: &mut Resolver, occurrences: &[&Occurrence<ScoreFact>]) -> (Option<KeyMap>, MeterMap) {
    let mut ordered: Vec<&&Occurrence<ScoreFact>> = occurrences.iter().collect();
    ordered.sort_by_cached_key(|occurrence| {
        (
            occurrence.span().start(),
            occurrence.span().end(),
            occurrence.payload().canonical_key(),
        )
    });
    let mut key = None;
    let mut meter = MeterMap::default();
    for occurrence in ordered {
        let fact = occurrence.payload();
        let at = MusicalTime::new(occurrence.span().start().as_ratio());
        match &fact.kind {
            FactKind::Key { tonic, mode } => {
                key = Some(KeyMap::new(*tonic, *mode));
            }
            FactKind::Meter { numerator, denominator } => {
                meter = MeterMap::new(*numerator, *denominator);
            }
            FactKind::Section { name } => resolver.annotations.push_section(SectionMark {
                name: name.clone(),
                at,
                origin: fact.origin.clone(),
            }),
            FactKind::Harmony { symbol } => resolver.annotations.push_harmony(HarmonyMark {
                symbol: symbol.clone(),
                at,
                origin: fact.origin.clone(),
            }),
            FactKind::Note { .. }
            | FactKind::Rest { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. } => {}
        }
    }
    (key, meter)
}

/// One voice: its events, and the annotations that name them.
fn project_voice(resolver: &mut Resolver, occurrences: &[&Occurrence<ScoreFact>]) -> Voice {
    let mut events: Vec<ScoreEvent> = Vec::with_capacity(occurrences.len());
    // Where each event sits, so a region can be resolved to the ids at its
    // ends without a second pass over the timeline.
    let mut extents: Vec<(MusicalTime, MusicalTime, EventId)> = Vec::with_capacity(occurrences.len());
    let mut regions: Vec<&Occurrence<ScoreFact>> = Vec::new();
    let mut points: Vec<&Occurrence<ScoreFact>> = Vec::new();

    let mut index = 0;
    while index < occurrences.len() {
        let Some(occurrence) = occurrences.get(index).copied() else {
            break;
        };
        let fact = occurrence.payload();
        match &fact.kind {
            FactKind::Note { .. } | FactKind::Rest { .. } => {
                let consumed = chord_len(occurrences, index);
                if let Some(event) = event_from(resolver, occurrences, index, consumed) {
                    let end = event.onset + event.notated_duration.value;
                    extents.push((event.onset, end, event.id));
                    for articulation in articulations(occurrences, index) {
                        resolver.annotations.push_articulation(ArticulationMarking {
                            at: event.id,
                            mark: articulation,
                            origin: event.origin.clone(),
                        });
                    }
                    events.push(event);
                }
                index = index.saturating_add(consumed);
            }
            FactKind::Dynamic { .. } => {
                points.push(occurrence);
                index = index.saturating_add(1);
            }
            FactKind::Slur | FactKind::Phrase { .. } | FactKind::Tuplet { .. } | FactKind::Hairpin { .. } => {
                regions.push(occurrence);
                index = index.saturating_add(1);
            }
            // Piece-scoped facts were bucketed away before this ran; they are
            // named here only because the match is total.
            FactKind::Key { .. } | FactKind::Meter { .. } | FactKind::Section { .. } | FactKind::Harmony { .. } => {
                index = index.saturating_add(1);
            }
        }
    }

    project_points(resolver, &points, &extents);
    project_regions(resolver, &mut regions, &extents);
    Voice::new(events)
}

/// How many occurrences at `index` spell one written statement: a chord's
/// pitches share a span and an origin.
fn chord_len(occurrences: &[&Occurrence<ScoreFact>], index: usize) -> usize {
    let Some(first) = occurrences.get(index).copied() else {
        return 1;
    };
    if first.payload().pitch_of().is_none() {
        return 1;
    }
    let mut consumed = 1;
    while let Some(next) = occurrences.get(index.saturating_add(consumed)).copied() {
        let same = next.span() == first.span()
            && next.payload().origin == first.payload().origin
            && next.payload().pitch_of().is_some();
        if !same {
            break;
        }
        consumed = consumed.saturating_add(1);
    }
    consumed
}

/// The event spelled by `consumed` occurrences starting at `index`, or
/// `None` when there is nothing there to spell (unreachable at the call
/// site, and a skipped event rather than a panic if it ever happens).
fn event_from(
    resolver: &mut Resolver,
    occurrences: &[&Occurrence<ScoreFact>],
    index: usize,
    consumed: usize,
) -> Option<ScoreEvent> {
    let first = occurrences.get(index).copied()?;
    let fact = first.payload();
    let duration = fact.kind.duration_of()?.clone();
    let pitches: Vec<crate::pitch::WrittenPitch> = occurrences
        .get(index..index.saturating_add(consumed))
        .unwrap_or_default()
        .iter()
        .filter_map(|occurrence| occurrence.payload().pitch_of())
        .collect();
    let kind = match pitches.as_slice() {
        [] => ScoreEventKind::Rest,
        [pitch] => ScoreEventKind::Note { pitch: *pitch },
        _ => ScoreEventKind::Chord { pitches },
    };
    Some(ScoreEvent {
        id: resolver.event_id(),
        origin: fact.origin.clone(),
        onset: MusicalTime::new(first.span().start().as_ratio()),
        notated_duration: duration,
        kind,
    })
}

/// The articulations of the statement at `index` — a chord writes them once,
/// on every pitch, so the first occurrence is the one that carries them.
fn articulations(occurrences: &[&Occurrence<ScoreFact>], index: usize) -> Vec<crate::score::ArticulationMark> {
    occurrences
        .get(index)
        .map(|occurrence| occurrence.payload().kind.articulations_of().to_vec())
        .unwrap_or_default()
}

/// Point facts — dynamics — take effect at the first event at or after them.
fn project_points(
    resolver: &mut Resolver,
    points: &[&Occurrence<ScoreFact>],
    extents: &[(MusicalTime, MusicalTime, EventId)],
) {
    let mut ordered: Vec<&&Occurrence<ScoreFact>> = points.iter().collect();
    ordered.sort_by_key(|occurrence| occurrence.span().start());
    for occurrence in ordered {
        let fact = occurrence.payload();
        let FactKind::Dynamic { mark } = fact.kind else {
            continue;
        };
        let at = MusicalTime::new(occurrence.span().start().as_ratio());
        let Some((_, _, id)) = extents.iter().find(|(onset, _, _)| *onset >= at) else {
            resolver.error("this dynamic marking has no note after it", fact.origin.definition_span);
            continue;
        };
        resolver.annotations.push_dynamic(DynamicMarking {
            at: *id,
            mark,
            origin: fact.origin.clone(),
        });
    }
}

/// Region facts name the events at their ends.
///
/// Order is time order, outermost first, which is the order the annotation
/// lists have always been in: a region that starts earlier comes first, and
/// where two start together the one that ends later encloses the other.
/// Membership follows the kernel's containment convention (docs/kernel/03 D10)
/// and is derived by one ordered pass, not by a query per event.
fn project_regions(
    resolver: &mut Resolver,
    regions: &mut [&Occurrence<ScoreFact>],
    extents: &[(MusicalTime, MusicalTime, EventId)],
) {
    // Time order, outermost first: a bracket that opens earlier is written
    // first, and of two brackets opening together the wider one encloses the
    // narrower. Until prompt 44 this sorted on the bracket's source position, which
    // agreed only because nothing yet moves a region away from where it was
    // written.
    regions.sort_by_key(|occurrence| {
        let span = occurrence.span();
        (span.start(), std::cmp::Reverse(span.end()))
    });
    for occurrence in regions.iter() {
        let fact = occurrence.payload();
        let span = occurrence.span();
        let from_time = MusicalTime::new(span.start().as_ratio());
        let to_time = MusicalTime::new(span.end().as_ratio());
        // The containment convention of docs/kernel/03 D10, applied in one
        // ordered pass rather than one `covering` call per event (D10, "the
        // performance rule"): a region `[s, e)` holds the events whose onset
        // satisfies `s ≤ onset < e`, and a point region holds the events at
        // its own instant. `extents` is already in onset order, so the first
        // and last held event bound the region.
        let held = |onset: MusicalTime| {
            if from_time == to_time {
                onset == from_time
            } else {
                from_time <= onset && onset < to_time
            }
        };
        let mut first = None;
        let mut last = None;
        for (onset, _, id) in extents {
            if !held(*onset) {
                continue;
            }
            first.get_or_insert(*id);
            last = Some(*id);
        }
        let (Some(from), Some(to)) = (first, last) else {
            // A bracket with no events under it annotates nothing, which is
            // what it has always done.
            continue;
        };
        let origin = fact.origin.clone();
        match &fact.kind {
            FactKind::Slur => resolver.annotations.push_slur(SlurSpan { from, to, origin }),
            FactKind::Phrase { name } => resolver.annotations.push_phrase(PhraseSpan {
                name: name.clone(),
                from,
                to,
                origin,
            }),
            FactKind::Tuplet { num, den } => resolver.annotations.push_tuplet(TupletSpan {
                from,
                to,
                num: *num,
                den: *den,
                origin,
            }),
            FactKind::Hairpin { grows, target, shape } => resolver.annotations.push_hairpin(HairpinSpan {
                from,
                to,
                grows: *grows,
                target: *target,
                shape: shape.clone(),
                origin,
            }),
            FactKind::Note { .. }
            | FactKind::Rest { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. } => {}
        }
    }
}
