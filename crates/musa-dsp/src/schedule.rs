//! Checked conversion from a finite event track to a frame source.
//!
//! Scheduling is preparation, not callback work: exact rational conversion,
//! validation, allocation, and sorting all finish here. [`ScheduledSource`]
//! subsequently advances with a table cursor and a countdown only.
#![allow(clippy::arithmetic_side_effects)]

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use musa_events::{Canonical, Coordinate, EventTrack, PhysicalTime, Position};
use num_rational::Ratio;

/// The physical format whose frame lattice receives exact seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioFormat {
    sample_rate: NonZeroU32,
    layout: ChannelLayout,
}

/// Native sample-frame channel layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelLayout {
    /// One sample per frame.
    Mono,
    /// Left and right samples per frame.
    Stereo,
}

impl AudioFormat {
    /// A format at the stated nonzero frames per second and explicit layout.
    pub const fn new(sample_rate: NonZeroU32, layout: ChannelLayout) -> Self {
        Self { sample_rate, layout }
    }

    /// Frames per physical second.
    pub const fn sample_rate(self) -> NonZeroU32 {
        self.sample_rate
    }

    /// Samples carried by one physical frame.
    pub const fn layout(self) -> ChannelLayout {
        self.layout
    }
}

/// Exact answers for the finite source-boundary questions a schedule asks.
///
/// Construction intentionally retains repeated keys. A conflicting repeated
/// answer is a scheduling error with the source boundary in its diagnostic;
/// silently letting a map container choose one would make insertion order
/// semantic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeMap<C: Coordinate> {
    version: u32,
    assignments: Vec<(Position<C>, Position<PhysicalTime>)>,
}

impl<C: Coordinate> TimeMap<C> {
    /// A versioned finite exact boundary map.
    pub fn new(version: u32, assignments: impl IntoIterator<Item = (Position<C>, Position<PhysicalTime>)>) -> Self {
        Self {
            version,
            assignments: assignments.into_iter().collect(),
        }
    }

    /// Identity version recorded beside each scheduling decision.
    pub const fn version(&self) -> u32 {
        self.version
    }
}

/// Exact-to-frame rounding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameRounding {
    /// Greatest frame no later than the exact result.
    Floor,
    /// Least frame no earlier than the exact result.
    Ceil,
    /// Nearest frame, with exact half-frame ties going later.
    NearestTiesLater,
}

/// Where the chosen frame lies relative to the exact rational frame value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingChoice {
    /// The physical result was already an integer frame.
    Exact,
    /// The chosen frame is earlier than the physical result.
    Earlier,
    /// The chosen frame is later than the physical result.
    Later,
}

/// What to do when a positive source span's boundaries choose one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollapsePolicy {
    /// Refuse the schedule.
    Reject,
    /// Emit both boundaries in the one batch, in the policy's message order.
    Ordered,
    /// Move only this occurrence's end far enough to provide this many frames.
    Expand { minimum_frames: NonZeroU32 },
}

/// A message class used to state same-frame order without inspecting handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageKind {
    /// End a positive occurrence.
    End,
    /// Observe a point occurrence.
    Point,
    /// Begin a positive occurrence.
    Begin,
}

/// Explicit preparation bounds. They make both work and retained memory a
/// checked property rather than a consequence of allocator exhaustion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduleLimits {
    /// Greatest accepted frame number.
    pub max_frame: u64,
    /// Greatest accepted number of entries in the supplied finite time map.
    pub max_time_map_entries: usize,
    /// Greatest accepted number of occurrences.
    pub max_occurrences: usize,
    /// Greatest accepted number of boundary messages.
    pub max_messages: usize,
    /// Greatest accepted number of nonempty frame batches.
    pub max_batches: usize,
}

/// Every choice that turns exact physical positions into ordered batches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchedulePolicy {
    version: u32,
    rounding: FrameRounding,
    collapse: CollapsePolicy,
    order: [MessageKind; 3],
    limits: ScheduleLimits,
}

