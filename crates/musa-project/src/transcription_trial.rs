//! Private, production-neutral models for prompt 203's transcription trial.
//!
//! This is a measurement seam, not the transcription interface. It consumes
//! repository-owned exact event fixtures, compares deliberately small model
//! families, and returns aggregate evidence. Prompts 204 and 205 implement the
//! admitted model behind a different, narrow project facade.

#![allow(clippy::arithmetic_side_effects)]

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Exact trial grid. Twenty-four ticks admit binary subdivisions through a
/// thirty-second note and ternary subdivisions through an eighth-note triplet.
pub const TICKS_PER_QUARTER: u32 = 24;
/// The largest ranked set considered by the trial.
pub const TOP_K: usize = 5;
const SEARCH_BEAM: usize = 96;
const MAX_VOICES: u8 = 4;

/// One semantic note transition emitted by the private scripted/QWERTY driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualNoteEvent {
    /// A key became held.
    On {
        /// MIDI note number at the device boundary.
        note: u8,
        /// Seven-bit strike velocity.
        velocity: u8,
        /// Exact functional-test timestamp.
        at_micros: u64,
    },
    /// A held key was released.
    Off {
        /// MIDI note number at the device boundary.
        note: u8,
        /// Exact functional-test timestamp.
        at_micros: u64,
    },
}

/// Hardware-independent controller for exercising the capture seam manually.
///
/// The mapping is deliberately a controller concern: its output has the same
/// meaning as device note transitions and carries no written pitch or duration.
#[derive(Debug, Default)]
pub struct QwertyPerformanceDriver {
    held: BTreeMap<char, u8>,
}

impl QwertyPerformanceDriver {
    /// Translate a typing-key transition into a semantic note transition.
    ///
    /// Repeated presses and unmatched releases are ignored, matching the
    /// lifecycle discipline of the physical MIDI adapter.
    pub fn transition(&mut self, key: char, pressed: bool, velocity: u8, at_micros: u64) -> Option<VirtualNoteEvent> {
        let note = qwerty_note(key)?;
        if pressed {
            if self.held.insert(key, note).is_some() {
                return None;
            }
            Some(VirtualNoteEvent::On {
                note,
                velocity: velocity.min(127),
                at_micros,
            })
        } else {
            self.held
                .remove(&key)
                .map(|note| VirtualNoteEvent::Off { note, at_micros })
        }
    }
}

fn qwerty_note(key: char) -> Option<u8> {
    "awsedftgyhujkolp;"
        .chars()
        .position(|candidate| candidate == key.to_ascii_lowercase())
        .and_then(|offset| u8::try_from(offset).ok())
        .map(|offset| 60_u8.saturating_add(offset))
}

/// A repository-owned transcription trial corpus.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TrialCorpus {
    schema: u16,
    ticks_per_quarter: u32,
    fixtures: Vec<TrialFixture>,
}

impl TrialCorpus {
    /// Decode the exact checked-in event format.
    pub fn read(json: &str) -> Result<Self, TrialError> {
        let corpus: Self = serde_json::from_str(json).map_err(|error| TrialError(error.to_string()))?;
        if corpus.schema != 1 {
            return Err(TrialError(format!("unsupported trial schema {}", corpus.schema)));
        }
        if corpus.ticks_per_quarter != TICKS_PER_QUARTER {
            return Err(TrialError(format!(
                "trial grid is {}, expected {TICKS_PER_QUARTER}",
                corpus.ticks_per_quarter
            )));
        }
        for fixture in &corpus.fixtures {
            fixture.validate()?;
        }
        Ok(corpus)
    }

    /// Fixtures in deterministic corpus order.
    #[must_use]
    pub fn fixtures(&self) -> &[TrialFixture] {
        &self.fixtures
    }

    /// Canonical pretty JSON used by the generator law.
    pub fn canonical_json(&self) -> Result<String, TrialError> {
        serde_json::to_string_pretty(self)
            .map(|json| format!("{json}\n"))
            .map_err(|error| TrialError(error.to_string()))
    }
}

/// One paired intended score and symbolic performance trace.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TrialFixture {
    id: String,
    source: String,
    tags: Vec<String>,
    clock: TrialClock,
    notes: Vec<TrialNote>,
    controls: Vec<TrialControl>,
    expected_review: TrialReview,
}

impl TrialFixture {
    fn validate(&self) -> Result<(), TrialError> {
        if self.id.is_empty() || self.notes.is_empty() {
            return Err(TrialError("a trial fixture needs an id and notes".to_owned()));
        }
        let mut ids = BTreeSet::new();
        for note in &self.notes {
            if !ids.insert(note.id) {
                return Err(TrialError(format!("{} repeats event {}", self.id, note.id)));
            }
            if note.release_micros < note.onset_micros || note.sounding_end_micros < note.release_micros {
                return Err(TrialError(format!("{} has a backwards note lifecycle", self.id)));
            }
        }
        Ok(())
    }

    /// Stable fixture identity.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Musa source naming the intended score.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Dimensions deliberately exercised by this fixture.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Number of observed notes.
    #[must_use]
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }
}

/// Whether the performance carries a score clock.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrialClock {
    /// Exact transport/count-in clock.
    Known {
        /// Physical duration of one quarter note.
        quarter_micros: u64,
        /// Calibrated physical timestamp of score tick zero.
        origin_micros: u64,
    },
    /// No transport clock; tempo and phase are hypotheses.
    Free,
    /// No metrical claim is intended.
    Unmeasured,
}

