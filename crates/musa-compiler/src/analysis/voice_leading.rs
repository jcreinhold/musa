//! The `voice-leading` kind: how the voices of a chordal texture move, under
//! a named style.
//!
//! **Abstract domain.** For the requested profile, the set of pairs *(rule,
//! place)* where the music departs from that rule, together with the list of
//! rules the profile looked at. A place is a span, the voices involved, and
//! the interval at issue where the rule is about one.
//!
//! **Abstraction map.** α cuts the requested lanes at every attack, reads each
//! instant as a column of sounding pitches, and evaluates each rule of the
//! profile over the columns and over the motion between adjacent columns.
//! Under [`AnalysisProfile::Satb`] a voice is a lane the source wrote; under
//! [`AnalysisProfile::JazzVoiceLeading`] a voice is a position counted from
//! the bottom of the voicing, because a jazz voicing is a chord and its
//! "voices" are where the notes sit in it.
//!
//! **Soundness.** Every departure reported here is decidable from the notated
//! score: two voices either move in parallel fifths or they do not. So a
//! departure is a [`Standing::Fact`] — about the *motion*. What it is not is a
//! fact about the music being wrong, and the separation is the whole point of
//! this kind: the finding states the motion, and the rule's
//! [`super::Strength`] states what one historical pedagogy makes of it. The
//! two rules that read a key are the exception: under a key the request
//! assumed rather than one the source wrote, their departures are candidates,
//! because the reading that produced them is one the request chose.
//!
//! γ of a report is every score that departs from those rules in those places.
//! It is not "every score in this style" — nothing here can decide whether a
//! passage is in a style, and a profile is a lens the request picked up.

// Rational arithmetic on musical time is exact mathematical arithmetic, not
// raw integer ops; clippy::arithmetic_side_effects does not apply to it.
#![allow(clippy::arithmetic_side_effects)]

use super::motion::{self, Strand, Tone};
use super::rules::{self, Departure, Rule};
use super::segment::Slice;
use super::{AnalysisError, AnalysisFinding, AnalysisProfile, Lane, NoteRef, Segmentation, Standing};
use crate::chord::ChordClass;
use crate::pitch::{PitchClass, WrittenPitch};
use crate::score::{Key, ScoreSnapshot};
use crate::time::MusicalTime;

/// The four voices, bottom first, with the range OMT `022` writes for each.
///
/// Written as text and parsed rather than built out of letters and octaves:
/// these are the ranges as the source prints them, and a reader checking this
/// table against the page should be able to read the same words.
const SATB_RANGES: [(&str, &str, &str); 4] = [
    ("bass", "e2", "c4"),
    ("tenor", "c3", "g4"),
    ("alto", "g3", "d5"),
    ("soprano", "c4", "g5"),
];

pub(super) fn observe(
    snapshot: &ScoreSnapshot,
    lanes: &[Lane],
    profile: Option<AnalysisProfile>,
    key: Option<Key>,
    window: Option<(MusicalTime, MusicalTime)>,
) -> Result<Vec<AnalysisFinding>, AnalysisError> {
    match profile {
        Some(AnalysisProfile::Satb) => Ok(satb(snapshot, lanes, key, window)),
        Some(AnalysisProfile::JazzVoiceLeading) => Ok(jazz(snapshot, lanes, window)),
        // Unreachable: `profile_of` has already refused a profile of another
        // kind and a request with none. Answering with nothing rather than
        // asserting keeps that check the single place the rule is stated.
        Some(
            AnalysisProfile::Species1
            | AnalysisProfile::Species2
            | AnalysisProfile::Species3
            | AnalysisProfile::Species4
            | AnalysisProfile::Species5,
        )
        | None => Ok(Vec::new()),
    }
}

/// The rules the SATB profile checks, in the order it checks them.
const SATB: [&Rule; 9] = [
    &rules::SATB_FOUR_VOICES,
    &rules::SATB_RANGES,
    &rules::SATB_SPACING,
    &rules::SATB_CROSSING,
    &rules::SATB_OVERLAP,
    &rules::SATB_PARALLEL_PERFECTS,
    &rules::SATB_DIRECT_PERFECTS,
    &rules::SATB_DOUBLING,
    &rules::SATB_TENDENCY_RESOLUTION,
];

