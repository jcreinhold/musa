//! The exact gesture → schedule → one-frame audio benchmark.
//!
//! `with_inputs` prepares a fresh render plan outside the timed region, so
//! the measurement is render time and allocation rather than fixture setup.

#![allow(clippy::expect_used)]

use std::fmt::Write as _;

use musa_compiler::{
    CompileOptions, SourceDocument, checked_standard_instrument_machine, checked_standard_instruments, compile,
    lower_gestures,
};
use musa_dsp::{
    AudioFormat, AudioLimits, AudioOptions, ChannelLayout, CollapsePolicy, FrameRounding, MessageKind, PreparedAudio,
    ScheduleLimits, SchedulePolicy, prepare_execution, render_offline,
};
use musa_score::Tuning;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const SOURCE: &str = include_str!("../../../tests/fixtures/audio-bridge.musa");
const SAMPLE_RATE: u32 = 48_000;

struct PreparationInput {
    gestures: musa_score::GesturePlan,
    instruments: musa_calculus::CheckedSource,
    instrument_machine: musa_score::MachineSpec,
    studio: musa_dsp::StudioExecution,
    options: AudioOptions,
}

fn options() -> AudioOptions {
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
    }
}

fn preparation_input(source: &str, path: &str) -> PreparationInput {
    let compilation = compile(&SourceDocument::new(source, path), &CompileOptions::default());
    let score = compilation.snapshot().expect("fixture compiles");
    let gestures = lower_gestures(score).expect("fixture gestures");
    let instruments = checked_standard_instruments().expect("standard instruments");
    let instrument_machine = checked_standard_instrument_machine().expect("standard instrument machine");
    let studio = musa_dsp::decode_studio_execution(compilation.studio_source().expect("checked studio"))
        .expect("production studio artifact");
    PreparationInput {
        gestures,
        instruments,
        instrument_machine,
        studio,
        options: options(),
    }
}

fn prepare_input(input: &PreparationInput) -> PreparedAudio {
    prepare_execution(
        &input.gestures,
        &input.instruments,
        &input.instrument_machine,
        &input.studio,
        input.options,
    )
    .expect("audio prepares")
}

fn prepare() -> PreparedAudio {
    prepare_input(&preparation_input(SOURCE, "tests/fixtures/audio-bridge.musa"))
}

fn ensemble_source() -> String {
    let mut source = String::from(
        "piece \"Eight-part dense mix\" { tempo 1/4 = 120; meter 4/4; key c major; \
         instrument ensemble conforms note_instrument { implementation graph { \
         oscillator(sine) |> gain(-24 dB) |> output; } } score {",
    );
    for part in 0..8 {
        let _ = write!(source, "part p{part} {{ sound ensemble using neutral; voice v {{");
        for note in 0..64 {
            let pitch = ["c4", "d4", "e4", "f4", "g4", "a4", "b4", "c5"]
                .get(note % 8)
                .copied()
                .expect("pitch cycle index");
            let _ = write!(source, "{pitch}/64 ");
        }
        source.push_str("} } ");
    }
    source.push_str("} }");
    source
}

#[divan::bench(sample_count = 10)]
fn interpret_performance_source(bencher: divan::Bencher<'_, '_>) {
    let compilation = compile(
        &SourceDocument::new(SOURCE, "tests/fixtures/audio-bridge.musa"),
        &CompileOptions::default(),
    );
    let score = compilation.snapshot().expect("fixture compiles");
    bencher.bench(|| lower_gestures(divan::black_box(score)).expect("fixture gestures"));
}

#[divan::bench(sample_count = 30)]
fn prepare_audio_bridge(bencher: divan::Bencher<'_, '_>) {
    bencher
        .with_inputs(|| preparation_input(SOURCE, "tests/fixtures/audio-bridge.musa"))
        .bench_refs(|input| prepare_input(input));
}

#[divan::bench(sample_count = 30)]
fn render_audio_bridge(bencher: divan::Bencher<'_, '_>) {
    bencher
        .with_inputs(prepare)
        .bench_values(|mut audio| render_offline(&mut audio));
}

#[divan::bench(sample_count = 10)]
fn prepare_eight_part_dense_mix(bencher: divan::Bencher<'_, '_>) {
    bencher
        .with_inputs(|| preparation_input(&ensemble_source(), "eight-part-dense-mix.musa"))
        .bench_refs(|input| prepare_input(input));
}

#[divan::bench(sample_count = 10)]
fn render_eight_part_dense_mix(bencher: divan::Bencher<'_, '_>) {
    bencher
        .with_inputs(|| prepare_input(&preparation_input(&ensemble_source(), "eight-part-dense-mix.musa")))
        .bench_values(|mut audio| render_offline(&mut audio));
}

fn main() {
    callback_distribution();
    divan::main();
}

fn callback_distribution() {
    const BLOCK_FRAMES: usize = 128;
    const TRIALS: usize = 1_000;
    let mut audio = prepare_input(&preparation_input(&ensemble_source(), "eight-part-dense-mix.musa"));
    eprintln!(
        "audio workloads: audio-bridge={} frames/12 gestures; eight-part={} frames/512 gestures/8 instances",
        prepare().total_frames(),
        audio.total_frames()
    );
    let mut block = [0.0_f32; BLOCK_FRAMES * 2];
    let mut elapsed = Vec::with_capacity(TRIALS);
    for _ in 0..TRIALS {
        let started = std::time::Instant::now();
        audio.render(&mut block);
        elapsed.push(started.elapsed());
    }
    elapsed.sort_unstable();
    let p95_index = TRIALS.saturating_mul(95).div_ceil(100).saturating_sub(1);
    let p95 = *elapsed.get(p95_index).expect("p95 sample");
    let maximum = *elapsed.last().expect("maximum sample");
    let deadline = std::time::Duration::from_secs_f64(BLOCK_FRAMES as f64 / f64::from(SAMPLE_RATE));
    let misses = elapsed.iter().filter(|sample| **sample > deadline).count();
    eprintln!("callback 8-part/128-frame: p95={p95:?} max={maximum:?} deadline={deadline:?} misses={misses}/{TRIALS}");
}
