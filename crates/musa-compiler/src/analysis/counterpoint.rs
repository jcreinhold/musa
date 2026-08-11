//! The `counterpoint` kind: a written line against a given one, under the
//! rules of one species (OMT `023`–`028`).
//!
//! **Abstract domain.** The measures of the exercise — one per note of the
//! cantus firmus — each carrying the cantus note, the counterpoint notes
//! inside it, and the vertical intervals they form; and over that, the set of
//! *(rule, place)* pairs where the music departs from the species asked for.
//!
//! **Abstraction map.** α designates one voice as the cantus, takes its notes
//! as the measures — which is what a measure *is* in a species exercise, one
//! cantus note long — and reads the other voice against it. Every interval is
//! computed from the lower sounding pitch upward, and every one is asked about
//! with the bass present, because in a two-voice texture the lower voice is
//! the bass and OMT `023` makes the perfect fourth's consonance depend on
//! exactly that.
//!
//! **Soundness.** As in [`super::voice_leading`]: a departure is decidable
//! from the notation, so it is a fact about the notes, and the rule's strength
//! is what a pedagogy makes of it. The one thing α cannot see is which voice
//! is given — a first-species cantus and a first-species counterpoint are both
//! one note per measure — so the request designates it. Designating the wrong
//! one produces a different and equally sound reading of different music,
//! which is why it is asked for rather than guessed.
//!
//! The fifth species is not a branch here. Its rule list is the union of the
//! other four's and its rhythmic test is "any measure the other four would
//! accept", which is what "florid counterpoint combines the species" means.

// Rational arithmetic on musical time is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use super::motion::{self, Consonance, Strand, Tone};
use super::rules::{self, Departure, Rule};
use super::{AnalysisError, AnalysisFinding, AnalysisProfile, Ground, Lane, Standing};
use crate::origin::Interval;
use crate::score::ScoreSnapshot;
use crate::time::{MusicalDuration, MusicalTime};

pub(super) fn observe(
    snapshot: &ScoreSnapshot,
    lanes: &[Lane],
    profile: Option<AnalysisProfile>,
    cantus: Option<&str>,
    window: Option<(MusicalTime, MusicalTime)>,
) -> Result<Vec<AnalysisFinding>, AnalysisError> {
    let Some(profile) = profile else {
        return Ok(Vec::new());
    };
    let cantus = cantus.ok_or(AnalysisError::NoCantus { profile })?;
    let strands = motion::strands(snapshot, lanes, window);
    let [first, second] = <&[Strand; 2]>::try_from(strands.as_slice())
        .map_err(|_| AnalysisError::NotTwoVoices { found: strands.len() })?;
    let (given, written) = match (first.label == cantus, second.label == cantus) {
        (true, _) => (first, second),
        (_, true) => (second, first),
        _ => {
            return Err(AnalysisError::NoSuchCantus {
                name: cantus.to_owned(),
                available: strands.iter().map(|strand| strand.label.clone()).collect(),
            });
        }
    };
    let exercise = Exercise::new(profile, given, written);
    let start = exercise.measures.first().map_or(MusicalTime::ZERO, |bar| bar.from);
    let mut found: Vec<AnalysisFinding> = exercise
        .checked
        .iter()
        .map(|rule| rules::in_force(rule, start))
        .collect();
    found.extend(exercise.departures().into_iter().map(Departure::finding));
    Ok(found)
}

/// One measure of the exercise: a cantus note, and what is written against it.
struct Measure {
    from: MusicalTime,
    to: MusicalTime,
    /// The counterpoint notes that *begin* inside this measure, in time order.
    notes: Vec<Tone>,
}

/// A species exercise, resolved: two voices, the measures they make, and the
/// rules this species checks.
struct Exercise<'a> {
    profile: AnalysisProfile,
    cantus: &'a Strand,
    counter: &'a Strand,
    measures: Vec<Measure>,
    checked: Vec<&'static Rule>,
}

