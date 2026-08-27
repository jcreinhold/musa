//! Voice assignment jointly with the rhythm facts prompt 205 pairs (prompt 205a).
//!
//! The search reads the completed notes and onset groups prompt 205 produced —
//! exact physical/sounding intervals, never raw MIDI — and assigns each note
//! one of at most four voices by a bounded cost search. Pitch proximity,
//! temporal continuity, and crossings are candidate costs, never laws: a
//! crossing is a finite penalty, and an explicit pin overrides inference for
//! exactly the event it names.

// Group member indices come from `group_notes`, which builds them from
// `0..notes.len()`; reading a note through one cannot leave the slice.
#![allow(clippy::indexing_slicing)]

use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::transcription_pairing::{CompletedNote, OnsetGroup, group_notes};

/// The hard voice ceiling prompt 203 measured and prompt 205a keeps.
pub(crate) const MAX_VOICES: u8 = 4;
/// The surviving-state beam, shared with the rhythm search's 96-state bound.
const VOICE_BEAM: usize = 96;
/// The new-voice cost, matching the trial's measured baseline.
const NEW_VOICE_COST: u64 = 400;
/// The crossing penalty multiplier, matching the trial.
const CROSSING_COST: u64 = 36;

/// A complete voice decision over one take: one slot per completed note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VoiceAssignment {
    /// `voices[index]` is the proposed voice (0..[`MAX_VOICES`]) for notes
    /// index in the input order.
    pub voices: Vec<u8>,
    /// `groups[index]` is the onset-group index for notes index.
    pub groups: Vec<usize>,
    /// Number of distinct voices the assignment uses.
    pub voice_count: usize,
}

/// A musician-supplied exact constraint over the assignment. Overrides
/// inference for exactly the event it names, like a rhythm tap-anchor pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceConstraint {
    /// Pin one note to an exact voice. Changing one pin leaves every other
    /// note's assigned voice, and the whole grouping, byte-identical.
    Pin { note_index: usize, voice: u8 },
}

/// Why a take cannot be voiced within the four-voice ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceRefusal {
    /// One onset group holds more simultaneous notes than [`MAX_VOICES`] can
    /// voice; a voice holds one pitch at a time, so this is refused rather
    /// than silently merged.
    TooManyVoices { onset_micros: u64 },
}

/// Assign voices over completed notes: group by the adaptive window, then a
/// bounded cost search with proximity/continuity/new-voice/crossing terms.
pub(crate) fn assign_voices(
    notes: &[CompletedNote],
    quarter_micros: u64,
    constraints: &[VoiceConstraint],
) -> Result<VoiceAssignment, VoiceRefusal> {
    let groups = group_notes(notes, quarter_micros);
    if let Some(group) = groups.iter().find(|group| group.notes.len() > usize::from(MAX_VOICES)) {
        return Err(VoiceRefusal::TooManyVoices {
            onset_micros: group.onset_micros,
        });
    }
    let mut voices = bounded_voices(&groups, notes);
    for constraint in constraints {
        let VoiceConstraint::Pin { note_index, voice } = *constraint;
        if let Some(slot) = voices.get_mut(note_index) {
            *slot = voice.min(MAX_VOICES.saturating_sub(1));
        }
    }
    let groups_of_notes = group_of_each_note(&groups, notes.len());
    let voice_count = voices.iter().copied().collect::<BTreeSet<_>>().len();
    Ok(VoiceAssignment {
        voices,
        groups: groups_of_notes,
        voice_count,
    })
}

/// One surviving assignment state in the bounded search.
#[derive(Clone)]
struct VoiceState {
    last: Vec<(u8, u64)>,
    assignments: Vec<u8>,
    cost: u64,
}

