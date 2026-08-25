//! Laws for checked event-track scheduling (`03-machine-calculus.md` §6).
#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::num::NonZeroU32;

use musa_dsp::{
    AudioFormat, BoundaryCollision, CollapsePolicy, EventMessage, FrameRounding, MessageKind, RoundingChoice, Schedule,
    ScheduleError, ScheduleLimits, SchedulePolicy, SourceState, TimeMap, merge_schedules, schedule,
};
use musa_events::{
    Duration, EventTrack, Occurrence, PhysicalTime, Position, Span, WrittenTime, follow, together, track,
};
use num_rational::Ratio;

fn position<C>(numerator: i64, denominator: i64) -> Position<C> {
    Position::new(Ratio::new(numerator, denominator))
}

fn span(start: i64, end: i64) -> Span<WrittenTime> {
    Span::new(position(start, 1), position(end, 1)).expect("valid test span")
}

fn track_of(duration: i64, occurrences: &[(i64, i64, u8)]) -> EventTrack<WrittenTime, u8> {
    track(
        Duration::from_integer(duration).expect("nonnegative duration"),
        occurrences
            .iter()
            .map(|(start, end, payload)| Occurrence::new(span(*start, *end), *payload))
            .collect(),
    )
    .expect("bounded track")
}

fn map(points: &[(i64, i64)]) -> TimeMap<WrittenTime> {
    TimeMap::new(
        7,
        points
            .iter()
            .map(|(source, physical)| (position(*source, 1), position::<PhysicalTime>(*physical, 10))),
    )
}

fn limits() -> ScheduleLimits {
    ScheduleLimits {
        max_frame: 10_000,
        max_time_map_entries: 200,
        max_occurrences: 100,
        max_messages: 200,
        max_batches: 200,
    }
}

fn policy(rounding: FrameRounding, collapse: CollapsePolicy) -> SchedulePolicy {
    SchedulePolicy::new(
        11,
        rounding,
        collapse,
        [MessageKind::End, MessageKind::Point, MessageKind::Begin],
        limits(),
    )
    .expect("complete order")
}

fn format() -> AudioFormat {
    AudioFormat::new(NonZeroU32::new(10).expect("nonzero"))
}

fn messages<C: musa_events::Coordinate>(schedule: &Schedule<C, u8>) -> Vec<(u64, MessageKind, u8)> {
    schedule
        .batches()
        .flat_map(|(frame, batch)| {
            batch.messages().iter().map(move |message| {
                let payload = match message {
                    EventMessage::Begin(_, payload) | EventMessage::Point(_, payload) => *payload,
                    EventMessage::End(_) => 0,
                };
                (frame, message.kind(), payload)
            })
        })
        .collect()
}

#[test]
fn every_positive_boundary_is_emitted_once_with_half_open_order() {
    let scheduled = schedule(
        format(),
        policy(FrameRounding::Floor, CollapsePolicy::Ordered),
        &map(&[(0, 0), (1, 1), (2, 2)]),
        &track_of(2, &[(0, 1, 1), (1, 2, 2)]),
    )
    .expect("schedule succeeds");
    assert_eq!(
        messages(&scheduled),
        vec![
            (0, MessageKind::Begin, 1),
            (1, MessageKind::End, 0),
            (1, MessageKind::Begin, 2),
            (2, MessageKind::End, 0),
        ]
    );
    assert_eq!(
        scheduled.decisions().len(),
        5,
        "four occurrence boundaries plus the ambient end"
    );
}