/// Four-voice part writing (OMT 022).
fn satb(
    snapshot: &ScoreSnapshot,
    lanes: &[Lane],
    assumed: Option<Key>,
    window: Option<(MusicalTime, MusicalTime)>,
) -> Vec<AnalysisFinding> {
    let strands = motion::strands(snapshot, lanes, window);
    let instants = motion::attacks(&strands);
    let start = instants.first().copied().unwrap_or(MusicalTime::ZERO);
    let mut found: Vec<AnalysisFinding> = SATB.iter().map(|rule| rules::in_force(rule, start)).collect();
    let mut departures = Vec::new();
    departures.extend(four_voices(&strands, &instants));
    if strands.len() == 4 {
        departures.extend(ranges(&strands));
        for pair in instants.windows(2) {
            let [now, next] = *pair else { continue };
            departures.extend(spacing(&strands, now, next));
            departures.extend(crossing(&strands, now, next));
            departures.extend(overlap(&strands, now, next));
            departures.extend(parallels(&strands, now, next));
            departures.extend(direct(&strands, now, next));
        }
        if let Some(last) = instants.last().copied() {
            departures.extend(spacing(&strands, last, last));
            departures.extend(crossing(&strands, last, last));
        }
        departures.extend(tendency(snapshot, &strands, &instants, assumed));
    }
    found.extend(departures.into_iter().map(Departure::finding));
    found
}

/// The profile's own definition: four voices, one note each.
///
/// Reported as one departure per stretch rather than per instant, because a
/// three-voice passage is one thing that is true of a stretch of music and not
/// a mistake repeated at every attack.
fn four_voices(strands: &[Strand], instants: &[MusicalTime]) -> Vec<Departure> {
    let mut departures: Vec<Departure> = Vec::new();
    let mut run: Option<(MusicalTime, Vec<NoteRef>)> = None;
    let close = |run: Option<(MusicalTime, Vec<NoteRef>)>, to: MusicalTime, into: &mut Vec<Departure>| {
        if let Some((from, notes)) = run {
            into.push(Departure {
                rule: &rules::SATB_FOUR_VOICES,
                standing: Standing::Conflict,
                voices: Vec::new(),
                interval: None,
                from,
                to,
                notes,
                also: Vec::new(),
            });
        }
    };
    for at in instants {
        let sounding: Vec<Tone> = strands.iter().filter_map(|strand| strand.at(*at)).collect();
        let doubled = strands.iter().any(|strand| {
            strand
                .tones
                .iter()
                .filter(|tone| tone.onset <= *at && tone.end > *at)
                .count()
                > 1
        });
        if sounding.len() == 4 && strands.len() == 4 && !doubled {
            close(run.take(), *at, &mut departures);
            continue;
        }
        let notes = sounding.iter().map(|tone| tone.note).collect::<Vec<_>>();
        match run {
            Some((_, ref mut carried)) => carried.extend(notes),
            None => run = Some((*at, notes)),
        }
    }
    let end = instants.last().copied().unwrap_or(MusicalTime::ZERO);
    close(run, end, &mut departures);
    departures
}

/// Each voice within the range its part is written for.
fn ranges(strands: &[Strand]) -> Vec<Departure> {
    let mut departures = Vec::new();
    for (index, strand) in strands.iter().enumerate() {
        let Some((_, low, high)) = SATB_RANGES.get(index).copied() else {
            continue;
        };
        let (Some(low), Some(high)) = (WrittenPitch::parse(low), WrittenPitch::parse(high)) else {
            continue;
        };
        for tone in &strand.tones {
            if tone.pitch.chromatic_height() < low.chromatic_height()
                || tone.pitch.chromatic_height() > high.chromatic_height()
            {
                departures.push(Departure {
                    rule: &rules::SATB_RANGES,
                    standing: Standing::Fact,
                    voices: vec![strand.label.clone()],
                    interval: None,
                    from: tone.onset,
                    to: tone.end,
                    notes: vec![tone.note],
                    also: Vec::new(),
                });
            }
        }
    }
    departures
}

