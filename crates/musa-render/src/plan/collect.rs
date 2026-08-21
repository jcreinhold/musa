//! The marks gathered from a snapshot before any planning happens.
//!
//! One concern of the `plan` module; see its docs for what a plan is.

use std::collections::{HashMap, HashSet};

use musa_compiler::{DynamicMark, EventId, Mark, ScoreSnapshot};
use num_rational::Ratio;

use super::items::PlannedGrace;
use super::marks::{HairpinMark, MarkRange, PhraseMark, SpanMark, TupletMark};

/// The annotation store, indexed the way planning reads it: by the event a
/// symbol belongs to.
#[derive(Default)]
pub(super) struct Marks {
    pub(super) tuplets: HashMap<EventId, TupletMark>,
    pub(super) slur_ends: HashMap<EventId, EventId>,
    pub(super) slur_stops: HashSet<EventId>,
    pub(super) dynamics: HashMap<EventId, DynamicMark>,
    pub(super) articulations: HashMap<EventId, Vec<Mark>>,
    pub(super) graces: HashMap<EventId, Vec<PlannedGrace>>,
    pub(super) phrases: HashMap<EventId, PhraseMark>,
    pub(super) phrase_ends: HashMap<EventId, EventId>,
    pub(super) hairpins: HashMap<EventId, HairpinMark>,
    pub(super) hairpin_ends: HashMap<EventId, EventId>,
    pub(super) spans: HashMap<EventId, Vec<SpanMark>>,
    pub(super) span_ends: HashMap<EventId, Vec<MarkRange>>,
}

impl Marks {
    pub(super) fn collect(score: &ScoreSnapshot) -> Self {
        let annotations = score.annotations();
        let mut marks = Self::default();
        for slur in annotations.slurs() {
            marks.slur_ends.insert(slur.from, slur.to);
            marks.slur_stops.insert(slur.to);
        }
        for dynamic in annotations.dynamics() {
            marks.dynamics.insert(dynamic.at, dynamic.mark);
        }
        for articulation in annotations.articulations() {
            marks
                .articulations
                .entry(articulation.at)
                .or_default()
                .push(articulation.mark);
        }
        // Sorted by the payload's own index, not by the order the annotation
        // lane happens to hold: `grace { c5 d5 }` and `grace { d5 c5 }`
        // differ only in that number (docs/rules/kernel/05 N2), so it is what the
        // page has to print by.
        let mut graces: Vec<_> = annotations.graces().iter().collect();
        graces.sort_by_key(|grace| (grace.at.0, grace.index));
        for grace in graces {
            marks.graces.entry(grace.at).or_default().push(PlannedGrace {
                pitch: grace.pitch,
                articulations: grace.articulations.clone(),
            });
        }
        // A group's members are the events between its ends; the snapshot
        // answers which those are, so planning does not re-derive it.
        for phrase in annotations.phrases() {
            marks.phrase_ends.insert(phrase.from, phrase.to);
            for event in score.events_in(phrase.from, phrase.to) {
                marks.phrases.insert(
                    event.id,
                    PhraseMark {
                        name: phrase.name.clone(),
                        start: event.id == phrase.from,
                        stop: event.id == phrase.to,
                    },
                );
            }
        }
        // A note may be under a pedal and an ottava at once, so these are
        // lists where a phrase or a hairpin is one value: nothing stops a
        // player holding the pedal through an octave shift.
        for span in annotations.marks() {
            marks.span_ends.entry(span.from).or_default().push(MarkRange {
                mark: span.mark,
                argument: span.argument.clone(),
                from: span.from,
                to: span.to,
            });
            for event in score.events_in(span.from, span.to) {
                marks.spans.entry(event.id).or_default().push(SpanMark {
                    mark: span.mark,
                    argument: span.argument.clone(),
                    start: event.id == span.from,
                    stop: event.id == span.to,
                });
            }
        }
        for hairpin in annotations.hairpins() {
            marks.hairpin_ends.insert(hairpin.from, hairpin.to);
            for event in score.events_in(hairpin.from, hairpin.to) {
                marks.hairpins.insert(
                    event.id,
                    HairpinMark {
                        grows: hairpin.grows,
                        target: hairpin.target,
                        start: event.id == hairpin.from,
                        stop: event.id == hairpin.to,
                    },
                );
            }
        }
        for tuplet in annotations.tuplets() {
            for event in score.events_in(tuplet.from, tuplet.to) {
                marks.tuplets.insert(
                    event.id,
                    TupletMark {
                        num: tuplet.num,
                        den: tuplet.den,
                        start: event.id == tuplet.from,
                        stop: event.id == tuplet.to,
                    },
                );
            }
        }
        marks
    }

    /// What a tuplet does to a written value: a `3/2` triplet eighth sounds
    /// `1/12` and is printed as the `1/8` it was written as.
    pub(super) fn symbol_scale(&self, event: EventId) -> Ratio<i64> {
        self.tuplets.get(&event).map_or(Ratio::ONE, |tuplet| {
            Ratio::new(i64::from(tuplet.num), i64::from(tuplet.den))
        })
    }
}
