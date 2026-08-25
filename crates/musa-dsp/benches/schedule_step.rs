//! Scheduled-source hot path. Exact conversion and table construction are
//! deliberately outside the measured loop: only callback work belongs here.
#![allow(clippy::expect_used)]

use std::num::NonZeroU32;

use divan::Bencher;
use musa_dsp::{
    AudioFormat, CollapsePolicy, FrameRounding, MessageKind, ScheduleLimits, SchedulePolicy, TimeMap, schedule,
};
use musa_events::{Duration, Occurrence, PhysicalTime, Position, Span, WrittenTime, track};
use num_rational::Ratio;

fn main() {
    divan::main();
}

#[divan::bench]
fn four_thousand_source_steps(bencher: Bencher<'_, '_>) {
    let at = |value| Position::new(Ratio::from_integer(value));
    let span = Span::new(at(0), at(4_095)).expect("valid span");
    let track = track(
        Duration::<WrittenTime>::from_integer(4_095).expect("duration"),
        vec![Occurrence::new(span, 1u8)],
    )
    .expect("track");
    let map = TimeMap::new(
        1,
        [
            (at(0), Position::<PhysicalTime>::ZERO),
            (at(4_095), Position::<PhysicalTime>::new(Ratio::new(4_095, 48_000))),
        ],
    );
    let policy = SchedulePolicy::new(
        1,
        FrameRounding::NearestTiesLater,
        CollapsePolicy::Ordered,
        [MessageKind::End, MessageKind::Point, MessageKind::Begin],
        ScheduleLimits {
            max_frame: 4_095,
            max_time_map_entries: 2,
            max_occurrences: 1,
            max_messages: 2,
            max_batches: 2,
        },
    )
    .expect("policy");
    let scheduled = schedule(
        AudioFormat::new(
            NonZeroU32::new(48_000).expect("sample rate"),
            musa_dsp::ChannelLayout::Stereo,
        ),
        policy,
        &map,
        &track,
    )
    .expect("schedule");
    bencher.bench_local(move || {
        let mut source = scheduled.source();
        for _ in 0..4_096 {
            divan::black_box(source.step());
        }
    });
}
