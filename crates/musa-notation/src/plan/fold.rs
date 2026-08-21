//! The repeat structure the derivation reads time through.
//!
//! One concern of the `plan` module; see its docs for what a plan is.

use musa_score::{BarLines, MusicalTime, ScoreEvent, ScoreSnapshot, Voice};
use num_rational::Ratio;

use super::marks::{RepeatMark, VoltaMark};

/// Performed time folded onto the page.
///
/// A repeat plays its body every pass and prints it once, so the page is the
/// timeline with the passes after the first taken out and the rest closed up.
/// Everything below this point — measure numbers, where a mark falls, which
/// measure carries a barline — is in **notated** time; the two coordinates
/// agree exactly where a piece has no repeats, which is why nothing else in
/// this module had to learn the difference.
///
/// This is roadmap §2's layer table doing work: notated position ≠ performed
/// position, and a repeat is the construct that separates them.
#[derive(Debug, Default)]
pub(super) struct Fold {
    /// Stretches of performed time the page does not print, ascending and
    /// disjoint.
    pub(super) dropped: Vec<(MusicalTime, MusicalTime)>,
}

impl Fold {
    pub(super) fn of(score: &ScoreSnapshot) -> Self {
        let mut dropped = Vec::new();
        for repeat in score.annotations().repeats() {
            // Everything from the end of the printed body to the end of the
            // last pass, except the one pass of each ending that prints.
            let mut cursor = repeat.body_end;
            for ending in &repeat.endings {
                if cursor < ending.start {
                    dropped.push((cursor, ending.start));
                }
                cursor = cursor.max(ending.end);
            }
            if cursor < repeat.end {
                dropped.push((cursor, repeat.end));
            }
        }
        dropped.sort_by_key(|interval| interval.0);
        Self { dropped }
    }

    /// Where `at` falls on the page, or `None` when it falls in a stretch the
    /// page does not print.
    pub(super) fn at(&self, at: MusicalTime) -> Option<MusicalTime> {
        let mut shift = Ratio::ZERO;
        for (from, to) in &self.dropped {
            if at < *from {
                break;
            }
            if at < *to {
                return None;
            }
            shift += to.as_ratio() - from.as_ratio();
        }
        Some(MusicalTime::new(at.as_ratio() - shift))
    }

    /// The same, for the exclusive end of something: a stretch that ends where
    /// a dropped one begins ends *there*, not nowhere.
    fn end_at(&self, at: MusicalTime) -> Option<MusicalTime> {
        let mut shift = Ratio::ZERO;
        for (from, to) in &self.dropped {
            if at <= *from {
                break;
            }
            if at < *to {
                return None;
            }
            shift += to.as_ratio() - from.as_ratio();
        }
        Some(MusicalTime::new(at.as_ratio() - shift))
    }

    /// One voice's events as the page holds them: the passes that print, at
    /// the times they print at.
    pub(super) fn events(&self, voice: &Voice) -> Vec<ScoreEvent> {
        if self.dropped.is_empty() {
            return voice.events().to_vec();
        }
        voice
            .events()
            .iter()
            .filter_map(|event| {
                let onset = self.at(event.onset)?;
                Some(ScoreEvent { onset, ..event.clone() })
            })
            .collect()
    }

    /// The repeat barlines and volta brackets, in measures.
    pub(super) fn marks(&self, score: &ScoreSnapshot, bars: &BarLines) -> Vec<RepeatMark> {
        score
            .annotations()
            .repeats()
            .iter()
            .filter_map(|repeat| {
                let from = self.at(repeat.start)?;
                let to = self.end_at(repeat.body_end)?;
                let endings = repeat
                    .endings
                    .iter()
                    .filter_map(|ending| {
                        Some(VoltaMark {
                            passes: ending.passes.clone(),
                            from: bars.at(self.at(ending.start)?).measure,
                            to: bars.closing(self.end_at(ending.end)?),
                        })
                    })
                    .collect();
                Some(RepeatMark {
                    from: bars.at(from).measure,
                    to: bars.closing(to),
                    times: repeat.times,
                    range: repeat.range,
                    endings,
                })
            })
            .collect()
    }
}
