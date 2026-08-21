//! Planning one voice lane: its items, their beams, and tie decomposition.
//!
//! One concern of the `plan` module; see its docs for what a plan is.

use musa_score::{Meter, MusicalDuration, MusicalTime, NotatedDuration, ScoreEvent, ScoreEventKind};
use num_rational::Ratio;

use super::collect::Marks;
use super::items::{Edges, NotatedItem, NotatedKind};
use super::marks::{
    BeamGroup, HairpinMark, HairpinRange, MarkRange, PhraseMark, PhraseRange, SlurRange, SpanMark, TupletMark,
};
use super::staff::beat_group_at;

/// Notate one voice's events inside one measure.
pub(super) fn plan_lane(
    events: &[ScoreEvent],
    meter: Meter,
    measure_len: Ratio<i64>,
    start: MusicalTime,
    end: MusicalTime,
    marks: &Marks,
) -> Result<Lane, crate::NotationError> {
    let mut items = Vec::new();
    for event in events {
        let event_end = event.onset + event.notated_duration.value;
        if event_end <= start || event.onset >= end {
            continue;
        }
        let scale = marks.symbol_scale(event.id);
        // The noteheads this event needs here: the pieces the composer wrote,
        // each clipped to this measure and then split into standard values.
        let mut pieces: Vec<(MusicalTime, NotatedDuration)> = Vec::new();
        let mut cursor = event.onset;
        for written in &event.notated_duration.pieces {
            let piece_start = cursor;
            let piece_end = cursor + *written;
            cursor = piece_end;
            if piece_end <= start || piece_start >= end {
                continue;
            }
            let clipped_start = piece_start.max(start);
            let clipped_end = piece_end.min(end);
            let sounding = MusicalDuration::new(clipped_end.as_ratio() - clipped_start.as_ratio());
            let symbol = MusicalDuration::new(sounding.as_ratio() * scale);
            let mut at = clipped_start;
            for value in decompose(event, symbol)? {
                let sounds = MusicalDuration::new(value.value.as_ratio() / scale);
                pieces.push((at, value));
                at = at + sounds;
            }
        }
        let piece_count = pieces.len();
        let continues_after = event_end > end;
        let started_before = event.onset < start;
        let tuplet = marks.tuplets.get(&event.id).copied();
        for (piece_index, (at, piece)) in pieces.into_iter().enumerate() {
            let is_first = piece_index == 0 && !started_before;
            let is_last = piece_index + 1 == piece_count && !continues_after;
            let tied = !matches!(event.kind, ScoreEventKind::Rest);
            let tie = if tied {
                Edges {
                    start: !is_last,
                    stop: !is_first,
                }
            } else {
                Edges::default()
            };
            // A symbol that spells one event only once carries what was
            // written on that event: an accent on the first notehead of a
            // tied pair, not on both.
            items.push(NotatedItem {
                event: event.id,
                kind: kind_of(event),
                onset_in_measure: MusicalDuration::new(at.as_ratio() - start.as_ratio()),
                duration: piece,
                tie,
                beam: None,
                tuplet: tuplet.map(|tuplet| TupletMark {
                    start: tuplet.start && is_first,
                    stop: tuplet.stop && is_last,
                    ..tuplet
                }),
                slur: Edges {
                    start: is_first && marks.slur_ends.contains_key(&event.id),
                    stop: is_last && marks.slur_stops.contains(&event.id),
                },
                phrase: marks.phrases.get(&event.id).map(|phrase| PhraseMark {
                    start: phrase.start && is_first,
                    stop: phrase.stop && is_last,
                    name: phrase.name.clone(),
                }),
                hairpin: marks.hairpins.get(&event.id).map(|hairpin| HairpinMark {
                    start: hairpin.start && is_first,
                    stop: hairpin.stop && is_last,
                    ..*hairpin
                }),
                dynamic: if is_first {
                    marks.dynamics.get(&event.id).copied()
                } else {
                    None
                },
                articulations: if is_first {
                    marks.articulations.get(&event.id).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                },
                // Graces lean on the *attack*, so they print before the first
                // piece of a tied pair and nowhere else — the same rule as the
                // dynamic, for the same reason.
                graces: if is_first {
                    marks.graces.get(&event.id).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                },
                spans: marks
                    .spans
                    .get(&event.id)
                    .map(|spans| {
                        spans
                            .iter()
                            .map(|span| SpanMark {
                                start: span.start && is_first,
                                stop: span.stop && is_last,
                                mark: span.mark,
                                argument: span.argument.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                // The bracket is drawn from the first notehead of the symbol,
                // like every other thing written once on a tied pair.
                free: if is_first { event.free } else { None },
            });
        }
    }
    assign_beams(&mut items, meter, measure_len);
    let slurs = items
        .iter()
        .filter(|item| item.slur.start)
        .filter_map(|item| {
            marks.slur_ends.get(&item.event).map(|to| SlurRange {
                from: item.event,
                to: *to,
            })
        })
        .collect();
    let phrases = items
        .iter()
        .filter_map(|item| {
            let phrase = item.phrase.as_ref().filter(|phrase| phrase.start)?;
            let to = *marks.phrase_ends.get(&item.event)?;
            Some(PhraseRange {
                name: phrase.name.clone(),
                from: item.event,
                to,
            })
        })
        .collect();
    let hairpins = items
        .iter()
        .filter_map(|item| {
            let hairpin = item.hairpin.filter(|hairpin| hairpin.start)?;
            let to = *marks.hairpin_ends.get(&item.event)?;
            Some(HairpinRange {
                from: item.event,
                to,
                grows: hairpin.grows,
                target: hairpin.target,
            })
        })
        .collect();
    let spans = items
        .iter()
        .flat_map(|item| {
            marks
                .span_ends
                .get(&item.event)
                .into_iter()
                .flatten()
                .filter(|range| item.spans.iter().any(|span| span.start && span.mark == range.mark))
                .cloned()
        })
        .collect();
    Ok(Lane {
        items,
        slurs,
        phrases,
        hairpins,
        marks: spans,
    })
}

/// One lane's plan for one measure, before it is named.
pub(super) struct Lane {
    pub(super) items: Vec<NotatedItem>,
    pub(super) slurs: Vec<SlurRange>,
    pub(super) phrases: Vec<PhraseRange>,
    pub(super) hairpins: Vec<HairpinRange>,
    pub(super) marks: Vec<MarkRange>,
}

fn kind_of(event: &ScoreEvent) -> NotatedKind {
    match &event.kind {
        ScoreEventKind::Note { pitch } => NotatedKind::Note { pitch: *pitch },
        ScoreEventKind::Rest => NotatedKind::Rest,
        ScoreEventKind::Chord { pitches } => NotatedKind::Chord {
            pitches: pitches.clone(),
        },
    }
}

/// Beam consecutive eighth-and-shorter items that stay inside one beat.
fn assign_beams(items: &mut [NotatedItem], meter: Meter, measure_len: Ratio<i64>) {
    let eighth = Ratio::new(1, 8);
    for item in items.iter_mut() {
        // Whether a symbol beams is a question about the symbol: a triplet
        // eighth beams because it is written as an eighth. Where it *ends*
        // is a question about time, and inside a tuplet the two differ.
        let symbol = item.duration.value.as_ratio();
        let short = symbol <= eighth;
        let pitched = !matches!(item.kind, NotatedKind::Rest);
        if !short || !pitched {
            continue;
        }
        let sounding = item.tuplet.map_or(symbol, |tuplet| {
            symbol * Ratio::new(i64::from(tuplet.den), i64::from(tuplet.num))
        });
        let onset = item.onset_in_measure.as_ratio();
        let item_end = onset + sounding;
        if item_end > measure_len {
            continue;
        }
        let Some((group, group_end)) = beat_group_at(meter, onset) else {
            continue;
        };
        if item_end <= group_end {
            item.beam = Some(BeamGroup(u32::try_from(group).unwrap_or(u32::MAX)));
        }
    }
}

/// Decompose a duration inside one measure into standard note values with
/// ties: powers of two (`1/2`, `1/4`, …) and dotted values (`3/4`, `3/8`, …).
/// Anything whose reduced denominator is not a power of two needs tuplets.
fn decompose(event: &ScoreEvent, duration: MusicalDuration) -> Result<Vec<NotatedDuration>, crate::NotationError> {
    let value = duration.as_ratio();
    let denominator = *value.denom();
    if denominator <= 0 || (denominator & (denominator - 1)) != 0 {
        return Err(crate::NotationError::UnspellableDuration {
            event: event.id,
            duration: event.notated_duration.spelling.clone(),
        });
    }
    let mut remaining = value;
    let mut pieces = Vec::new();
    while remaining > Ratio::ZERO {
        let piece = largest_value(remaining);
        pieces.push(NotatedDuration::single(
            MusicalDuration::new(piece),
            event.notated_duration.spelling.clone(),
        ));
        remaining -= piece;
    }
    Ok(pieces)
}

/// The largest standard value (plain or dotted) not exceeding `limit`.
fn largest_value(limit: Ratio<i64>) -> Ratio<i64> {
    // Whole note and smaller: 1, 3/4, 1/2, 3/8, 1/4, 3/16, 1/8, …
    let mut plain = Ratio::from_integer(1);
    while plain > limit {
        plain /= 2;
    }
    let dotted = plain * Ratio::new(3, 2);
    if dotted <= limit { dotted } else { plain }
}