/// One immutable performed note and its intended notation fact, when any.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct TrialNote {
    id: u32,
    pitch: u8,
    onset_micros: u64,
    release_micros: u64,
    sounding_end_micros: u64,
    expected: Option<ExpectedNote>,
}

/// Exact intended notation for one observed event.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct ExpectedNote {
    onset_ticks: u32,
    duration_ticks: u32,
    voice: u8,
    group: u16,
}

/// A retained controller observation. It is evidence, never a written end.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct TrialControl {
    at_micros: u64,
    controller: u8,
    value: u8,
}

/// Review outcome the corpus says is honest.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TrialReview {
    Ready,
    ChooseReading,
    TapPulse,
    WriteSource,
}

/// Candidate model families compared by prompt 203.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrialModel {
    /// Independent nearest sixteenth grid.
    NearestGrid,
    /// Bounded top-K phrase lattice with explicit structural costs.
    StructuredDp,
    /// HMM-shaped negative-log timing/prior/transition baseline.
    BayesianHmm,
}

impl TrialModel {
    /// All compared model families.
    pub const ALL: [Self; 3] = [Self::NearestGrid, Self::StructuredDp, Self::BayesianHmm];
}

/// Voice models compared independently of rhythm candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrialVoiceModel {
    /// Assign simultaneous groups by their current vertical order.
    PitchOrder,
    /// Greedily continue the closest current stream.
    GreedyProximity,
    /// Bounded global beam with pitch, gap, overlap, and crossing costs.
    BoundedCost,
}

impl TrialVoiceModel {
    /// All compared voice models.
    pub const ALL: [Self; 3] = [Self::PitchOrder, Self::GreedyProximity, Self::BoundedCost];
}

/// One model's complete corpus aggregate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TrialAggregate {
    /// Model evaluated.
    pub model: TrialModel,
    /// Exact top-one onset matches.
    pub onset_correct: u32,
    /// Intended notes evaluated.
    pub onset_total: u32,
    /// Fixtures whose intended rhythm appears in top K.
    pub top_k_recall: u32,
    /// Fixtures eligible for a metrical candidate.
    pub top_k_total: u32,
    /// Exact key-duration matches.
    pub duration_correct: u32,
    /// Fixtures whose bar phase is exact.
    pub bar_phase_correct: u32,
    /// Fixtures whose rest locations are exact.
    pub rest_structure_correct: u32,
    /// Fixtures whose bar-crossing tie locations are exact.
    pub tie_structure_correct: u32,
    /// Fixtures whose ternary-division locations are exact.
    pub tuplet_structure_correct: u32,
    /// Token edits from the intended Musa note/rest sequence.
    pub source_edit_distance: u32,
    /// Extra performed notes retained as review corrections.
    pub extra_notes: u32,
    /// Sum of local corrections to the top-one candidate.
    pub review_corrections: u32,
    /// Maximum states retained at any search layer.
    pub peak_states: usize,
    /// Deterministic abstract bytes retained by the largest search.
    pub peak_search_bytes: usize,
}

/// One voice model's complete corpus aggregate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VoiceAggregate {
    /// Model evaluated.
    pub model: TrialVoiceModel,
    /// Notes placed in the intended voice after optimal label permutation.
    pub correct: u32,
    /// Intended notes evaluated.
    pub total: u32,
    /// Pairwise onset-group decisions that match the intended grouping.
    pub grouping_pairs_correct: u32,
    /// Pairwise grouping decisions evaluated.
    pub grouping_pairs_total: u32,
    /// Corrections needed after grouping and voice assignment.
    pub review_corrections: u32,
}

/// A complete deterministic trial report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TrialReport {
    /// Rhythm-model aggregates.
    pub rhythm: Vec<TrialAggregate>,
    /// Voice/group-model aggregates.
    pub voice: Vec<VoiceAggregate>,
}

/// A malformed fixture or impossible trial request.
#[derive(Clone, Debug, thiserror::Error)]
#[error("{0}")]
pub struct TrialError(String);

#[derive(Clone)]
struct RhythmCandidate {
    ticks: Vec<u32>,
    cost: Cost,
}

#[derive(Clone, Copy, Default)]
struct Cost {
    timing: u64,
    complexity: u64,
    transition: u64,
    grouping: u64,
}

impl Cost {
    fn total(self) -> u64 {
        self.timing
            .saturating_add(self.complexity)
            .saturating_add(self.transition)
            .saturating_add(self.grouping)
    }
}

/// Run every compared model over a decoded corpus.
#[must_use]
pub fn run(corpus: &TrialCorpus) -> TrialReport {
    TrialReport {
        rhythm: TrialModel::ALL
            .into_iter()
            .map(|model| rhythm_aggregate(corpus, model))
            .collect(),
        voice: TrialVoiceModel::ALL
            .into_iter()
            .map(|model| voice_aggregate(corpus, model))
            .collect(),
    }
}