impl SchedulePolicy {
    /// Build a policy, rejecting an incomplete or repeated message order.
    ///
    /// # Errors
    /// [`ScheduleError::InvalidMessageOrder`] when a kind repeats (and hence
    /// another is absent).
    pub fn new(
        version: u32,
        rounding: FrameRounding,
        collapse: CollapsePolicy,
        order: [MessageKind; 3],
        limits: ScheduleLimits,
    ) -> Result<Self, ScheduleError> {
        let mut seen = 0u8;
        for kind in order {
            let bit = 1u8 << kind as u8;
            if seen & bit != 0 {
                return Err(ScheduleError::InvalidMessageOrder);
            }
            seen |= bit;
        }
        Ok(Self {
            version,
            rounding,
            collapse,
            order,
            limits,
        })
    }

    /// Policy identity recorded in every decision.
    pub const fn version(self) -> u32 {
        self.version
    }

    /// Finite table and per-frame-message bounds carried by this policy.
    pub const fn limits(self) -> ScheduleLimits {
        self.limits
    }

    fn rank(self, kind: MessageKind) -> usize {
        self.order.iter().position(|candidate| *candidate == kind).unwrap_or(3)
    }
}

/// Which source boundary a decision describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryKind {
    /// Start of a positive occurrence.
    Start,
    /// End of a positive occurrence.
    End,
    /// The sole boundary of a point occurrence.
    Point,
    /// The event track's ambient end, including silence after its last event.
    TrackEnd,
}

/// Why a chosen frame differs from the ordinary two-boundary case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryCollision {
    /// No collapse occurred.
    None,
    /// The source occurrence itself is a point.
    Point,
    /// A positive occurrence was retained as an ordered same-frame pair.
    Ordered,
    /// A positive occurrence's end was expanded by the stated frame count.
    Expanded { minimum_frames: NonZeroU32 },
}

/// One auditable exact boundary conversion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeDecision<C: Coordinate> {
    /// Canonical occurrence ordinal; `None` denotes the ambient track end.
    /// Exact copies have distinct ordinals but equal timing decisions.
    pub occurrence: Option<usize>,
    /// Which boundary this is.
    pub boundary: BoundaryKind,
    /// Exact source position.
    pub source: Position<C>,
    /// Exact physical position returned by the map.
    pub physical: Position<PhysicalTime>,
    /// Chosen bounded frame.
    pub frame: u64,
    /// Exact rounding operation used.
    pub rounding_rule: FrameRounding,
    /// The direction actually chosen for this value.
    pub rounding_choice: RoundingChoice,
    /// Point/collapse treatment.
    pub collision: BoundaryCollision,
    /// Time-map version.
    pub time_map_version: u32,
    /// Schedule-policy version.
    pub policy_version: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum HandleSide {
    Left,
    Right,
}

/// Private occurrence identity. Its representation is deliberately absent
/// from the public API; consumers can clone, hash, and compare it only.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EventHandle {
    namespace: Arc<[HandleSide]>,
    ordinal: usize,
}

impl std::fmt::Debug for EventHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("EventHandle(_)")
    }
}

impl EventHandle {
    pub(crate) fn root(ordinal: usize) -> Self {
        Self {
            namespace: Arc::from([]),
            ordinal,
        }
    }

    fn inject(&self, side: HandleSide) -> Self {
        let mut namespace = Vec::with_capacity(self.namespace.len().saturating_add(1));
        namespace.push(side);
        namespace.extend_from_slice(&self.namespace);
        Self {
            namespace: namespace.into(),
            ordinal: self.ordinal,
        }
    }
}

/// One occurrence boundary delivered to an instrument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventMessage<A> {
    /// Begin a positive occurrence.
    Begin(EventHandle, A),
    /// End the occurrence paired by this handle.
    End(EventHandle),
    /// Observe a point occurrence once.
    Point(EventHandle, A),
}

impl<A> EventMessage<A> {
    /// Its policy-order class.
    pub const fn kind(&self) -> MessageKind {
        match self {
            Self::Begin(_, _) => MessageKind::Begin,
            Self::End(_) => MessageKind::End,
            Self::Point(_, _) => MessageKind::Point,
        }
    }

    /// The opaque occurrence handle.
    pub const fn handle(&self) -> &EventHandle {
        match self {
            Self::Begin(handle, _) | Self::End(handle) | Self::Point(handle, _) => handle,
        }
    }
}

/// An immutable ordered batch for one audio frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventBatch<A> {
    messages: Box<[EventMessage<A>]>,
}

impl<A> EventBatch<A> {
    /// Messages in the policy's fixed order.
    pub fn messages(&self) -> &[EventMessage<A>] {
        &self.messages
    }