/// The rules every species checks.
const BASE: [&Rule; 7] = [
    &rules::SPECIES_RHYTHM,
    &rules::SPECIES_BEGIN,
    &rules::SPECIES_END,
    &rules::SPECIES_CONSONANCE,
    &rules::SPECIES_PARALLEL_PERFECTS,
    &rules::SPECIES_DIRECT_PERFECTS,
    &rules::SPECIES_LEAP_RECOVERY,
];

impl<'a> Exercise<'a> {
    fn new(profile: AnalysisProfile, cantus: &'a Strand, counter: &'a Strand) -> Self {
        let measures = cantus
            .tones
            .iter()
            .map(|tone| Measure {
                from: tone.onset,
                to: tone.end,
                notes: counter
                    .tones
                    .iter()
                    .copied()
                    .filter(|note| (tone.onset..tone.end).contains(&note.onset))
                    .collect(),
            })
            .collect();
        Self {
            profile,
            cantus,
            counter,
            measures,
            checked: checked(profile),
        }
    }

    /// Whether this species checks that rule.
    fn checks(&self, rule: &'static Rule) -> bool {
        self.checked.iter().any(|checked| checked.id == rule.id)
    }

    /// What the two voices sound at `at`, lower first.
    fn sounding(&self, at: MusicalTime) -> Option<(Tone, Tone)> {
        let (cantus, counter) = (self.cantus.at(at)?, self.counter.at(at)?);
        if counter.pitch.chromatic_height() < cantus.pitch.chromatic_height() {
            Some((counter, cantus))
        } else {
            Some((cantus, counter))
        }
    }

    /// The interval sounding at `at`, measured upward from the lower voice.
    fn interval(&self, at: MusicalTime) -> Option<Interval> {
        let (low, high) = self.sounding(at)?;
        motion::between(low.pitch, high.pitch)
    }

    /// How the two notes at `at` stand to each other.
    ///
    /// Always asked with `over_bass`: in a two-voice texture the lower voice
    /// *is* the bass, so a fourth here is the dissonant fourth of OMT 023 and
    /// not the consonant fourth that can stand between two upper voices.
    fn consonance(&self, at: MusicalTime) -> Option<Consonance> {
        self.interval(at).map(|interval| motion::consonance(interval, true))
    }

    /// Every departure this species has to report.
    fn departures(&self) -> Vec<Departure> {
        let mut departures = Vec::new();
        departures.extend(self.rhythm());
        departures.extend(self.begins());
        departures.extend(self.ends());
        departures.extend(self.consonances());
        departures.extend(self.dissonances());
        departures.extend(self.suspensions());
        departures.extend(self.parallels());
        departures.extend(self.leaps());
        departures.retain(|departure| self.checks(departure.rule));
        departures
    }

    /// The rhythmic relation that *is* the species.
    fn rhythm(&self) -> Vec<Departure> {
        let last = self.measures.len().saturating_sub(1);
        self.measures
            .iter()
            .enumerate()
            .filter(|(index, bar)| !stands_in(self.profile, bar, *index == last))
            .map(|(_, bar)| Departure {
                rule: &rules::SPECIES_RHYTHM,
                standing: Standing::Conflict,
                voices: vec![self.counter.label.clone()],
                interval: None,
                from: bar.from,
                to: bar.to,
                notes: bar.notes.iter().map(|note| note.note).collect(),
                also: Vec::new(),
            })
            .collect()
    }

    /// A perfect consonance to begin with (OMT 024).
    fn begins(&self) -> Vec<Departure> {
        let Some(first) = self.measures.first() else {
            return Vec::new();
        };
        let at = first.notes.first().map_or(first.from, |note| note.onset);
        if matches!(self.consonance(at), Some(Consonance::Perfect)) {
            return Vec::new();
        }
        vec![Departure {
            rule: &rules::SPECIES_BEGIN,
            standing: Standing::Fact,
            voices: self.both(),
            interval: self.interval(at),
            from: first.from,
            to: first.to,
            notes: self.notes_at(at),
            also: Vec::new(),
        }]
    }