fn rhythm_aggregate(corpus: &TrialCorpus, model: TrialModel) -> TrialAggregate {
    let mut aggregate = TrialAggregate {
        model,
        onset_correct: 0,
        onset_total: 0,
        top_k_recall: 0,
        top_k_total: 0,
        duration_correct: 0,
        bar_phase_correct: 0,
        rest_structure_correct: 0,
        tie_structure_correct: 0,
        tuplet_structure_correct: 0,
        source_edit_distance: 0,
        extra_notes: 0,
        review_corrections: 0,
        peak_states: 0,
        peak_search_bytes: 0,
    };
    for fixture in corpus.fixtures() {
        let Some((candidates, peak, peak_bytes)) = rhythm_candidates(fixture, model) else {
            continue;
        };
        aggregate.peak_states = aggregate.peak_states.max(peak);
        aggregate.peak_search_bytes = aggregate.peak_search_bytes.max(peak_bytes);
        let Some(best) = candidates.first() else { continue };
        let expected = fixture
            .notes
            .iter()
            .filter_map(|note| note.expected.map(|expected| (note, expected)))
            .collect::<Vec<_>>();
        aggregate.top_k_total = aggregate.top_k_total.saturating_add(1);
        if candidates.iter().any(|candidate| {
            candidate
                .ticks
                .iter()
                .copied()
                .eq(expected.iter().map(|(_, expected)| expected.onset_ticks))
        }) {
            aggregate.top_k_recall = aggregate.top_k_recall.saturating_add(1);
        }
        for ((note, expected), predicted) in expected.iter().zip(&best.ticks) {
            aggregate.onset_total = aggregate.onset_total.saturating_add(1);
            if expected.onset_ticks == *predicted {
                aggregate.onset_correct = aggregate.onset_correct.saturating_add(1);
            } else {
                aggregate.review_corrections = aggregate.review_corrections.saturating_add(1);
            }
            let duration = quantized_duration(fixture, note, model);
            if duration == expected.duration_ticks {
                aggregate.duration_correct = aggregate.duration_correct.saturating_add(1);
            } else {
                aggregate.review_corrections = aggregate.review_corrections.saturating_add(1);
            }
        }
        let predicted_durations = expected
            .iter()
            .map(|(note, _)| quantized_duration(fixture, note, model))
            .collect::<Vec<_>>();
        let expected_shape = notation_shape(
            expected.iter().map(|(_, expected)| expected.onset_ticks),
            expected.iter().map(|(_, expected)| expected.duration_ticks),
        );
        let predicted_shape = notation_shape(best.ticks.iter().copied(), predicted_durations.iter().copied());
        aggregate.bar_phase_correct = aggregate
            .bar_phase_correct
            .saturating_add(u32::from(expected_shape.phase == predicted_shape.phase));
        aggregate.rest_structure_correct = aggregate
            .rest_structure_correct
            .saturating_add(u32::from(expected_shape.rests == predicted_shape.rests));
        aggregate.tie_structure_correct = aggregate
            .tie_structure_correct
            .saturating_add(u32::from(expected_shape.ties == predicted_shape.ties));
        aggregate.tuplet_structure_correct = aggregate
            .tuplet_structure_correct
            .saturating_add(u32::from(expected_shape.tuplets == predicted_shape.tuplets));
        aggregate.source_edit_distance = aggregate.source_edit_distance.saturating_add(
            u32::try_from(token_edit_distance(&expected_shape.tokens, &predicted_shape.tokens)).unwrap_or(u32::MAX),
        );
        let extras = fixture.notes.iter().filter(|note| note.expected.is_none()).count();
        let extras = u32::try_from(extras).unwrap_or(u32::MAX);
        aggregate.extra_notes = aggregate.extra_notes.saturating_add(extras);
        aggregate.review_corrections = aggregate.review_corrections.saturating_add(extras);
    }
    aggregate
}

#[derive(Default)]
struct NotationShape {
    phase: u32,
    rests: Vec<(u32, u32)>,
    ties: Vec<u32>,
    tuplets: Vec<u32>,
    tokens: Vec<String>,
}

fn notation_shape(onsets: impl IntoIterator<Item = u32>, durations: impl IntoIterator<Item = u32>) -> NotationShape {
    let notes = onsets.into_iter().zip(durations).collect::<Vec<_>>();
    let mut shape = NotationShape {
        phase: notes.first().map_or(0, |(onset, _)| onset % 96),
        ..NotationShape::default()
    };
    let mut previous_end = None;
    for (onset, duration) in notes {
        if let Some(end) = previous_end
            && onset > end
        {
            shape.rests.push((end, onset.saturating_sub(end)));
            shape.tokens.push(format!("rest@{end}/{}", onset.saturating_sub(end)));
        }
        if duration > 0 && onset / 96 != onset.saturating_add(duration).saturating_sub(1) / 96 {
            shape.ties.push(onset);
        }
        if !onset.is_multiple_of(6) || !duration.is_multiple_of(6) {
            shape.tuplets.push(onset);
        }
        shape.tokens.push(format!("note@{onset}/{duration}"));
        previous_end = Some(previous_end.map_or_else(
            || onset.saturating_add(duration),
            |end: u32| end.max(onset.saturating_add(duration)),
        ));
    }
    shape
}

fn token_edit_distance(left: &[String], right: &[String]) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    for (left_index, left_token) in left.iter().enumerate() {
        let mut current = Vec::with_capacity(right.len().saturating_add(1));
        current.push(left_index.saturating_add(1));
        for (right_index, right_token) in right.iter().enumerate() {
            let insertion = current.last().copied().unwrap_or(usize::MAX).saturating_add(1);
            let deletion = previous
                .get(right_index.saturating_add(1))
                .copied()
                .unwrap_or(usize::MAX)
                .saturating_add(1);
            let replacement = previous
                .get(right_index)
                .copied()
                .unwrap_or(usize::MAX)
                .saturating_add(usize::from(left_token != right_token));
            current.push(insertion.min(deletion).min(replacement));
        }
        previous = current;
    }
    previous.last().copied().unwrap_or(0)
}