/// The bounded cost search over groups, matching the trial's measured
/// `bounded_cost` baseline (59/63 labels, 176/176 grouping pairs).
fn bounded_voices(groups: &[OnsetGroup], notes: &[CompletedNote]) -> Vec<u8> {
    let mut beam = vec![VoiceState {
        last: Vec::new(),
        assignments: Vec::with_capacity(notes.len()),
        cost: 0,
    }];
    for group in groups {
        let mut members = group.notes.clone();
        members.sort_by_key(|&index| (notes[index].note, index));
        let mut local = beam
            .into_iter()
            .map(|state| (state, BTreeSet::new()))
            .collect::<Vec<_>>();
        for &index in &members {
            let pitch = notes[index].note;
            let at = group.onset_micros;
            let mut next = Vec::new();
            for (state, used) in &local {
                let choices = state.last.len().saturating_add(1).min(usize::from(MAX_VOICES));
                for voice in 0..choices {
                    if used.contains(&voice) {
                        continue;
                    }
                    let mut candidate = state.clone();
                    candidate.cost = candidate.cost.saturating_add(voice_cost(&candidate, voice, pitch, at));
                    if let Some(last) = candidate.last.get_mut(voice) {
                        *last = (pitch, at);
                    } else {
                        candidate.last.push((pitch, at));
                    }
                    candidate.assignments.push(u8::try_from(voice).unwrap_or(u8::MAX));
                    let mut next_used = used.clone();
                    next_used.insert(voice);
                    next.push((candidate, next_used));
                }
            }
            next.sort_by(|(left, _), (right, _)| voice_state_order(left, right));
            next.truncate(VOICE_BEAM);
            local = next;
        }
        beam = local.into_iter().map(|(state, _)| state).collect();
    }

    // Flatten the best state's per-group, pitch-sorted assignments back into
    // input order.
    let mut voices = vec![u8::MAX; notes.len()];
    if let Some(best) = beam.first() {
        let ordered = groups
            .iter()
            .flat_map(|group| {
                let mut members = group.notes.clone();
                members.sort_by_key(|&index| (notes[index].note, index));
                members
            })
            .collect::<Vec<_>>();
        for (global, &note_index) in ordered.iter().enumerate() {
            if let Some(&voice) = best.assignments.get(global)
                && let Some(slot) = voices.get_mut(note_index)
            {
                *slot = voice;
            }
        }
    }
    voices
}

/// The local cost of placing `pitch` into `voice`: continuity (leap squared
/// plus bounded gap), a finite new-voice cost, and a finite crossing penalty.
fn voice_cost(state: &VoiceState, voice: usize, pitch: u8, at: u64) -> u64 {
    let continuity = state
        .last
        .get(voice)
        .map_or(NEW_VOICE_COST, |(previous_pitch, previous_at)| {
            let leap = u64::from(previous_pitch.abs_diff(pitch));
            let gap = at.saturating_sub(*previous_at) / 100_000;
            leap.saturating_mul(leap).saturating_add(gap.min(20))
        });
    let crossings = state
        .last
        .iter()
        .enumerate()
        .filter(|(other, (other_pitch, _))| {
            (*other < voice && *other_pitch > pitch) || (*other > voice && *other_pitch < pitch)
        })
        .count();
    continuity.saturating_add(
        u64::try_from(crossings)
            .unwrap_or(u64::MAX)
            .saturating_mul(CROSSING_COST),
    )
}

fn voice_state_order(left: &VoiceState, right: &VoiceState) -> Ordering {
    left.cost
        .cmp(&right.cost)
        .then_with(|| left.assignments.cmp(&right.assignments))
}

/// Map each completed-note index to its onset group index.
fn group_of_each_note(groups: &[OnsetGroup], note_count: usize) -> Vec<usize> {
    let mut result = vec![usize::MAX; note_count];
    for (group_index, group) in groups.iter().enumerate() {
        for &note_index in &group.notes {
            if let Some(slot) = result.get_mut(note_index) {
                *slot = group_index;
            }
        }
    }
    result
}

#[cfg(test)]
mod laws {
    #![allow(clippy::expect_used)]
    #![allow(clippy::indexing_slicing)]
    use super::*;
    use serde::Deserialize;

    const CORPUS: &str = include_str!("../tests/fixtures/transcription/corpus.json");

