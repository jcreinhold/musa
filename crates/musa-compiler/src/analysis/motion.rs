//! What the profiles in [`super::voice_leading`] and [`super::counterpoint`]
//! both need: voices as lines of notes, the instants where anything changes,
//! and the two interval questions every rule below is built out of.
//!
//! Nothing here is a rule. A rule belongs to a style and cites a page; this
//! module knows only what the notation says — which notes are in which voice,
//! when each one starts and stops, and how far apart two of them are. Keeping
//! that separate is what lets one implementation serve three pedagogies
//! without any of them leaking into the others.
//!
//! # Exact notated spans
//!
//! Every span here is the *notated* one, in whole notes, and a tie has already
//! been merged into it: `crate::elaborate` resolves a tie into one occurrence
//! before a snapshot exists, so a whole note tied to a whole note is one tone
//! of length 2 here rather than two of length 1. That matters for the fourth
//! species, where the suspension *is* the tie, and it is the reason no rule
//! below ever looks at performed time. Performed time is groove, tempo, and
//! swing; a counterpoint rule that read it would be judging a performance for
//! a notational mistake.

// Rational arithmetic on musical time is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use super::{Lane, NoteRef, inside};
use crate::origin::Interval;
use crate::pitch::WrittenPitch;
use crate::score::{PartId, ScoreEventKind, ScoreSnapshot, VoiceId};
use crate::time::MusicalTime;

/// One note of one voice, with the exact span it occupies.
#[derive(Clone, Copy, Debug)]
pub(super) struct Tone {
    /// The written pitch, spelled as written.
    pub(super) pitch: WrittenPitch,
    /// Where the note is, by every coordinate a reader can act on.
    pub(super) note: NoteRef,
    /// When it starts.
    pub(super) onset: MusicalTime,
    /// When it stops.
    pub(super) end: MusicalTime,
}

/// One voice, as a line of notes in time.
///
/// A *strand* rather than a voice, because `crate::score::Voice` is the lane
/// the source wrote and this is what a style rule argues about: the notes of
/// that lane, in order, with the rests between them left as gaps rather than
/// filled in.
pub(super) struct Strand {
    /// What to call it in a finding: the voice's name, qualified by its part
    /// when the reading spans more than one.
    pub(super) label: String,
    /// Which lane it came from.
    pub(super) lane: (PartId, VoiceId),
    /// Its notes, in time order.
    pub(super) tones: Vec<Tone>,
}

impl Strand {
    /// What this voice sounds at `at`, or `None` when it rests.
    pub(super) fn at(&self, at: MusicalTime) -> Option<Tone> {
        self.tones
            .iter()
            .copied()
            .find(|tone| tone.onset <= at && tone.end > at)
    }

    /// Where the voice sits overall, for ordering the texture: the sum of its
    /// diatonic heights over its note count, as a pair so no division is done.
    fn height(&self) -> (i64, i64) {
        let sum = self
            .tones
            .iter()
            .try_fold(0_i64, |total, tone| total.checked_add(tone.pitch.diatonic_height()))
            .unwrap_or(i64::MAX);
        (sum, i64::try_from(self.tones.len()).unwrap_or(1).max(1))
    }
}

