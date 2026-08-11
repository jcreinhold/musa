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
    ArticulationMarking, DynamicMarking, EventId, HairpinSpan, HarmonyMark, Key, Meter, PhraseSpan, ScoreEvent,
    ScoreEventKind, SectionMark, SlurSpan, TupletSpan, Voice,
};
use crate::time::MusicalTime;

/// The projected voices, keyed the way the snapshot's parts are.
pub(crate) type Voices = IndexMap<(u32, u32), Voice>;

/// Everything the snapshot reads off one piece timeline.
///
/// The context maps are *here*, not in the header resolver, because
/// the key and the meter are occurrences: the timeline states them
/// and this is the reading of it. A piece with no key written has none, which
/// is why `key` is an `Option` and `meter` is not — 4/4 governs a piece that
/// never says so.
pub(crate) struct Projection {
    /// The voices, keyed by (part, voice).
    pub(crate) voices: Voices,
    /// What is in force where: key, meter and clef, each with its scope.
    pub(crate) contexts: crate::score::Contexts,
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
    let mut stated: Vec<Vec<crate::score::RepeatRegion>> = Vec::with_capacity(buckets.len());
    for (key, occurrences) in buckets {
        let (voice, repeats) = project_voice(resolver, &occurrences);
        stated.push(repeats);
        voices.insert(key, voice);
    }
    let repeats = agreed_repeats(resolver, &voices, &stated);
    resolver.annotations.set_repeats(repeats);
    let contexts = project_piece(resolver, &piece);
    Projection { voices, contexts }
}

