//! The bounded, exact, deterministic rhythm-candidate search of prompt 204a.
//!
//! This module is the production-neutral optimizer prompt 203 admitted and
//! prompt 204a implements. It consumes one decoded [`crate::transcription_policy`]
//! policy and one take (exact captured timestamps plus a clock and a bar
//! length), and returns a small ranked set of exact metrical candidates as an
//! immutable candidate DAG with shared back-pointers and per-layer compaction.
//! It is test-gated until prompt 204b gives it its production caller (the
//! report facade) and its admission laws reach it through that facade.
//!
//! The search is a port of prompt 203's structural dynamic programming, with
//! its cost multipliers moved into the checked policy and its per-state cloned
//! tick vectors replaced by a compact shared arena. Every musical position and
//! every tie-break is exact; floating point never appears here.

#![allow(clippy::arithmetic_side_effects)]
// The candidate DAG is an index arena: every `arena[i]` read is a back-pointer
// whose bounds the compaction pass maintains as an invariant, the same way the
// trial module's integer arithmetic is allowed rather than defensively checked
// at every site.
#![allow(clippy::indexing_slicing)]

use std::cmp::Ordering;

use num_rational::Ratio;

use crate::transcription_policy::TranscriptionPolicy;

/// The number of published cost-record fields, in their fixed order.
pub(crate) const COST_FIELDS: usize = 9;

/// Sentinel back-pointer naming "no previous note", held by the arena root.
const ROOT: u16 = u16::MAX;

/// One exact performed note transition a take carries into the search.
///
/// `pitch` and `velocity` deliberately do not appear: this is a rhythm-only
/// pass, and prompt 205 owns everything that reads pitch. The derivation from
/// raw event ids is `id`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RhythmEvent {
    pub(crate) id: u32,
    pub(crate) onset_micros: u64,
    pub(crate) release_micros: u64,
    pub(crate) sounding_end_micros: u64,
}

/// The clock evidence a take carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TakeClock {
    /// Calibrated transport/count-in clock, never rebased to the first onset.
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

/// One take: the exact events in their captured order, the clock, and the bar
/// length (in ticks) the destination meter implies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Take {
    pub(crate) clock: TakeClock,
    /// One bar in the destination meter, as exact grid ticks.
    pub(crate) bar_ticks: u32,
    pub(crate) events: Vec<RhythmEvent>,
    /// Musician-supplied exact constraints: `(event index, pinned tick)`. A pin
    /// overrides inference for that event and nothing else; reproducing the
    /// same take and pins reproduces the identical result.
    pub(crate) pins: Vec<(usize, u32)>,
}

/// Why a take produced no grid under the policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// Ametric input, or the `unmeasured` policy.
    WriteSource,
    /// More completed notes than the policy admits and no caller-supplied split
    /// (the split path is prompt 204b's facade concern).
    Unsplittable {
        /// Exact retained length.
        note_count: usize,
    },
    /// A retained candidate proposes more simultaneous voices than the policy
    /// admits.
    TooManyVoices,
}

/// The complete, exact, ordered cost record of one candidate.
///
/// Fields are in the published order of the policy's `cost_fields`. Field 0
/// (onset displacement) and field 3 (notation complexity) are the weighted
/// ranking terms the trial measured; fields 1 and 2 (duration displacement and
/// tempo smoothness) are measured and reported but weighted at zero by the
/// standard policy; fields 4–8 restate structure — rests, ties, tuplets,
/// syncopation preservation, and user constraints — for the reported vector
/// only, never for ranking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CostRecord {
    values: [u64; COST_FIELDS],
}

impl CostRecord {
    fn new(values: [u64; COST_FIELDS]) -> Self {
        Self { values }
    }

    /// The weighted ranking total: exactly the trial's two nonzero ranking
    /// fields summed.
    pub(crate) const fn rank_total(&self) -> u64 {
        self.values[0].saturating_add(self.values[3])
    }

    /// The complete ordered vector, for the published field-order tie-break.
    pub(crate) const fn fields(&self) -> &[u64; COST_FIELDS] {
        &self.values
    }
}

/// One materialized exact rhythm candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    /// Exact onset ticks, one per take event in order.
    pub(crate) onsets: Vec<u32>,
    /// Quantized key-release durations, one per take event.
    pub(crate) durations: Vec<u32>,
    /// Raw event ids in the same order, deriving the candidate from the take.
    pub(crate) event_ids: Vec<u32>,
    /// The complete cost breakdown.
    pub(crate) cost: CostRecord,
    /// Rests between written ends and next onsets, as `(end_tick, length)`.
    pub(crate) rests: Vec<(u32, u32)>,
    /// Onsets whose written note crosses the bar.
    pub(crate) ties: Vec<u32>,
    /// Onsets or durations not on the sixteenth grid (candidate tuplets).
    pub(crate) tuplets: Vec<u32>,
}

impl Candidate {
    /// The canonical structural bytes ordering a tied candidate after the cost
    /// vector: the exact onset and then duration sequences.
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.onsets.len().saturating_add(self.durations.len()).saturating_mul(4));
        for onset in &self.onsets {
            bytes.extend_from_slice(&onset.to_be_bytes());
        }
        for duration in &self.durations {
            bytes.extend_from_slice(&duration.to_be_bytes());
        }
        bytes
    }
}