/// The requested lanes as lines of notes, lowest voice first.
///
/// Ordered by where each voice *lies* rather than by the order the source
/// declares them, because "the bass" means the bottom of the texture and a
/// source is free to write its voices in any order. Voice crossing is still
/// visible after this ordering, and that is the point: crossing is exactly the
/// place where the order at one instant disagrees with the order overall.
///
/// A lane sounding two pitches at once — a chord written inside one voice —
/// contributes both tones to that strand. That is not silently a second voice:
/// the profiles that identify voices this way check that no lane does it, and
/// report it as a departure from their own definition rather than inventing a
/// voice the source did not write.
pub(super) fn strands(
    snapshot: &ScoreSnapshot,
    lanes: &[Lane],
    window: Option<(MusicalTime, MusicalTime)>,
) -> Vec<Strand> {
    let mut strands: Vec<Strand> = Vec::new();
    let many_parts = lanes
        .iter()
        .map(|lane| lane.part)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        > 1;
    for lane in lanes {
        let Some((part, voice)) = snapshot
            .parts()
            .get(lane.part)
            .and_then(|part| part.voice(lane.voice).map(|voice| (part, voice)))
        else {
            continue;
        };
        let name = part.voice_name(lane.voice).unwrap_or("voice").to_owned();
        let label = if many_parts {
            format!("{}/{name}", part.name())
        } else {
            name
        };
        let mut tones = Vec::new();
        for event in voice.events() {
            if !inside(window, event.onset) {
                continue;
            }
            let note = NoteRef {
                part: lane.part,
                voice: lane.voice,
                id: event.id,
                span: event.origin.definition_span,
            };
            let end = event.onset + event.notated_duration.value;
            let mut push = |pitch| {
                tones.push(Tone {
                    pitch,
                    note,
                    onset: event.onset,
                    end,
                });
            };
            match event.kind {
                ScoreEventKind::Note { pitch } => push(pitch),
                ScoreEventKind::Chord { ref pitches } => pitches.iter().copied().for_each(push),
                ScoreEventKind::Rest => {}
            }
        }
        tones.sort_by_key(|tone| (tone.onset, tone.pitch.diatonic_height(), tone.note.id.0));
        strands.push(Strand {
            label,
            lane: (lane.part, lane.voice),
            tones,
        });
    }
    strands.retain(|strand| !strand.tones.is_empty());
    strands.sort_by(|left, right| {
        let ((left_sum, left_count), (right_sum, right_count)) = (left.height(), right.height());
        // Compare the two averages by cross-multiplying, so an empty-safe
        // integer comparison stands in for a division that could not be exact.
        let ordering = left_sum
            .checked_mul(right_count)
            .zip(right_sum.checked_mul(left_count))
            .map_or(std::cmp::Ordering::Equal, |(left, right)| left.cmp(&right));
        ordering.then_with(|| left.lane.cmp(&right.lane))
    });
    strands
}

/// Every instant any voice attacks a note, in order.
pub(super) fn attacks(strands: &[Strand]) -> Vec<MusicalTime> {
    let mut instants: Vec<MusicalTime> = strands
        .iter()
        .flat_map(|strand| strand.tones.iter().map(|tone| tone.onset))
        .collect();
    instants.sort_unstable();
    instants.dedup();
    instants
}

/// What every voice sounds at `at`, in voice order.
pub(super) fn column(strands: &[Strand], at: MusicalTime) -> Vec<Option<Tone>> {
    strands.iter().map(|strand| strand.at(at)).collect()
}

/// The interval from `low` to `high`, signed the way [`Interval`] signs one.
pub(super) fn between(low: WrittenPitch, high: WrittenPitch) -> Option<Interval> {
    Some(Interval {
        diatonic_steps: high.diatonic_height().checked_sub(low.diatonic_height())?,
        semitones: high.chromatic_height().checked_sub(low.chromatic_height())?,
    })
}

/// The interval reduced into one octave, upward: a tenth becomes a third, and
/// a descending fifth becomes an ascending fourth.
pub(super) fn simple(interval: Interval) -> Interval {
    let steps = interval.diatonic_steps.rem_euclid(7);
    let semitones = interval.semitones.rem_euclid(12);
    Interval {
        diatonic_steps: steps,
        semitones,
    }
}

/// The diatonic size a musician says out loud: 1 for a unison, 5 for a fifth,
/// 8 for an octave.
pub(super) fn size(interval: Interval) -> i64 {
    let simple = simple(interval);
    if simple.diatonic_steps == 0 && interval.diatonic_steps != 0 {
        8
    } else {
        simple.diatonic_steps.saturating_add(1)
    }
}

/// How two sounding notes stand to each other (OMT 023).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Consonance {
    /// Unison, fifth, octave: the perfect consonances.
    Perfect,
    /// Third and sixth: the imperfect consonances.
    Imperfect,
    /// Everything else, including the fourth **when the lower note is the
    /// bass**.
    Dissonant,
}

/// Whether an interval is a consonance, and of which kind.
///
/// `over_bass` is not a convenience: OMT `023` states that the perfect fourth
/// is a dissonance when it is formed with the lowest sounding voice and a
/// consonance when it is not, and an interval table that answered the question
/// without it would be encoding one of the two readings as the truth. So the
/// caller says which situation it is asking about, every time.
pub(super) fn consonance(interval: Interval, over_bass: bool) -> Consonance {
    let simple = simple(interval);
    match (size(interval), simple.semitones) {
        (1 | 8, 0) => Consonance::Perfect,
        (5, 7) => Consonance::Perfect,
        (4, 5) if over_bass => Consonance::Dissonant,
        (4, 5) => Consonance::Perfect,
        (3, 3 | 4) | (6, 8 | 9) => Consonance::Imperfect,
        _ => Consonance::Dissonant,
    }
}