#[test]
fn point_and_each_collapse_policy_are_explicit() {
    let source = track_of(1, &[(0, 0, 4), (0, 1, 5)]);
    let collapsed = map(&[(0, 0), (1, 0)]);
    let rejected = schedule(
        format(),
        policy(FrameRounding::NearestTiesLater, CollapsePolicy::Reject),
        &collapsed,
        &source,
    );
    assert!(matches!(rejected, Err(ScheduleError::CollapseRejected { .. })));

    let ordered = schedule(
        format(),
        policy(FrameRounding::NearestTiesLater, CollapsePolicy::Ordered),
        &collapsed,
        &source,
    )
    .expect("ordered collapse");
    assert_eq!(messages(&ordered).len(), 3);
    assert!(ordered.decisions().iter().any(|decision| {
        decision.collision == BoundaryCollision::Point && decision.rounding_choice == RoundingChoice::Exact
    }));

    let expanded = schedule(
        format(),
        policy(
            FrameRounding::NearestTiesLater,
            CollapsePolicy::Expand {
                minimum_frames: NonZeroU32::new(2).expect("nonzero"),
            },
        ),
        &collapsed,
        &track_of(1, &[(0, 1, 5)]),
    )
    .expect("local expansion");
    assert_eq!(
        messages(&expanded),
        vec![(0, MessageKind::Begin, 5), (2, MessageKind::End, 0)]
    );
}

#[test]
fn decisions_are_exact_deterministic_and_tracks_remain_distinguishable() {
    let time_map = TimeMap::new(
        7,
        [
            (position(0, 1), position::<PhysicalTime>(0, 1)),
            (position(1, 1), position::<PhysicalTime>(1, 20)),
        ],
    );
    let policy = policy(FrameRounding::NearestTiesLater, CollapsePolicy::Ordered);
    let first = schedule(format(), policy, &time_map, &track_of(1, &[(0, 1, 8)])).expect("first");
    let again = schedule(format(), policy, &time_map, &track_of(1, &[(0, 1, 8)])).expect("again");
    assert_eq!(first, again);
    assert_eq!(
        first.decisions().get(1).expect("end decision").rounding_choice,
        RoundingChoice::Later
    );
    let unequal = schedule(format(), policy, &time_map, &track_of(1, &[(0, 1, 9)])).expect("unequal");
    assert_ne!(messages(&first), messages(&unequal));
}

#[test]
fn malformed_and_nonmonotone_maps_name_exact_values() {
    let source = track_of(1, &[(0, 1, 1)]);
    let missing = schedule(
        format(),
        policy(FrameRounding::Floor, CollapsePolicy::Ordered),
        &map(&[(0, 0)]),
        &source,
    )
    .expect_err("end is unanswered");
    assert!(missing.to_string().contains("source boundary 1"));

    let reversed = schedule(
        format(),
        policy(FrameRounding::Floor, CollapsePolicy::Ordered),
        &map(&[(0, 2), (1, 1)]),
        &source,
    )
    .expect_err("exact order reverses");
    assert!(reversed.to_string().contains("1/5 > 1/10"));

    let conflict = TimeMap::new(
        7,
        [
            (position(0, 1), position::<PhysicalTime>(0, 1)),
            (position(0, 1), position::<PhysicalTime>(1, 1)),
            (position(1, 1), position::<PhysicalTime>(1, 1)),
        ],
    );
    assert!(matches!(
        schedule(
            format(),
            policy(FrameRounding::Floor, CollapsePolicy::Ordered),
            &conflict,
            &source
        ),
        Err(ScheduleError::ConflictingTimeMap { .. })
    ));

    let irrelevant_conflict = TimeMap::new(
        7,
        [
            (position(0, 1), position::<PhysicalTime>(0, 1)),
            (position(1, 1), position::<PhysicalTime>(1, 10)),
            (position(9, 1), position::<PhysicalTime>(1, 1)),
            (position(9, 1), position::<PhysicalTime>(2, 1)),
        ],
    );
    assert!(
        schedule(
            format(),
            policy(FrameRounding::Floor, CollapsePolicy::Ordered),
            &irrelevant_conflict,
            &source
        )
        .is_ok()
    );
}

#[test]
fn expansion_cannot_give_one_shared_boundary_two_frames() {
    let source = track_of(2, &[(0, 1, 1), (1, 2, 2)]);
    let result = schedule(
        format(),
        policy(
            FrameRounding::Floor,
            CollapsePolicy::Expand {
                minimum_frames: NonZeroU32::new(2).expect("nonzero"),
            },
        ),
        &map(&[(0, 0), (1, 0), (2, 1)]),
        &source,
    );
    assert!(matches!(result, Err(ScheduleError::NonMonotone { .. })));
}