fn rhythm_candidates(fixture: &TrialFixture, model: TrialModel) -> Option<(Vec<RhythmCandidate>, usize, usize)> {
    if matches!(fixture.clock, TrialClock::Unmeasured) {
        return None;
    }
    let notes = fixture
        .notes
        .iter()
        .filter(|note| note.expected.is_some())
        .collect::<Vec<_>>();
    let origin = match fixture.clock {
        TrialClock::Known { origin_micros, .. } => origin_micros,
        TrialClock::Free => notes.iter().map(|note| note.onset_micros).min()?,
        TrialClock::Unmeasured => return None,
    };
    let tempos = tempo_hypotheses(fixture, &notes);
    let mut all = Vec::new();
    let mut peak = 0;
    let mut peak_bytes = 0;
    for quarter_micros in tempos {
        let (mut candidates, local_peak, local_bytes) = match model {
            TrialModel::NearestGrid => {
                let candidate = nearest_candidate(&notes, origin, quarter_micros);
                let bytes = candidate_bytes(&candidate);
                (vec![candidate], 1, bytes)
            }
            TrialModel::StructuredDp | TrialModel::BayesianHmm => {
                searched_candidates(&notes, origin, quarter_micros, model)
            }
        };
        peak = peak.max(local_peak);
        peak_bytes = peak_bytes.max(local_bytes);
        all.append(&mut candidates);
    }
    all.sort_by(candidate_order);
    all.dedup_by(|left, right| left.ticks == right.ticks);
    all.truncate(TOP_K);
    Some((all, peak, peak_bytes))
}

fn tempo_hypotheses(fixture: &TrialFixture, notes: &[&TrialNote]) -> Vec<u64> {
    if let TrialClock::Known { quarter_micros, .. } = fixture.clock {
        return vec![quarter_micros];
    }
    let mut onsets = notes.iter().map(|note| note.onset_micros).collect::<Vec<_>>();
    onsets.sort_unstable();
    onsets.dedup();
    let mut intervals = onsets
        .windows(2)
        .filter_map(|pair| pair.get(1)?.checked_sub(*pair.first()?))
        .filter(|interval| *interval >= 20_000)
        .collect::<Vec<_>>();
    intervals.sort_unstable();
    let median = intervals
        .get(intervals.len().saturating_sub(1) / 2)
        .copied()
        .unwrap_or(500_000);
    let mut hypotheses = [
        median,
        median.saturating_mul(2),
        median.saturating_mul(3),
        median.saturating_mul(4),
        median.saturating_mul(3) / 2,
        median.saturating_mul(2) / 3,
    ]
    .into_iter()
    .filter(|tempo| (200_000..=1_500_000).contains(tempo))
    .collect::<Vec<_>>();
    hypotheses.sort_unstable();
    hypotheses.dedup();
    hypotheses
}

fn nearest_candidate(notes: &[&TrialNote], origin: u64, quarter_micros: u64) -> RhythmCandidate {
    let step = TICKS_PER_QUARTER / 4;
    let ticks = notes
        .iter()
        .map(|note| physical_ticks(note.onset_micros.saturating_sub(origin), quarter_micros, step))
        .collect();
    RhythmCandidate {
        ticks,
        cost: Cost::default(),
    }
}

fn searched_candidates(
    notes: &[&TrialNote],
    origin: u64,
    quarter_micros: u64,
    model: TrialModel,
) -> (Vec<RhythmCandidate>, usize, usize) {
    let mut beam = vec![RhythmCandidate {
        ticks: Vec::with_capacity(notes.len()),
        cost: Cost::default(),
    }];
    let mut peak = 1;
    let mut peak_bytes: usize = beam.iter().map(candidate_bytes).sum();
    for (index, note) in notes.iter().enumerate() {
        let physical = physical_ticks(note.onset_micros.saturating_sub(origin), quarter_micros, 1);
        let low = physical.saturating_sub(4);
        let high = physical.saturating_add(4);
        let mut next = Vec::with_capacity(beam.len().saturating_mul(9));
        for state in &beam {
            for tick in low..=high {
                if state.ticks.last().is_some_and(|previous| tick < *previous) {
                    continue;
                }
                let mut candidate = state.clone();
                let residual = u64::from(tick.abs_diff(physical));
                candidate.cost.timing = candidate.cost.timing.saturating_add(match model {
                    TrialModel::BayesianHmm => residual.saturating_mul(residual).saturating_mul(3),
                    TrialModel::NearestGrid | TrialModel::StructuredDp => {
                        residual.saturating_mul(residual).saturating_mul(2)
                    }
                });
                candidate.cost.complexity = candidate.cost.complexity.saturating_add(
                    u64::from(tick_complexity(tick)).saturating_mul(if model == TrialModel::BayesianHmm {
                        3
                    } else {
                        2
                    }),
                );
                if let (Some(previous), Some(previous_note)) = (
                    candidate.ticks.last().copied(),
                    index.checked_sub(1).and_then(|at| notes.get(at)),
                ) {
                    let interval = tick.saturating_sub(previous);
                    candidate.cost.transition = candidate
                        .cost
                        .transition
                        .saturating_add(u64::from(interval_complexity(interval)));
                    let spread = note.onset_micros.abs_diff(previous_note.onset_micros);
                    if spread <= adaptive_group_window(quarter_micros) && tick != previous {
                        candidate.cost.grouping = candidate.cost.grouping.saturating_add(10);
                    }
                }
                candidate.ticks.push(tick);
                next.push(candidate);
            }
        }
        next.sort_by(candidate_order);
        next.dedup_by(|left, right| left.ticks == right.ticks);
        next.truncate(SEARCH_BEAM);
        peak = peak.max(next.len());
        peak_bytes = peak_bytes.max(next.iter().map(candidate_bytes).sum());
        beam = next;
    }
    beam.sort_by(candidate_order);
    beam.truncate(TOP_K);
    (beam, peak, peak_bytes)
}