/// A local region where retained candidates disagree structurally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReviewRegion {
    /// First event index of the disagreement.
    pub(crate) event_index: usize,
    /// One of phase, grouping, end, rest, tie, tuplet.
    pub(crate) dimension: &'static str,
}

/// The outcome of one search over one take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SearchOutcome {
    /// At most five materialized candidates, ordered.
    Ranked {
        candidates: Vec<Candidate>,
        /// Structural disagreements, never a percentage.
        needs_review: Vec<ReviewRegion>,
    },
    /// The take cannot be turned into a ranked grid under this policy.
    Refused(Refusal),
}

/// The bounding facts a search law needs: the largest surviving beam and the
/// largest compacted arena + beam storage, both measured after truncation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Measurements {
    pub(crate) peak_states: usize,
    pub(crate) peak_storage_bytes: usize,
}

/// Search one take under one policy, discarding the bounding measurements.
pub(crate) fn search_take(take: &Take, policy: &TranscriptionPolicy) -> SearchOutcome {
    search_take_measured(take, policy).0
}

/// Search one take under one policy, returning the bounding measurements too.
pub(crate) fn search_take_measured(take: &Take, policy: &TranscriptionPolicy) -> (SearchOutcome, Measurements) {
    if matches!(take.clock, TakeClock::Unmeasured) {
        return (
            SearchOutcome::Refused(Refusal::WriteSource),
            Measurements {
                peak_states: 0,
                peak_storage_bytes: 0,
            },
        );
    }
    let notes = take.events.as_slice();
    if notes.is_empty() {
        return (
            SearchOutcome::Ranked {
                candidates: Vec::new(),
                needs_review: Vec::new(),
            },
            Measurements {
                peak_states: 0,
                peak_storage_bytes: 0,
            },
        );
    }
    let max_notes = policy.bounds().max_notes();
    if notes.len() > usize::try_from(max_notes).unwrap_or(usize::MAX) {
        return (
            SearchOutcome::Refused(Refusal::Unsplittable {
                note_count: notes.len(),
            }),
            Measurements {
                peak_states: 0,
                peak_storage_bytes: 0,
            },
        );
    }
    let grid = grid_ticks(policy);
    let origin = match take.clock {
        TakeClock::Known { origin_micros, .. } => origin_micros,
        TakeClock::Free => notes.iter().map(|note| note.onset_micros).min().unwrap_or(0),
        TakeClock::Unmeasured => 0,
    };
    let tempos = tempo_hypotheses(take);
    let mut ranked = Vec::new();
    let mut peak_states = 0;
    let mut peak_storage = 0;
    for quarter_micros in tempos {
        let (candidates, states, storage) = search_at_tempo(take, policy, grid, origin, quarter_micros);
        ranked.extend(candidates);
        peak_states = peak_states.max(states);
        peak_storage = peak_storage.max(storage);
    }
    ranked.sort_by(candidate_order);
    ranked.dedup_by(|left, right| left.onsets == right.onsets);
    let max_voices = policy.bounds().max_voices();
    let had_candidates = !ranked.is_empty();
    ranked.retain(|candidate| max_simultaneous(&candidate.onsets) <= max_voices);
    if had_candidates && ranked.is_empty() {
        return (
            SearchOutcome::Refused(Refusal::TooManyVoices),
            Measurements {
                peak_states,
                peak_storage_bytes: peak_storage,
            },
        );
    }
    ranked.truncate(usize::try_from(policy.bounds().top_k()).unwrap_or(5));
    let needs_review = review_regions(&ranked, take.bar_ticks);
    (
        SearchOutcome::Ranked {
            candidates: ranked,
            needs_review,
        },
        Measurements {
            peak_states,
            peak_storage_bytes: peak_storage,
        },
    )
}

/// The largest number of onsets a candidate places on one tick.
fn max_simultaneous(onsets: &[u32]) -> u64 {
    let mut counts = std::collections::BTreeMap::<u32, u64>::new();
    for onset in onsets {
        *counts.entry(*onset).or_default() += 1;
    }
    counts.values().copied().max().unwrap_or(0)
}

/// The exact search grid: the least common multiple of every declared
/// subdivision and tuplet denominator. For the standard policy this is 24.
fn grid_ticks(policy: &TranscriptionPolicy) -> u32 {
    let mut grid = 1_u64;
    for subdivision in policy.subdivisions() {
        grid = lcm(grid, ratio_denominator(subdivision.beat_fraction()));
    }
    for tuplet in policy.tuplets() {
        grid = lcm(grid, ratio_denominator(tuplet.member_fraction()));
    }
    u32::try_from(grid).unwrap_or(u32::MAX)
}

fn ratio_denominator(fraction: &Ratio<i64>) -> u64 {
    u64::try_from(*fraction.denom()).unwrap_or(1)
}

/// A non-negative policy weight, reduced to its integer scale (every standard
/// policy ratio is an integer; a fractional one would be applied exactly in
/// [`rational_times`]).
fn ratio_scalar(weight: &Ratio<i64>) -> u64 {
    weight.numer().unsigned_abs() / weight.denom().unsigned_abs().max(1)
}

/// Multiply a non-negative term by a non-negative policy ratio, rounding down
/// to an exact integer. Negative magnitudes are impossible here.
fn rational_times(term: u64, weight: &Ratio<i64>) -> u64 {
    let numerator = u128::from(term).saturating_mul(u128::from(weight.numer().unsigned_abs()));
    let denominator = u128::from(weight.denom().unsigned_abs()).max(1);
    u64::try_from(numerator / denominator).unwrap_or(u64::MAX)
}

const fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left
}

const fn lcm(left: u64, right: u64) -> u64 {
    if left == 0 || right == 0 {
        return 0;
    }
    left / gcd(left, right) * right
}

/// The tick divisor a declared fraction implies: the number of grid ticks one
/// member occupies, `grid * numerator / denominator`. Because the grid is the
/// least common multiple of every declared denominator, this division is exact.
fn fraction_divisor(fraction: &Ratio<i64>, grid: u32) -> u32 {
    let numerator = u32::try_from(fraction.numer().unsigned_abs()).unwrap_or(u32::MAX);
    let denominator = u32::try_from(fraction.denom().unsigned_abs()).unwrap_or(u32::MAX);
    if numerator == 0 || denominator == 0 {
        return 0;
    }
    u32::try_from(u64::from(grid) * u64::from(numerator) / u64::from(denominator)).unwrap_or(u32::MAX)
}

/// The declared notation-complexity of a grid position, derived from the policy
/// rather than from prompt 203's host ladder. The first declared division
/// (subdivisions then tuplets) whose divisor divides the position wins; a
/// position no division names gets `unsubdivided_complexity`.
fn position_complexity(within: u32, grid: u32, policy: &TranscriptionPolicy) -> u64 {
    for subdivision in policy.subdivisions() {
        let divisor = fraction_divisor(subdivision.beat_fraction(), grid);
        if divisor != 0 && within.is_multiple_of(divisor) {
            return subdivision.complexity();
        }
    }
    for tuplet in policy.tuplets() {
        let divisor = fraction_divisor(tuplet.member_fraction(), grid);
        if divisor != 0 && within.is_multiple_of(divisor) {
            return tuplet.complexity();
        }
    }
    policy.unsubdivided_complexity()
}

/// The trial's interval preference, restated over the policy: the position
/// ladder on the interval, plus one when an interval exceeds four beats.
fn interval_complexity(interval: u32, grid: u32, policy: &TranscriptionPolicy) -> u64 {
    if interval == 0 {
        0
    } else {
        position_complexity(interval % grid, grid, policy).saturating_add(u64::from(interval > grid.saturating_mul(4)))
    }
}

/// The adaptive onset-group window in microseconds: one twelfth of the local
/// beat, clamped to the policy's exact physical-time band.
fn adaptive_group_window_micros(quarter_micros: u64, policy: &TranscriptionPolicy) -> u64 {
    let divisor = ratio_denominator(policy.group_window().beat_fraction());
    let min = ratio_to_micros(policy.group_window().min_seconds());
    let max = ratio_to_micros(policy.group_window().max_seconds());
    (quarter_micros / divisor.max(1)).clamp(min, max)
}

fn ratio_to_micros(seconds: &Ratio<i64>) -> u64 {
    seconds.numer().unsigned_abs().saturating_mul(1_000_000) / seconds.denom().unsigned_abs().max(1)
}