/// Adjacent upper voices within an octave (OMT 022).
///
/// The bass is exempt, as the source has it: the distance from the bass to the
/// tenor is a matter of sonority rather than of spacing, and a bass an
/// eleventh below its tenor is ordinary writing.
fn spacing(strands: &[Strand], at: MusicalTime, _next: MusicalTime) -> Vec<Departure> {
    let column = motion::column(strands, at);
    let mut departures = Vec::new();
    for lower in 1..3_usize {
        let (Some(Some(below)), Some(Some(above))) = (column.get(lower), column.get(lower.saturating_add(1))) else {
            continue;
        };
        let Some(interval) = motion::wide(below.pitch, above.pitch) else {
            continue;
        };
        {
            departures.push(Departure {
                rule: &rules::SATB_SPACING,
                standing: Standing::Fact,
                voices: labels(strands, &[lower, lower.saturating_add(1)]),
                interval: Some(interval),
                from: at,
                to: below.end.min(above.end),
                notes: vec![below.note, above.note],
                also: Vec::new(),
            });
        }
    }
    departures
}

/// No voice sounding below the voice beneath it.
fn crossing(strands: &[Strand], at: MusicalTime, _next: MusicalTime) -> Vec<Departure> {
    let column = motion::column(strands, at);
    let mut departures = Vec::new();
    for lower in 0..column.len().saturating_sub(1) {
        let (Some(Some(below)), Some(Some(above))) = (column.get(lower), column.get(lower.saturating_add(1))) else {
            continue;
        };
        if above.pitch.chromatic_height() < below.pitch.chromatic_height() {
            departures.push(Departure {
                rule: &rules::SATB_CROSSING,
                standing: Standing::Fact,
                voices: labels(strands, &[lower, lower.saturating_add(1)]),
                interval: motion::between(above.pitch, below.pitch),
                from: at,
                to: below.end.min(above.end),
                notes: vec![below.note, above.note],
                also: Vec::new(),
            });
        }
    }
    departures
}

/// No voice moving past the note its neighbour has just left.
fn overlap(strands: &[Strand], now: MusicalTime, next: MusicalTime) -> Vec<Departure> {
    let (before, after) = (motion::column(strands, now), motion::column(strands, next));
    let mut departures = Vec::new();
    for lower in 0..before.len().saturating_sub(1) {
        let upper = lower.saturating_add(1);
        let (Some(Some(low_was)), Some(Some(high_was))) = (before.get(lower), before.get(upper)) else {
            continue;
        };
        let (Some(Some(low_now)), Some(Some(high_now))) = (after.get(lower), after.get(upper)) else {
            continue;
        };
        if motion::overlaps((low_was.pitch, high_was.pitch), (low_now.pitch, high_now.pitch)) {
            departures.push(Departure {
                rule: &rules::SATB_OVERLAP,
                standing: Standing::Fact,
                voices: labels(strands, &[lower, upper]),
                interval: None,
                from: now,
                to: next,
                notes: vec![low_was.note, high_was.note, low_now.note, high_now.note],
                also: Vec::new(),
            });
        }
    }
    departures
}

/// No two voices moving into a perfect consonance of the size they just had.
///
/// Both voices must move, and both intervals must be perfect and the same
/// size. A fifth reached from a fifth by keeping one voice still is oblique
/// motion and no parallel; a fifth followed by an octave is not "the same
/// size" and this rule says nothing about it.
fn parallels(strands: &[Strand], now: MusicalTime, next: MusicalTime) -> Vec<Departure> {
    let (before, after) = (motion::column(strands, now), motion::column(strands, next));
    let mut departures = Vec::new();
    for lower in 0..before.len() {
        for upper in lower.saturating_add(1)..before.len() {
            let Some((was, is)) = pair(&before, &after, lower, upper) else {
                continue;
            };
            let Some(arrived) = motion::parallel_perfect((was.0.pitch, was.1.pitch), (is.0.pitch, is.1.pitch)) else {
                continue;
            };
            {
                departures.push(Departure {
                    rule: &rules::SATB_PARALLEL_PERFECTS,
                    standing: Standing::Fact,
                    voices: labels(strands, &[lower, upper]),
                    interval: Some(arrived),
                    from: now,
                    to: next,
                    notes: vec![was.0.note, was.1.note, is.0.note, is.1.note],
                    also: Vec::new(),
                });
            }
        }
    }
    departures
}