    /// Whether the batch has no messages.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FrameBatch<A> {
    frame: u64,
    batch: EventBatch<A>,
}

/// A checked finite schedule and its complete decision record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schedule<C: Coordinate, A> {
    table: Arc<[FrameBatch<A>]>,
    finish_frame: u64,
    decisions: Box<[TimeDecision<C>]>,
    policy: SchedulePolicy,
}

impl<C: Coordinate, A> Schedule<C, A> {
    /// Exact decisions in canonical-occurrence, then boundary order.
    pub fn decisions(&self) -> &[TimeDecision<C>] {
        &self.decisions
    }

    /// Nonempty batches in ascending frame order.
    pub fn batches(&self) -> impl ExactSizeIterator<Item = (u64, &EventBatch<A>)> {
        self.table.iter().map(|entry| (entry.frame, &entry.batch))
    }

    /// The frame at which this scheduled source is complete.
    ///
    /// This is the later of the chosen ambient end and any occurrence end
    /// moved by an explicit collapse-expansion policy. It can lie after the
    /// final nonempty batch when the track has trailing silence, and is zero
    /// for an unexpanded zero-duration track.
    pub const fn finish_frame(&self) -> u64 {
        self.finish_frame
    }

    /// A source at frame zero over this immutable table.
    pub fn source(&self) -> ScheduledSource<A> {
        ScheduledSource::new(Arc::clone(&self.table), self.finish_frame, 0)
    }

    /// The policy version under which the table was produced.
    pub const fn policy_version(&self) -> u32 {
        self.policy.version
    }
}

/// Observable source phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceState {
    /// More scheduled batches remain.
    Running,
    /// The ambient end has been reached and the final batch has been emitted.
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cursor {
    Waiting { index: usize, remaining: u64 },
    WaitingForFinish { remaining: u64 },
    Finished,
}

/// A fixed-memory source machine. It owns no growing frame counter.
#[derive(Clone, Debug)]
pub struct ScheduledSource<A> {
    table: Arc<[FrameBatch<A>]>,
    finish_frame: u64,
    cursor: Cursor,
}

impl<A> ScheduledSource<A> {
    fn new(table: Arc<[FrameBatch<A>]>, finish_frame: u64, frame: u64) -> Self {
        let mut source = Self {
            table,
            finish_frame,
            cursor: Cursor::Finished,
        };
        source.seek(frame);
        source
    }

    /// Move to an absolute frame by binary search. The next step reads that
    /// frame's batch; past the table is the fixed finished state.
    pub fn seek(&mut self, frame: u64) {
        if frame > self.finish_frame {
            self.cursor = Cursor::Finished;
            return;
        }
        let index = self.table.partition_point(|entry| entry.frame < frame);
        self.cursor = self.table.get(index).map_or_else(
            || Cursor::WaitingForFinish {
                remaining: self.finish_frame.saturating_sub(frame),
            },
            |entry| Cursor::Waiting {
                index,
                remaining: entry.frame.saturating_sub(frame),
            },
        );
    }

    /// Read one frame's batch and advance exactly one audio-frame step.
    /// Allocation-free and logarithm-free after preparation.
    pub fn step(&mut self) -> (SourceState, &[EventMessage<A>]) {
        match self.cursor {
            Cursor::Finished => (SourceState::Finished, &[]),
            Cursor::WaitingForFinish { remaining } if remaining > 0 => {
                self.cursor = Cursor::WaitingForFinish {
                    remaining: remaining.saturating_sub(1),
                };
                (SourceState::Running, &[])
            }
            Cursor::WaitingForFinish { .. } => {
                self.cursor = Cursor::Finished;
                (SourceState::Finished, &[])
            }
            Cursor::Waiting { index, remaining } if remaining > 0 => {
                self.cursor = Cursor::Waiting {
                    index,
                    remaining: remaining.saturating_sub(1),
                };
                (SourceState::Running, &[])
            }
            Cursor::Waiting { index, .. } => {
                let Some(entry) = self.table.get(index) else {
                    self.cursor = Cursor::Finished;
                    return (SourceState::Finished, &[]);
                };
                let messages = entry.batch.messages();
                self.cursor = self.table.get(index.saturating_add(1)).map_or_else(
                    || {
                        if entry.frame >= self.finish_frame {
                            Cursor::Finished
                        } else {
                            Cursor::WaitingForFinish {
                                remaining: self.finish_frame.saturating_sub(entry.frame).saturating_sub(1),
                            }
                        }
                    },
                    |next| Cursor::Waiting {
                        index: index.saturating_add(1),
                        remaining: next.frame.saturating_sub(entry.frame).saturating_sub(1),
                    },
                );
                let state = if matches!(self.cursor, Cursor::Finished) {
                    SourceState::Finished
                } else {
                    SourceState::Running
                };
                (state, messages)
            }
        }
    }
}