    /// A unison or an octave to end on, stepped into (OMT 024).
    fn ends(&self) -> Vec<Departure> {
        let Some(last) = self.measures.last() else {
            return Vec::new();
        };
        let Some(final_note) = last.notes.last().or_else(|| self.counter.tones.last()) else {
            return Vec::new();
        };
        let at = final_note.onset.max(last.from);
        let interval = self.interval(at);
        let closed = interval.is_some_and(|interval| {
            matches!(motion::consonance(interval, true), Consonance::Perfect) && motion::size(interval) != 5
        });
        let approached = self
            .counter
            .tones
            .iter()
            .rev()
            .nth(1)
            .is_some_and(|before| motion::leap(before.pitch, final_note.pitch) == 1);
        if closed && approached {
            return Vec::new();
        }
        vec![Departure {
            rule: &rules::SPECIES_END,
            standing: Standing::Fact,
            voices: self.both(),
            interval,
            from: last.from,
            to: last.to,
            notes: self.notes_at(at),
            also: vec![
                Ground {
                    criterion: "the last interval is a unison or an octave",
                    satisfied: closed,
                    cites: "OMT 024 §Ending",
                },
                Ground {
                    criterion: "the counterpoint steps into its last note",
                    satisfied: approached,
                    cites: "OMT 024 §Ending",
                },
            ],
        }]
    }

    /// A consonance on every strong beat.
    ///
    /// A note held over from the measure before is left to the suspension
    /// rule: that is exactly the place where a dissonance on a strong beat is
    /// the point rather than a mistake, and reporting it twice would say the
    /// fourth species forbids what it is made of.
    fn consonances(&self) -> Vec<Departure> {
        self.measures
            .iter()
            .filter(|bar| !self.tied_into(bar))
            .filter(|bar| matches!(self.consonance(bar.from), Some(Consonance::Dissonant)))
            .map(|bar| Departure {
                rule: &rules::SPECIES_CONSONANCE,
                standing: Standing::Fact,
                voices: self.both(),
                interval: self.interval(bar.from),
                from: bar.from,
                to: bar.to,
                notes: self.notes_at(bar.from),
                also: Vec::new(),
            })
            .collect()
    }

    /// A dissonance off the strong beat, approached and left by step in one
    /// direction (OMT 025).
    fn dissonances(&self) -> Vec<Departure> {
        let mut departures = Vec::new();
        for (index, note) in self.counter.tones.iter().enumerate() {
            let strong = self.measures.iter().any(|bar| bar.from == note.onset);
            if strong || !matches!(self.consonance(note.onset), Some(Consonance::Dissonant)) {
                continue;
            }
            let before = index.checked_sub(1).and_then(|at| self.counter.tones.get(at));
            let after = self.counter.tones.get(index.saturating_add(1));
            let (Some(before), Some(after)) = (before, after) else {
                continue;
            };
            let stepwise = motion::leap(before.pitch, note.pitch) == 1 && motion::leap(note.pitch, after.pitch) == 1;
            let onwards = (note.pitch.diatonic_height() - before.pitch.diatonic_height()).signum()
                == (after.pitch.diatonic_height() - note.pitch.diatonic_height()).signum();
            if stepwise && onwards {
                continue;
            }
            departures.push(Departure {
                rule: &rules::SPECIES_DISSONANCE_PASSING,
                standing: Standing::Fact,
                voices: self.both(),
                interval: self.interval(note.onset),
                from: note.onset,
                to: note.end,
                notes: self.notes_at(note.onset),
                also: vec![
                    Ground {
                        criterion: "the dissonance is approached and left by step",
                        satisfied: stepwise,
                        cites: "OMT 025 §Dissonance",
                    },
                    Ground {
                        criterion: "the line continues in the direction it came from",
                        satisfied: onwards,
                        cites: "OMT 025 §Dissonance",
                    },
                ],
            });
        }
        departures
    }