/// The outer voices not arriving at a perfect consonance in similar motion
/// with a leap on top (OMT 022's direct fifths and octaves).
fn direct(strands: &[Strand], now: MusicalTime, next: MusicalTime) -> Vec<Departure> {
    let (before, after) = (motion::column(strands, now), motion::column(strands, next));
    let top = before.len().saturating_sub(1);
    let Some((was, is)) = pair(&before, &after, 0, top) else {
        return Vec::new();
    };
    let (Some(first), Some(second)) = (
        motion::between(was.0.pitch, was.1.pitch),
        motion::between(is.0.pitch, is.1.pitch),
    ) else {
        return Vec::new();
    };
    let similar = matches!(
        motion::motion((was.0.pitch, is.0.pitch), (was.1.pitch, is.1.pitch)),
        motion::Motion::Similar | motion::Motion::Parallel
    );
    let leaps = motion::leap(was.1.pitch, is.1.pitch) > 1;
    // A parallel perfect is already reported by its own rule; reporting it
    // again as a direct perfect would be one motion counted twice.
    let already = motion::is_perfect(first) && motion::size(first) == motion::size(second);
    if similar && leaps && motion::is_perfect(second) && !already {
        return vec![Departure {
            rule: &rules::SATB_DIRECT_PERFECTS,
            standing: Standing::Fact,
            voices: labels(strands, &[0, top]),
            interval: Some(second),
            from: now,
            to: next,
            notes: vec![was.0.note, was.1.note, is.0.note, is.1.note],
            also: Vec::new(),
        }];
    }
    Vec::new()
}

/// The two rules that need a key: the leading tone is not doubled, and it
/// rises when it moves.
///
/// Both are silent where no key is in force. That is not a gap to be filled by
/// guessing one: OMT 022 states these rules about the leading tone *of a key*,
/// and a reading that inferred the key in order to check them would be
/// reporting its own guess as the source's.
fn tendency(
    snapshot: &ScoreSnapshot,
    strands: &[Strand],
    instants: &[MusicalTime],
    assumed: Option<Key>,
) -> Vec<Departure> {
    let scope = strands
        .first()
        .map_or(crate::scope::Scope::Piece, |strand| crate::scope::Scope::Voice {
            part: strand.lane.0.0,
            voice: strand.lane.1.0,
        });
    let standing = if assumed.is_some() {
        Standing::Candidate
    } else {
        Standing::Fact
    };
    let mut departures = Vec::new();
    for (index, at) in instants.iter().enumerate() {
        let Some(key) = assumed.or_else(|| snapshot.key_at(scope, *at)) else {
            continue;
        };
        let column = motion::column(strands, *at);
        let leading: Vec<usize> = column
            .iter()
            .enumerate()
            .filter(|(_, tone)| tone.is_some_and(|tone| is_leading_tone(tone.pitch, key)))
            .map(|(position, _)| position)
            .collect();
        let notes = |positions: &[usize]| {
            positions
                .iter()
                .filter_map(|position| column.get(*position).copied().flatten().map(|tone| tone.note))
                .collect::<Vec<_>>()
        };
        if leading.len() > 1 {
            departures.push(Departure {
                rule: &rules::SATB_DOUBLING,
                standing,
                voices: labels(strands, &leading),
                interval: None,
                from: *at,
                to: *at,
                notes: notes(&leading),
                also: Vec::new(),
            });
        }
        let Some(next) = instants.get(index.saturating_add(1)).copied() else {
            continue;
        };
        let after = motion::column(strands, next);
        for position in leading {
            let (Some(Some(was)), Some(Some(is))) = (column.get(position), after.get(position)) else {
                continue;
            };
            if was.pitch != is.pitch && is.pitch.pitch_class() != key.tonic() {
                departures.push(Departure {
                    rule: &rules::SATB_TENDENCY_RESOLUTION,
                    standing,
                    voices: labels(strands, &[position]),
                    interval: motion::between(was.pitch, is.pitch),
                    from: *at,
                    to: next,
                    notes: vec![was.note, is.note],
                    also: Vec::new(),
                });
            }
        }
    }
    departures
}