/// The repeats every voice agrees about.
///
/// A repeat barline crosses the system, so it can only be drawn where the
/// whole system repeats: one voice writing `repeat 2 { … }` while another
/// writes the passage out means the page would have to show one voice folded
/// and the other flat, which is not a page. Those repeats are written out
/// instead, and the composer is told why rather than left to notice.
///
/// A voice that is silent under the repeat agrees with it by not disagreeing.
fn agreed_repeats(
    resolver: &mut Resolver,
    voices: &Voices,
    stated: &[Vec<crate::score::RepeatRegion>],
) -> Vec<crate::score::RepeatRegion> {
    let same = |left: &crate::score::RepeatRegion, right: &crate::score::RepeatRegion| {
        left.start == right.start && left.end == right.end && left.times == right.times && left.endings == right.endings
    };
    let mut agreed: Vec<crate::score::RepeatRegion> = Vec::new();
    for repeat in stated.iter().flatten() {
        if agreed.iter().any(|existing| same(existing, repeat)) {
            continue;
        }
        let sounds = |voice: &Voice| {
            voice.events().iter().any(|event| {
                let end = event.onset + event.notated_duration.value;
                event.onset < repeat.end && repeat.start < end
            })
        };
        let dissenting = voices
            .values()
            .zip(stated)
            .any(|(voice, theirs)| sounds(voice) && !theirs.iter().any(|other| same(other, repeat)));
        if dissenting {
            // A repeat inside a motif, bar, or fragment is the *material's*
            // rather than the page's: shared material stands at places that
            // have nothing to do with each other, so there was never a
            // system-crossing barline to lose. It is written out, and
            // silently — the composer did not ask for one. The same argument
            // the elaboration makes for a `meter` written inside a body.
            if repeat
                .origin
                .expansion_path
                .iter()
                .any(|step| matches!(step, crate::origin::ExpansionStep::MotifApplication { .. }))
            {
                continue;
            }
            resolver.report(
                crate::diagnose::Diagnostic::warning(
                    crate::diagnose::Code::Ignored,
                    "this repeat is written out on the page",
                )
                .at(repeat.origin.definition_span, "not every voice repeats here")
                .help("write the same `repeat` in each voice that sounds under this one")
                .note("a repeat barline crosses the system, so it can only be drawn where the whole system repeats"),
            );
            continue;
        }
        agreed.push(repeat.clone());
    }
    agreed.sort_by_key(|repeat| repeat.start);
    agreed
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
/// changes at bar 40. Sorting on the fact's *source* position
/// was only ever right while every context fact spanned the whole
/// piece. For the markers it preserves the order the outline has always had:
/// a form marker's place is where the composer wrote it, and two markers at
/// one instant cannot swap on a re-elaboration.
///
/// This is D11's *definition* applied by one ordered pass, not a `prevailing`
/// call per fact. The kernel says what the answer is; bulk derivation sweeps
/// (docs/kernel/03 D11, "the performance rule").
fn project_piece(resolver: &mut Resolver, occurrences: &[&Occurrence<ScoreFact>]) -> crate::score::Contexts {
    let mut ordered: Vec<&&Occurrence<ScoreFact>> = occurrences.iter().collect();
    ordered.sort_by_cached_key(|occurrence| {
        (
            occurrence.span().start(),
            occurrence.span().end(),
            occurrence.payload().canonical_key(),
        )
    });
    let mut contexts = crate::score::Contexts::default();
    for occurrence in ordered {
        let fact = occurrence.payload();
        let at = MusicalTime::new(occurrence.span().start().as_ratio());
        match &fact.kind {
            FactKind::Key { tonic, mode } => {
                contexts.keys.state(fact.scope, at, Key::new(*tonic, *mode));
            }
            FactKind::Meter { numerator, denominator } => {
                contexts
                    .meters
                    .state(fact.scope, at, Meter::new(*numerator, *denominator));
            }
            FactKind::Clef { clef } => {
                contexts.clefs.state(fact.scope, at, *clef);
            }
            FactKind::Tempo { metronome, text, ramp } => {
                contexts.tempos.state(
                    fact.scope,
                    at,
                    crate::score::TempoMarking {
                        metronome: *metronome,
                        text: text.clone(),
                        ramp: ramp.clone(),
                    },
                );
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
            | FactKind::Grace { .. }
            | FactKind::Mark { .. }
            | FactKind::Slur
            | FactKind::Phrase { .. }
            | FactKind::Tuplet { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Hairpin { .. }
            | FactKind::Repeat { .. }
            | FactKind::Ending { .. }
            | FactKind::Mobile { .. }
            | FactKind::Improvise { .. } => {}
        }
    }
    contexts
}

/// One voice: its events, the annotations that name them, and the repeats it
/// states — which are the piece's business and are reconciled once every voice
/// has been read.
fn project_voice(
    resolver: &mut Resolver,
    occurrences: &[&Occurrence<ScoreFact>],
) -> (Voice, Vec<crate::score::RepeatRegion>) {
    let mut repeats: Vec<&Occurrence<ScoreFact>> = Vec::new();
    let mut events: Vec<ScoreEvent> = Vec::with_capacity(occurrences.len());
    // Where each event sits, so a region can be resolved to the ids at its
    // ends without a second pass over the timeline.
    let mut extents: Vec<(MusicalTime, MusicalTime, EventId)> = Vec::with_capacity(occurrences.len());
    let mut regions: Vec<&Occurrence<ScoreFact>> = Vec::new();
    let mut points: Vec<&Occurrence<ScoreFact>> = Vec::new();
    // Grace notes stand at the onset of the note they lean on, and a point
    // sorts before a span that starts with it, so they arrive first and wait
    // here for their principal.
    let mut pending_graces: Vec<&Occurrence<ScoreFact>> = Vec::new();

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
                    for grace in std::mem::take(&mut pending_graces) {
                        let FactKind::Grace {
                            pitch,
                            articulations,
                            index,
                        } = &grace.payload().kind
                        else {
                            continue;
                        };
                        resolver.annotations.push_grace(crate::score::GraceNote {
                            at: event.id,
                            pitch: *pitch,
                            index: *index,
                            articulations: articulations.clone(),
                            origin: grace.payload().origin.clone(),
                        });
                    }
                    events.push(event);
                }
                index = index.saturating_add(consumed);
            }
            FactKind::Grace { .. } => {
                pending_graces.push(occurrence);
                index = index.saturating_add(1);
            }
            FactKind::Dynamic { .. } => {
                points.push(occurrence);
                index = index.saturating_add(1);
            }
            FactKind::Slur | FactKind::Phrase { .. } | FactKind::Tuplet { .. } | FactKind::Hairpin { .. } => {
                regions.push(occurrence);
                index = index.saturating_add(1);
            }
            // A span mark names the events it covers, like a slur; a point
            // mark names a time, because there may be no note where it stands.
            FactKind::Mark { mark, argument } => {
                match mark.anchor() {
                    crate::marks::Anchor::Span => regions.push(occurrence),
                    crate::marks::Anchor::Point | crate::marks::Anchor::Note(_) => {
                        let (part, voice) = fact.scope.voice().unwrap_or_default();
                        resolver.annotations.push_point(crate::score::PointMark {
                            mark: *mark,
                            argument: argument.clone(),
                            part: crate::score::PartId(part),
                            voice: crate::score::VoiceId(voice),
                            at: MusicalTime::new(occurrence.span().start().as_ratio()),
                            origin: fact.origin.clone(),
                        });
                    }
                }
                index = index.saturating_add(1);
            }
            FactKind::Repeat { .. } | FactKind::Ending { .. } => {
                repeats.push(occurrence);
                index = index.saturating_add(1);
            }
            // An open region spans real time rather than events: what it
            // covers was chosen by the performance, so there is no event id
            // either end that survives a different reading of the piece.
            FactKind::Mobile { fragments, order } => {
                resolver.annotations.push_open(crate::score::OpenRegion {
                    start: MusicalTime::new(occurrence.span().start().as_ratio()),
                    end: MusicalTime::new(occurrence.span().end().as_ratio()),
                    kind: crate::score::OpenKind::Mobile {
                        fragments: fragments.clone(),
                        order: order.clone(),
                    },
                    origin: fact.origin.clone(),
                });
                index = index.saturating_add(1);
            }
            FactKind::Improvise { over } => {
                resolver.annotations.push_open(crate::score::OpenRegion {
                    start: MusicalTime::new(occurrence.span().start().as_ratio()),
                    end: MusicalTime::new(occurrence.span().end().as_ratio()),
                    kind: crate::score::OpenKind::Improvise { over: over.clone() },
                    origin: fact.origin.clone(),
                });
                index = index.saturating_add(1);
            }
            // Piece-scoped facts were bucketed away before this ran; they are
            // named here only because the match is total.
            FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Clef { .. }
            | FactKind::Tempo { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. } => {
                index = index.saturating_add(1);
            }
        }
    }

    // A grace note leans on the note after it, so one with nothing after it
    // was written where it cannot be played. Reported rather than dropped:
    // the pitches are in the source and would vanish from the page.
    for grace in pending_graces {
        resolver.report(
            crate::diagnose::Diagnostic::error(
                crate::diagnose::Code::Misplaced,
                "this grace note has no note to lean on",
            )
            .at(grace.payload().origin.source_span, "nothing follows it")
            .help("a grace note is written before the note it belongs to"),
        );
    }

    project_points(resolver, &points, &extents);
    project_regions(resolver, &mut regions, &extents);
    (Voice::new(events), repeats_of(&repeats))
}