/// A checked scheduling failure. Times are retained in exact reduced form in
/// every timing diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ScheduleError {
    /// Same source boundary received two exact answers.
    #[error("source boundary {boundary} maps to both exact physical times {first} and {second}")]
    ConflictingTimeMap {
        /// Exact source boundary.
        boundary: String,
        /// First answer.
        first: String,
        /// Conflicting answer.
        second: String,
    },
    /// The finite map omitted a queried boundary.
    #[error("time map has no exact physical answer for source boundary {boundary}")]
    MissingBoundary { boundary: String },
    /// A source boundary mapped before physical zero.
    #[error("source boundary {boundary} maps to negative physical time {physical}")]
    NegativePhysical { boundary: String, physical: String },
    /// Exact or rounded order reversed.
    #[error(
        "source boundaries {earlier_source} <= {later_source}, but assignments reverse: {earlier_value} > {later_value}"
    )]
    NonMonotone {
        earlier_source: String,
        later_source: String,
        earlier_value: String,
        later_value: String,
    },
    /// Frame arithmetic exceeded the policy's stated range.
    #[error(
        "source boundary {boundary} at exact physical time {physical} chooses frame {frame}, beyond policy maximum {maximum}"
    )]
    FrameOutOfRange {
        boundary: String,
        physical: String,
        frame: String,
        maximum: u64,
    },
    /// A positive occurrence collapsed under a rejecting policy.
    #[error(
        "positive occurrence [{start}, {end}) maps to the one frame {frame}; policy version {policy_version} rejects collapse"
    )]
    CollapseRejected {
        start: String,
        end: String,
        frame: u64,
        policy_version: u32,
    },
    /// The schedule exceeded one explicit preparation bound.
    #[error("schedule {resource} count {actual} exceeds explicit limit {limit}")]
    ResourceLimit {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    /// Same-frame order did not name each kind once.
    #[error("same-frame order must contain Begin, End, and Point exactly once")]
    InvalidMessageOrder,
    /// Two schedules were prepared under a different policy than their merge.
    #[error("batch merge policy {merge:?} differs from prepared source policy {prepared:?}")]
    MergePolicyMismatch {
        /// Policy requested for the merged result.
        merge: SchedulePolicy,
        /// Policy that prepared one input table.
        prepared: SchedulePolicy,
    },
}