/// Whether `pitch` is the leading tone of `key`: a diatonic step and a
/// semitone below the tonic, which is what makes it lead.
///
/// Stated over the two coordinates rather than over a scale, so the raised
/// seventh of a minor key is the leading tone and the natural seventh is not —
/// which is the distinction the rule is about.
fn is_leading_tone(pitch: WrittenPitch, key: Key) -> bool {
    let tonic = key.tonic();
    let letter = i64::from(tonic.letter.steps());
    let semitone = i64::from(tonic.letter.natural_semitone()).saturating_add(i64::from(tonic.accidental.0));
    pitch.diatonic_height().saturating_add(1).rem_euclid(7) == letter.rem_euclid(7)
        && pitch.chromatic_height().saturating_add(1).rem_euclid(12) == semitone.rem_euclid(12)
}

/// The two voices' notes before and after, when both sound at both instants.
type Moved = ((Tone, Tone), (Tone, Tone));

fn pair(before: &[Option<Tone>], after: &[Option<Tone>], lower: usize, upper: usize) -> Option<Moved> {
    let was = ((*before.get(lower)?)?, (*before.get(upper)?)?);
    let is = ((*after.get(lower)?)?, (*after.get(upper)?)?);
    Some((was, is))
}

/// The names of the voices at those positions.
fn labels(strands: &[Strand], positions: &[usize]) -> Vec<String> {
    positions
        .iter()
        .filter_map(|position| strands.get(*position).map(|strand| strand.label.clone()))
        .collect()
}

/// The rules the jazz profile checks, in the order it checks them.
const JAZZ: [&Rule; 6] = [
    &rules::JAZZ_GUIDE_TONES,
    &rules::JAZZ_GUIDE_TONE_MOTION,
    &rules::JAZZ_COMMON_TONE,
    &rules::JAZZ_SMALL_MOTION,
    &rules::JAZZ_SPACING,
    &rules::JAZZ_OMISSION,
];

/// Jazz voicing motion (OMT 076), whose rules are guidelines by its own
/// account.
///
/// Voices here are positions in the voicing, not lanes: a pianist's voicing is
/// written as a chord, and "the voice that held the third" means the note that
/// sat in that place. The chord itself comes from the written harmony lane —
/// nothing here derives a symbol from the notes, which is `crate::harmony`'s
/// standing rule.
fn jazz(snapshot: &ScoreSnapshot, lanes: &[Lane], window: Option<(MusicalTime, MusicalTime)>) -> Vec<AnalysisFinding> {
    let slices = super::segment::slices(snapshot, lanes, Segmentation::HarmonyLane, window);
    let start = slices.first().map_or(MusicalTime::ZERO, |slice| slice.onset);
    let mut found: Vec<AnalysisFinding> = JAZZ.iter().map(|rule| rules::in_force(rule, start)).collect();
    let mut departures = Vec::new();
    for slice in &slices {
        departures.extend(voicing(snapshot, slice));
        departures.extend(jazz_spacing(slice));
    }
    for pair in slices.windows(2) {
        let [now, next] = pair else { continue };
        departures.extend(jazz_motion(now, next, symbol_at(snapshot, now)));
    }
    found.extend(departures.into_iter().map(Departure::finding));
    found
}

/// The chord written over a slice, when one is written there.
fn symbol_at(snapshot: &ScoreSnapshot, slice: &Slice) -> Option<ChordClass> {
    let mark = snapshot
        .annotations()
        .harmony()
        .iter()
        .rfind(|mark| mark.at <= slice.onset)?;
    super::chords::spelled(&mark.symbol)
}