fn candidate_bytes(candidate: &RhythmCandidate) -> usize {
    std::mem::size_of::<RhythmCandidate>()
        .saturating_add(candidate.ticks.capacity().saturating_mul(std::mem::size_of::<u32>()))
}

fn candidate_order(left: &RhythmCandidate, right: &RhythmCandidate) -> Ordering {
    left.cost
        .total()
        .cmp(&right.cost.total())
        .then_with(|| left.ticks.cmp(&right.ticks))
}

fn physical_ticks(elapsed_micros: u64, quarter_micros: u64, step: u32) -> u32 {
    let numerator = u128::from(elapsed_micros)
        .saturating_mul(u128::from(TICKS_PER_QUARTER))
        .saturating_add(u128::from(quarter_micros / 2));
    let raw = numerator / u128::from(quarter_micros.max(1));
    let raw = u32::try_from(raw).unwrap_or(u32::MAX);
    raw.saturating_add(step / 2) / step * step
}

fn tick_complexity(tick: u32) -> u32 {
    let within = tick % TICKS_PER_QUARTER;
    if within == 0 {
        0
    } else if within.is_multiple_of(12) {
        1
    } else if within.is_multiple_of(6) {
        2
    } else if within.is_multiple_of(8) {
        3
    } else if within.is_multiple_of(3) {
        4
    } else if within.is_multiple_of(4) {
        5
    } else {
        8
    }
}

fn interval_complexity(interval: u32) -> u32 {
    if interval == 0 {
        0
    } else {
        tick_complexity(interval).saturating_add(u32::from(interval > TICKS_PER_QUARTER.saturating_mul(4)))
    }
}

fn adaptive_group_window(quarter_micros: u64) -> u64 {
    (quarter_micros / 12).clamp(18_000, 70_000)
}

fn quantized_duration(fixture: &TrialFixture, note: &TrialNote, model: TrialModel) -> u32 {
    let quarter = match fixture.clock {
        TrialClock::Known { quarter_micros, .. } => quarter_micros,
        TrialClock::Free => tempo_hypotheses(fixture, &[note]).first().copied().unwrap_or(500_000),
        TrialClock::Unmeasured => return 0,
    };
    let step = if model == TrialModel::NearestGrid {
        TICKS_PER_QUARTER / 4
    } else {
        1
    };
    physical_ticks(note.release_micros.saturating_sub(note.onset_micros), quarter, step).max(1)
}

fn voice_aggregate(corpus: &TrialCorpus, model: TrialVoiceModel) -> VoiceAggregate {
    let mut aggregate = VoiceAggregate {
        model,
        correct: 0,
        total: 0,
        grouping_pairs_correct: 0,
        grouping_pairs_total: 0,
        review_corrections: 0,
    };
    for fixture in corpus.fixtures() {
        let expected = fixture
            .notes
            .iter()
            .filter_map(|note| note.expected.map(|expected| (note, expected)))
            .collect::<Vec<_>>();
        let groups = observed_groups(fixture, model);
        let assigned = assign_voices(&groups, model);
        let predicted = expected
            .iter()
            .map(|(note, _)| assigned.get(&note.id).copied().unwrap_or(0))
            .collect::<Vec<_>>();
        let intended = expected.iter().map(|(_, expected)| expected.voice).collect::<Vec<_>>();
        let correct = best_label_matches(&predicted, &intended);
        aggregate.correct = aggregate
            .correct
            .saturating_add(u32::try_from(correct).unwrap_or(u32::MAX));
        aggregate.total = aggregate
            .total
            .saturating_add(u32::try_from(expected.len()).unwrap_or(u32::MAX));
        aggregate.review_corrections = aggregate
            .review_corrections
            .saturating_add(u32::try_from(expected.len().saturating_sub(correct)).unwrap_or(u32::MAX));
        for (left, left_expected) in expected.iter().enumerate() {
            for right_expected in expected.iter().skip(left.saturating_add(1)) {
                let expected_same = left_expected.1.group == right_expected.1.group;
                let observed_same = group_of(&groups, left_expected.0.id) == group_of(&groups, right_expected.0.id);
                aggregate.grouping_pairs_total = aggregate.grouping_pairs_total.saturating_add(1);
                if expected_same == observed_same {
                    aggregate.grouping_pairs_correct = aggregate.grouping_pairs_correct.saturating_add(1);
                } else {
                    aggregate.review_corrections = aggregate.review_corrections.saturating_add(1);
                }
            }
        }
    }
    aggregate
}

#[derive(Clone)]
struct OnsetGroup<'a> {
    notes: Vec<&'a TrialNote>,
    at: u64,
    pitch: u8,
}