/// Check and schedule an already-admitted finite event track.
///
/// `A: Canonical` supplies only the deterministic finite ordering key. It is
/// not a Rust replacement for language `Storable`: source callers reach this
/// function only after the compiler has discharged that generated constraint
/// and the kernel has rechecked the term.
///
/// # Errors
/// [`ScheduleError`] identifies a missing, conflicting, negative,
/// nonmonotone, unrepresentable, collapsed, or resource-exceeding assignment.
pub fn schedule<C, A>(
    format: AudioFormat,
    policy: SchedulePolicy,
    time_map: &TimeMap<C>,
    track: &EventTrack<C, A>,
) -> Result<Schedule<C, A>, ScheduleError>
where
    C: Coordinate,
    A: Canonical + Clone,
{
    check_limit("occurrence", track.occurrences().len(), policy.limits.max_occurrences)?;
    let ordered_boundaries = boundary_set(track);
    check_limit(
        "time-map entry",
        time_map.assignments.len(),
        policy.limits.max_time_map_entries,
    )?;
    let answers = checked_answers(time_map, &ordered_boundaries)?;
    let mut converted = BTreeMap::new();
    let mut previous: Option<(Position<C>, Position<PhysicalTime>, u64)> = None;
    for source in ordered_boundaries {
        let physical = answers
            .get(&source)
            .copied()
            .ok_or_else(|| ScheduleError::MissingBoundary {
                boundary: source.to_string(),
            })?;
        if physical.as_ratio() < Ratio::ZERO {
            return Err(ScheduleError::NegativePhysical {
                boundary: source.to_string(),
                physical: physical.to_string(),
            });
        }
        let (frame, rounding_choice) = choose_frame(format, policy, source, physical)?;
        if let Some((earlier_source, earlier_physical, earlier_frame)) = previous {
            if earlier_physical > physical {
                return Err(ScheduleError::NonMonotone {
                    earlier_source: earlier_source.to_string(),
                    later_source: source.to_string(),
                    earlier_value: earlier_physical.to_string(),
                    later_value: physical.to_string(),
                });
            }
            if earlier_frame > frame {
                return Err(ScheduleError::NonMonotone {
                    earlier_source: earlier_source.to_string(),
                    later_source: source.to_string(),
                    earlier_value: format!("frame {earlier_frame}"),
                    later_value: format!("frame {frame}"),
                });
            }
        }
        converted.insert(source, (physical, frame, rounding_choice));
        previous = Some((source, physical, frame));
    }

    let occurrences = track.canonical_occurrences();
    let message_count = occurrences
        .iter()
        .map(|occurrence| usize::from(occurrence.span().start() != occurrence.span().end()).saturating_add(1))
        .try_fold(0usize, usize::checked_add)
        .unwrap_or(usize::MAX);
    check_limit("boundary message", message_count, policy.limits.max_messages)?;
    let mut by_frame: BTreeMap<u64, Vec<EventMessage<A>>> = BTreeMap::new();
    let mut decisions = Vec::with_capacity(message_count.saturating_add(1));
    for (ordinal, occurrence) in occurrences.into_iter().enumerate() {
        let span = occurrence.span();
        let handle = EventHandle::root(ordinal);
        let (start_physical, start_frame, start_choice) =
            converted
                .get(&span.start())
                .copied()
                .ok_or_else(|| ScheduleError::MissingBoundary {
                    boundary: span.start().to_string(),
                })?;
        if span.start() == span.end() {
            by_frame
                .entry(start_frame)
                .or_default()
                .push(EventMessage::Point(handle, occurrence.payload().clone()));
            decisions.push(decision(
                ordinal,
                BoundaryKind::Point,
                span.start(),
                start_physical,
                start_frame,
                start_choice,
                BoundaryCollision::Point,
                policy,
                time_map,
            ));
            continue;
        }
        let (end_physical, raw_end_frame, end_choice) =
            converted
                .get(&span.end())
                .copied()
                .ok_or_else(|| ScheduleError::MissingBoundary {
                    boundary: span.end().to_string(),
                })?;
        let (end_frame, collision) = if start_frame == raw_end_frame {
            match policy.collapse {
                CollapsePolicy::Reject => {
                    return Err(ScheduleError::CollapseRejected {
                        start: span.start().to_string(),
                        end: span.end().to_string(),
                        frame: start_frame,
                        policy_version: policy.version,
                    });
                }
                CollapsePolicy::Ordered => (raw_end_frame, BoundaryCollision::Ordered),
                CollapsePolicy::Expand { minimum_frames } => {
                    let expanded = start_frame
                        .checked_add(u64::from(minimum_frames.get()))
                        .filter(|frame| *frame <= policy.limits.max_frame)
                        .ok_or_else(|| ScheduleError::FrameOutOfRange {
                            boundary: span.end().to_string(),
                            physical: end_physical.to_string(),
                            frame: "overflow during collapse expansion".to_owned(),
                            maximum: policy.limits.max_frame,
                        })?;
                    (expanded, BoundaryCollision::Expanded { minimum_frames })
                }
            }
        } else {
            (raw_end_frame, BoundaryCollision::None)
        };
        by_frame
            .entry(start_frame)
            .or_default()
            .push(EventMessage::Begin(handle.clone(), occurrence.payload().clone()));
        by_frame.entry(end_frame).or_default().push(EventMessage::End(handle));
        decisions.push(decision(
            ordinal,
            BoundaryKind::Start,
            span.start(),
            start_physical,
            start_frame,
            start_choice,
            collision,
            policy,
            time_map,
        ));
        decisions.push(decision(
            ordinal,
            BoundaryKind::End,
            span.end(),
            end_physical,
            end_frame,
            end_choice,
            collision,
            policy,
            time_map,
        ));
    }
    let track_end = track.duration().reach();
    let (end_physical, raw_finish_frame, end_choice) =
        converted
            .get(&track_end)
            .copied()
            .ok_or_else(|| ScheduleError::MissingBoundary {
                boundary: track_end.to_string(),
            })?;
    decisions.push(TimeDecision {
        occurrence: None,
        boundary: BoundaryKind::TrackEnd,
        source: track_end,
        physical: end_physical,
        frame: raw_finish_frame,
        rounding_rule: policy.rounding,
        rounding_choice: end_choice,
        collision: BoundaryCollision::None,
        time_map_version: time_map.version,
        policy_version: policy.version,
    });
    check_decision_order(&decisions)?;
    check_limit("frame batch", by_frame.len(), policy.limits.max_batches)?;
    let table = finish_table(by_frame, policy);
    let finish_frame = table
        .last()
        .map_or(raw_finish_frame, |entry| entry.frame.max(raw_finish_frame));
    Ok(Schedule {
        table: table.into(),
        finish_frame,
        decisions: decisions.into(),
        policy,
    })
}