/// The repeats one voice states, each with its endings gathered under it.
///
/// The elaboration nests the ending regions inside the repeat region, so
/// containment is the whole rule; a `pass` appears once per time through, and
/// the bracket that prints is the first of its number.
fn repeats_of(occurrences: &[&Occurrence<ScoreFact>]) -> Vec<crate::score::RepeatRegion> {
    let time = |beat: musa_kernel::Beat| MusicalTime::new(beat.as_ratio());
    let mut repeats: Vec<crate::score::RepeatRegion> = occurrences
        .iter()
        .filter_map(|occurrence| {
            let FactKind::Repeat { times, range } = occurrence.payload().kind else {
                return None;
            };
            let start = time(occurrence.span().start());
            let end = time(occurrence.span().end());
            Some(crate::score::RepeatRegion {
                start,
                body_end: end,
                end,
                times,
                range,
                endings: Vec::new(),
                origin: occurrence.payload().origin.clone(),
            })
        })
        .collect();
    repeats.sort_by_key(|repeat| repeat.start);
    for occurrence in occurrences {
        let FactKind::Ending { bracket, pass } = occurrence.payload().kind else {
            continue;
        };
        let start = time(occurrence.span().start());
        let end = time(occurrence.span().end());
        let Some(repeat) = repeats
            .iter_mut()
            .find(|repeat| repeat.start <= start && end <= repeat.end)
        else {
            continue;
        };
        let index = (bracket as usize).saturating_sub(1);
        match repeat.endings.get_mut(index) {
            Some(existing) => existing.passes.push(pass),
            None => repeat.endings.push(crate::score::EndingRegion {
                passes: vec![pass],
                start,
                end,
            }),
        }
    }
    for repeat in &mut repeats {
        // The first ending starts where the body stops, which is where the
        // closing repeat barline goes. Without endings the body is the whole
        // span divided by the passes.
        repeat.body_end = match repeat.endings.first() {
            Some(ending) => ending.start,
            None => body_end(repeat),
        };
        for ending in &mut repeat.endings {
            ending.passes.sort_unstable();
        }
    }
    repeats
}

/// Where a plain repeat's body ends: one pass in.
fn body_end(repeat: &crate::score::RepeatRegion) -> MusicalTime {
    let times = i64::from(repeat.times.max(1));
    let whole = repeat.end.as_ratio() - repeat.start.as_ratio();
    MusicalTime::new(repeat.start.as_ratio() + whole / num_rational::Ratio::from_integer(times))
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
        free: fact.kind.free_of().copied(),
        kind,
    })
}

/// The articulations of the statement at `index` — a chord writes them once,
/// on every pitch, so the first occurrence is the one that carries them.
fn articulations(occurrences: &[&Occurrence<ScoreFact>], index: usize) -> Vec<crate::Mark> {
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
            resolver.error(
                crate::diagnose::Code::Misplaced,
                "this dynamic marking has nothing to mark",
                fact.origin.definition_span,
                "no note follows it",
            );
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
    // narrower. Sorting on the bracket's source position would agree
    // only because nothing yet moves a region away from where it was
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
            FactKind::Mark { mark, argument } => resolver.annotations.push_mark(crate::score::MarkSpan {
                mark: *mark,
                argument: argument.clone(),
                from,
                to,
                origin,
            }),
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
            // A repeat and an open region are answered in time, not in event
            // ids: a barline falls between measures and applies to the whole
            // system, and what an open region covers is not fixed. Collected
            // by [`repeats_of`] and in [`project_voice`] instead.
            FactKind::Note { .. }
            | FactKind::Rest { .. }
            | FactKind::Grace { .. }
            | FactKind::Dynamic { .. }
            | FactKind::Key { .. }
            | FactKind::Meter { .. }
            | FactKind::Clef { .. }
            | FactKind::Tempo { .. }
            | FactKind::Section { .. }
            | FactKind::Harmony { .. }
            | FactKind::Repeat { .. }
            | FactKind::Ending { .. }
            | FactKind::Mobile { .. }
            | FactKind::Improvise { .. } => {}
        }
    }
}