    /// A dissonance on a strong beat prepared, held, and resolved down by step
    /// (OMT 027).
    fn suspensions(&self) -> Vec<Departure> {
        let mut departures = Vec::new();
        for bar in &self.measures {
            if !matches!(self.consonance(bar.from), Some(Consonance::Dissonant)) {
                continue;
            }
            let Some(held) = self.counter.at(bar.from) else {
                continue;
            };
            let prepared = held.onset < bar.from
                && matches!(
                    self.consonance(held.onset),
                    Some(Consonance::Imperfect | Consonance::Perfect)
                );
            let tied = held.onset < bar.from;
            let resolved = self
                .counter
                .tones
                .iter()
                .find(|note| note.onset >= held.end)
                .is_some_and(|next| next.pitch.diatonic_height() == held.pitch.diatonic_height().saturating_sub(1));
            if prepared && tied && resolved {
                continue;
            }
            departures.push(Departure {
                rule: &rules::SPECIES_SUSPENSION,
                standing: Standing::Fact,
                voices: self.both(),
                interval: self.interval(bar.from),
                from: held.onset.min(bar.from),
                to: bar.to,
                notes: self.notes_at(bar.from),
                also: vec![
                    Ground {
                        criterion: "the dissonant note began as a consonance",
                        satisfied: prepared,
                        cites: "OMT 027 §Suspensions",
                    },
                    Ground {
                        criterion: "it is held over the barline rather than struck",
                        satisfied: tied,
                        cites: "OMT 027 §Suspensions",
                    },
                    Ground {
                        criterion: "it resolves down by step",
                        satisfied: resolved,
                        cites: "OMT 027 §Suspensions",
                    },
                ],
            });
        }
        departures
    }

    /// No perfect consonance repeated across a barline, and none arrived at in
    /// similar motion.
    fn parallels(&self) -> Vec<Departure> {
        let mut departures = Vec::new();
        for pair in self.measures.windows(2) {
            let [now, next] = pair else { continue };
            let (Some(was), Some(is)) = (self.sounding(now.from), self.sounding(next.from)) else {
                continue;
            };
            let (Some(first), Some(second)) = (self.interval(now.from), self.interval(next.from)) else {
                continue;
            };
            if !motion::is_perfect(second) {
                continue;
            }
            let how = motion::motion((was.0.pitch, is.0.pitch), (was.1.pitch, is.1.pitch));
            let same = motion::is_perfect(first) && motion::size(first) == motion::size(second);
            let similar = matches!(how, motion::Motion::Parallel | motion::Motion::Similar);
            if same && similar {
                departures.push(Departure {
                    rule: &rules::SPECIES_PARALLEL_PERFECTS,
                    standing: Standing::Fact,
                    voices: self.both(),
                    interval: Some(second),
                    from: now.from,
                    to: next.to,
                    notes: self.notes_at(now.from),
                    also: Vec::new(),
                });
            } else if similar {
                departures.push(Departure {
                    rule: &rules::SPECIES_DIRECT_PERFECTS,
                    standing: Standing::Fact,
                    voices: self.both(),
                    interval: Some(second),
                    from: now.from,
                    to: next.to,
                    notes: self.notes_at(now.from),
                    also: Vec::new(),
                });
            }
        }
        departures
    }