/// Hygienically merge two prepared sources, injecting their handles into
/// disjoint namespaces before applying the stated ordering policy.
///
/// # Errors
/// [`ScheduleError::MergePolicyMismatch`] for any policy mismatch, or
/// [`ScheduleError::ResourceLimit`] when the merged finite table exceeds an
/// explicit policy bound.
pub fn merge_schedules<C, A>(
    policy: SchedulePolicy,
    left: &Schedule<C, A>,
    right: &Schedule<C, A>,
) -> Result<Schedule<C, A>, ScheduleError>
where
    C: Coordinate,
    A: Canonical + Clone,
{
    for source in [left, right] {
        if source.policy != policy {
            return Err(ScheduleError::MergePolicyMismatch {
                merge: policy,
                prepared: source.policy,
            });
        }
    }
    let message_count = left
        .table
        .iter()
        .chain(right.table.iter())
        .map(|entry| entry.batch.messages.len())
        .try_fold(0usize, usize::checked_add)
        .unwrap_or(usize::MAX);
    check_limit("boundary message", message_count, policy.limits.max_messages)?;
    let mut by_frame = BTreeMap::<u64, Vec<EventMessage<A>>>::new();
    for (schedule, side) in [(left, HandleSide::Left), (right, HandleSide::Right)] {
        for entry in schedule.table.iter() {
            let target = by_frame.entry(entry.frame).or_default();
            target.extend(
                entry
                    .batch
                    .messages
                    .iter()
                    .cloned()
                    .map(|message| inject(message, side)),
            );
        }
    }
    check_limit("frame batch", by_frame.len(), policy.limits.max_batches)?;
    let mut decisions = Vec::with_capacity(left.decisions.len().saturating_add(right.decisions.len()));
    decisions.extend_from_slice(&left.decisions);
    decisions.extend_from_slice(&right.decisions);
    Ok(Schedule {
        table: finish_table(by_frame, policy).into(),
        finish_frame: left.finish_frame.max(right.finish_frame),
        decisions: decisions.into(),
        policy,
    })
}

fn inject<A>(message: EventMessage<A>, side: HandleSide) -> EventMessage<A> {
    match message {
        EventMessage::Begin(handle, payload) => EventMessage::Begin(handle.inject(side), payload),
        EventMessage::End(handle) => EventMessage::End(handle.inject(side)),
        EventMessage::Point(handle, payload) => EventMessage::Point(handle.inject(side), payload),
    }
}

fn finish_table<A: Canonical>(
    by_frame: BTreeMap<u64, Vec<EventMessage<A>>>,
    policy: SchedulePolicy,
) -> Vec<FrameBatch<A>> {
    by_frame
        .into_iter()
        .map(|(frame, mut messages)| {
            messages.sort_by_cached_key(|message| {
                let payload = match message {
                    EventMessage::Begin(_, payload) | EventMessage::Point(_, payload) => payload.canonical_key(),
                    EventMessage::End(_) => String::new(),
                };
                (policy.rank(message.kind()), payload)
            });
            FrameBatch {
                frame,
                batch: EventBatch {
                    messages: messages.into(),
                },
            }
        })
        .collect()
}