fn observed_groups(fixture: &TrialFixture, model: TrialVoiceModel) -> Vec<OnsetGroup<'_>> {
    let mut notes = fixture
        .notes
        .iter()
        .filter(|note| note.expected.is_some())
        .collect::<Vec<_>>();
    notes.sort_by_key(|note| (note.onset_micros, note.pitch, note.id));
    let quarter = match fixture.clock {
        TrialClock::Known { quarter_micros, .. } => quarter_micros,
        TrialClock::Free | TrialClock::Unmeasured => 500_000,
    };
    let window = if model == TrialVoiceModel::PitchOrder {
        35_000
    } else {
        adaptive_group_window(quarter)
    };
    let mut groups: Vec<OnsetGroup<'_>> = Vec::new();
    for note in notes {
        if let Some(group) = groups.last_mut()
            && note.onset_micros.saturating_sub(group.at) <= window
        {
            group.notes.push(note);
            group.pitch = median_pitch(&group.notes);
            continue;
        }
        groups.push(OnsetGroup {
            notes: vec![note],
            at: note.onset_micros,
            pitch: note.pitch,
        });
    }
    groups
}

fn median_pitch(notes: &[&TrialNote]) -> u8 {
    let mut pitches = notes.iter().map(|note| note.pitch).collect::<Vec<_>>();
    pitches.sort_unstable();
    pitches.get(pitches.len() / 2).copied().unwrap_or(60)
}

