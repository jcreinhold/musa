//! Production-path audio fixtures for callback state-machine laws.

#![allow(clippy::expect_used)]

use musa_compiler::{
    CompileOptions, SourceDocument, checked_standard_instrument_machine, checked_standard_instruments, compile,
};
use musa_dsp::{
    AudioFormat, AudioLimits, AudioOptions, ChannelLayout, CollapsePolicy, FrameRounding, MessageKind, ScheduleLimits,
    SchedulePolicy, decode_studio_execution, prepare_execution,
};
use musa_playback::PreparedPlaybackPlan;
use musa_score::{Tuning, lower_gestures};

const RATE: u32 = 48_000;
const EMPTY: &str = "piece \"silent\" { score { part p { voice v { } } } }";

pub(crate) fn silent_plan(total_frames: u64) -> PreparedPlaybackPlan {
    source_plan(EMPTY, total_frames)
}

pub(crate) fn source_plan(source: &str, tail_frames: u64) -> PreparedPlaybackPlan {
    let compilation = compile(
        &SourceDocument::new(source, "playback.musa"),
        &CompileOptions::default(),
    );
    let score = compilation.snapshot().expect("silent fixture compiles");
    let gestures = lower_gestures(score).expect("empty gesture track");
    let instruments = checked_standard_instruments().expect("standard instruments check");
    let instrument_machine = checked_standard_instrument_machine().expect("standard instrument machine checks");
    let studio = decode_studio_execution(compilation.studio_source().expect("studio artifact checks"))
        .expect("studio artifact decodes");
    let format = AudioFormat::new(
        std::num::NonZeroU32::new(RATE).expect("nonzero rate"),
        ChannelLayout::Stereo,
    );
    let schedule = SchedulePolicy::new(
        1,
        FrameRounding::NearestTiesLater,
        CollapsePolicy::Ordered,
        [MessageKind::End, MessageKind::Point, MessageKind::Begin],
        ScheduleLimits {
            max_frame: 10_000_000,
            max_time_map_entries: 2_001,
            max_occurrences: 1_000,
            max_messages: 2_000,
            max_batches: 2_000,
        },
    )
    .expect("schedule policy");
    let audio = prepare_execution(
        &gestures,
        &instruments,
        &instrument_machine,
        &studio,
        AudioOptions {
            format,
            schedule,
            tuning: Tuning::default(),
            render_seed: 0,
            limits: AudioLimits {
                max_primitives: 64,
                max_state_bytes: 64 << 20,
                max_step_work: 1_000_000,
            },
            tail_frames,
            max_total_frames: 10_000_000,
        },
    )
    .expect("silent audio prepares");
    PreparedPlaybackPlan::new(audio)
}