/// Ranked tempo hypotheses for a free-clock take, ported from the trial's
/// median-onset-interval generator under the same explicit bounds.
fn tempo_hypotheses(take: &Take) -> Vec<u64> {
    if let TakeClock::Known { quarter_micros, .. } = take.clock {
        return vec![quarter_micros];
    }
    let mut onsets = take.events.iter().map(|note| note.onset_micros).collect::<Vec<_>>();
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

/// The exact grid tick of an elapsed physical time, half-up, with no floating
/// point anywhere.
fn physical_ticks(elapsed_micros: u64, quarter_micros: u64, grid: u32) -> u32 {
    let numerator = u128::from(elapsed_micros)
        .saturating_mul(u128::from(grid))
        .saturating_add(u128::from(quarter_micros / 2));
    let raw = numerator / u128::from(quarter_micros.max(1));
    u32::try_from(raw).unwrap_or(u32::MAX)
}

/// The quantized key-release duration of one event, as the trial measured it.
fn quantized_duration(note: &RhythmEvent, quarter_micros: u64, grid: u32) -> u32 {
    physical_ticks(
        note.release_micros.saturating_sub(note.onset_micros),
        quarter_micros,
        grid,
    )
    .max(1)
}

/// The quarter-micros reference the trial measured key durations against: the
/// known clock's own quarter, or, for a free take, the first hypothesis of the
/// single-note fallback (one third of the 500 ms median, i.e. `333_333` µs).
fn duration_quarter_micros(take: &Take) -> u64 {
    match take.clock {
        TakeClock::Known { quarter_micros, .. } => quarter_micros,
        TakeClock::Free => free_duration_quarter_micros(),
        TakeClock::Unmeasured => 0,
    }
}

fn free_duration_quarter_micros() -> u64 {
    // Reproduces the trial's measured free-clock reference exactly: with no
    // onset intervals the median is 500_000 µs, and the sorted single-note
    // hypothesis list (median, 2m, 3m, 4m, 3m/2, 2m/3, filtered to
    // 200_000–1_500_000 µs) starts at 2m/3.
    let median = 500_000_u64;
    let mut list = [
        median,
        median.saturating_mul(2),
        median.saturating_mul(3),
        median.saturating_mul(4),
        median.saturating_mul(3) / 2,
        median.saturating_mul(2) / 3,
    ];
    list.sort_unstable();
    list.into_iter()
        .find(|tempo| (200_000..=1_500_000).contains(tempo))
        .unwrap_or(median)
}

/// One compact node in the candidate DAG: an exact onset decision, a back
/// pointer into the shared arena, and the cumulative ranking cost. All three
/// fit in `u16` under the policy bounds.
#[derive(Clone, Copy, Debug)]
struct Node {
    tick: u16,
    prev: u16,
    cost: u16,
}

impl Node {
    /// Arena storage one node occupies, for the storage law.
    const fn bytes() -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Run the beam search at one quarter-micros tempo hypothesis.
#[allow(clippy::too_many_arguments)]
fn search_at_tempo(
    take: &Take,
    policy: &TranscriptionPolicy,
    grid: u32,
    origin: u64,
    quarter_micros: u64,
) -> (Vec<Candidate>, usize, usize) {
    let notes = take.events.as_slice();
    let mut arena: Vec<Node> = vec![Node {
        tick: 0,
        prev: ROOT,
        cost: 0,
    }];
    let mut beam: Vec<u16> = vec![0];
    let mut peak_states = 1;
    let mut peak_storage = storage(&arena, &beam);

    for (index, note) in notes.iter().enumerate() {
        let next_beam = expand_layer(
            &mut arena,
            &beam,
            notes,
            index,
            note,
            origin,
            quarter_micros,
            grid,
            policy,
            &take.pins,
        );
        let next_beam = sort_and_truncate(
            &arena,
            next_beam,
            usize::try_from(policy.bounds().layer_states()).unwrap_or(96),
        );
        peak_states = peak_states.max(next_beam.len());
        let compacted = compact(&arena, &next_beam);
        arena = compacted.0;
        beam = compacted.1;
        peak_storage = peak_storage.max(
            arena
                .len()
                .saturating_mul(Node::bytes())
                .saturating_add(beam.len().saturating_mul(2)),
        );
    }

    let mut beam = sort_and_truncate(&arena, beam, usize::try_from(policy.bounds().top_k()).unwrap_or(5));
    let compacted = compact(&arena, &beam);
    arena = compacted.0;
    beam = compacted.1;
    peak_storage = peak_storage.max(
        arena
            .len()
            .saturating_mul(Node::bytes())
            .saturating_add(beam.len().saturating_mul(2)),
    );

    let candidates = beam
        .iter()
        .map(|&leaf| materialize(take, policy, grid, quarter_micros, &arena, leaf))
        .collect();
    (candidates, peak_states, peak_storage)
}

/// Expand one take event over every surviving state, appending candidate nodes
/// to the arena and returning their indices.
#[allow(clippy::too_many_arguments)]
fn expand_layer(
    arena: &mut Vec<Node>,
    beam: &[u16],
    notes: &[RhythmEvent],
    index: usize,
    note: &RhythmEvent,
    origin: u64,
    quarter_micros: u64,
    grid: u32,
    policy: &TranscriptionPolicy,
    pins: &[(usize, u32)],
) -> Vec<u16> {
    let onset_weight = policy.weights().onset_residual();
    let notation_weight = policy.weights().notation_complexity();
    let transition_weight = policy.weights().transition();
    let group_split = ratio_scalar(policy.weights().group_split());
    let window = adaptive_group_window_micros(quarter_micros, policy);

    let physical = physical_ticks(note.onset_micros.saturating_sub(origin), quarter_micros, grid);
    let (low, high) = match pins.iter().find(|(at, _)| *at == index) {
        Some((_, pinned)) => (*pinned, *pinned),
        None => (physical.saturating_sub(4), physical.saturating_add(4)),
    };
    let mut next_beam = Vec::with_capacity(beam.len().saturating_mul(9));
    for &state in beam {
        let parent = arena[usize::from(state)];
        for tick_u32 in low..=high {
            if parent.prev != ROOT && tick_u32 < u32::from(parent.tick) {
                continue;
            }
            let residual = u64::from(tick_u32.abs_diff(physical));
            let onset_cost = rational_times(residual.saturating_mul(residual), onset_weight);
            let mut notation_cost = rational_times(position_complexity(tick_u32 % grid, grid, policy), notation_weight);
            if parent.prev != ROOT {
                let interval = tick_u32.saturating_sub(u32::from(parent.tick));
                notation_cost = notation_cost.saturating_add(rational_times(
                    interval_complexity(interval, grid, policy),
                    transition_weight,
                ));
                let spread = note
                    .onset_micros
                    .saturating_sub(notes[index.saturating_sub(1)].onset_micros);
                if spread <= window && tick_u32 != u32::from(parent.tick) {
                    notation_cost = notation_cost.saturating_add(group_split);
                }
            }
            let cost = u16::try_from(
                u64::from(parent.cost)
                    .saturating_add(onset_cost)
                    .saturating_add(notation_cost),
            );
            let Ok(cost) = cost else {
                continue;
            };
            let node = Node {
                tick: u16::try_from(tick_u32).unwrap_or(u16::MAX),
                prev: state,
                cost,
            };
            let next_index = u16::try_from(arena.len()).unwrap_or(u16::MAX);
            arena.push(node);
            next_beam.push(next_index);
        }
    }
    next_beam
}

/// The exact abstract storage of an arena and its beam: node records plus two
/// bytes per surviving index, with no allocator metadata.
fn storage(arena: &[Node], beam: &[u16]) -> usize {
    arena
        .len()
        .saturating_mul(Node::bytes())
        .saturating_add(beam.len().saturating_mul(2))
}

/// Sort surviving node indices by cost then exact tick sequence, and truncate.
fn sort_and_truncate(arena: &[Node], mut beam: Vec<u16>, limit: usize) -> Vec<u16> {
    beam.sort_by(|&left, &right| beam_order(arena, left, right));
    beam.truncate(limit);
    beam
}

/// Drop every arena node not reachable from the surviving beam, remapping
/// back-pointers densely so shared prefixes keep sharing.
fn compact(arena: &[Node], beam: &[u16]) -> (Vec<Node>, Vec<u16>) {
    let mut reachable = vec![false; arena.len()];
    let mut stack = beam.to_vec();
    while let Some(node) = stack.pop() {
        let index = usize::from(node);
        if index >= reachable.len() || reachable[index] {
            continue;
        }
        reachable[index] = true;
        if arena[index].prev != ROOT {
            stack.push(arena[index].prev);
        }
    }
    let mut new_arena = Vec::with_capacity(reachable.iter().filter(|yes| **yes).count());
    let mut map = vec![ROOT; arena.len()];
    for (old, node) in arena.iter().enumerate() {
        if !reachable[old] {
            continue;
        }
        let new = u16::try_from(new_arena.len()).unwrap_or(u16::MAX);
        map[old] = new;
        new_arena.push(Node {
            tick: node.tick,
            prev: if node.prev == ROOT {
                ROOT
            } else {
                map[usize::from(node.prev)]
            },
            cost: node.cost,
        });
    }
    let new_beam = beam.iter().map(|&leaf| map[usize::from(leaf)]).collect();
    (new_arena, new_beam)
}

/// Order two surviving beam nodes by cumulative cost, then by their exact tick
/// sequences, the way the trial ordered its cloned candidates.
fn beam_order(arena: &[Node], left: u16, right: u16) -> Ordering {
    arena[usize::from(left)]
        .cost
        .cmp(&arena[usize::from(right)].cost)
        .then_with(|| compare_paths(arena, left, right))
}

fn compare_paths(arena: &[Node], mut left: u16, mut right: u16) -> Ordering {
    let mut left_ticks = Vec::new();
    let mut right_ticks = Vec::new();
    loop {
        let node = arena[usize::from(left)];
        if node.prev == ROOT {
            break;
        }
        left_ticks.push(node.tick);
        left = node.prev;
    }
    loop {
        let node = arena[usize::from(right)];
        if node.prev == ROOT {
            break;
        }
        right_ticks.push(node.tick);
        right = node.prev;
    }
    left_ticks.reverse();
    right_ticks.reverse();
    left_ticks.cmp(&right_ticks)
}

/// Reconstruct one leaf into an exact candidate with its full cost breakdown.
fn materialize(
    take: &Take,
    policy: &TranscriptionPolicy,
    grid: u32,
    quarter_micros: u64,
    arena: &[Node],
    leaf: u16,
) -> Candidate {
    let mut onsets: Vec<u32> = Vec::new();
    let mut cursor = leaf;
    loop {
        let node = arena[usize::from(cursor)];
        if node.prev == ROOT {
            break;
        }
        onsets.push(u32::from(node.tick));
        cursor = node.prev;
    }
    onsets.reverse();

    let notes = take.events.as_slice();
    let duration_quarter = duration_quarter_micros(take);
    let durations = notes
        .iter()
        .map(|note| quantized_duration(note, duration_quarter, grid))
        .collect::<Vec<_>>();
    let (rests, ties, tuplets, syncopation) = notation_shape(&onsets, &durations, take.bar_ticks, grid);
    let cost = cost_record(
        take,
        policy,
        grid,
        quarter_micros,
        &onsets,
        &durations,
        rests.len(),
        ties.len(),
        tuplets.len(),
        syncopation,
    );

    Candidate {
        onsets,
        durations,
        event_ids: notes.iter().map(|note| note.id).collect(),
        cost,
        rests,
        ties,
        tuplets,
    }
}

/// The structural notation facts of one onset/duration sequence: rests, bar
/// crossings, off-grid tuplets, and off-beat syncopations, ported from the
/// trial's `notation_shape`.
fn notation_shape(
    onsets: &[u32],
    durations: &[u32],
    bar_ticks: u32,
    grid: u32,
) -> (Vec<(u32, u32)>, Vec<u32>, Vec<u32>, usize) {
    let mut rests = Vec::new();
    let mut ties = Vec::new();
    let mut tuplets = Vec::new();
    let sixteenth = grid.saturating_div(4).max(1);
    let mut previous_end: Option<u32> = None;
    let mut syncopation = 0_usize;
    for (onset, duration) in onsets.iter().copied().zip(durations.iter().copied()) {
        if let Some(end) = previous_end
            && onset > end
        {
            rests.push((end, onset.saturating_sub(end)));
        }
        let ends = onset.saturating_add(duration).saturating_sub(1);
        if duration > 0 && onset.saturating_div(bar_ticks.max(1)) != ends.saturating_div(bar_ticks.max(1)) {
            ties.push(onset);
        }
        if !onset.is_multiple_of(sixteenth) || !duration.is_multiple_of(sixteenth) {
            tuplets.push(onset);
        }
        if onset % grid != 0 {
            syncopation = syncopation.saturating_add(1);
        }
        previous_end = Some(previous_end.map_or_else(
            || onset.saturating_add(duration),
            |end| end.max(onset.saturating_add(duration)),
        ));
    }
    (rests, ties, tuplets, syncopation)
}

/// Build the complete nine-field cost record of a materialized candidate.
#[allow(clippy::too_many_arguments)]
fn cost_record(
    take: &Take,
    policy: &TranscriptionPolicy,
    grid: u32,
    quarter_micros: u64,
    onsets: &[u32],
    durations: &[u32],
    rest_count: usize,
    tie_count: usize,
    tuplet_count: usize,
    syncopation: usize,
) -> CostRecord {
    let origin = match take.clock {
        TakeClock::Known { origin_micros, .. } => origin_micros,
        TakeClock::Free => take.events.iter().map(|note| note.onset_micros).min().unwrap_or(0),
        TakeClock::Unmeasured => 0,
    };
    let onset_weight = policy.weights().onset_residual();
    let notation_weight = policy.weights().notation_complexity();
    let transition_weight = policy.weights().transition();
    let group_split = ratio_scalar(policy.weights().group_split());
    let window = adaptive_group_window_micros(quarter_micros, policy);

    let mut onset_displacement = 0_u64;
    let mut duration_displacement = 0_u64;
    let mut notation_complexity = 0_u64;
    for (index, note) in take.events.iter().enumerate() {
        let physical = physical_ticks(note.onset_micros.saturating_sub(origin), quarter_micros, grid);
        let onset = onsets[index];
        let residual = u64::from(onset.abs_diff(physical));
        onset_displacement =
            onset_displacement.saturating_add(rational_times(residual.saturating_mul(residual), onset_weight));

        // The written duration this onset implies without a voice assignment:
        // the next onset, or the key-release duration for the last event.
        let written = onsets
            .get(index.saturating_add(1))
            .map_or(durations[index], |next| next.saturating_sub(onset));
        duration_displacement = duration_displacement.saturating_add(u64::from(written.abs_diff(durations[index])));

        notation_complexity = notation_complexity.saturating_add(rational_times(
            position_complexity(onset % grid, grid, policy),
            notation_weight,
        ));
        if let Some(previous_onset) = onsets.get(index.saturating_sub(1)) {
            let interval = onset.saturating_sub(*previous_onset);
            notation_complexity = notation_complexity.saturating_add(rational_times(
                interval_complexity(interval, grid, policy),
                transition_weight,
            ));
            let spread = note
                .onset_micros
                .saturating_sub(take.events[index.saturating_sub(1)].onset_micros);
            if spread <= window && onset != *previous_onset {
                notation_complexity = notation_complexity.saturating_add(group_split);
            }
        }
    }

    let mut values = [0_u64; COST_FIELDS];
    values[0] = onset_displacement;
    values[1] = duration_displacement;
    values[2] = 0;
    values[3] = notation_complexity;
    values[4] = u64::try_from(rest_count).unwrap_or(u64::MAX);
    values[5] = u64::try_from(tie_count).unwrap_or(u64::MAX);
    values[6] = u64::try_from(tuplet_count).unwrap_or(u64::MAX);
    values[7] = u64::try_from(syncopation).unwrap_or(u64::MAX);
    values[8] = 0;
    CostRecord::new(values)
}

/// Order two materialized candidates by ranking total, then the complete cost
/// vector in published field order, then canonical structural bytes.
fn candidate_order(left: &Candidate, right: &Candidate) -> Ordering {
    left.cost
        .rank_total()
        .cmp(&right.cost.rank_total())
        .then_with(|| left.cost.fields().cmp(right.cost.fields()))
        .then_with(|| left.canonical_bytes().cmp(&right.canonical_bytes()))
}

/// Compute structural review regions over the retained candidates: compare each
/// retained candidate to the top candidate and record every local disagreement
/// on phase, grouping, written end, rest, tie, or tuplet.
fn review_regions(candidates: &[Candidate], bar_ticks: u32) -> Vec<ReviewRegion> {
    let Some(first) = candidates.first() else {
        return Vec::new();
    };
    let mut regions = Vec::new();
    for candidate in candidates.iter().skip(1) {
        let first_phase = first.onsets.first().copied().unwrap_or(0) % bar_ticks.max(1);
        let other_phase = candidate.onsets.first().copied().unwrap_or(0) % bar_ticks.max(1);
        if first_phase != other_phase {
            regions.push(ReviewRegion {
                event_index: 0,
                dimension: "phase",
            });
        }
        let length = first.onsets.len().min(candidate.onsets.len());
        for index in 0..length {
            if first.onsets[index] != candidate.onsets[index] {
                regions.push(ReviewRegion {
                    event_index: index,
                    dimension: "grouping",
                });
            }
            if first.durations[index] != candidate.durations[index] {
                regions.push(ReviewRegion {
                    event_index: index,
                    dimension: "end",
                });
            }
            let first_end = first.onsets[index].saturating_add(first.durations[index]);
            let other_end = candidate.onsets[index].saturating_add(candidate.durations[index]);
            if first.rests.iter().any(|(end, _)| *end == first_end)
                != candidate.rests.iter().any(|(end, _)| *end == other_end)
            {
                regions.push(ReviewRegion {
                    event_index: index,
                    dimension: "rest",
                });
            }
            if first.ties.contains(&first.onsets[index]) != candidate.ties.contains(&candidate.onsets[index]) {
                regions.push(ReviewRegion {
                    event_index: index,
                    dimension: "tie",
                });
            }
            if first.tuplets.contains(&first.onsets[index]) != candidate.tuplets.contains(&candidate.onsets[index]) {
                regions.push(ReviewRegion {
                    event_index: index,
                    dimension: "tuplet",
                });
            }
        }
    }
    regions.dedup();
    regions
}

#[cfg(test)]
mod laws {
    #![allow(clippy::arithmetic_side_effects)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::indexing_slicing)]
    #![allow(clippy::panic)]

    use super::*;
    use crate::transcription_policy::standard as checked_policies;
    use crate::transcription_trial::{TrialClock, TrialCorpus};

    const CORPUS: &str = include_str!("../tests/fixtures/transcription/corpus.json");

    fn standard_policy() -> TranscriptionPolicy {
        checked_policies()
            .expect("the bundled policy must check and decode")
            .policy("standard")
            .expect("standard policy exists")
            .clone()
    }

    fn take(clock: TakeClock, ticks: &[u32], quarter_micros: u64) -> Take {
        let events = ticks
            .iter()
            .copied()
            .enumerate()
            .map(|(index, tick)| {
                let onset = u64::from(tick).saturating_mul(quarter_micros) / 24;
                RhythmEvent {
                    id: u32::try_from(index).unwrap_or(u32::MAX),
                    onset_micros: onset,
                    release_micros: onset.saturating_add(quarter_micros / 2),
                    sounding_end_micros: onset.saturating_add(quarter_micros / 2),
                }
            })
            .collect();
        Take {
            clock,
            bar_ticks: 96,
            events,
            pins: Vec::new(),
        }
    }

    #[test]
    fn search_is_deterministic() {
        let policy = standard_policy();
        let input = take(
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            &[0, 12, 24, 48],
            500_000,
        );
        let first = search_take(&input, &policy);
        let second = search_take(&input, &policy);
        assert_eq!(
            first, second,
            "the same take and policy must produce the identical result"
        );
    }

    #[test]
    fn a_known_clock_keeps_its_calibrated_origin_and_never_rebases() {
        let policy = standard_policy();
        // A pickup starting at tick 18 with a transport origin at 0 must not be
        // rebased to its first onset.
        let input = take(
            TakeClock::Known {
                quarter_micros: 520_000,
                origin_micros: 0,
            },
            &[18, 24, 42],
            520_000,
        );
        let SearchOutcome::Ranked { candidates, .. } = search_take(&input, &policy) else {
            panic!("a known-clock pickup take must rank");
        };
        assert_eq!(candidates[0].onsets, vec![18, 24, 42]);
    }

    #[test]
    fn ranked_candidates_are_at_most_five_sorted_and_distinct() {
        let policy = standard_policy();
        let input = take(
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            &[0, 12, 24, 36, 48],
            500_000,
        );
        let SearchOutcome::Ranked { candidates, .. } = search_take(&input, &policy) else {
            panic!("must rank");
        };
        assert!(candidates.len() <= 5);
        for pair in candidates.windows(2) {
            assert!(
                pair[0].cost.rank_total() <= pair[1].cost.rank_total(),
                "candidates must order by weighted total"
            );
        }
        for left in 0..candidates.len() {
            for right in left + 1..candidates.len() {
                assert_ne!(
                    candidates[left].onsets, candidates[right].onsets,
                    "candidates must be distinct"
                );
            }
        }
    }

    #[test]
    fn a_tap_anchor_changes_only_the_constrained_event() {
        let policy = standard_policy();
        let input = take(
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            &[0, 12, 24, 48],
            500_000,
        );
        let unconstrained = match search_take(&input, &policy) {
            SearchOutcome::Ranked { candidates, .. } => candidates,
            SearchOutcome::Refused(reason) => panic!("expected a rank: {reason:?}"),
        };
        let top = unconstrained[0].onsets.clone();
        // Pinning event 2 to the tick it already had changes nothing: matched
        // constraints are locally idempotent, not global perturbations.
        let mut pinned = input;
        pinned.pins = vec![(2, top[2])];
        let SearchOutcome::Ranked { candidates, .. } = search_take(&pinned, &policy) else {
            panic!("a matched pin must rank");
        };
        assert_eq!(
            candidates, unconstrained,
            "a matched pin must reproduce the identical ranking"
        );
    }

    #[test]
    fn unmeasured_scope_refuses_with_write_source() {
        let policy = standard_policy();
        let input = take(TakeClock::Unmeasured, &[0, 24, 48], 500_000);
        assert_eq!(
            search_take(&input, &policy),
            SearchOutcome::Refused(Refusal::WriteSource)
        );
    }

    #[test]
    fn a_proposal_beyond_four_simultaneous_voices_is_refused() {
        let policy = standard_policy();
        // Five onsets the musician pins to one tick need five simultaneous
        // voices; the policy admits four, so every retained candidate is
        // refused rather than silently rewritten.
        let mut input = take(
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            &[0, 24, 48, 72, 96],
            500_000,
        );
        input.pins = (0..5).map(|index| (index, 0_u32)).collect();
        assert_eq!(
            search_take(&input, &policy),
            SearchOutcome::Refused(Refusal::TooManyVoices)
        );
    }

    #[test]
    fn oversized_unsplittable_take_refuses_with_retained_length() {
        let policy = standard_policy();
        let ticks = (0..129)
            .map(|index| u32::try_from(index).unwrap_or(u32::MAX).saturating_mul(12))
            .collect::<Vec<_>>();
        let input = take(
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            &ticks,
            500_000,
        );
        assert_eq!(
            search_take(&input, &policy),
            SearchOutcome::Refused(Refusal::Unsplittable { note_count: 129 })
        );
    }

    #[test]
    fn a_128_note_take_stays_within_the_published_bounds() {
        let policy = standard_policy();
        let ticks = (0..128)
            .map(|index| u32::try_from(index).unwrap_or(u32::MAX).saturating_mul(12))
            .collect::<Vec<_>>();
        let input = take(
            TakeClock::Known {
                quarter_micros: 500_000,
                origin_micros: 0,
            },
            &ticks,
            500_000,
        );
        let (_, measurements) = search_take_measured(&input, &policy);
        assert!(
            measurements.peak_states <= 96,
            "peak states {0} exceeds 96",
            measurements.peak_states
        );
        assert!(
            measurements.peak_storage_bytes < 131_072,
            "storage {0} exceeds 128 KiB",
            measurements.peak_storage_bytes
        );
    }

    /// The prompt 203 corpus admission thresholds, computed through the private
    /// optimizer rather than the trial's cloned-vector search: at least 54/58
    /// exact onsets, 8/9 top-five recall, 47/58 key-duration matches, and at
    /// most 23 source-token edits.
    #[test]
    fn corpus_thresholds_hold_through_the_optimizer() {
        let policy = standard_policy();
        let body = TrialCorpus::read(CORPUS).expect("decode the checked-in corpus");
        let mut onset_correct = 0_usize;
        let mut onset_total = 0_usize;
        let mut top_k_recall = 0_usize;
        let mut top_k_total = 0_usize;
        let mut duration_correct = 0_usize;
        let mut duration_total = 0_usize;
        let mut edits = 0_usize;

        for fixture in body.fixtures() {
            let clock = match fixture.clock() {
                TrialClock::Known {
                    quarter_micros,
                    origin_micros,
                } => TakeClock::Known {
                    quarter_micros,
                    origin_micros,
                },
                TrialClock::Free => TakeClock::Free,
                TrialClock::Unmeasured => continue,
            };
            let expected = fixture
                .notes()
                .iter()
                .map(|note| note.rhythm_parts())
                .filter_map(|(id, onset, release, sounding, expected)| {
                    expected.map(|e| (id, onset, release, sounding, e))
                })
                .collect::<Vec<_>>();
            let events = expected
                .iter()
                .map(|(id, onset, release, sounding, _)| RhythmEvent {
                    id: *id,
                    onset_micros: *onset,
                    release_micros: *release,
                    sounding_end_micros: *sounding,
                })
                .collect::<Vec<_>>();
            let input = Take {
                clock,
                bar_ticks: 96,
                events,
                pins: Vec::new(),
            };
            let SearchOutcome::Ranked { candidates, .. } = search_take(&input, &policy) else {
                panic!("{} must rank", fixture.id());
            };
            let Some(best) = candidates.first() else {
                continue;
            };
            let intended_onsets = expected.iter().map(|(_, _, _, _, e)| e.0).collect::<Vec<_>>();
            top_k_total = top_k_total.saturating_add(1);
            if candidates.iter().any(|candidate| candidate.onsets == intended_onsets) {
                top_k_recall = top_k_recall.saturating_add(1);
            }
            for ((_, _, _, _, expected), onset) in expected.iter().zip(&best.onsets) {
                onset_total = onset_total.saturating_add(1);
                if expected.0 == *onset {
                    onset_correct = onset_correct.saturating_add(1);
                }
            }
            let intended_durations = expected.iter().map(|(_, _, _, _, e)| e.1).collect::<Vec<_>>();
            for ((_, _, _, _, expected), duration) in expected.iter().zip(&best.durations) {
                duration_total = duration_total.saturating_add(1);
                if expected.1 == *duration {
                    duration_correct = duration_correct.saturating_add(1);
                }
            }
            edits = edits.saturating_add(token_edit_distance(
                &tokens(&intended_onsets, &intended_durations),
                &tokens(&best.onsets, &best.durations),
            ));
        }

        assert!(
            onset_total >= 58,
            "the corpus must retain at least 58 expected onsets, saw {onset_total}"
        );
        assert!(
            onset_correct >= 54,
            "exact onsets regressed: {onset_correct}/{onset_total}"
        );
        assert_eq!(top_k_total, 9, "nine fixtures make a metrical claim");
        assert!(
            top_k_recall >= 8,
            "top-five recall regressed: {top_k_recall}/{top_k_total}"
        );
        assert!(
            duration_correct >= 47,
            "key-duration matches regressed: {duration_correct}/{duration_total}"
        );
        assert!(edits <= 23, "source-token edits regressed: {edits}");
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

    fn tokens(onsets: &[u32], durations: &[u32]) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut previous_end: Option<u32> = None;
        for (onset, duration) in onsets.iter().copied().zip(durations.iter().copied()) {
            if let Some(end) = previous_end
                && onset > end
            {
                tokens.push(format!("rest@{end}/{}", onset.saturating_sub(end)));
            }
            tokens.push(format!("note@{onset}/{duration}"));
            previous_end = Some(previous_end.map_or_else(
                || onset.saturating_add(duration),
                |end| end.max(onset.saturating_add(duration)),
            ));
        }
        tokens
    }
}