fn group_of(groups: &[OnsetGroup<'_>], note: u32) -> Option<usize> {
    groups
        .iter()
        .position(|group| group.notes.iter().any(|candidate| candidate.id == note))
}

fn assign_voices(groups: &[OnsetGroup<'_>], model: TrialVoiceModel) -> BTreeMap<u32, u8> {
    match model {
        TrialVoiceModel::PitchOrder => pitch_order(groups),
        TrialVoiceModel::GreedyProximity => greedy_voices(groups),
        TrialVoiceModel::BoundedCost => bounded_voices(groups),
    }
}

fn pitch_order(groups: &[OnsetGroup<'_>]) -> BTreeMap<u32, u8> {
    groups
        .iter()
        .flat_map(|group| {
            let mut notes = group.notes.clone();
            notes.sort_by_key(|note| (note.pitch, note.id));
            notes
                .into_iter()
                .enumerate()
                .map(|(voice, note)| (note.id, u8::try_from(voice).unwrap_or(u8::MAX)))
        })
        .collect()
}

fn greedy_voices(groups: &[OnsetGroup<'_>]) -> BTreeMap<u32, u8> {
    let mut last: Vec<u8> = Vec::new();
    let mut answer = BTreeMap::new();
    for group in groups {
        let mut notes = group.notes.clone();
        notes.sort_by_key(|note| (note.pitch, note.id));
        let mut used = BTreeSet::new();
        for note in notes {
            let closest = last
                .iter()
                .enumerate()
                .filter(|(voice, _)| !used.contains(voice))
                .min_by_key(|(_, pitch)| pitch.abs_diff(note.pitch))
                .map(|(voice, _)| voice);
            let voice = if let Some(voice) = closest
                && last.get(voice).is_some_and(|pitch| pitch.abs_diff(note.pitch) <= 12)
            {
                voice
            } else if last.len() < usize::from(MAX_VOICES) {
                last.push(note.pitch);
                last.len().saturating_sub(1)
            } else {
                closest.unwrap_or(0)
            };
            used.insert(voice);
            if let Some(pitch) = last.get_mut(voice) {
                *pitch = note.pitch;
            }
            answer.insert(note.id, u8::try_from(voice).unwrap_or(u8::MAX));
        }
    }
    answer
}

#[derive(Clone)]
struct VoiceState {
    last: Vec<(u8, u64)>,
    assignments: Vec<u8>,
    cost: u64,
}

fn bounded_voices(groups: &[OnsetGroup<'_>]) -> BTreeMap<u32, u8> {
    let mut beam = vec![VoiceState {
        last: Vec::new(),
        assignments: Vec::with_capacity(groups.iter().map(|group| group.notes.len()).sum()),
        cost: 0,
    }];
    for group in groups {
        let mut notes = group.notes.clone();
        notes.sort_by_key(|note| (note.pitch, note.id));
        let mut local = beam
            .into_iter()
            .map(|state| (state, BTreeSet::new()))
            .collect::<Vec<_>>();
        for note in notes {
            let mut next = Vec::new();
            for (state, used) in &local {
                let choices = state.last.len().saturating_add(1).min(usize::from(MAX_VOICES));
                for voice in 0..choices {
                    if used.contains(&voice) {
                        continue;
                    }
                    let mut candidate = state.clone();
                    candidate.cost = candidate
                        .cost
                        .saturating_add(voice_cost(&candidate, voice, note.pitch, group.at));
                    if voice == candidate.last.len() {
                        candidate.last.push((note.pitch, group.at));
                    } else if let Some(last) = candidate.last.get_mut(voice) {
                        *last = (note.pitch, group.at);
                    }
                    candidate.assignments.push(u8::try_from(voice).unwrap_or(u8::MAX));
                    let mut next_used = used.clone();
                    next_used.insert(voice);
                    next.push((candidate, next_used));
                }
            }
            next.sort_by(|(left, _), (right, _)| voice_state_order(left, right));
            next.truncate(SEARCH_BEAM);
            local = next;
        }
        beam = local.into_iter().map(|(state, _)| state).collect();
    }
    let Some(best) = beam.first() else {
        return BTreeMap::new();
    };
    groups
        .iter()
        .flat_map(|group| {
            let mut notes = group.notes.clone();
            notes.sort_by_key(|note| (note.pitch, note.id));
            notes
        })
        .zip(&best.assignments)
        .map(|(note, voice)| (note.id, *voice))
        .collect()
}

fn voice_cost(state: &VoiceState, voice: usize, pitch: u8, at: u64) -> u64 {
    let continuity = state.last.get(voice).map_or(400, |(previous_pitch, previous_at)| {
        let leap = u64::from(previous_pitch.abs_diff(pitch));
        let gap = at.saturating_sub(*previous_at) / 100_000;
        leap.saturating_mul(leap).saturating_add(gap.min(20))
    });
    let crossings = state
        .last
        .iter()
        .enumerate()
        .filter(|(other_voice, (other_pitch, _))| {
            (*other_voice < voice && *other_pitch > pitch) || (*other_voice > voice && *other_pitch < pitch)
        })
        .count();
    continuity.saturating_add(u64::try_from(crossings).unwrap_or(u64::MAX).saturating_mul(36))
}

fn voice_state_order(left: &VoiceState, right: &VoiceState) -> Ordering {
    left.cost
        .cmp(&right.cost)
        .then_with(|| left.assignments.cmp(&right.assignments))
}

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

/// Deterministically generate the repository-owned corpus.
#[must_use]
pub fn generated_corpus() -> TrialCorpus {
    TrialCorpus {
        schema: 1,
        ticks_per_quarter: TICKS_PER_QUARTER,
        fixtures: generated_fixtures(),
    }
}

/// Generate one regular phrase for bounded-search scaling measurements.
///
/// This corpus is benchmark-only and deliberately carries no claim about
/// musical diversity; the checked-in ten-fixture corpus carries that role.
#[must_use]
pub fn generated_stress_corpus(note_count: usize) -> TrialCorpus {
    let ticks = (0..note_count)
        .map(|index| u32::try_from(index).unwrap_or(u32::MAX).saturating_mul(12))
        .collect::<Vec<_>>();
    let jitter = (0..note_count)
        .map(|index| i64::try_from(index % 5).unwrap_or(0).saturating_sub(2))
        .collect::<Vec<_>>();
    let pitches = (0..note_count)
        .map(|index| 60_u8.saturating_add(u8::try_from(index % 8).unwrap_or(0)))
        .collect::<Vec<_>>();
    let voices = vec![0; note_count];
    let groups = (0..note_count)
        .map(|index| u16::try_from(index).unwrap_or(u16::MAX))
        .collect::<Vec<_>>();
    TrialCorpus {
        schema: 1,
        ticks_per_quarter: TICKS_PER_QUARTER,
        fixtures: vec![fixture(
            "regular-stress",
            known_clock(500_000),
            &ticks,
            &jitter,
            &pitches,
            &voices,
            &groups,
            &["benchmark", "regular"],
        )],
    }
}

fn generated_fixtures() -> Vec<TrialFixture> {
    vec![
        fixture(
            "straight-known",
            known_clock(500_000),
            &[0, 24, 48, 72],
            &[0, 5, -7, 8],
            &[60, 62, 64, 65],
            &[0, 0, 0, 0],
            &[0, 1, 2, 3],
            &["known", "straight"],
        ),
        fixture(
            "syncopated-known",
            known_clock(600_000),
            &[0, 12, 36, 42, 60, 84],
            &[0, 8, -9, 12, -5, 7],
            &[60, 62, 64, 65, 67, 69],
            &[0, 0, 0, 0, 0, 0],
            &[0, 1, 2, 3, 4, 5],
            &["known", "syncopation", "ties", "rests"],
        ),
        fixture(
            "triplet-known",
            known_clock(480_000),
            &[0, 8, 16, 24, 32, 40],
            &[0, -6, 7, -4, 6, -5],
            &[60, 62, 64, 65, 67, 69],
            &[0, 0, 0, 0, 0, 0],
            &[0, 1, 2, 3, 4, 5],
            &["known", "tuplet"],
        ),
        fixture(
            "swing-known",
            known_clock(500_000),
            &[0, 12, 24, 36, 48, 60, 72, 84],
            &[0, 70, 0, 65, 0, 72, 0, 68],
            &[60, 62, 64, 65, 67, 69, 71, 72],
            &[0, 0, 0, 0, 0, 0, 0, 0],
            &[0, 1, 2, 3, 4, 5, 6, 7],
            &["known", "swing", "systematic-displacement"],
        ),
        fixture(
            "rubato-free",
            TrialClock::Free,
            &[0, 24, 48, 72, 96, 120],
            &[0, 18, 52, 92, 145, 205],
            &[60, 64, 67, 65, 62, 60],
            &[0, 0, 0, 0, 0, 0],
            &[0, 1, 2, 3, 4, 5],
            &["free", "rubato", "tempo-drift"],
        ),
        fixture(
            "pickup-asymmetric",
            known_clock(520_000),
            &[18, 24, 42, 60, 78, 96, 114],
            &[0, 4, -7, 9, -4, 6, -8],
            &[67, 60, 62, 64, 65, 67, 69],
            &[0, 0, 0, 0, 0, 0, 0],
            &[0, 1, 2, 3, 4, 5, 6],
            &["known", "pickup", "asymmetric-meter"],
        ),
        polyphonic_fixture(false),
        polyphonic_fixture(true),
        pedal_fixture(),
        unmeasured_fixture(),
    ]
}

const fn known_clock(quarter_micros: u64) -> TrialClock {
    TrialClock::Known {
        quarter_micros,
        origin_micros: 0,
    }
}

#[allow(clippy::too_many_arguments)]
fn fixture(
    id: &str,
    clock: TrialClock,
    ticks: &[u32],
    jitter_millis: &[i64],
    pitches: &[u8],
    voices: &[u8],
    groups: &[u16],
    tags: &[&str],
) -> TrialFixture {
    let quarter = match clock {
        TrialClock::Known { quarter_micros, .. } => quarter_micros,
        TrialClock::Free | TrialClock::Unmeasured => 500_000,
    };
    let notes = ticks
        .iter()
        .zip(jitter_millis)
        .zip(pitches)
        .zip(voices)
        .zip(groups)
        .enumerate()
        .map(|(index, ((((&tick, &jitter), &pitch), &voice), &group))| {
            let duration_ticks = intended_duration(index, ticks, voices);
            let base = u64::from(tick).saturating_mul(quarter) / u64::from(TICKS_PER_QUARTER);
            let onset = if jitter.is_negative() {
                base.saturating_sub(jitter.unsigned_abs().saturating_mul(1_000))
            } else {
                base.saturating_add(jitter.unsigned_abs().saturating_mul(1_000))
            };
            let duration_micros = u64::from(duration_ticks).saturating_mul(quarter) / u64::from(TICKS_PER_QUARTER);
            TrialNote {
                id: u32::try_from(index).unwrap_or(u32::MAX),
                pitch,
                onset_micros: onset,
                release_micros: onset.saturating_add(duration_micros),
                sounding_end_micros: onset.saturating_add(duration_micros),
                expected: Some(ExpectedNote {
                    onset_ticks: tick,
                    duration_ticks,
                    voice,
                    group,
                }),
            }
        })
        .collect();
    TrialFixture {
        id: id.to_owned(),
        source: format!("tests/fixtures/transcription/{id}.musa"),
        tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        clock,
        notes,
        controls: Vec::new(),
        expected_review: if matches!(clock, TrialClock::Free) {
            TrialReview::TapPulse
        } else {
            TrialReview::Ready
        },
    }
}

fn intended_duration(index: usize, ticks: &[u32], voices: &[u8]) -> u32 {
    let Some((&onset, &voice)) = ticks.get(index).zip(voices.get(index)) else {
        return TICKS_PER_QUARTER;
    };
    ticks
        .iter()
        .zip(voices)
        .skip(index.saturating_add(1))
        .find_map(|(&later, &later_voice)| {
            (later_voice == voice && later > onset).then_some(later.saturating_sub(onset))
        })
        .unwrap_or(TICKS_PER_QUARTER)
}

fn polyphonic_fixture(crossing: bool) -> TrialFixture {
    let mut fixture = fixture(
        if crossing {
            "crossing-voices"
        } else {
            "rolled-and-block-chords"
        },
        known_clock(500_000),
        &[0, 0, 24, 24, 48, 48, 72, 72],
        if crossing {
            &[0, 12, 4, -6, -5, 7, 8, -4]
        } else {
            &[0, 28, 0, 31, 0, 4, 0, 40]
        },
        if crossing {
            &[48, 72, 55, 67, 64, 59, 72, 52]
        } else {
            &[48, 60, 50, 62, 52, 64, 53, 65]
        },
        &[0, 1, 0, 1, 0, 1, 0, 1],
        &[0, 0, 1, 1, 2, 2, 3, 3],
        if crossing {
            &["known", "polyphony", "voice-crossing"]
        } else {
            &["known", "polyphony", "block-chord", "rolled-chord"]
        },
    );
    fixture.expected_review = TrialReview::ChooseReading;
    fixture
}

fn pedal_fixture() -> TrialFixture {
    let mut fixture = fixture(
        "pedal-repetition-mistake",
        known_clock(500_000),
        &[0, 24, 48, 60, 72],
        &[0, 7, -3, 12, 5],
        &[60, 60, 64, 65, 67],
        &[0, 0, 0, 0, 0],
        &[0, 1, 2, 3, 4],
        &["known", "pedal", "repeated-note", "mistake", "articulation"],
    );
    fixture.controls = vec![
        TrialControl {
            at_micros: 200_000,
            controller: 64,
            value: 127,
        },
        TrialControl {
            at_micros: 1_700_000,
            controller: 64,
            value: 0,
        },
    ];
    for note in &mut fixture.notes {
        note.release_micros = note.onset_micros.saturating_add(180_000);
        note.sounding_end_micros = 1_700_000_u64.max(note.release_micros);
    }
    fixture.notes.push(TrialNote {
        id: 99,
        pitch: 61,
        onset_micros: 910_000,
        release_micros: 980_000,
        sounding_end_micros: 1_700_000,
        expected: None,
    });
    fixture.expected_review = TrialReview::ChooseReading;
    fixture
}

fn unmeasured_fixture() -> TrialFixture {
    let mut fixture = fixture(
        "unmeasured",
        TrialClock::Unmeasured,
        &[0, 7, 19, 36, 58],
        &[0, 8, -4, 12, -7],
        &[60, 62, 65, 64, 60],
        &[0, 0, 0, 0, 0],
        &[0, 1, 2, 3, 4],
        &["unmeasured", "silence", "ametric"],
    );
    fixture.expected_review = TrialReview::WriteSource;
    fixture
}