    /// A leap answered by motion the other way (OMT 023's melodic shape).
    fn leaps(&self) -> Vec<Departure> {
        let mut departures = Vec::new();
        for triple in self.counter.tones.windows(3) {
            let [first, second, third] = triple else { continue };
            let leap = second.pitch.diatonic_height() - first.pitch.diatonic_height();
            let after = third.pitch.diatonic_height() - second.pitch.diatonic_height();
            if leap.abs() >= 3 && after != 0 && leap.signum() == after.signum() {
                departures.push(Departure {
                    rule: &rules::SPECIES_LEAP_RECOVERY,
                    standing: Standing::Fact,
                    voices: vec![self.counter.label.clone()],
                    interval: motion::between(first.pitch, second.pitch),
                    from: first.onset,
                    to: third.end,
                    notes: vec![first.note, second.note, third.note],
                    also: Vec::new(),
                });
            }
        }
        departures
    }

    /// Whether a note held from the previous measure sounds at this one's
    /// downbeat.
    fn tied_into(&self, bar: &Measure) -> bool {
        self.counter.at(bar.from).is_some_and(|tone| tone.onset < bar.from)
    }

    /// Both voices, cantus first, for a finding that involves the pair.
    fn both(&self) -> Vec<String> {
        vec![self.cantus.label.clone(), self.counter.label.clone()]
    }

    /// The two notes sounding at `at`, as evidence.
    fn notes_at(&self, at: MusicalTime) -> Vec<crate::analysis::NoteRef> {
        self.sounding(at)
            .map(|(low, high)| vec![low.note, high.note])
            .unwrap_or_default()
    }
}

/// The rules a species checks.
///
/// The fifth species' list is computed as the union of the other four rather
/// than written out: "florid counterpoint mixes the species" is a statement
/// about the other four lists, and writing a fifth list by hand would let it
/// drift away from what it claims to combine.
fn checked(profile: AnalysisProfile) -> Vec<&'static Rule> {
    let mut rules: Vec<&'static Rule> = BASE.to_vec();
    match profile {
        AnalysisProfile::Species2 | AnalysisProfile::Species3 => rules.push(&rules::SPECIES_DISSONANCE_PASSING),
        AnalysisProfile::Species4 => rules.push(&rules::SPECIES_SUSPENSION),
        AnalysisProfile::Species5 => {
            rules.push(&rules::SPECIES_DISSONANCE_PASSING);
            rules.push(&rules::SPECIES_SUSPENSION);
        }
        AnalysisProfile::Species1 | AnalysisProfile::Satb | AnalysisProfile::JazzVoiceLeading => {}
    }
    rules.sort_by_key(|rule| {
        rules::RULES
            .iter()
            .position(|known| known.id == rule.id)
            .unwrap_or(usize::MAX)
    });
    rules
}

/// Whether a measure stands in the rhythmic relation the species asks for.
///
/// The last measure is exempt from the subdivisions: every species closes on
/// one note against the final cantus note, and a reading that called that a
/// rhythmic departure would report the ending of every correct exercise.
fn stands_in(profile: AnalysisProfile, bar: &Measure, last: bool) -> bool {
    let span = bar.to - bar.from;
    let at = |count: i64, index: i64| bar.from + MusicalDuration::new(span.as_ratio() * index / count);
    let attacks = |count: i64| {
        i64::try_from(bar.notes.len()).is_ok_and(|found| found == count)
            && bar
                .notes
                .iter()
                .enumerate()
                .all(|(index, note)| i64::try_from(index).is_ok_and(|index| note.onset == at(count, index)))
    };
    let syncopated = bar.notes.len() == 1
        && bar
            .notes
            .first()
            .is_some_and(|note| note.onset == at(2, 1) && note.end > bar.to);
    let closing = last && bar.notes.len() <= 1;
    match profile {
        AnalysisProfile::Species1 => closing || attacks(1),
        AnalysisProfile::Species2 => closing || attacks(2),
        AnalysisProfile::Species3 => closing || attacks(4),
        AnalysisProfile::Species4 => closing || syncopated,
        AnalysisProfile::Species5 => closing || attacks(1) || attacks(2) || attacks(4) || syncopated,
        AnalysisProfile::Satb | AnalysisProfile::JazzVoiceLeading => true,
    }
}
