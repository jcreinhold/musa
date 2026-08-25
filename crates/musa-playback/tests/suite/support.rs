//! Production-path audio fixtures for callback state-machine laws.

#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_dsp::{
    AudioFormat, AudioLimits, AudioOptions, ChannelLayout, CollapsePolicy, FrameRounding, MessageKind, ScheduleLimits,
    SchedulePolicy, prepare_audio,
};
use musa_playback::PreparedPlaybackPlan;
use musa_score::{Tuning, lower_gestures};

const RATE: u32 = 48_000;
const EMPTY: &str = "piece \"silent\" { score { part p { voice v { } } } }";

pub(crate) fn silent_plan(total_frames: u64) -> PreparedPlaybackPlan {
    let compilation = compile(&SourceDocument::new(EMPTY, "silent.musa"), &CompileOptions::default());
    let score = compilation.snapshot().expect("silent fixture compiles");
    let gestures = lower_gestures(score).expect("empty gesture track");
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
            max_frame: total_frames,
            max_time_map_entries: 1,
            max_occurrences: 0,
            max_messages: 0,
            max_batches: 0,
        },
    )
    .expect("schedule policy");
    let audio = prepare_audio(
        &gestures,
        compilation.studio(),
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
            tail_frames: total_frames,
            max_total_frames: total_frames,
        },
    )
    .expect("silent audio prepares");
    PreparedPlaybackPlan::new(audio)
}