/// Guide tones present, and the root or fifth accounted for.
fn voicing(snapshot: &ScoreSnapshot, slice: &Slice) -> Vec<Departure> {
    let Some(chord) = symbol_at(snapshot, slice) else {
        return Vec::new();
    };
    let classes = slice.classes();
    let member = |at: usize| chord.member_class(at);
    let mut departures = Vec::new();
    let guides: Vec<PitchClass> = [member(1), member(3)].into_iter().flatten().collect();
    if guides.iter().any(|guide| !classes.contains(guide)) {
        departures.push(Departure {
            rule: &rules::JAZZ_GUIDE_TONES,
            standing: Standing::Fact,
            voices: Vec::new(),
            interval: None,
            from: slice.onset,
            to: slice.onset + slice.extent,
            notes: slice.refs(),
            also: Vec::new(),
        });
    }
    let anchors: Vec<PitchClass> = [member(0), member(2)].into_iter().flatten().collect();
    if !anchors.is_empty() && anchors.iter().all(|anchor| !classes.contains(anchor)) {
        departures.push(Departure {
            rule: &rules::JAZZ_OMISSION,
            standing: Standing::Fact,
            voices: Vec::new(),
            interval: None,
            from: slice.onset,
            to: slice.onset + slice.extent,
            notes: slice.refs(),
            also: Vec::new(),
        });
    }
    departures
}

/// No interval within a voicing wider than an octave, and no second in the
/// bass register.
fn jazz_spacing(slice: &Slice) -> Vec<Departure> {
    let mut departures = Vec::new();
    for pair in slice.notes.windows(2) {
        let [below, above] = pair else { continue };
        let Some(interval) =
            motion::wide(below.pitch, above.pitch).or_else(|| motion::crowded(below.pitch, above.pitch))
        else {
            continue;
        };
        {
            departures.push(Departure {
                rule: &rules::JAZZ_SPACING,
                standing: Standing::Fact,
                voices: Vec::new(),
                interval: Some(interval),
                from: slice.onset,
                to: slice.onset + slice.extent,
                notes: vec![below.note, above.note],
                also: Vec::new(),
            });
        }
    }
    departures
}

/// What happens between two voicings: guide tones, common tones, and how far
/// any voice travels.
fn jazz_motion(now: &Slice, next: &Slice, chord: Option<ChordClass>) -> Vec<Departure> {
    let mut departures = Vec::new();
    let span = (now.onset, next.onset + next.extent);
    let guides: Vec<PitchClass> = chord
        .into_iter()
        .flat_map(|chord| [chord.member_class(1), chord.member_class(3)])
        .flatten()
        .collect();
    let notes = |left: &Slice, right: &Slice| {
        let mut notes = left.refs();
        notes.extend(right.refs());
        notes
    };
    for (position, below) in now.notes.iter().enumerate() {
        let Some(above) = next.notes.get(position) else {
            continue;
        };
        if guides.contains(&below.pitch.pitch_class()) && motion::leap(below.pitch, above.pitch) > 1 {
            departures.push(Departure {
                rule: &rules::JAZZ_GUIDE_TONE_MOTION,
                standing: Standing::Fact,
                voices: vec![format!("voice {}", position.saturating_add(1))],
                interval: motion::between(below.pitch, above.pitch),
                from: span.0,
                to: span.1,
                notes: vec![below.note, above.note],
                also: Vec::new(),
            });
        }
        if motion::far(below.pitch, above.pitch) {
            departures.push(Departure {
                rule: &rules::JAZZ_SMALL_MOTION,
                standing: Standing::Fact,
                voices: vec![format!("voice {}", position.saturating_add(1))],
                interval: motion::between(below.pitch, above.pitch),
                from: span.0,
                to: span.1,
                notes: vec![below.note, above.note],
                also: Vec::new(),
            });
        }
    }
    let (here, there) = (now.classes(), next.classes());
    let shared: Vec<PitchClass> = here.iter().copied().filter(|class| there.contains(class)).collect();
    let held = now.notes.iter().any(|note| {
        shared.contains(&note.pitch.pitch_class()) && next.notes.iter().any(|other| other.pitch == note.pitch)
    });
    if !shared.is_empty() && !held {
        departures.push(Departure {
            rule: &rules::JAZZ_COMMON_TONE,
            standing: Standing::Fact,
            voices: Vec::new(),
            interval: None,
            from: span.0,
            to: span.1,
            notes: notes(now, next),
            also: Vec::new(),
        });
    }
    departures
}
