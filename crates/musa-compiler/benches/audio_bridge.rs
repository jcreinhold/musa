//! The exact gesture → schedule → one-frame audio benchmark.
//!
//! `with_inputs` prepares a fresh render plan outside the timed region, so
//! the measurement is render time and allocation rather than fixture setup.

#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile, lower_gestures};
use musa_dsp::{
    AudioFormat, AudioLimits, AudioOptions, ChannelLayout, CollapsePolicy, FrameRounding, MessageKind, PreparedAudio,
    ScheduleLimits, SchedulePolicy, prepare_audio, render_offline,
};
use musa_score::Tuning;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const SOURCE: &str = include_str!("../../../tests/fixtures/audio-bridge.musa");
const SAMPLE_RATE: u32 = 48_000;

fn prepare() -> PreparedAudio {
    let compilation = compile(
        &SourceDocument::new(SOURCE, "tests/fixtures/audio-bridge.musa"),
        &CompileOptions::default(),
    );
    let score = compilation.snapshot().expect("fixture compiles");
    let gestures = lower_gestures(score).expect("fixture gestures");
    let format = AudioFormat::new(
        std::num::NonZeroU32::new(SAMPLE_RATE).expect("sample rate"),
        ChannelLayout::Stereo,
    );
    let maximum = u64::from(SAMPLE_RATE).saturating_mul(60 * 60);
    let schedule = SchedulePolicy::new(
        1,
        FrameRounding::NearestTiesLater,
        CollapsePolicy::Ordered,
        [MessageKind::End, MessageKind::Point, MessageKind::Begin],
        ScheduleLimits {
            max_frame: maximum,
            max_time_map_entries: 20_001,
            max_occurrences: 10_000,
            max_messages: 20_000,
            max_batches: 20_000,
        },
    )
    .expect("schedule policy");
    prepare_audio(
        &gestures,
        compilation.studio(),
        AudioOptions {
            format,
            schedule,
            tuning: Tuning::default(),
            render_seed: 0x4D55_5341,
            limits: AudioLimits {
                max_primitives: 10_000,
                max_state_bytes: 1 << 30,
                max_step_work: 10_000_000,
            },
            tail_frames: u64::from(SAMPLE_RATE),
            max_total_frames: maximum.saturating_add(u64::from(SAMPLE_RATE)),
        },
    )
    .expect("audio prepares")
}

#[divan::bench(sample_count = 30)]
fn render_audio_bridge(bencher: divan::Bencher<'_, '_>) {
    bencher
        .with_inputs(prepare)
        .bench_values(|mut audio| render_offline(&mut audio));
}

fn main() {
    divan::main();
}