/// Whether the interval is a perfect fifth, octave, or unison — the sizes the
/// parallel and direct rules are about.
pub(super) fn is_perfect(interval: Interval) -> bool {
    matches!(consonance(interval, false), Consonance::Perfect) && size(interval) != 4
}

/// How two voices move from one instant to the next (OMT 023).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Motion {
    /// Both voices hold, or both hold their pitch class.
    Oblique,
    /// One voice holds and the other moves.
    Static,
    /// Both move the same direction.
    Similar,
    /// Both move, in opposite directions.
    Contrary,
    /// Both move the same direction by the same interval.
    Parallel,
}

/// Which motion carries `(low, high)` to `(next_low, next_high)`.
pub(super) fn motion(low: (WrittenPitch, WrittenPitch), high: (WrittenPitch, WrittenPitch)) -> Motion {
    let step = |pair: (WrittenPitch, WrittenPitch)| {
        pair.1
            .diatonic_height()
            .checked_sub(pair.0.diatonic_height())
            .unwrap_or(0)
    };
    let (lower, upper) = (step(low), step(high));
    match (lower.signum(), upper.signum()) {
        (0, 0) => Motion::Oblique,
        (0, _) | (_, 0) => Motion::Static,
        (left, right) if left != right => Motion::Contrary,
        _ if lower == upper => Motion::Parallel,
        _ => Motion::Similar,
    }
}

/// How far a voice moves, in diatonic steps, ignoring direction.
pub(super) fn leap(from: WrittenPitch, to: WrittenPitch) -> i64 {
    to.diatonic_height()
        .checked_sub(from.diatonic_height())
        .unwrap_or(0)
        .abs()
}

// The four predicates below are `pub(crate)` because they are the rules
// themselves, and `crate::assert` checks the same rules over a passage a
// composer wrote `assert follows(…)` on. Two traversals — one over a
// snapshot's voices, one over a passage's sonorities — and one statement of
// what each rule says, so an assertion and an analysis cannot come to
// different answers about the same two chords.

/// The interval two voices arrive at when they move into a perfect consonance
/// of the size they just had, or `None` when they do not.
///
/// Both voices must move. A perfect fifth reached from a perfect fifth by
/// holding one voice still is oblique motion, which no tradition calls
/// parallel, and a fifth followed by an octave is not the same size.
pub(crate) fn parallel_perfect(
    was: (WrittenPitch, WrittenPitch),
    is: (WrittenPitch, WrittenPitch),
) -> Option<Interval> {
    let (first, second) = (between(was.0, was.1)?, between(is.0, is.1)?);
    let moving = matches!(motion((was.0, is.0), (was.1, is.1)), Motion::Parallel | Motion::Similar);
    (moving && is_perfect(first) && is_perfect(second) && size(first) == size(second)).then_some(second)
}

/// The interval between two adjacent voices when it exceeds an octave.
pub(crate) fn wide(below: WrittenPitch, above: WrittenPitch) -> Option<Interval> {
    between(below, above).filter(|interval| interval.diatonic_steps > 7)
}

/// The interval between two adjacent voices when they sound a second in the
/// register below middle C, where OMT 076 says a voicing turns muddy.
pub(crate) fn crowded(below: WrittenPitch, above: WrittenPitch) -> Option<Interval> {
    const MIDDLE_C: i64 = 48;
    between(below, above).filter(|interval| interval.diatonic_steps <= 1 && below.chromatic_height() < MIDDLE_C)
}

/// Whether one voice moves past the note its neighbour has just left.
pub(crate) fn overlaps(was: (WrittenPitch, WrittenPitch), is: (WrittenPitch, WrittenPitch)) -> bool {
    is.1.chromatic_height() < was.0.chromatic_height() || is.0.chromatic_height() > was.1.chromatic_height()
}

/// Whether a voice travels more than a third.
pub(crate) fn far(from: WrittenPitch, to: WrittenPitch) -> bool {
    leap(from, to) > 2
}
