//! What reading stems beside the mix actually costs.
//!
//! Prompt 211 requires the choice between one traversal and one render per
//! output to be measured rather than asserted, so both are here over the same
//! generated many-part/many-return workload. `with_inputs` prepares outside
//! the timed region, which is what keeps the comparison about rendering.

#![allow(clippy::expect_used)]
#![allow(clippy::arithmetic_side_effects)]

use std::fmt::Write as _;

use musa_compiler::{
    CompileOptions, SourceDocument, checked_standard_instrument_machine, checked_standard_instruments, compile,
    lower_gestures,
};
use musa_dsp::{
    AudioFormat, AudioLimits, AudioOptions, ChannelLayout, CollapsePolicy, FrameRounding, MessageKind, PreparedAudio,
    ScheduleLimits, SchedulePolicy, prepare_execution, render_offline, render_offline_multitrack,
};
use musa_score::Tuning;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const SAMPLE_RATE: u32 = 48_000;
const PARTS: usize = 8;
const RETURNS: usize = 4;

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

/// Eight parts through four shared returns: twelve taps and one mix.
fn routed_source() -> String {
    let mut source = String::from("piece \"Routed mix\" { tempo 1/4 = 120; meter 4/4; key c major; score {");
    for part in 0..PARTS {
        let _ = write!(source, "part p{part} {{ voice v {{");
        for note in 0..64 {
            let pitch = ["c4", "d4", "e4", "f4", "g4", "a4", "b4", "c5"]
                .get(note % 8)
                .copied()
                .expect("pitch cycle index");
            let _ = write!(source, "{pitch}/64 ");
        }
        source.push_str("} } ");
    }
    source.push_str("} studio { patch tone { oscillator(sine) |> gain(-24 dB) |> output; } ");
    for bus in 0..RETURNS {
        let _ = write!(
            source,
            "bus r{bus} {{ reverb(room: 0.{}, damping: 0.5, mix: 0.3); }} ",
            5 + bus
        );
    }
    for part in 0..PARTS {
        let _ = write!(
            source,
            "assign p{part} -> tone; route p{part} -> master; send p{part} -> r{} at -15 dB; ",
            part % RETURNS
        );
    }
    for bus in 0..RETURNS {
        let _ = write!(source, "route r{bus} -> master; ");
    }
    source.push('}');
    source.push('}');
    source
}

fn prepare(source: &str) -> PreparedAudio {
    let compilation = compile(
        &SourceDocument::new(source, "routed-mix.musa"),
        &CompileOptions::default(),
    );
    let score = compilation.snapshot().expect("fixture compiles");
    let gestures = lower_gestures(score).expect("fixture gestures");
    let instruments = checked_standard_instruments().expect("standard instruments");
    let machine = checked_standard_instrument_machine().expect("standard instrument machine");
    let studio = musa_dsp::decode_studio_execution(compilation.studio_source().expect("checked studio"))
        .expect("production studio artifact");
    prepare_execution(&gestures, &instruments, &machine, &studio, options()).expect("audio prepares")
}

#[divan::bench(sample_count = 10)]
fn render_mix_only(bencher: divan::Bencher<'_, '_>) {
    let source = routed_source();
    bencher
        .with_inputs(|| prepare(&source))
        .bench_values(|mut audio| render_offline(&mut audio));
}

#[divan::bench(sample_count = 10)]
fn render_mix_and_stems_in_one_traversal(bencher: divan::Bencher<'_, '_>) {
    let source = routed_source();
    bencher
        .with_inputs(|| prepare(&source))
        .bench_values(|mut audio| render_offline_multitrack(&mut audio));
}

/// The alternative the design had to rule out: render the whole graph once
/// per output. Preparation is still outside the timed region, so this is the
/// honest lower bound for the separate-render shape rather than a straw man.
#[divan::bench(sample_count = 10)]
fn render_one_pass_per_output(bencher: divan::Bencher<'_, '_>) {
    let source = routed_source();
    let outputs = prepare(&source).taps().len().saturating_add(1);
    bencher
        .with_inputs(|| (0..outputs).map(|_| prepare(&source)).collect::<Vec<_>>())
        .bench_values(|passes| {
            passes
                .into_iter()
                .map(|mut audio| render_offline(&mut audio))
                .collect::<Vec<_>>()
        });
}

fn main() {
    let audio = prepare(&routed_source());
    eprintln!(
        "stem workload: {} parts, {} returns, {} taps, {} frames",
        PARTS,
        RETURNS,
        audio.taps().len(),
        audio.total_frames()
    );
    divan::main();
}