#[test]
fn duplicate_overlay_is_occurrence_local_and_merge_namespaces_handles() {
    let one = track_of(1, &[(0, 1, 3)]);
    let doubled = together(vec![one.clone(), one.clone()]);
    let time_map = map(&[(0, 0), (1, 1)]);
    let policy = policy(FrameRounding::Floor, CollapsePolicy::Ordered);
    let combined = schedule(format(), policy, &time_map, &doubled).expect("combined");
    let separate = schedule(format(), policy, &time_map, &one).expect("separate");
    let merged = merge_schedules(policy, &separate, &separate).expect("hygienic merge");
    assert_eq!(messages(&combined), messages(&merged));
    let handles: Vec<_> = merged
        .batches()
        .flat_map(|(_, batch)| batch.messages())
        .filter(|message| matches!(message, EventMessage::Begin(_, _)))
        .map(EventMessage::handle)
        .collect();
    let first_handle = handles.first().expect("first begin handle");
    let second_handle = handles.get(1).expect("second begin handle");
    assert_eq!(handles.len(), 2);
    assert_ne!(first_handle, second_handle);
    let duplicate_decisions: Vec<_> = combined
        .decisions()
        .iter()
        .filter(|decision| decision.boundary == musa_dsp::BoundaryKind::Start)
        .map(|decision| (decision.physical, decision.frame))
        .collect();
    let first_decision = duplicate_decisions.first().expect("first duplicate decision");
    let second_decision = duplicate_decisions.get(1).expect("second duplicate decision");
    assert_eq!(duplicate_decisions.len(), 2);
    assert_eq!(first_decision, second_decision);
}

#[test]
fn merge_requires_the_complete_policy_not_only_its_version() {
    let source = track_of(1, &[(0, 1, 3)]);
    let floor = policy(FrameRounding::Floor, CollapsePolicy::Ordered);
    let prepared = schedule(format(), floor, &map(&[(0, 0), (1, 1)]), &source).expect("prepared");
    let same_version_different_rounding = policy(FrameRounding::Ceil, CollapsePolicy::Ordered);
    assert!(matches!(
        merge_schedules(same_version_different_rounding, &prepared, &prepared),
        Err(ScheduleError::MergePolicyMismatch { .. })
    ));
}

#[test]
fn additive_follow_is_the_delayed_union_of_separate_tables() {
    let left = track_of(1, &[(0, 1, 1)]);
    let right = track_of(1, &[(0, 1, 2)]);
    let combined = follow(vec![left.clone(), right.clone()]);
    let policy = policy(FrameRounding::Floor, CollapsePolicy::Ordered);
    let scheduled = schedule(format(), policy, &map(&[(0, 0), (1, 1), (2, 2)]), &combined).expect("follow");
    let left = schedule(format(), policy, &map(&[(0, 0), (1, 1)]), &left).expect("left");
    let right = schedule(format(), policy, &map(&[(0, 0), (1, 1)]), &right).expect("right");
    let mut expected = messages(&left);
    expected.extend(
        messages(&right)
            .into_iter()
            .map(|(frame, kind, payload)| (frame + 1, kind, payload)),
    );
    expected.sort_by_key(|(frame, kind, _)| (*frame, *kind != MessageKind::End));
    assert_eq!(messages(&scheduled), expected);
}

#[test]
fn source_countdown_finishes_and_seek_reads_the_target_frame() {
    let scheduled = schedule(
        format(),
        policy(FrameRounding::Floor, CollapsePolicy::Ordered),
        &map(&[(0, 0), (2, 2)]),
        &track_of(2, &[(0, 2, 1)]),
    )
    .expect("schedule");
    let mut source = scheduled.source();
    assert_eq!(source.step().1.len(), 1);
    assert!(source.step().1.is_empty());
    let (state, messages) = source.step();
    assert_eq!((state, messages.len()), (SourceState::Finished, 1));
    for _ in 0..10_000 {
        assert_eq!(source.step(), (SourceState::Finished, &[][..]));
    }
    source.seek(2);
    assert_eq!(source.step().1.len(), 1);
    source.seek(3);
    assert_eq!(source.step(), (SourceState::Finished, &[][..]));
}