    fn ok(result: Result<VoiceAssignment, VoiceRefusal>) -> VoiceAssignment {
        result.expect("synthetic notes stay within the four-voice ceiling")
    }

    #[derive(Deserialize)]
    struct Corpus {
        fixtures: Vec<Fixture>,
    }

    #[derive(Deserialize)]
    struct Fixture {
        clock: Clock,
        notes: Vec<Note>,
    }

    #[derive(Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum Clock {
        Known { quarter_micros: u64 },
        Free,
        Unmeasured,
    }

    #[derive(Deserialize)]
    struct Note {
        pitch: u8,
        onset_micros: u64,
        release_micros: u64,
        sounding_end_micros: u64,
        expected: Option<Expected>,
    }

    #[derive(Clone, Copy, Deserialize)]
    struct Expected {
        voice: u8,
        group: u16,
    }

    fn how_many_voices(assignment: &VoiceAssignment) -> usize {
        assignment.voices.iter().copied().collect::<BTreeSet<_>>().len()
    }

    #[test]
    fn the_corpus_voice_and_grouping_thresholds_hold() {
        let corpus: Corpus = serde_json::from_str(CORPUS).expect("decode the checked-in corpus");
        let mut voice_correct = 0_usize;
        let mut voice_total = 0_usize;
        let mut grouping_correct = 0_usize;
        let mut grouping_total = 0_usize;
        for fixture in corpus.fixtures {
            let quarter = match fixture.clock {
                Clock::Known { quarter_micros, .. } => quarter_micros,
                Clock::Free | Clock::Unmeasured => 500_000,
            };
            let expected: Vec<(usize, Expected)> = fixture
                .notes
                .iter()
                .enumerate()
                .filter_map(|(index, note)| note.expected.map(|expected| (index, expected)))
                .collect();
            if expected.is_empty() {
                continue;
            }
            let mut notes = Vec::new();
            for (index, _) in &expected {
                let note = &fixture.notes[*index];
                notes.push(CompletedNote {
                    note: note.pitch,
                    attack_velocity: 64,
                    release_velocity: 0,
                    onset_micros: note.onset_micros,
                    key_release_micros: note.release_micros,
                    sounding_end_micros: note.sounding_end_micros,
                    derivation: [u64::try_from(*index).unwrap_or(u64::MAX); 2],
                    pedal_extended: false,
                });
            }
            let assignment = ok(assign_voices(&notes, quarter, &[]));
            let predicted = assignment.voices;
            let intended = expected.iter().map(|(_, e)| e.voice).collect::<Vec<_>>();
            voice_correct = voice_correct.saturating_add(best_label_matches(&predicted, &intended));
            voice_total = voice_total.saturating_add(intended.len());

            // Grouping pairs over the expected notes, like the trial. The
            // assignment's `groups` is indexed by the expected-note order this
            // law built, so position equality is the pairing decision.
            for left in 0..expected.len() {
                for right in left.saturating_add(1)..expected.len() {
                    let expected_same = expected[left].1.group == expected[right].1.group;
                    let observed_same = assignment.groups[left] == assignment.groups[right];
                    grouping_total = grouping_total.saturating_add(1);
                    if expected_same == observed_same {
                        grouping_correct = grouping_correct.saturating_add(1);
                    }
                }
            }
        }
        assert!(
            voice_correct >= 59 && voice_total == 63,
            "voice labels {voice_correct}/{voice_total}"
        );
        assert!(
            grouping_correct >= 176 && grouping_total == 176,
            "grouping pairs {grouping_correct}/{grouping_total}"
        );
    }

    #[test]
    fn assignment_is_deterministic() {
        let notes = scale(5);
        assert_eq!(
            ok(assign_voices(&notes, 500_000, &[])),
            ok(assign_voices(&notes, 500_000, &[]))
        );
    }

    #[test]
    fn a_chord_beyond_four_simultaneous_notes_is_refused() {
        let notes = chord_stack(5, 90);
        assert_eq!(
            assign_voices(&notes, 500_000, &[]),
            Err(VoiceRefusal::TooManyVoices { onset_micros: 90 })
        );
        let within = chord_stack(4, 90);
        let assignment = ok(assign_voices(&within, 500_000, &[]));
        assert!(how_many_voices(&assignment) <= usize::from(MAX_VOICES));
        assert!(assignment.voices.iter().all(|voice| *voice < MAX_VOICES));
    }

    #[test]
    fn a_pin_changes_only_the_pinned_note() {
        let notes = scale(8);
        let base = ok(assign_voices(&notes, 500_000, &[]));
        let pinned = ok(assign_voices(
            &notes,
            500_000,
            &[VoiceConstraint::Pin {
                note_index: 3,
                voice: 2,
            }],
        ));
        assert_eq!(pinned.voices[3], 2);
        for index in 0..notes.len() {
            if index != 3 {
                assert_eq!(pinned.voices[index], base.voices[index], "note {index} changed");
            }
        }
        assert_eq!(pinned.groups, base.groups, "a pin never re-groups");
    }

    #[test]
    fn a_pin_restating_inference_is_byte_identical() {
        let notes = scale(8);
        let base = ok(assign_voices(&notes, 500_000, &[]));
        let again = ok(assign_voices(
            &notes,
            500_000,
            &[VoiceConstraint::Pin {
                note_index: 2,
                voice: base.voices[2],
            }],
        ));
        assert_eq!(again, base);
    }

    #[test]
    fn crossings_are_a_penalty_not_a_refusal() {
        // Two lines that keep passing each other still yield an assignment:
        // the search never refuses on a crossing, it only prices it.
        let mut notes = Vec::new();
        for index in 0..6_u64 {
            notes.push(note(
                60 + u8::try_from(index % 7).unwrap_or(0),
                index.saturating_mul(50_000),
            ));
            notes.push(note(
                72_u8.saturating_sub(u8::try_from(index % 7).unwrap_or(0)),
                index.saturating_mul(50_000),
            ));
        }
        let assignment = ok(assign_voices(&notes, 500_000, &[]));
        assert_eq!(assignment.voices.len(), notes.len());
        assert!(how_many_voices(&assignment) >= 2, "two crossing lines stay two voices");
    }

    fn note(note: u8, onset_micros: u64) -> CompletedNote {
        CompletedNote {
            note,
            attack_velocity: 64,
            release_velocity: 0,
            onset_micros,
            key_release_micros: onset_micros.saturating_add(40_000),
            sounding_end_micros: onset_micros.saturating_add(40_000),
            derivation: [0, 0],
            pedal_extended: false,
        }
    }

    fn scale(count: u8) -> Vec<CompletedNote> {
        (0..count)
            .map(|index| note(60_u8.saturating_add(index), u64::from(index).saturating_mul(50_000)))
            .collect()
    }

    fn chord_stack(count: u8, at: u64) -> Vec<CompletedNote> {
        (0..count)
            .map(|index| note(48_u8.saturating_add(index.saturating_mul(3)), at))
            .collect()
    }

    /// Exact label matches under the best one-to-one voice permutation.
    fn best_label_matches(predicted: &[u8], intended: &[u8]) -> usize {
        let labels = intended
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut permutation = (0..labels.len())
            .map(|index| u8::try_from(index).unwrap_or(u8::MAX))
            .collect::<Vec<_>>();
        let mut best = 0;
        permute(&mut permutation, 0, &mut |mapping| {
            let matched = predicted
                .iter()
                .zip(intended)
                .filter(|(predicted, intended)| {
                    labels
                        .iter()
                        .position(|label| label == *intended)
                        .and_then(|index| mapping.get(index))
                        .is_some_and(|mapped| mapped == *predicted)
                })
                .count();
            best = best.max(matched);
        });
        best
    }

    fn permute(values: &mut [u8], at: usize, visit: &mut impl FnMut(&[u8])) {
        if at == values.len() {
            visit(values);
            return;
        }
        for index in at..values.len() {
            values.swap(at, index);
            permute(values, at.saturating_add(1), visit);
            values.swap(at, index);
        }
    }
}