fn checked_answers<C: Coordinate>(
    map: &TimeMap<C>,
    queried: &[Position<C>],
) -> Result<BTreeMap<Position<C>, Position<PhysicalTime>>, ScheduleError> {
    let mut answers = BTreeMap::new();
    for (source, physical) in &map.assignments {
        if queried.binary_search(source).is_err() {
            continue;
        }
        if let Some(first) = answers.insert(*source, *physical)
            && first != *physical
        {
            return Err(ScheduleError::ConflictingTimeMap {
                boundary: source.to_string(),
                first: first.to_string(),
                second: physical.to_string(),
            });
        }
    }
    Ok(answers)
}

fn boundary_set<C: Coordinate, A>(track: &EventTrack<C, A>) -> Vec<Position<C>> {
    let mut boundaries = Vec::with_capacity(track.occurrences().len().saturating_mul(2).saturating_add(1));
    for occurrence in track.occurrences() {
        boundaries.push(occurrence.span().start());
        boundaries.push(occurrence.span().end());
    }
    boundaries.push(track.duration().reach());
    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries
}

fn choose_frame<C: Coordinate>(
    format: AudioFormat,
    policy: SchedulePolicy,
    source: Position<C>,
    physical: Position<PhysicalTime>,
) -> Result<(u64, RoundingChoice), ScheduleError> {
    let exact = physical.as_ratio();
    let numerator = i128::from(*exact.numer()) * i128::from(format.sample_rate.get());
    let denominator = i128::from(*exact.denom());
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let chosen = match policy.rounding {
        FrameRounding::Floor => quotient,
        FrameRounding::Ceil => quotient + i128::from(remainder != 0),
        FrameRounding::NearestTiesLater => quotient + i128::from(remainder.saturating_mul(2) >= denominator),
    };
    let frame = u64::try_from(chosen)
        .ok()
        .filter(|frame| *frame <= policy.limits.max_frame)
        .ok_or_else(|| ScheduleError::FrameOutOfRange {
            boundary: source.to_string(),
            physical: physical.to_string(),
            frame: chosen.to_string(),
            maximum: policy.limits.max_frame,
        })?;
    let choice = if remainder == 0 {
        RoundingChoice::Exact
    } else if chosen == quotient {
        RoundingChoice::Earlier
    } else {
        RoundingChoice::Later
    };
    Ok((frame, choice))
}

#[allow(clippy::too_many_arguments)]
fn decision<C: Coordinate>(
    occurrence: usize,
    boundary: BoundaryKind,
    source: Position<C>,
    physical: Position<PhysicalTime>,
    frame: u64,
    rounding_choice: RoundingChoice,
    collision: BoundaryCollision,
    policy: SchedulePolicy,
    time_map: &TimeMap<C>,
) -> TimeDecision<C> {
    TimeDecision {
        occurrence: Some(occurrence),
        boundary,
        source,
        physical,
        frame,
        rounding_rule: policy.rounding,
        rounding_choice,
        collision,
        time_map_version: time_map.version,
        policy_version: policy.version,
    }
}

fn check_decision_order<C: Coordinate>(decisions: &[TimeDecision<C>]) -> Result<(), ScheduleError> {
    let mut assigned = BTreeMap::<Position<C>, u64>::new();
    for decision in decisions
        .iter()
        .filter(|decision| decision.boundary != BoundaryKind::TrackEnd)
    {
        if let Some(previous) = assigned.insert(decision.source, decision.frame)
            && previous != decision.frame
        {
            return Err(ScheduleError::NonMonotone {
                earlier_source: decision.source.to_string(),
                later_source: decision.source.to_string(),
                earlier_value: format!("frame {previous}"),
                later_value: format!("frame {}", decision.frame),
            });
        }
    }
    let ordered: Vec<_> = assigned.into_iter().collect();
    for pair in ordered.windows(2) {
        let [(earlier_source, earlier_frame), (later_source, later_frame)] = pair else {
            continue;
        };
        if earlier_frame > later_frame {
            return Err(ScheduleError::NonMonotone {
                earlier_source: earlier_source.to_string(),
                later_source: later_source.to_string(),
                earlier_value: format!("frame {earlier_frame}"),
                later_value: format!("frame {later_frame}"),
            });
        }
    }
    Ok(())
}

fn check_limit(resource: &'static str, actual: usize, limit: usize) -> Result<(), ScheduleError> {
    if actual > limit {
        Err(ScheduleError::ResourceLimit {
            resource,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}