#[test]
fn ambient_silence_delays_finished_without_storing_empty_batches() {
    let scheduled = schedule(
        format(),
        policy(FrameRounding::Floor, CollapsePolicy::Ordered),
        &map(&[(3, 3)]),
        &track_of(3, &[]),
    )
    .expect("silent track schedules");
    assert_eq!(scheduled.batches().len(), 0);
    assert_eq!(scheduled.finish_frame(), 3);
    assert_eq!(scheduled.decisions().len(), 1);
    assert_eq!(
        scheduled.decisions().first().expect("track-end decision").boundary,
        musa_dsp::BoundaryKind::TrackEnd
    );
    let mut source = scheduled.source();
    for _ in 0..3 {
        assert_eq!(source.step(), (SourceState::Running, &[][..]));
    }
    assert_eq!(source.step(), (SourceState::Finished, &[][..]));

    let trailing = schedule(
        format(),
        policy(FrameRounding::Floor, CollapsePolicy::Ordered),
        &map(&[(0, 0), (1, 1), (3, 3)]),
        &track_of(3, &[(0, 1, 1)]),
    )
    .expect("trailing silence schedules");
    let mut source = trailing.source();
    assert_eq!(source.step().1.len(), 1);
    let (state, messages) = source.step();
    assert_eq!((state, messages.len()), (SourceState::Running, 1));
    assert_eq!(source.step(), (SourceState::Running, &[][..]));
    assert_eq!(source.step(), (SourceState::Finished, &[][..]));
}

#[test]
fn adversarial_bounds_fail_before_building_an_unbounded_table() {
    let tiny = ScheduleLimits {
        max_frame: 3,
        max_time_map_entries: 2,
        max_occurrences: 1,
        max_messages: 1,
        max_batches: 1,
    };
    let policy = SchedulePolicy::new(
        1,
        FrameRounding::Floor,
        CollapsePolicy::Ordered,
        [MessageKind::End, MessageKind::Point, MessageKind::Begin],
        tiny,
    )
    .expect("policy");
    let too_many = track_of(1, &[(0, 0, 1), (1, 1, 2)]);
    assert!(matches!(
        schedule(format(), policy, &map(&[(0, 0), (1, 1)]), &too_many),
        Err(ScheduleError::ResourceLimit {
            resource: "occurrence",
            ..
        })
    ));
    let far = track_of(1, &[(0, 1, 1)]);
    assert!(matches!(
        schedule(format(), policy, &map(&[(0, 0), (1, 10)]), &far),
        Err(ScheduleError::FrameOutOfRange { .. })
    ));

    let invalid = SchedulePolicy::new(
        1,
        FrameRounding::Floor,
        CollapsePolicy::Ordered,
        [MessageKind::End, MessageKind::End, MessageKind::Begin],
        limits(),
    );
    assert_eq!(invalid, Err(ScheduleError::InvalidMessageOrder));

    let one_point = track_of(0, &[(0, 0, 1)]);
    let result = schedule(format(), policy, &map(&[(0, 0)]), &one_point);
    assert!(result.is_ok(), "a point consumes one message, not two");
}

#[test]
fn handle_debug_does_not_reveal_private_spelling() {
    let scheduled = schedule(
        format(),
        policy(FrameRounding::Floor, CollapsePolicy::Ordered),
        &map(&[(0, 0)]),
        &track_of(0, &[(0, 0, 1)]),
    )
    .expect("point");
    let handle = scheduled
        .batches()
        .next()
        .expect("batch")
        .1
        .messages()
        .first()
        .expect("point")
        .handle();
    assert_eq!(format!("{handle:?}"), "EventHandle(_)");

    let mut names = HashMap::new();
    names.insert(handle.clone(), "voice");
    assert_eq!(names.get(handle), Some(&"voice"));
}
